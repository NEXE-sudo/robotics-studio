import math
import unittest
from types import SimpleNamespace

from server_core import EventHub, validate_twist_request


def request(topic="/robot/cmd_vel", value=0.0):
    return SimpleNamespace(
        topic_name=topic,
        linear_x=value,
        linear_y=0.0,
        linear_z=0.0,
        angular_x=0.0,
        angular_y=0.0,
        angular_z=0.0,
    )


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

        self.assertEqual([subscriber.get_nowait(), subscriber.get_nowait()], [1, 2])

    def test_unsubscribed_client_receives_no_events(self):
        hub = EventHub()
        subscriber = hub.subscribe()
        hub.unsubscribe(subscriber)

        hub.publish("event")

        self.assertTrue(subscriber.empty())


class TwistValidationTests(unittest.TestCase):
    def test_accepts_valid_request(self):
        validate_twist_request(request())

    def test_rejects_malformed_topic(self):
        with self.assertRaises(ValueError):
            validate_twist_request(request("/robot/../cmd_vel"))

    def test_rejects_non_finite_or_unbounded_velocity(self):
        for value in (math.nan, math.inf, 5.01):
            with self.subTest(value=value), self.assertRaises(ValueError):
                validate_twist_request(request(value=value))


if __name__ == "__main__":
    unittest.main()
