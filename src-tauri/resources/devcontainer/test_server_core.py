import math
import json
import os
import tempfile
import time
import unittest
from unittest import mock
from types import SimpleNamespace

from server_core import (
    CommandLeaseManager,
    EventPersistenceError,
    EventOverflow,
    EventHub,
    RobotConfigRegistry,
    RobotMotionConfig,
    SafetyController,
    action_from_send_goal_service,
    discover_workspace_packages,
    is_valid_auth_token,
    load_auth_credentials,
    validate_twist_request,
)


def request(topic="/robot/cmd_vel", value=0.0, timestamp_ns=None, linear_y=0.0,
            angular_x=0.0, angular_z=0.0):
    return SimpleNamespace(
        topic_name=topic,
        linear_x=value,
        linear_y=linear_y,
        linear_z=0.0,
        angular_x=angular_x,
        angular_y=0.0,
        angular_z=angular_z,
        timestamp_ns=time.time_ns() if timestamp_ns is None else timestamp_ns,
    )


class RosGraphDiscoveryTests(unittest.TestCase):
    def test_extracts_canonical_ros_action_from_send_goal_service(self):
        self.assertEqual(
            action_from_send_goal_service(
                "/fibonacci/_action/send_goal",
                "example_interfaces/action/Fibonacci_SendGoal",
            ),
            ("/fibonacci", "example_interfaces/action/Fibonacci"),
        )

    def test_ignores_non_action_or_malformed_send_goal_services(self):
        self.assertIsNone(
            action_from_send_goal_service(
                "/fibonacci/send_goal",
                "example_interfaces/action/Fibonacci_SendGoal",
            )
        )
        self.assertIsNone(
            action_from_send_goal_service(
                "/fibonacci/_action/send_goal",
                "example_interfaces/srv/SendGoal",
            )
        )


class WorkspacePackageDiscoveryTests(unittest.TestCase):
    @staticmethod
    def create_package(workspace, package_path, manifest):
        package = os.path.join(workspace, package_path)
        os.makedirs(package, exist_ok=True)
        with open(os.path.join(package, "package.xml"), "w", encoding="utf-8") as file:
            file.write(manifest)
        return package

    def test_discovers_source_package_metadata_and_workspace_executables(self):
        with tempfile.TemporaryDirectory() as workspace:
            package = self.create_package(
                workspace,
                "src/robot_app",
                """<package format="3">
                  <name>robot_app</name><version>1.2.3</version>
                  <description>Robot runtime package</description>
                  <buildtool_depend>ament_cmake</buildtool_depend>
                  <depend>rclpy</depend><exec_depend>geometry_msgs</exec_depend>
                </package>""",
            )
            os.makedirs(os.path.join(package, "launch"))
            os.makedirs(os.path.join(package, "config"))
            with open(os.path.join(package, "launch", "start.launch.py"), "w") as file:
                file.write("pass\n")
            with open(os.path.join(package, "config", "robot.yaml"), "w") as file:
                file.write("name: robot\n")
            executable = os.path.join(workspace, "install", "lib", "robot_app", "driver")
            os.makedirs(os.path.dirname(executable))
            with open(executable, "w") as file:
                file.write("#!/bin/sh\n")
            os.chmod(executable, 0o755)

            packages = discover_workspace_packages(workspace)

            self.assertEqual(len(packages), 1)
            self.assertEqual(
                packages[0],
                {
                    "name": "robot_app",
                    "version": "1.2.3",
                    "description": "Robot runtime package",
                    "package_type": "ament_cmake",
                    "dependencies": ["ament_cmake", "geometry_msgs", "rclpy"],
                    "metadata_error": None,
                    "path": "src/robot_app",
                    "executables": ["driver"],
                    "launch_files": ["launch/start.launch.py"],
                    "config_files": ["config/robot.yaml"],
                },
            )

    def test_detects_ament_python_and_malformed_package_metadata(self):
        with tempfile.TemporaryDirectory() as workspace:
            self.create_package(
                workspace,
                "src/python_node",
                """<package format="3"><name>python_node</name>
                  <version>0.1.0</version><description>Python node package</description>
                  <buildtool_depend>ament_python</buildtool_depend></package>""",
            )
            self.create_package(workspace, "src/broken_node", "<package>")

            packages = {
                package["name"]: package
                for package in discover_workspace_packages(workspace)
            }

            self.assertEqual(packages["python_node"]["package_type"], "ament_python")
            self.assertEqual(packages["python_node"]["version"], "0.1.0")
            self.assertIsNone(packages["python_node"]["metadata_error"])
            self.assertEqual(packages["broken_node"]["package_type"], "unknown")
            self.assertTrue(packages["broken_node"]["metadata_error"])

    def test_rejects_missing_workspace_and_isolates_selected_workspace(self):
        with tempfile.TemporaryDirectory() as parent:
            first = os.path.join(parent, "first")
            second = os.path.join(parent, "second")
            os.makedirs(first)
            os.makedirs(second)
            self.create_package(
                first, "src/first_pkg", "<package><name>first_pkg</name></package>"
            )
            self.create_package(
                second, "src/second_pkg", "<package><name>second_pkg</name></package>"
            )

            self.assertEqual(
                [package["name"] for package in discover_workspace_packages(first)],
                ["first_pkg"],
            )
            with self.assertRaisesRegex(ValueError, "missing or is not a directory"):
                discover_workspace_packages(os.path.join(parent, "missing"))

    def test_inaccessible_workspace_tree_is_reported_instead_of_returning_empty(self):
        with tempfile.TemporaryDirectory() as workspace:
            os.makedirs(os.path.join(workspace, "src"))

            def inaccessible_walk(*_args, **kwargs):
                kwargs["onerror"](PermissionError("permission denied"))
                return iter(())

            with mock.patch("server_core.os.walk", side_effect=inaccessible_walk):
                with self.assertRaisesRegex(PermissionError, "permission denied"):
                    discover_workspace_packages(workspace)

    @unittest.skipUnless(hasattr(os, "symlink"), "symlinks are unavailable")
    def test_does_not_follow_symlinked_package_directories(self):
        with tempfile.TemporaryDirectory() as parent:
            workspace = os.path.join(parent, "workspace")
            outside = os.path.join(parent, "outside")
            os.makedirs(os.path.join(workspace, "src"))
            self.create_package(
                parent, "outside/escaped", "<package><name>escaped</name></package>"
            )
            os.symlink(outside, os.path.join(workspace, "src", "linked"))

            self.assertEqual(discover_workspace_packages(workspace), [])


class EventHubTests(unittest.TestCase):
    def test_subscribers_receive_independent_events(self):
        hub = EventHub()
        first = hub.subscribe()
        second = hub.subscribe()
        event = object()

        hub.publish(event)

        self.assertIs(first.get_nowait(), event)
        self.assertIs(second.get_nowait(), event)

    def test_full_subscriber_queue_keeps_latest_event(self):
        hub = EventHub(max_queue_size=2)
        subscriber = hub.subscribe()

        for value in range(3):
            hub.publish(value)

        with self.assertRaises(EventOverflow):
            subscriber.get_nowait()
        self.assertEqual(subscriber.dropped_critical, 3)
        self.assertEqual(subscriber.pending_count, 0)

    def test_overflowed_subscriber_requires_snapshot_resynchronization(self):
        hub = EventHub(max_queue_size=1)
        hub.publish("node-snapshot-v1", key="nodes", snapshot=True)
        subscriber = hub.subscribe()
        hub.publish("node-started")
        hub.publish("node-crashed")
        with self.assertRaises(EventOverflow):
            subscriber.get_nowait()
        recovered = hub.subscribe()
        self.assertEqual(recovered.get_nowait(), "node-snapshot-v1")

    def test_telemetry_is_coalesced_by_key_and_drop_counted(self):
        hub = EventHub(max_queue_size=3)
        subscriber = hub.subscribe()

        hub.publish("odom-1", priority="telemetry", key="odom")
        hub.publish("odom-2", priority="telemetry", key="odom")
        hub.publish("joint-1", priority="telemetry", key="joint")
        hub.publish("tf-1", priority="telemetry", key="tf")

        self.assertEqual(
            [subscriber.get_nowait(), subscriber.get_nowait(), subscriber.get_nowait()],
            ["odom-2", "joint-1", "tf-1"],
        )
        self.assertEqual(subscriber.dropped_telemetry, 1)

    def test_critical_event_is_delivered_ahead_of_telemetry(self):
        hub = EventHub(max_queue_size=1)
        subscriber = hub.subscribe()

        hub.publish("telemetry", priority="telemetry", key="odom")
        hub.publish("node-crashed", key=("node", "camera"))

        self.assertEqual(subscriber.get_nowait(), "node-crashed")
        self.assertEqual(subscriber.get_nowait(), "telemetry")
        self.assertEqual(subscriber.dropped_telemetry, 0)

    def test_reconnecting_subscriber_receives_latest_snapshots(self):
        hub = EventHub()
        hub.publish("topics-v1", key="topics", snapshot=True)
        hub.publish("topics-v2", key="topics", snapshot=True)
        hub.publish("nodes", key="nodes", snapshot=True)

        subscriber = hub.subscribe()

        self.assertEqual(
            {subscriber.get_nowait(), subscriber.get_nowait()},
            {"topics-v2", "nodes"},
        )

    def test_slow_subscriber_memory_stays_bounded_under_telemetry_load(self):
        hub = EventHub(max_queue_size=8)
        slow = hub.subscribe()
        another_client = hub.subscribe()

        for value in range(10_000):
            hub.publish(value, priority="telemetry", key=f"topic-{value % 4}")

        self.assertLessEqual(slow.pending_count, 8)
        self.assertLessEqual(another_client.pending_count, 8)
        self.assertEqual(slow.dropped_telemetry, 9_996)
        self.assertEqual(another_client.get_nowait(), 9_996)
        self.assertEqual(another_client.get_nowait(), 9_997)
        self.assertEqual(another_client.get_nowait(), 9_998)
        self.assertEqual(another_client.get_nowait(), 9_999)

    def test_unsubscribed_client_receives_no_events(self):
        hub = EventHub()
        subscriber = hub.subscribe()
        hub.unsubscribe(subscriber)

        hub.publish("event")

        self.assertTrue(subscriber.empty())

    def test_event_sequences_and_health_metrics_are_exposed(self):
        hub = EventHub(max_queue_size=2)
        subscriber = hub.subscribe()
        first = SimpleNamespace(sequence=0)
        second = SimpleNamespace(sequence=0)
        hub.publish(first)
        hub.publish(second)
        self.assertEqual((first.sequence, second.sequence), (1, 2))
        health = hub.health()
        self.assertEqual(health["queue_capacity"], 2)
        self.assertEqual(health["subscriber_count"], 1)
        self.assertEqual(health["last_sequence"], 2)
        self.assertEqual(health["max_backlog"], 2)

    @staticmethod
    def encode_event(event):
        return json.dumps(
            {"sequence": event.sequence, "name": event.name}
        ).encode("utf-8")

    @staticmethod
    def decode_event(payload):
        return SimpleNamespace(**json.loads(payload.decode("utf-8")))

    def test_persistent_history_replays_critical_events_after_restart(self):
        with tempfile.TemporaryDirectory() as directory:
            path = os.path.join(directory, "events.json")
            options = {
                "history_path": path,
                "event_encoder": self.encode_event,
                "event_decoder": self.decode_event,
            }
            first = EventHub(**options)
            temporary = os.path.join(directory, ".event-history.tmp")
            with open(temporary, "w", encoding="utf-8") as file:
                file.write("interrupted write")
            first.publish(SimpleNamespace(sequence=0, name="node-started"))
            self.assertFalse(os.path.exists(temporary))
            first.publish(
                SimpleNamespace(sequence=99, name="telemetry"),
                priority="telemetry",
            )
            first.publish(SimpleNamespace(sequence=0, name="node-crashed"))
            self.assertEqual(first.health()["last_sequence"], 2)

            restarted = EventHub(**options)
            restarted.publish(
                SimpleNamespace(sequence=0, name="current-state"),
                key="state",
                snapshot=True,
                durable=False,
            )
            subscriber, recovery, replay = restarted.subscribe_with_replay(0)
            self.assertFalse(recovery["gap_detected"])
            self.assertTrue(recovery["snapshot_included"])
            self.assertEqual(
                [(event.sequence, event.name) for event in replay],
                [
                    (1, "node-started"),
                    (2, "node-crashed"),
                    (0, "current-state"),
                ],
            )
            self.assertEqual(restarted.health()["last_sequence"], 2)
            restarted.unsubscribe(subscriber)

    def test_history_retention_reports_gaps_and_includes_latest_snapshot(self):
        with tempfile.TemporaryDirectory() as directory:
            hub = EventHub(
                history_path=os.path.join(directory, "events.json"),
                event_encoder=self.encode_event,
                event_decoder=self.decode_event,
                history_capacity=2,
            )
            for value in range(1, 5):
                hub.publish(SimpleNamespace(sequence=0, name=f"critical-{value}"))
            snapshot = SimpleNamespace(sequence=0, name="latest-state")
            hub.publish(snapshot, key="state", snapshot=True, durable=False)

            subscriber, recovery, replay = hub.subscribe_with_replay(1)
            self.assertTrue(recovery["gap_detected"])
            self.assertTrue(recovery["snapshot_included"])
            self.assertEqual(recovery["oldest_available_sequence"], 3)
            self.assertEqual(recovery["retained_critical_events"], 2)
            self.assertEqual(
                [event.name for event in replay],
                ["critical-3", "critical-4", "latest-state"],
            )
            hub.unsubscribe(subscriber)

    def test_replay_subscription_has_no_gap_with_concurrent_live_publish(self):
        with tempfile.TemporaryDirectory() as directory:
            hub = EventHub(
                history_path=os.path.join(directory, "events.json"),
                event_encoder=self.encode_event,
                event_decoder=self.decode_event,
            )
            hub.publish(SimpleNamespace(sequence=0, name="before-subscribe"))
            subscriber, recovery, replay = hub.subscribe_with_replay(0)
            hub.publish(SimpleNamespace(sequence=0, name="after-subscribe"))

            self.assertFalse(recovery["gap_detected"])
            self.assertEqual([event.name for event in replay], ["before-subscribe"])
            self.assertEqual(subscriber.get_nowait().name, "after-subscribe")
            hub.unsubscribe(subscriber)

    def test_future_replay_cursor_reports_gap_and_recovers_retained_history(self):
        with tempfile.TemporaryDirectory() as directory:
            hub = EventHub(
                history_path=os.path.join(directory, "events.json"),
                event_encoder=self.encode_event,
                event_decoder=self.decode_event,
            )
            hub.publish(SimpleNamespace(sequence=0, name="critical-1"))
            hub.publish(SimpleNamespace(sequence=0, name="critical-2"))

            subscriber, recovery, replay = hub.subscribe_with_replay(9)
            self.assertTrue(recovery["gap_detected"])
            self.assertEqual(
                [event.name for event in replay],
                ["critical-1", "critical-2"],
            )
            hub.unsubscribe(subscriber)

    def test_critical_event_persistence_failure_is_reported_without_sequence_advance(self):
        with tempfile.TemporaryDirectory() as directory:
            hub = EventHub(
                history_path=os.path.join(directory, "events.json"),
                event_encoder=self.encode_event,
                event_decoder=self.decode_event,
            )
            subscriber = hub.subscribe()
            event = SimpleNamespace(sequence=0, name="must-not-disappear")
            with mock.patch.object(
                hub,
                "_persist_history",
                side_effect=EventPersistenceError("disk unavailable"),
            ):
                with self.assertRaisesRegex(EventPersistenceError, "disk unavailable"):
                    hub.publish(event)

            self.assertEqual(hub.health()["last_sequence"], 0)
            self.assertFalse(hub.health()["persistence_healthy"])
            self.assertIn("disk unavailable", hub.health()["persistence_error"])
            self.assertTrue(subscriber.empty())
            self.assertEqual(event.sequence, 0)
            hub.publish(SimpleNamespace(sequence=0, name="later-event"))
            self.assertFalse(hub.health()["persistence_healthy"])

    def test_persistent_history_rejects_symlinked_journal(self):
        with tempfile.TemporaryDirectory() as directory:
            target = os.path.join(directory, "outside.json")
            journal = os.path.join(directory, "events.json")
            with open(target, "w", encoding="utf-8") as file:
                json.dump({"version": 1, "last_sequence": 0, "events": []}, file)
            os.symlink(target, journal)
            with self.assertRaisesRegex(ValueError, "could not be loaded safely"):
                EventHub(
                    history_path=journal,
                    event_encoder=self.encode_event,
                    event_decoder=self.decode_event,
                )


class TwistValidationTests(unittest.TestCase):
    def test_accepts_valid_request(self):
        validate_twist_request(request())
        validate_twist_request(request(value=0.5))

    def test_rejects_malformed_topic(self):
        with self.assertRaises(ValueError):
            validate_twist_request(request("/robot/../cmd_vel"))

    def test_rejects_non_finite_or_unbounded_velocity(self):
        for value in (math.nan, math.inf, 0.51):
            with self.subTest(value=value), self.assertRaises(ValueError):
                validate_twist_request(request(value=value))

    def test_rejects_velocity_vectors_over_magnitude_limits(self):
        with self.assertRaisesRegex(ValueError, "Linear velocity magnitude"):
            validate_twist_request(request(value=0.4, linear_y=0.4))
        with self.assertRaisesRegex(ValueError, "Angular velocity magnitude"):
            validate_twist_request(request(angular_x=0.8, angular_z=0.8))

    def test_robot_configs_apply_different_limits_and_modes(self):
        registry = RobotConfigRegistry()
        small_robot = RobotMotionConfig(
            "/small/cmd_vel",
            max_linear_velocity=0.2,
            max_angular_velocity=0.4,
            permitted_motion_modes=("planar",),
        )
        registry.set(small_robot)
        self.assertEqual(registry.get("/large/cmd_vel").max_linear_velocity, 0.5)
        with self.assertRaisesRegex(ValueError, "robot limit"):
            registry.validate_command(request("/small/cmd_vel", value=0.3))
        with self.assertRaisesRegex(ValueError, "planar"):
            registry.validate_command(
                request("/small/cmd_vel", value=0.1, linear_y=0.01)
            )

    def test_robot_config_rejects_unsafe_limits_and_timeouts(self):
        with self.assertRaises(ValueError):
            RobotMotionConfig("/robot/cmd_vel", max_linear_velocity=100).validate()
        with self.assertRaises(ValueError):
            RobotMotionConfig("/robot/cmd_vel", max_linear_velocity=True).validate()
        with self.assertRaises(ValueError):
            RobotMotionConfig(
                "/robot/cmd_vel",
                command_lease_ms=2000,
                command_timeout_ms=1000,
            ).validate()

    def test_robot_config_can_explicitly_allow_full_twist(self):
        registry = RobotConfigRegistry()
        registry.set(
            RobotMotionConfig(
                "/arm/cmd_vel",
                permitted_motion_modes=("full_twist",),
            )
        )
        command = request("/arm/cmd_vel", value=0.1, linear_y=0.1)
        registry.validate_command(command, monotonic_now=1.0)

    def test_robot_configs_persist_and_reload(self):
        with tempfile.TemporaryDirectory() as directory:
            path = os.path.join(directory, "config", "robots.json")
            registry = RobotConfigRegistry(path)
            saved = RobotMotionConfig(
                "/robot/cmd_vel",
                max_linear_velocity=0.25,
                permitted_motion_modes=("planar",),
            )
            registry.set(saved)
            reloaded = RobotConfigRegistry(path)
            self.assertEqual(reloaded.get("/robot/cmd_vel"), saved)
            reloaded.reset("/robot/cmd_vel")
            self.assertEqual(reloaded.get("/robot/cmd_vel").max_linear_velocity, 0.5)

    def test_acceleration_limits_are_enforced_per_robot(self):
        registry = RobotConfigRegistry()
        registry.set(
            RobotMotionConfig(
                "/robot/cmd_vel",
                max_linear_acceleration=0.5,
                max_angular_acceleration=1.0,
            )
        )
        registry.validate_command(request(value=0.1), monotonic_now=10.0)
        with self.assertRaisesRegex(ValueError, "acceleration"):
            registry.validate_command(request(value=0.2), monotonic_now=10.1)

    def test_command_lease_expires_and_refreshes(self):
        leases = CommandLeaseManager(timeout_seconds=0.5)
        leases.refresh("/robot/cmd_vel", now=10.0)
        leases.refresh("/robot/cmd_vel", now=10.25)

        self.assertEqual(leases.take_expired(now=10.74), [])
        self.assertEqual(leases.take_expired(now=10.75), ["/robot/cmd_vel"])
        self.assertEqual(leases.take_expired(now=11.0), [])

    def test_rejects_stale_or_future_commands(self):
        now_ns = 10_000_000_000
        with self.assertRaises(ValueError):
            validate_twist_request(request(timestamp_ns=now_ns - 1_000_000_001), now_ns)
        with self.assertRaises(ValueError):
            validate_twist_request(request(timestamp_ns=now_ns + 250_000_001), now_ns)


class AuthenticationTests(unittest.TestCase):
    def test_accepts_valid_unexpired_token(self):
        self.assertTrue(is_valid_auth_token("session-token", "session-token", 101, now=100))

    def test_rejects_missing_invalid_expired_and_rotated_tokens(self):
        self.assertFalse(is_valid_auth_token(None, "session-token", 101, now=100))
        self.assertFalse(is_valid_auth_token("wrong", "session-token", 101, now=100))
        self.assertFalse(is_valid_auth_token("session-token", "session-token", 100, now=100))
        self.assertFalse(is_valid_auth_token("old-token", "new-token", 101, now=100))

    def test_loads_credentials_without_logging_them(self):
        token = "a" * 64
        with tempfile.NamedTemporaryFile(mode="w+", encoding="utf-8") as credential_file:
            json.dump({"token": token, "expires_at": 200}, credential_file)
            credential_file.flush()

            self.assertEqual(load_auth_credentials(credential_file.name), (token, 200.0))

    def test_rejects_malformed_credentials(self):
        for token in ("short", "g" * 64):
            with self.subTest(token=token), tempfile.NamedTemporaryFile(
                mode="w+", encoding="utf-8"
            ) as credential_file:
                json.dump({"token": token, "expires_at": 200}, credential_file)
                credential_file.flush()
                with self.assertRaises(ValueError):
                    load_auth_credentials(credential_file.name)


class SafetyControllerTests(unittest.TestCase):
    def test_latches_activation_and_requires_explicit_reset(self):
        calls = []
        safety = SafetyController(lambda: calls.append("stop"))
        self.assertEqual(safety.snapshot()[0], "READY")
        self.assertEqual(safety.activate()[0], "ACTIVE")
        with self.assertRaisesRegex(ValueError, "ACTIVE"):
            safety.run_if_motion_permitted(lambda: None)
        self.assertEqual(safety.reset()[0], "READY")
        self.assertEqual(calls, ["stop", "stop"])

    def test_stop_failure_latches_fault_and_reset_failure_is_not_reported_ready(self):
        safety = SafetyController(lambda: (_ for _ in ()).throw(OSError("publisher failed")))
        with self.assertRaises(RuntimeError):
            safety.activate()
        self.assertEqual(safety.snapshot()[0], "FAULT")

    def test_event_history_failure_can_fail_closed_and_persist_safety_fault(self):
        with tempfile.TemporaryDirectory() as directory:
            stops = []
            safety = SafetyController(lambda: stops.append("stop"), os.path.join(directory, "safety.json"))
            safety.latch_fault("event history unavailable")
            self.assertEqual(safety.snapshot()[0], "FAULT")
            self.assertIn("event history unavailable", safety.snapshot()[1])
            self.assertEqual(stops, ["stop"])
            restarted = SafetyController(lambda: None, os.path.join(directory, "safety.json"))
            self.assertEqual(restarted.snapshot()[0], "FAULT")

    def test_latched_state_survives_controller_restart(self):
        with tempfile.TemporaryDirectory() as directory:
            path = os.path.join(directory, "safety.json")
            first = SafetyController(lambda: None, path)
            first.activate()
            first.reset()
            restarted = SafetyController(lambda: None, path)
            self.assertEqual(restarted.snapshot()[0], "READY")
            self.assertEqual(
                [event["event"] for event in restarted.history()],
                ["ACTIVATED", "RESET_REQUESTED", "RESET_CONFIRMED"],
            )

    def test_interrupted_reset_restores_as_latched_and_history_is_preserved(self):
        with tempfile.TemporaryDirectory() as directory:
            path = os.path.join(directory, "safety.json")
            first = SafetyController(lambda: None, path)
            first.activate()
            with open(path, encoding="utf-8") as state_file:
                state = json.load(state_file)
            state["state"] = "RESET_PENDING"
            with open(path, "w", encoding="utf-8") as state_file:
                json.dump(state, state_file)

            restarted = SafetyController(lambda: None, path)
            self.assertEqual(restarted.snapshot()[0], "ACTIVE")
            self.assertEqual(restarted.history()[0]["event"], "ACTIVATED")

    def test_restored_stop_is_reasserted_and_failure_latches_fault(self):
        with tempfile.TemporaryDirectory() as directory:
            path = os.path.join(directory, "safety.json")
            SafetyController(lambda: None, path).activate()
            calls = []
            restarted = SafetyController(lambda: calls.append("stop"), path)
            restarted.enforce_latched_stop()
            self.assertEqual(calls, ["stop"])
            self.assertEqual(restarted.snapshot()[0], "ACTIVE")
            self.assertEqual(restarted.history()[-1]["event"], "STOP_REASSERTED")

            failing = SafetyController(
                lambda: (_ for _ in ()).throw(OSError("publisher failed")),
                path,
            )
            with self.assertRaises(RuntimeError):
                failing.enforce_latched_stop()
            self.assertEqual(failing.snapshot()[0], "FAULT")


if __name__ == "__main__":
    unittest.main()
