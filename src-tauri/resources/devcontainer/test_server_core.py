import math
import json
import tempfile
import time
import unittest
from types import SimpleNamespace

from server_core import (
    CommandLeaseManager,
    EventHub,
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
        self.assertEqual(subscriber.dropped_critical, 1)

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


if __name__ == "__main__":
    unittest.main()
