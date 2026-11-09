import math
import queue
import re
import threading

MAX_EVENT_QUEUE_SIZE = 256
MAX_LINEAR_VELOCITY = 5.0
MAX_ANGULAR_VELOCITY = 5.0
ROS_TOPIC_PATTERN = re.compile(r"/[A-Za-z_][A-Za-z0-9_]*(/[A-Za-z_][A-Za-z0-9_]*)*")


class EventHub:
    def __init__(self, max_queue_size=MAX_EVENT_QUEUE_SIZE):
        if max_queue_size < 1:
            raise ValueError("max_queue_size must be positive")
        self._max_queue_size = max_queue_size
        self._subscribers = set()
        self._lock = threading.Lock()

    def subscribe(self):
        subscriber = queue.Queue(maxsize=self._max_queue_size)
        with self._lock:
            self._subscribers.add(subscriber)
        return subscriber

    def unsubscribe(self, subscriber):
        with self._lock:
            self._subscribers.discard(subscriber)

    def publish(self, event):
        with self._lock:
            subscribers = tuple(self._subscribers)

        for subscriber in subscribers:
            try:
                subscriber.put_nowait(event)
            except queue.Full:
                try:
                    subscriber.get_nowait()
                except queue.Empty:
                    pass
                try:
                    subscriber.put_nowait(event)
                except queue.Full:
                    pass


def validate_twist_request(request):
    if len(request.topic_name) > 200 or not ROS_TOPIC_PATTERN.fullmatch(request.topic_name):
        raise ValueError("Invalid ROS topic name")

    for name, value, limit in (
        ("linear_x", request.linear_x, MAX_LINEAR_VELOCITY),
        ("linear_y", request.linear_y, MAX_LINEAR_VELOCITY),
        ("linear_z", request.linear_z, MAX_LINEAR_VELOCITY),
        ("angular_x", request.angular_x, MAX_ANGULAR_VELOCITY),
        ("angular_y", request.angular_y, MAX_ANGULAR_VELOCITY),
        ("angular_z", request.angular_z, MAX_ANGULAR_VELOCITY),
    ):
        if not math.isfinite(value) or abs(value) > limit:
            raise ValueError(f"{name} must be finite and between -{limit} and {limit}")
