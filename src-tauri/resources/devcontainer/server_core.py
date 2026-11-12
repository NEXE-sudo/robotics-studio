from collections import OrderedDict, deque
from dataclasses import dataclass
import base64
import hmac
import json
import math
import os
import queue
import re
import stat
import tempfile
import threading
import time
from pathlib import Path
import xml.etree.ElementTree as ET

AUTH_PROTOCOL_VERSION = 1
MAX_EVENT_QUEUE_SIZE = 256
MAX_EVENT_HISTORY_EVENTS = 256
MAX_EVENT_HISTORY_BYTES = 4 * 1024 * 1024
MAX_LINEAR_VELOCITY = 0.5
MAX_ANGULAR_VELOCITY = 1.0
MAX_COMMAND_AGE_SECONDS = 1.0
COMMAND_TIMEOUT_SECONDS = 0.5
ROS_TOPIC_PATTERN = re.compile(r"/[A-Za-z_][A-Za-z0-9_]*(/[A-Za-z_][A-Za-z0-9_]*)*")
MAX_CONFIG_LINEAR_VELOCITY = 0.5
MAX_CONFIG_ANGULAR_VELOCITY = 1.0
MAX_CONFIG_LINEAR_ACCELERATION = 2.0
MAX_CONFIG_ANGULAR_ACCELERATION = 4.0
ALLOWED_MOTION_MODES = frozenset(("planar", "full_twist"))
MAX_WORKSPACE_PACKAGES = 1000
MAX_UNDERLAY_PACKAGES = 2000
MAX_PACKAGE_SCAN_FILES = 5000
MAX_PACKAGE_DISCOVERY_FILES = 250
ACTION_SEND_GOAL_SUFFIX = "/_action/send_goal"
PACKAGE_METADATA_LIMIT = 1024 * 1024
_PACKAGE_PRUNED_DIRECTORIES = frozenset(
    (".git", ".robotics-studio", "__pycache__", "build", "install", "log", "node_modules")
)


def _package_directory_is_within(root, path):
    try:
        path.resolve(strict=True).relative_to(root)
        return True
    except (OSError, ValueError):
        return False


def _workspace_package_directories(workspace):
    root = Path(workspace)
    if root.is_symlink() or not root.is_dir():
        raise ValueError("Selected ROS workspace is missing or is not a directory")
    root = root.resolve(strict=True)
    found = []

    root_manifest = root / "package.xml"
    if root_manifest.is_file() and not root_manifest.is_symlink():
        found.append(root)

    source = root / "src"
    if source.is_dir() and not source.is_symlink():
        def on_error(error):
            raise error

        for current, directories, files in os.walk(
            source, topdown=True, followlinks=False, onerror=on_error
        ):
            current_path = Path(current)
            directories[:] = sorted(
                name
                for name in directories
                if name not in _PACKAGE_PRUNED_DIRECTORIES
                and not (current_path / name).is_symlink()
            )
            if "package.xml" in files:
                manifest = current_path / "package.xml"
                if not manifest.is_symlink() and _package_directory_is_within(root, current_path):
                    found.append(current_path)
                    directories[:] = []
                    if len(found) > MAX_WORKSPACE_PACKAGES:
                        raise ValueError("ROS workspace contains too many packages")

    install = root / "install"
    if install.is_dir() and not install.is_symlink():
        share_roots = [install / "share"]
        try:
            prefixes = sorted(
                child
                for child in install.iterdir()
                if child.is_dir()
                and not child.is_symlink()
                and child.name not in _PACKAGE_PRUNED_DIRECTORIES
            )
        except OSError as error:
            raise ValueError(f"Could not inspect ROS workspace install directory: {error}") from error
        share_roots.extend(prefix / "share" for prefix in prefixes)
        for share_root in share_roots:
            if not share_root.is_dir() or share_root.is_symlink():
                continue
            try:
                packages = sorted(share_root.iterdir())
            except OSError as error:
                raise ValueError(f"Could not inspect ROS package index directory: {error}") from error
            for package_share in packages:
                if (
                    package_share.is_dir()
                    and not package_share.is_symlink()
                    and (package_share / "package.xml").is_file()
                    and not (package_share / "package.xml").is_symlink()
                    and _package_directory_is_within(root, package_share)
                ):
                    found.append(package_share)
                    if len(found) > MAX_WORKSPACE_PACKAGES:
                        raise ValueError("ROS workspace contains too many packages")

    return root, found


def _parse_package_manifest(package_directory):
    manifest_path = package_directory / "package.xml"
    package = {
        "name": package_directory.name,
        "version": None,
        "description": None,
        "package_type": "unknown",
        "dependencies": [],
        "metadata_error": None,
    }
    try:
        if manifest_path.stat().st_size > PACKAGE_METADATA_LIMIT:
            raise ValueError("package.xml exceeds the metadata size limit")
        contents = manifest_path.read_bytes()
        if b"<!DOCTYPE" in contents.upper() or b"<!ENTITY" in contents.upper():
            raise ValueError("package.xml must not declare a DTD or entities")
        root = ET.fromstring(contents)
        if root.tag.rsplit("}", 1)[-1] != "package":
            raise ValueError("package.xml root element must be <package>")

        name = (root.findtext("name") or "").strip()
        if name:
            package["name"] = name
        package["version"] = (root.findtext("version") or "").strip() or None
        package["description"] = (root.findtext("description") or "").strip() or None
        missing_fields = [
            field
            for field, value in (
                ("name", name),
                ("version", package["version"]),
                ("description", package["description"]),
            )
            if not value
        ]
        if missing_fields:
            package["metadata_error"] = (
                "package.xml is missing required fields: " + ", ".join(missing_fields)
            )[:256]

        build_type = (root.findtext("export/build_type") or "").strip()
        build_tools = {
            (element.text or "").strip()
            for tag in ("buildtool_depend", "buildtool_export_depend")
            for element in root.findall(tag)
            if (element.text or "").strip()
        }
        if build_type in ("ament_cmake", "ament_python", "cmake"):
            package["package_type"] = build_type
        elif "ament_cmake" in build_tools:
            package["package_type"] = "ament_cmake"
        elif "ament_python" in build_tools:
            package["package_type"] = "ament_python"

        dependencies = {
            (element.text or "").strip()
            for tag in (
                "depend",
                "build_depend",
                "buildtool_depend",
                "buildtool_export_depend",
                "exec_depend",
                "test_depend",
                "build_export_depend",
                "doc_depend",
                "group_depend",
            )
            for element in root.findall(tag)
            if (element.text or "").strip()
        }
        package["dependencies"] = sorted(dependencies)
    except (OSError, ET.ParseError, ValueError) as error:
        package["metadata_error"] = str(error)[:256]
    return package


def _package_files(package_directory):
    launch_files = []
    config_files = []
    files_seen = 0

    def on_error(error):
        raise error

    for current, directories, files in os.walk(
        package_directory, topdown=True, followlinks=False, onerror=on_error
    ):
        current_path = Path(current)
        directories[:] = sorted(
            name
            for name in directories
            if name not in _PACKAGE_PRUNED_DIRECTORIES
            and not (current_path / name).is_symlink()
        )
        for name in sorted(files):
            candidate = current_path / name
            if candidate.is_symlink() or not candidate.is_file():
                continue
            files_seen += 1
            if files_seen > MAX_PACKAGE_SCAN_FILES:
                raise ValueError("ROS package contains too many files to inspect")
            relative = candidate.relative_to(package_directory).as_posix()
            if name.endswith((".launch.py", ".launch.xml")) or name.endswith(".launch"):
                if len(launch_files) < MAX_PACKAGE_DISCOVERY_FILES:
                    launch_files.append(relative)
            if candidate.suffix.lower() in (".yaml", ".yml", ".json", ".config", ".ini"):
                if len(config_files) < MAX_PACKAGE_DISCOVERY_FILES:
                    config_files.append(relative)

    return sorted(launch_files), sorted(config_files)


def _package_executables(workspace, package_name):
    install = workspace / "install"
    if install.is_symlink() or not install.is_dir():
        return []
    candidates = (install / "lib" / package_name,)
    try:
        prefixes = sorted(
            child
            for child in install.iterdir()
            if child.is_dir()
            and not child.is_symlink()
            and child.name not in _PACKAGE_PRUNED_DIRECTORIES
        )
    except (FileNotFoundError, NotADirectoryError):
        prefixes = ()
    candidates += tuple(prefix / "lib" / package_name for prefix in prefixes)

    executables = set()
    for directory in candidates:
        if (
            not directory.is_dir()
            or directory.is_symlink()
            or directory.parent.is_symlink()
            or not _package_directory_is_within(workspace, directory)
        ):
            continue
        try:
            entries = sorted(directory.iterdir())
        except OSError as error:
            raise ValueError(f"Could not inspect executables for package {package_name}: {error}") from error
        for entry in entries:
            if (
                not entry.is_symlink()
                and entry.is_file()
                and os.access(entry, os.X_OK)
            ):
                executables.add(entry.name)
    return sorted(executables)[:MAX_PACKAGE_DISCOVERY_FILES]


def discover_workspace_packages(workspace="/workspace"):
    """Return bounded package metadata found only inside the selected workspace."""
    root, package_directories = _workspace_package_directories(workspace)
    result_by_name = {}
    for package_directory in package_directories:
        package = _parse_package_manifest(package_directory)
        name = package["name"]
        existing = result_by_name.get(name)
        relative_directory = package_directory.relative_to(root)
        is_source_package = not relative_directory.parts or relative_directory.parts[0] == "src"
        if existing and (not is_source_package or existing["is_source"]):
            continue

        launch_files, config_files = _package_files(package_directory)
        package.update(
            {
                "path": relative_directory.as_posix(),
                "executables": _package_executables(root, name),
                "launch_files": launch_files,
                "config_files": config_files,
                "is_source": is_source_package,
            }
        )
        result_by_name[name] = package

    return [
        {key: value for key, value in package.items() if key != "is_source"}
        for _, package in sorted(result_by_name.items())
    ]


def _discover_underlay_packages(underlay_roots):
    packages = {}
    for prefix in underlay_roots:
        root = Path(prefix)
        if root.is_symlink() or not root.is_dir():
            raise ValueError(f"ROS underlay prefix is missing or unsafe: {root}")
        root = root.resolve(strict=True)
        share = root / "share"
        if not share.is_dir() or share.is_symlink():
            raise ValueError(f"ROS underlay package index is missing or unsafe: {share}")
        try:
            package_directories = sorted(share.iterdir())
        except OSError as error:
            raise ValueError(f"Could not inspect ROS underlay {share}: {error}") from error
        for package_directory in package_directories:
            manifest = package_directory / "package.xml"
            if (
                not package_directory.is_dir()
                or package_directory.is_symlink()
                or manifest.is_symlink()
                or not manifest.is_file()
                or not _package_directory_is_within(root, package_directory)
            ):
                continue
            package = _parse_package_manifest(package_directory)
            name = package["name"]
            if not name or name in packages:
                continue
            package.update(
                {
                    "path": "",
                    "executables": [],
                    "launch_files": [],
                    "config_files": [],
                    "source": "underlay",
                }
            )
            packages[name] = package
            if len(packages) > MAX_UNDERLAY_PACKAGES:
                raise ValueError(
                    f"ROS underlay contains more than {MAX_UNDERLAY_PACKAGES} packages"
                )
    return packages


def discover_ros_packages(
    workspace="/workspace", underlay_roots=("/opt/ros/jazzy",)
):
    """Return bounded workspace and ROS underlay packages with dependency resolution."""
    workspace_packages = {
        package["name"]: {**package, "source": "workspace"}
        for package in discover_workspace_packages(workspace)
    }
    underlay_packages = _discover_underlay_packages(underlay_roots)
    package_sources = {
        name: "workspace" for name in workspace_packages
    }
    package_sources.update(
        {
            name: "underlay"
            for name in underlay_packages
            if name not in workspace_packages
        }
    )
    resolved_packages = {**underlay_packages, **workspace_packages}
    for package in resolved_packages.values():
        package["dependency_sources"] = {
            dependency: package_sources.get(dependency, "missing")
            for dependency in package["dependencies"]
        }
    return [resolved_packages[name] for name in sorted(resolved_packages)]


def action_from_send_goal_service(service_name, service_type):
    """Return an action name/type only for canonical ROS 2 send-goal endpoints."""
    if not service_name.endswith(ACTION_SEND_GOAL_SUFFIX):
        return None
    action_type, separator, suffix = service_type.rpartition("_SendGoal")
    if not separator or suffix or not action_type:
        return None
    return service_name[: -len(ACTION_SEND_GOAL_SUFFIX)], action_type


class EventOverflow(Exception):
    pass


class EventPersistenceError(Exception):
    pass


class SafetyController:
    def __init__(self, stop_callback, storage_path=None):
        self._stop_callback = stop_callback
        self._state = "READY"
        self._detail = "Motion commands are permitted"
        self._updated_at_ns = time.time_ns()
        self._events = []
        self._lock = threading.RLock()
        self._storage_path = Path(storage_path) if storage_path else None
        if self._storage_path and self._storage_path.exists():
            if self._storage_path.is_symlink() or self._storage_path.stat().st_size > 64 * 1024:
                raise ValueError("Safety state file is unsafe or too large")
            try:
                stored = json.loads(self._storage_path.read_text(encoding="utf-8"))
                if not isinstance(stored, dict):
                    raise ValueError("Invalid persisted safety state")
                stored_state = stored["state"]
                if stored_state not in ("READY", "ACTIVE", "RESET_PENDING", "FAULT"):
                    raise ValueError("Invalid persisted safety state")
                self._state = stored_state
                self._detail = str(stored.get("detail", "Restored safety state"))[:512]
                self._updated_at_ns = int(stored.get("updated_at_ns", self._updated_at_ns))
                events = stored.get("events", [])
                if not isinstance(events, list) or len(events) > 128:
                    raise ValueError("Invalid persisted safety event history")
                if any(not isinstance(item, dict) for item in events):
                    raise ValueError("Invalid persisted safety event")
                self._events = []
                for item in events:
                    event = item.get("event")
                    timestamp_ns = item.get("timestamp_ns")
                    event_state = item.get("state")
                    if (
                        not isinstance(event, str)
                        or not isinstance(timestamp_ns, int)
                        or isinstance(timestamp_ns, bool)
                        or event_state not in ("READY", "ACTIVE", "RESET_PENDING", "FAULT")
                    ):
                        raise ValueError("Invalid persisted safety event")
                    self._events.append(
                        {
                            "event": event[:64],
                            "timestamp_ns": timestamp_ns,
                            "state": event_state,
                        }
                    )
                if self._state in ("ACTIVE", "RESET_PENDING"):
                    self._state = "ACTIVE"
                    self._detail = "Emergency stop restored as latched after service restart"
                elif self._state == "FAULT":
                    self._detail = "Safety fault restored after service restart"
            except (OSError, UnicodeError, json.JSONDecodeError, KeyError, TypeError) as error:
                raise ValueError("Safety state file is invalid") from error

    def _persist(self):
        if self._storage_path is None:
            return
        parent = self._storage_path.parent
        parent.mkdir(mode=0o700, parents=True, exist_ok=True)
        if parent.is_symlink():
            raise OSError("Safety state directory must not be a symlink")
        descriptor, temporary = tempfile.mkstemp(
            prefix=".safety-state-", suffix=".tmp", dir=parent
        )
        try:
            with os.fdopen(descriptor, "w", encoding="utf-8") as file:
                json.dump(
                    {
                        "state": self._state,
                        "detail": self._detail,
                        "updated_at_ns": self._updated_at_ns,
                        "events": self._events[-128:],
                    },
                    file,
                )
                file.flush()
                os.fsync(file.fileno())
            os.replace(temporary, self._storage_path)
        except Exception:
            try:
                os.unlink(temporary)
            except FileNotFoundError:
                pass
            raise

    def _record_event(self, event):
        self._events.append(
            {
                "event": event,
                "timestamp_ns": self._updated_at_ns,
                "state": self._state,
            }
        )
        self._events = self._events[-128:]

    def history(self):
        with self._lock:
            return tuple(dict(event) for event in self._events)

    def snapshot(self):
        with self._lock:
            return self._state, self._detail, self._updated_at_ns

    def require_motion_permitted(self):
        with self._lock:
            if self._state != "READY":
                raise ValueError(f"Motion is blocked while safety state is {self._state}")

    def enforce_latched_stop(self):
        with self._lock:
            if self._state == "READY":
                return
            try:
                self._stop_callback()
            except Exception as error:
                self._state = "FAULT"
                self._detail = "Could not reassert emergency stop after service restart"
                self._updated_at_ns = time.time_ns()
                self._record_event("STOP_ENFORCEMENT_FAILED")
                try:
                    self._persist()
                except Exception:
                    self._detail += "; fault state could not be persisted"
                raise RuntimeError(self._detail) from error
            self._updated_at_ns = time.time_ns()
            self._record_event("STOP_REASSERTED")
            try:
                self._persist()
            except OSError as error:
                self._state = "FAULT"
                self._detail = "Could not persist reasserted emergency stop"
                self._updated_at_ns = time.time_ns()
                self._record_event("STOP_ENFORCEMENT_FAILED")
                raise RuntimeError(self._detail) from error

    def latch_fault(self, detail):
        with self._lock:
            self._state = "FAULT"
            self._detail = str(detail)[:512]
            self._updated_at_ns = time.time_ns()
            self._record_event("FAULT_LATCHED")
            try:
                self._stop_callback()
            except Exception:
                self._detail = "Safety fault latched; emergency stop enforcement also failed"
                self._record_event("STOP_ENFORCEMENT_FAILED")
            self._persist()

    def run_if_motion_permitted(self, operation):
        with self._lock:
            if self._state != "READY":
                raise ValueError(f"Motion is blocked while safety state is {self._state}")
            return operation()

    def activate(self):
        with self._lock:
            if self._state == "FAULT":
                raise RuntimeError(f"Emergency stop is faulted: {self._detail}")
            self._state = "ACTIVE"
            self._detail = "Emergency stop is latched"
            self._updated_at_ns = time.time_ns()
            self._record_event("ACTIVATED")
            try:
                self._persist()
                self._stop_callback()
            except Exception as error:
                self._state = "FAULT"
                self._detail = "Could not persist or publish emergency stop"
                self._updated_at_ns = time.time_ns()
                self._record_event("ACTIVATION_FAILED")
                try:
                    self._stop_callback()
                except Exception:
                    self._detail += "; retrying the zero command also failed"
                    self._record_event("STOP_RETRY_FAILED")
                try:
                    self._persist()
                except Exception:
                    self._detail += "; fault state could not be persisted"
                raise RuntimeError(self._detail) from error
            return self.snapshot()

    def reset(self):
        with self._lock:
            if self._state != "ACTIVE":
                raise ValueError("Reset is allowed only from the latched ACTIVE state")
            self._state = "RESET_PENDING"
            self._detail = "Applying stop before reset"
            self._updated_at_ns = time.time_ns()
            self._record_event("RESET_REQUESTED")
            try:
                self._persist()
                self._stop_callback()
            except Exception as error:
                self._state = "FAULT"
                self._detail = "Could not confirm stop during reset"
                self._updated_at_ns = time.time_ns()
                self._record_event("RESET_FAILED")
                try:
                    self._persist()
                except Exception:
                    self._detail += "; fault state could not be persisted"
                raise RuntimeError(self._detail) from error
            self._state = "READY"
            self._detail = "Reset confirmed; explicit new motion command required"
            self._updated_at_ns = time.time_ns()
            self._record_event("RESET_CONFIRMED")
            try:
                self._persist()
            except Exception as error:
                self._state = "FAULT"
                self._detail = "Could not persist reset; emergency stop remains faulted"
                self._updated_at_ns = time.time_ns()
                self._record_event("RESET_PERSISTENCE_FAILED")
                raise RuntimeError(self._detail) from error
            return self.snapshot()


@dataclass(frozen=True)
class RobotMotionConfig:
    topic_name: str
    max_linear_velocity: float = MAX_LINEAR_VELOCITY
    max_angular_velocity: float = MAX_ANGULAR_VELOCITY
    max_linear_acceleration: float = 1.0
    max_angular_acceleration: float = 2.0
    command_lease_ms: int = 500
    command_timeout_ms: int = 1000
    permitted_motion_modes: tuple = ("planar",)

    def validate(self):
        if len(self.topic_name) > 200 or not ROS_TOPIC_PATTERN.fullmatch(self.topic_name):
            raise ValueError("Invalid robot command topic")
        for name, value, maximum in (
            ("max_linear_velocity", self.max_linear_velocity, MAX_CONFIG_LINEAR_VELOCITY),
            ("max_angular_velocity", self.max_angular_velocity, MAX_CONFIG_ANGULAR_VELOCITY),
            ("max_linear_acceleration", self.max_linear_acceleration, MAX_CONFIG_LINEAR_ACCELERATION),
            ("max_angular_acceleration", self.max_angular_acceleration, MAX_CONFIG_ANGULAR_ACCELERATION),
        ):
            if (
                isinstance(value, bool)
                or not isinstance(value, (int, float))
                or not math.isfinite(value)
                or value <= 0
                or value > maximum
            ):
                raise ValueError(f"{name} must be positive and no greater than {maximum}")
        if (
            not isinstance(self.command_lease_ms, int)
            or isinstance(self.command_lease_ms, bool)
            or not 100 <= self.command_lease_ms <= 2000
        ):
            raise ValueError("command_lease_ms must be between 100 and 2000")
        if (
            not isinstance(self.command_timeout_ms, int)
            or isinstance(self.command_timeout_ms, bool)
            or not self.command_lease_ms <= self.command_timeout_ms <= 5000
        ):
            raise ValueError("command_timeout_ms must be at least the lease and no greater than 5000")
        if (
            not self.permitted_motion_modes
            or not isinstance(self.permitted_motion_modes, (list, tuple))
            or any(not isinstance(mode, str) for mode in self.permitted_motion_modes)
            or len(set(self.permitted_motion_modes)) != len(self.permitted_motion_modes)
            or any(mode not in ALLOWED_MOTION_MODES for mode in self.permitted_motion_modes)
        ):
            raise ValueError("permitted_motion_modes must contain supported unique modes")
        return self


class RobotConfigRegistry:
    def __init__(self, storage_path=None):
        self._configs = {}
        self._last_commands = {}
        self._lock = threading.Lock()
        self._storage_path = Path(storage_path) if storage_path else None
        if self._storage_path and self._storage_path.exists():
            if self._storage_path.is_symlink() or self._storage_path.stat().st_size > 1024 * 1024:
                raise ValueError("Robot configuration file is unsafe or too large")
            try:
                items = json.loads(self._storage_path.read_text(encoding="utf-8"))
            except (OSError, UnicodeError, json.JSONDecodeError) as error:
                raise ValueError("Robot configuration file is invalid") from error
            if not isinstance(items, list) or len(items) > 128:
                raise ValueError("Robot configuration file has an invalid shape")
            for item in items:
                if not isinstance(item, dict):
                    raise ValueError("Robot configuration entry is invalid")
                item["permitted_motion_modes"] = tuple(item.get("permitted_motion_modes", ()))
                config = RobotMotionConfig(**item).validate()
                self._configs[config.topic_name] = config

    def _persist(self, configs):
        if self._storage_path is None:
            return
        parent = self._storage_path.parent
        parent.mkdir(mode=0o700, parents=True, exist_ok=True)
        if parent.is_symlink():
            raise OSError("Robot configuration directory must not be a symlink")
        serialized = [
            {
                **config.__dict__,
                "permitted_motion_modes": list(config.permitted_motion_modes),
            }
            for config in configs.values()
        ]
        descriptor, temporary = tempfile.mkstemp(
            prefix=".robot-config-", suffix=".tmp", dir=parent
        )
        try:
            with os.fdopen(descriptor, "w", encoding="utf-8") as file:
                json.dump(serialized, file, allow_nan=False)
                file.flush()
                os.fsync(file.fileno())
            os.replace(temporary, self._storage_path)
        except Exception:
            try:
                os.unlink(temporary)
            except FileNotFoundError:
                pass
            raise

    def get(self, topic_name):
        with self._lock:
            return self._configs.get(topic_name, RobotMotionConfig(topic_name).validate())

    def set(self, config):
        config.validate()
        with self._lock:
            updated = dict(self._configs)
            if config.topic_name not in updated and len(updated) >= 128:
                raise ValueError("Robot configuration capacity reached")
            updated[config.topic_name] = config
            self._persist(updated)
            self._configs = updated
            self._last_commands.pop(config.topic_name, None)
        return config

    def reset(self, topic_name):
        default = RobotMotionConfig(topic_name).validate()
        with self._lock:
            updated = dict(self._configs)
            updated.pop(topic_name, None)
            self._persist(updated)
            self._configs = updated
            self._last_commands.pop(topic_name, None)
        return default

    def validate_command(self, request, now_ns=None, monotonic_now=None, record=True):
        topic_name = request.topic_name
        config = self.get(topic_name)
        linear = (request.linear_x, request.linear_y, request.linear_z)
        angular = (request.angular_x, request.angular_y, request.angular_z)
        if not all(math.isfinite(value) for value in (*linear, *angular)):
            raise ValueError("Velocity components must be finite")
        if math.hypot(*linear) > config.max_linear_velocity:
            raise ValueError("Linear velocity magnitude exceeds the robot limit")
        if math.hypot(*angular) > config.max_angular_velocity:
            raise ValueError("Angular velocity magnitude exceeds the robot limit")
        if "full_twist" not in config.permitted_motion_modes and any(
            value != 0 for value in (request.linear_y, request.linear_z, request.angular_x, request.angular_y)
        ):
            raise ValueError("Robot configuration permits planar motion only")

        current_time = time.monotonic() if monotonic_now is None else monotonic_now
        with self._lock:
            previous = self._last_commands.get(topic_name)
            if previous is not None:
                previous_linear, previous_angular, previous_time = previous
                elapsed = max(current_time - previous_time, 1e-6)
                linear_delta = math.sqrt(sum((a - b) ** 2 for a, b in zip(linear, previous_linear)))
                angular_delta = math.sqrt(sum((a - b) ** 2 for a, b in zip(angular, previous_angular)))
                if linear_delta > config.max_linear_acceleration * elapsed:
                    raise ValueError("Linear acceleration exceeds the robot limit")
                if angular_delta > config.max_angular_acceleration * elapsed:
                    raise ValueError("Angular acceleration exceeds the robot limit")
            if record:
                self._last_commands[topic_name] = (linear, angular, current_time)
        return config

    def record_command(self, request, monotonic_now=None):
        current_time = time.monotonic() if monotonic_now is None else monotonic_now
        with self._lock:
            self._last_commands[request.topic_name] = (
                (request.linear_x, request.linear_y, request.linear_z),
                (request.angular_x, request.angular_y, request.angular_z),
                current_time,
            )

    def reset_command(self, topic_name):
        with self._lock:
            self._last_commands.pop(topic_name, None)

    def reset_all_commands(self):
        with self._lock:
            self._last_commands.clear()


class EventHub:
    def __init__(
        self,
        max_queue_size=MAX_EVENT_QUEUE_SIZE,
        history_path=None,
        event_encoder=None,
        event_decoder=None,
        history_capacity=MAX_EVENT_HISTORY_EVENTS,
        history_max_bytes=MAX_EVENT_HISTORY_BYTES,
    ):
        if max_queue_size < 1:
            raise ValueError("max_queue_size must be positive")
        if history_capacity < 1 or history_max_bytes < 1:
            raise ValueError("event history limits must be positive")
        if history_path is not None and not callable(event_encoder):
            raise ValueError("event_encoder is required for persistent event history")
        if history_path is not None and not callable(event_decoder):
            raise ValueError("event_decoder is required for persistent event history")
        self._max_queue_size = max_queue_size
        self._subscribers = set()
        self._lock = threading.Lock()
        self._latest_state = {}
        self._sequence = 0
        self._historical_dropped_telemetry = 0
        self._historical_critical_overflows = 0
        self._history_path = Path(history_path) if history_path else None
        self._event_encoder = event_encoder
        self._event_decoder = event_decoder
        self._history_capacity = history_capacity
        self._history_max_bytes = history_max_bytes
        self._history = []
        self._persistence_error = None
        if self._history_path is not None:
            self._load_history()

    def _load_history(self):
        path = self._history_path
        parent = path.parent
        directory_fd = None
        try:
            try:
                directory_fd = os.open(
                    parent,
                    os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
                )
            except FileNotFoundError:
                return
            except NotADirectoryError as error:
                raise ValueError("Event history directory is unsafe") from error
            try:
                descriptor = os.open(
                    path.name,
                    os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC,
                    dir_fd=directory_fd,
                )
            except FileNotFoundError:
                return
            with os.fdopen(descriptor, "rb") as file:
                metadata = os.fstat(file.fileno())
                if (
                    not stat.S_ISREG(metadata.st_mode)
                    or metadata.st_size > self._history_max_bytes
                ):
                    raise ValueError("Event history file is invalid or too large")
                contents = file.read(self._history_max_bytes + 1)
            if len(contents) > self._history_max_bytes:
                raise ValueError("Event history file is invalid or too large")
            stored = json.loads(contents.decode("utf-8"))
            if not isinstance(stored, dict) or stored.get("version") != 1:
                raise ValueError("Unsupported event history format")
            sequence = stored.get("last_sequence")
            events = stored.get("events")
            if (
                not isinstance(sequence, int)
                or isinstance(sequence, bool)
                or sequence < 0
                or not isinstance(events, list)
                or len(events) > self._history_capacity
            ):
                raise ValueError("Event history metadata is invalid")
            previous = 0
            for item in events:
                if not isinstance(item, dict):
                    raise ValueError("Event history entry is invalid")
                item_sequence = item.get("sequence")
                payload = item.get("payload")
                if (
                    not isinstance(item_sequence, int)
                    or isinstance(item_sequence, bool)
                    or item_sequence <= previous
                    or item_sequence > sequence
                    or not isinstance(payload, str)
                ):
                    raise ValueError("Event history entry is invalid")
                encoded = base64.b64decode(payload, validate=True)
                event = self._event_decoder(encoded)
                if getattr(event, "sequence", item_sequence) != item_sequence:
                    raise ValueError("Event history sequence does not match its payload")
                self._history.append((item_sequence, payload, event))
                previous = item_sequence
            if sequence > 0 and (
                not self._history or self._history[-1][0] != sequence
            ):
                raise ValueError("Event history sequence is inconsistent")
            self._sequence = sequence
        except (OSError, UnicodeError, json.JSONDecodeError, TypeError, KeyError) as error:
            raise ValueError("Event history file could not be loaded safely") from error
        finally:
            if directory_fd is not None:
                os.close(directory_fd)

    def _persist_history(self, sequence, history):
        path = self._history_path
        parent = path.parent
        try:
            parent.mkdir(mode=0o700, parents=True, exist_ok=True)
            stored = {
                "version": 1,
                "last_sequence": sequence,
                "events": [
                    {"sequence": item_sequence, "payload": payload}
                    for item_sequence, payload, _ in history
                ],
            }
            encoded = json.dumps(stored, separators=(",", ":")).encode("utf-8")
            if len(encoded) > self._history_max_bytes:
                raise OSError("Event history exceeds its configured storage limit")
            directory_fd = os.open(
                parent,
                os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
            )
            temporary = ".event-history.tmp"
            try:
                flags = (
                    os.O_WRONLY
                    | os.O_CREAT
                    | os.O_EXCL
                    | os.O_NOFOLLOW
                    | os.O_CLOEXEC
                )
                try:
                    descriptor = os.open(
                        temporary, flags, 0o600, dir_fd=directory_fd
                    )
                except FileExistsError:
                    os.unlink(temporary, dir_fd=directory_fd)
                    descriptor = os.open(
                        temporary, flags, 0o600, dir_fd=directory_fd
                    )
                with os.fdopen(descriptor, "wb") as file:
                    file.write(encoded)
                    file.flush()
                    os.fsync(file.fileno())
                os.replace(
                    temporary,
                    path.name,
                    src_dir_fd=directory_fd,
                    dst_dir_fd=directory_fd,
                )
                os.fsync(directory_fd)
            except Exception:
                try:
                    os.unlink(temporary, dir_fd=directory_fd)
                except FileNotFoundError:
                    pass
                raise
            finally:
                os.close(directory_fd)
        except OSError as error:
            raise EventPersistenceError(f"Could not persist critical ROS event: {error}") from error

    def _retained_history(self, sequence, event):
        try:
            payload = base64.b64encode(self._event_encoder(event)).decode("ascii")
        except Exception as error:
            raise EventPersistenceError(f"Could not encode critical ROS event: {error}") from error
        history = self._history + [(sequence, payload, event)]
        history = history[-self._history_capacity :]
        while True:
            serialized = json.dumps(
                {
                    "version": 1,
                    "last_sequence": sequence,
                    "events": [
                        {"sequence": item_sequence, "payload": item_payload}
                        for item_sequence, item_payload, _ in history
                    ],
                },
                separators=(",", ":"),
            ).encode("utf-8")
            if len(serialized) <= self._history_max_bytes:
                break
            if not history:
                raise EventPersistenceError(
                    "Critical ROS event exceeds the history storage limit"
                )
            history.pop(0)
        if not history or history[-1][1] != payload:
            raise EventPersistenceError("Critical ROS event exceeds the history storage limit")
        return history

    def subscribe(self):
        subscriber = EventSubscriber(self._max_queue_size)
        with self._lock:
            for key, event in self._latest_state.items():
                subscriber.put(event, critical=True, key=key)
            self._subscribers.add(subscriber)
        return subscriber

    def unsubscribe(self, subscriber):
        with self._lock:
            if subscriber in self._subscribers:
                stats = subscriber.health()
                self._historical_dropped_telemetry += stats["dropped_telemetry"]
                self._historical_critical_overflows += stats["dropped_critical"]
                self._subscribers.remove(subscriber)

    def subscribe_with_replay(self, after_sequence=None):
        subscriber = EventSubscriber(self._max_queue_size)
        with self._lock:
            oldest = self._history[0][0] if self._history else self._sequence + 1
            gap = after_sequence is not None and (
                after_sequence > self._sequence
                or after_sequence < oldest - 1
                or (not self._history and after_sequence < self._sequence)
            )
            replay_after = (
                -1
                if after_sequence is not None and after_sequence > self._sequence
                else after_sequence
            )
            replay = (
                [
                    event
                    for sequence, _, event in self._history
                    if sequence > replay_after
                ]
                if replay_after is not None
                else []
            )
            snapshots = (
                list(self._latest_state.values())
                if after_sequence is None or after_sequence == 0 or gap
                else []
            )
            self._subscribers.add(subscriber)
            return subscriber, {
                "gap_detected": gap,
                "oldest_available_sequence": oldest,
                "latest_sequence": self._sequence,
                "snapshot_included": bool(snapshots),
                "retained_critical_events": len(self._history),
            }, replay + snapshots

    def publish(self, event, *, priority="critical", key=None, snapshot=False, durable=None):
        if priority not in ("critical", "telemetry"):
            raise ValueError("priority must be 'critical' or 'telemetry'")
        if key is None:
            key = id(event)
        with self._lock:
            if self._history_path is None:
                self._sequence += 1
                event_sequence = self._sequence
                if hasattr(event, "sequence"):
                    event.sequence = event_sequence
            else:
                if durable is None:
                    durable = priority == "critical" and not snapshot
                event_sequence = self._sequence + 1 if durable else 0
                if durable:
                    previous_sequence = getattr(event, "sequence", None)
                    if hasattr(event, "sequence"):
                        event.sequence = event_sequence
                    try:
                        history = self._retained_history(event_sequence, event)
                        self._persist_history(event_sequence, history)
                    except EventPersistenceError as error:
                        self._persistence_error = str(error)
                        if previous_sequence is not None:
                            event.sequence = previous_sequence
                        raise
                    self._sequence = event_sequence
                    self._history = history
                elif hasattr(event, "sequence"):
                    event.sequence = 0
            if snapshot:
                self._latest_state[key] = event
            subscribers = tuple(self._subscribers)
            for subscriber in subscribers:
                subscriber.put(event, critical=priority == "critical", key=key)

    def health(self):
        with self._lock:
            subscribers = tuple(self._subscribers)
            sequence = self._sequence
            historical_telemetry = self._historical_dropped_telemetry
            historical_critical = self._historical_critical_overflows
            persistence_error = self._persistence_error
            oldest = self._history[0][0] if self._history else sequence + 1
            retained = len(self._history)
        subscriber_stats = [subscriber.health() for subscriber in subscribers]
        return {
            "queue_capacity": self._max_queue_size,
            "subscriber_count": len(subscriber_stats),
            "last_sequence": sequence,
            "dropped_telemetry": historical_telemetry
            + sum(stat["dropped_telemetry"] for stat in subscriber_stats),
            "critical_overflows": historical_critical
            + sum(stat["dropped_critical"] for stat in subscriber_stats),
            "max_backlog": max((stat["pending_count"] for stat in subscriber_stats), default=0),
            "persistence_healthy": persistence_error is None,
            "persistence_error": persistence_error or "",
            "oldest_available_sequence": oldest,
            "retained_critical_events": retained,
        }


class EventSubscriber:
    def __init__(self, max_queue_size):
        self._max_queue_size = max_queue_size
        self._critical = deque()
        self._telemetry = OrderedDict()
        self._condition = threading.Condition()
        self.dropped_telemetry = 0
        self.dropped_critical = 0
        self._overflowed = False

    def put(self, event, *, critical, key):
        with self._condition:
            if self._overflowed:
                return
            if critical:
                if len(self._critical) >= self._max_queue_size:
                    self.dropped_critical += len(self._critical) + 1
                    self.dropped_telemetry += len(self._telemetry)
                    self._critical.clear()
                    self._telemetry.clear()
                    self._overflowed = True
                    self._condition.notify_all()
                    return
                self._critical.append((key, event))
            else:
                if key in self._telemetry:
                    self._telemetry.pop(key)
                    self.dropped_telemetry += 1
                elif len(self._telemetry) >= self._max_queue_size:
                    self._telemetry.popitem(last=False)
                    self.dropped_telemetry += 1
                self._telemetry[key] = event
            self._condition.notify()

    def get(self, timeout=None):
        deadline = None if timeout is None else time.monotonic() + timeout
        with self._condition:
            while not self._critical and not self._telemetry and not self._overflowed:
                remaining = None if deadline is None else deadline - time.monotonic()
                if remaining is not None and remaining <= 0:
                    raise queue.Empty
                self._condition.wait(remaining)
            if self._overflowed:
                raise EventOverflow("Critical event queue overflow; reconnect for state resynchronization")
            if self._critical:
                return self._critical.popleft()[1]
            return self._telemetry.popitem(last=False)[1]

    def get_nowait(self):
        return self.get(timeout=0)

    def empty(self):
        with self._condition:
            return not self._critical and not self._telemetry

    @property
    def pending_count(self):
        with self._condition:
            return len(self._critical) + len(self._telemetry)

    def health(self):
        with self._condition:
            return {
                "pending_count": len(self._critical) + len(self._telemetry),
                "dropped_telemetry": self.dropped_telemetry,
                "dropped_critical": self.dropped_critical,
                "overflowed": self._overflowed,
            }


class CommandLeaseManager:
    def __init__(self, timeout_seconds=COMMAND_TIMEOUT_SECONDS):
        if timeout_seconds <= 0:
            raise ValueError("timeout_seconds must be positive")
        self._timeout_seconds = timeout_seconds
        self._deadlines = {}
        self._lock = threading.Lock()

    def refresh(self, key, now=None, timeout_seconds=None):
        current_time = time.monotonic() if now is None else now
        with self._lock:
            timeout = self._timeout_seconds if timeout_seconds is None else timeout_seconds
            if timeout <= 0:
                raise ValueError("timeout_seconds must be positive")
            self._deadlines[key] = current_time + timeout

    def take_expired(self, now=None):
        current_time = time.monotonic() if now is None else now
        with self._lock:
            expired = [
                key for key, deadline in self._deadlines.items()
                if current_time >= deadline
            ]
            for key in expired:
                del self._deadlines[key]
            return expired

    def clear(self, key):
        with self._lock:
            self._deadlines.pop(key, None)

    def clear_all(self):
        with self._lock:
            self._deadlines.clear()


def validate_twist_request(request, now_ns=None, config_registry=None, record=True):
    if len(request.topic_name) > 200 or not ROS_TOPIC_PATTERN.fullmatch(request.topic_name):
        raise ValueError("Invalid ROS topic name")

    timestamp_ns = getattr(request, "timestamp_ns", None)
    if not isinstance(timestamp_ns, int) or timestamp_ns <= 0:
        raise ValueError("Command timestamp is missing or invalid")
    current_time_ns = time.time_ns() if now_ns is None else now_ns
    command_timeout = (
        config_registry.get(request.topic_name).command_timeout_ms / 1000.0
        if config_registry is not None
        else MAX_COMMAND_AGE_SECONDS
    )
    max_age_ns = int(command_timeout * 1_000_000_000)
    if timestamp_ns > current_time_ns + 250_000_000 or current_time_ns - timestamp_ns > max_age_ns:
        raise ValueError("Command is stale or has a future timestamp")

    if config_registry is not None:
        return config_registry.validate_command(
            request,
            now_ns=current_time_ns,
            record=record,
        )

    linear = (request.linear_x, request.linear_y, request.linear_z)
    angular = (request.angular_x, request.angular_y, request.angular_z)
    if not all(math.isfinite(value) for value in (*linear, *angular)):
        raise ValueError("Velocity components must be finite")
    if math.hypot(*linear) > MAX_LINEAR_VELOCITY:
        raise ValueError(
            f"Linear velocity magnitude must not exceed {MAX_LINEAR_VELOCITY}"
        )
    if math.hypot(*angular) > MAX_ANGULAR_VELOCITY:
        raise ValueError(
            f"Angular velocity magnitude must not exceed {MAX_ANGULAR_VELOCITY}"
        )


def is_valid_auth_token(provided, expected, expires_at, now=None):
    if not isinstance(provided, str) or not isinstance(expected, str):
        return False
    try:
        expiration = float(expires_at)
    except (TypeError, ValueError):
        return False
    current_time = time.time() if now is None else now
    if not math.isfinite(expiration) or current_time >= expiration:
        return False
    return hmac.compare_digest(provided, expected)


def load_auth_credentials(path):
    import json

    with open(path, encoding="utf-8") as credential_file:
        credentials = json.load(credential_file)
    token = credentials.get("token")
    expires_at = credentials.get("expires_at")
    if (
        not isinstance(token, str)
        or len(token) != 64
        or any(character not in "0123456789abcdef" for character in token)
    ):
        raise ValueError("Invalid gRPC credential file")
    if not isinstance(expires_at, (int, float)) or not math.isfinite(expires_at):
        raise ValueError("Invalid gRPC credential expiration")
    return token, float(expires_at)
