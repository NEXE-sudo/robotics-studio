from collections import OrderedDict, deque
import hmac
import math
import queue
import re
import threading
import time

AUTH_PROTOCOL_VERSION = 1
MAX_EVENT_QUEUE_SIZE = 256
MAX_LINEAR_VELOCITY = 0.5
MAX_ANGULAR_VELOCITY = 1.0
MAX_COMMAND_AGE_SECONDS = 1.0
COMMAND_TIMEOUT_SECONDS = 0.5
ROS_TOPIC_PATTERN = re.compile(r"/[A-Za-z_][A-Za-z0-9_]*(/[A-Za-z_][A-Za-z0-9_]*)*")


class EventHub:
    def __init__(self, max_queue_size=MAX_EVENT_QUEUE_SIZE):
        if max_queue_size < 1:
            raise ValueError("max_queue_size must be positive")
        self._max_queue_size = max_queue_size
        self._subscribers = set()
        self._lock = threading.Lock()
        self._latest_state = {}

    def subscribe(self):
        subscriber = EventSubscriber(self._max_queue_size)
        with self._lock:
            self._subscribers.add(subscriber)
            snapshots = tuple(self._latest_state.items())
        for key, event in snapshots:
            subscriber.put(event, critical=True, key=key)
        return subscriber

    def unsubscribe(self, subscriber):
        with self._lock:
            self._subscribers.discard(subscriber)

    def publish(self, event, *, priority="critical", key=None, snapshot=False):
        if priority not in ("critical", "telemetry"):
            raise ValueError("priority must be 'critical' or 'telemetry'")
        if key is None:
            key = id(event)
        with self._lock:
            if snapshot:
                self._latest_state[key] = event
            subscribers = tuple(self._subscribers)

        for subscriber in subscribers:
            subscriber.put(event, critical=priority == "critical", key=key)


class EventSubscriber:
    def __init__(self, max_queue_size):
        self._max_queue_size = max_queue_size
        self._critical = deque()
        self._telemetry = OrderedDict()
        self._condition = threading.Condition()
        self.dropped_telemetry = 0
        self.dropped_critical = 0

    def put(self, event, *, critical, key):
        with self._condition:
            if critical:
                if len(self._critical) >= self._max_queue_size:
                    if self._telemetry:
                        self._telemetry.popitem(last=False)
                        self.dropped_telemetry += 1
                    else:
                        for index, (queued_key, _) in enumerate(self._critical):
                            if queued_key == key:
                                del self._critical[index]
                                break
                        else:
                            self._critical.popleft()
                            self.dropped_critical += 1
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
            while not self._critical and not self._telemetry:
                remaining = None if deadline is None else deadline - time.monotonic()
                if remaining is not None and remaining <= 0:
                    raise queue.Empty
                self._condition.wait(remaining)
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


class CommandLeaseManager:
    def __init__(self, timeout_seconds=COMMAND_TIMEOUT_SECONDS):
        if timeout_seconds <= 0:
            raise ValueError("timeout_seconds must be positive")
        self._timeout_seconds = timeout_seconds
        self._deadlines = {}
        self._lock = threading.Lock()

    def refresh(self, key, now=None):
        current_time = time.monotonic() if now is None else now
        with self._lock:
            self._deadlines[key] = current_time + self._timeout_seconds

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


def validate_twist_request(request, now_ns=None):
    if len(request.topic_name) > 200 or not ROS_TOPIC_PATTERN.fullmatch(request.topic_name):
        raise ValueError("Invalid ROS topic name")

    timestamp_ns = getattr(request, "timestamp_ns", None)
    if not isinstance(timestamp_ns, int) or timestamp_ns <= 0:
        raise ValueError("Command timestamp is missing or invalid")
    current_time_ns = time.time_ns() if now_ns is None else now_ns
    max_age_ns = int(MAX_COMMAND_AGE_SECONDS * 1_000_000_000)
    if timestamp_ns > current_time_ns + 250_000_000 or current_time_ns - timestamp_ns > max_age_ns:
        raise ValueError("Command is stale or has a future timestamp")

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
