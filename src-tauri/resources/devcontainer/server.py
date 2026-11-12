import threading
import queue
import time
import logging
import os
import math
import re
import json
import uuid
from collections.abc import Sequence

import grpc
from concurrent import futures

import rclpy
from rclpy.node import Node
from rclpy.qos import DurabilityPolicy, QoSProfile, ReliabilityPolicy
from rclpy.action import ActionClient

AUTH_PROTOCOL_VERSION = 1

import sys
sys.path.insert(
    0,
    os.environ.get(
        "ROBOTICS_STUDIO_GENERATED_PATH",
        "/opt/robotics-studio/generated",
    ),
)
import ros_engine_pb2
import ros_engine_pb2_grpc
from tf2_msgs.msg import TFMessage

from nav_msgs.msg import Odometry
from geometry_msgs.msg import Twist
from sensor_msgs.msg import JointState
from rosidl_runtime_py.set_message import set_message_fields
from rosidl_runtime_py.utilities import get_action, get_message, get_service
from server_core import (
    CommandLeaseManager,
    EventPersistenceError,
    EventOverflow,
    EventHub,
    RobotConfigRegistry,
    RobotMotionConfig,
    SafetyController,
    is_valid_auth_token,
    load_auth_credentials,
    action_from_send_goal_service,
    validate_twist_request,
)

LOGGER = logging.getLogger("ros_engine")
MAX_GRAPH_NODES = 256
MAX_GRAPH_TOPICS = 512
MAX_GRAPH_SERVICES = 512
MAX_GRAPH_ACTIONS = 128
MAX_GRAPH_ENDPOINTS = 2048
MAX_JOINT_STATE_ENTRIES = 512
MAX_INSPECTED_TOPICS = 4
MAX_INSPECTION_RATE_HZ = 5.0
MAX_INSPECTION_MESSAGE_BYTES = 16 * 1024
MAX_INSPECTION_DEPTH = 5
MAX_INSPECTION_ITEMS = 256
MAX_INSPECTION_STRING_LENGTH = 512
MAX_SAFE_SERVICE_REQUEST_BYTES = 16 * 1024
MAX_SAFE_SERVICE_CALLS = 8
SAFE_READ_ONLY_SERVICE_TYPES = frozenset(
    {
        "lifecycle_msgs/srv/GetAvailableStates",
        "lifecycle_msgs/srv/GetAvailableTransitions",
        "lifecycle_msgs/srv/GetState",
        "lifecycle_msgs/srv/GetTransitionGraph",
        "rcl_interfaces/srv/DescribeParameters",
        "rcl_interfaces/srv/GetParameterTypes",
        "rcl_interfaces/srv/GetParameters",
        "rcl_interfaces/srv/ListParameters",
    }
)
SAFE_NON_MOTION_ACTION_TYPES = frozenset(
    {"action_tutorials_interfaces/action/Fibonacci"}
)
MAX_SAFE_ACTION_GOALS = 4
MAX_SAFE_ACTION_DURATION_SECONDS = 60.0
MAX_SAFE_ACTION_FEEDBACK_RATE_HZ = 5.0


class _MessageSerializationBudget:
    def __init__(self):
        self.items = 0
        self.truncated = False


def _bounded_message_value(value, budget, depth=0):
    if budget.items >= MAX_INSPECTION_ITEMS:
        budget.truncated = True
        return "<truncated>"
    budget.items += 1

    if value is None or isinstance(value, (bool, int)):
        return value
    if isinstance(value, float):
        return value if math.isfinite(value) else str(value)
    if isinstance(value, str):
        if len(value) > MAX_INSPECTION_STRING_LENGTH:
            budget.truncated = True
            return value[:MAX_INSPECTION_STRING_LENGTH]
        return value
    if isinstance(value, (bytes, bytearray)):
        if len(value) > 64:
            budget.truncated = True
        return {"hex": bytes(value[:64]).hex(), "length": len(value)}
    if depth >= MAX_INSPECTION_DEPTH:
        budget.truncated = True
        return "<maximum depth reached>"
    if hasattr(value, "get_fields_and_field_types"):
        result = {}
        for field_name in value.get_fields_and_field_types():
            if budget.items >= MAX_INSPECTION_ITEMS:
                budget.truncated = True
                result["_truncated"] = True
                break
            result[field_name] = _bounded_message_value(
                getattr(value, field_name), budget, depth + 1
            )
        return result
    if isinstance(value, Sequence):
        result = []
        for item in value:
            if budget.items >= MAX_INSPECTION_ITEMS:
                budget.truncated = True
                result.append("<truncated>")
                break
            result.append(_bounded_message_value(item, budget, depth + 1))
        return result
    return str(value)[:MAX_INSPECTION_STRING_LENGTH]


def _serialize_inspection_message(message):
    budget = _MessageSerializationBudget()
    value = _bounded_message_value(message, budget)
    payload = json.dumps(
        value,
        ensure_ascii=True,
        allow_nan=False,
        separators=(",", ":"),
    )
    if len(payload.encode("utf-8")) > MAX_INSPECTION_MESSAGE_BYTES:
        return (
            '{"error":"Serialized message exceeds the inspection size limit"}',
            True,
        )
    return payload, budget.truncated


def _qos_policy_name(value):
    return getattr(value, "name", str(value).rsplit(".", 1)[-1])


class RosIntrospector(Node):
    def __init__(self, event_hub: EventHub):
        super().__init__('ros_engine_wrapper')
        self.event_hub = event_hub
        self.known_nodes = set()
        self.odom_subscribers = {}
        self._graph_refresh_lock = threading.RLock()
        self.timer = self.create_timer(2.0, self.tick)
        self.twist_publishers = {}
        self._command_lock = threading.RLock()
        self.command_leases = CommandLeaseManager()
        self.robot_configs = RobotConfigRegistry(
            os.environ.get(
                "ROBOTICS_STUDIO_ROBOT_CONFIG",
                "/workspace/.robotics-studio/robot-config.json",
            )
        )
        self.safety = SafetyController(
            self._publish_stop_to_all,
            os.environ.get(
                "ROBOTICS_STUDIO_SAFETY_STATE",
                "/workspace/.robotics-studio/safety-state.json",
            ),
        )
        self._startup_stop_pending = self.safety.snapshot()[0] != "READY"
        self.command_timeout_timer = self.create_timer(
            0.1, self.stop_expired_commands
        )
        self.tf_subscribers = {}
        self.joint_state_subscribers = {}
        self.topic_inspection_subscriptions = {}
        self.topic_inspection_requests = queue.Queue(maxsize=16)
        self.topic_inspection_timer = self.create_timer(
            0.1, self._process_topic_inspection_requests
        )
        self.safe_service_requests = queue.Queue(maxsize=16)
        self.pending_safe_service_calls = {}
        self.safe_service_timer = self.create_timer(
            0.1, self._process_safe_service_requests
        )
        self.safe_action_requests = queue.Queue(maxsize=16)
        self.safe_action_goals = {}
        self.safe_action_timer = self.create_timer(
            0.1, self._process_safe_action_requests
        )

    def _publish_stop_to_all(self):
        with self._command_lock:
            stop_topics = set(self.twist_publishers)
            failures = []
            try:
                stop_topics.update(
                    topic_name
                    for topic_name, types in self.get_topic_names_and_types()
                    if topic_name.endswith("/cmd_vel")
                    and "geometry_msgs/msg/Twist" in types
                )
            except Exception:
                LOGGER.exception("Could not discover every cmd_vel topic during stop")
                failures.append("topic discovery")
            for topic_name in stop_topics:
                try:
                    publisher = self.twist_publishers.get(topic_name)
                    if publisher is None:
                        publisher = self.create_publisher(Twist, topic_name, 10)
                        self.twist_publishers[topic_name] = publisher
                    publisher.publish(Twist())
                    self.robot_configs.reset_command(topic_name)
                except Exception:
                    LOGGER.exception("Failed to publish stop command on %s", topic_name)
                    failures.append(topic_name)
            self.command_leases.clear_all()
            self.robot_configs.reset_all_commands()
            if failures:
                raise RuntimeError(
                    f"Could not publish stop commands on {len(failures)} topic(s)"
                )

    def odom_callback(self, msg, topic_name):
        evt = ros_engine_pb2.WorkspaceEvent(
            odometry_update=ros_engine_pb2.OdometryUpdate(
                topic_name=topic_name,
                x=msg.pose.pose.position.x,
                y=msg.pose.pose.position.y,
                z=msg.pose.pose.position.z,
                qx=msg.pose.pose.orientation.x,
                qy=msg.pose.pose.orientation.y,
                qz=msg.pose.pose.orientation.z,
                qw=msg.pose.pose.orientation.w,
            ),
            timestamp_ns=time.time_ns()
        )
        self.event_hub.publish(
            evt,
            priority="telemetry",
            key=("odometry", topic_name),
        )

    def tf_callback(self, msg, topic_name):
        for transform in msg.transforms:
            evt = ros_engine_pb2.WorkspaceEvent(
                tf_update=ros_engine_pb2.TFUpdate(
                    parent_frame=transform.header.frame_id,
                    child_frame=transform.child_frame_id,
                    x=transform.transform.translation.x,
                    y=transform.transform.translation.y,
                    z=transform.transform.translation.z,
                    qx=transform.transform.rotation.x,
                    qy=transform.transform.rotation.y,
                    qz=transform.transform.rotation.z,
                    qw=transform.transform.rotation.w,
                    topic_name=topic_name,
                    source_timestamp_ms=(
                        transform.header.stamp.sec * 1000
                        + transform.header.stamp.nanosec // 1_000_000
                    ),
                ),
                timestamp_ns=time.time_ns()
            )
            self.event_hub.publish(
                evt,
                priority="telemetry",
                key=("tf", transform.child_frame_id),
            )

    def joint_state_callback(self, msg, topic_name):
        names = list(msg.name[:MAX_JOINT_STATE_ENTRIES])
        positions = list(msg.position[:MAX_JOINT_STATE_ENTRIES])
        event = ros_engine_pb2.WorkspaceEvent(
            joint_state_update=ros_engine_pb2.JointStateUpdate(
                topic_name=topic_name,
                names=names,
                positions=positions,
                truncated=(
                    len(msg.name) > MAX_JOINT_STATE_ENTRIES
                    or len(msg.position) > MAX_JOINT_STATE_ENTRIES
                ),
            ),
            timestamp_ns=time.time_ns(),
        )
        self.event_hub.publish(
            event,
            priority="telemetry",
            key=("joint_state", topic_name),
        )

    def request_topic_inspection(self, action, topic_name):
        if not re.fullmatch(r"/[A-Za-z_][A-Za-z0-9_]*(/[A-Za-z_][A-Za-z0-9_]*)*", topic_name):
            raise ValueError("Topic name must be a fully qualified ROS topic name")
        future = futures.Future()
        try:
            self.topic_inspection_requests.put_nowait(
                (action, topic_name, future)
            )
        except queue.Full as error:
            raise RuntimeError("Topic inspection request queue is full") from error
        try:
            return future.result(timeout=5.0)
        except futures.TimeoutError as error:
            future.cancel()
            raise TimeoutError("Timed out waiting for the ROS executor") from error

    @staticmethod
    def _json_object_without_duplicate_keys(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError(f"Duplicate JSON field: {key}")
            result[key] = value
        return result

    @staticmethod
    def _reject_json_constant(value):
        raise ValueError(f"Invalid JSON number: {value}")

    def invoke_safe_service(self, service_name, service_type, request_json, timeout_ms):
        if not re.fullmatch(
            r"/[A-Za-z_][A-Za-z0-9_]*(/[A-Za-z_][A-Za-z0-9_]*)*",
            service_name,
        ):
            raise ValueError("Service name must be a fully qualified ROS name")
        if service_type not in SAFE_READ_ONLY_SERVICE_TYPES:
            raise PermissionError("This ROS service type is not on the read-only allowlist")
        if not 100 <= timeout_ms <= 5000:
            raise ValueError("Service timeout must be between 100 and 5000 milliseconds")
        try:
            request_size = len(request_json.encode("utf-8"))
        except UnicodeEncodeError as error:
            raise ValueError("Service request is not valid UTF-8") from error
        if request_size > MAX_SAFE_SERVICE_REQUEST_BYTES:
            raise ValueError("Service request exceeds the 16 KiB limit")
        try:
            request_values = json.loads(
                request_json,
                object_pairs_hook=self._json_object_without_duplicate_keys,
                parse_constant=self._reject_json_constant,
            )
        except (json.JSONDecodeError, RecursionError, ValueError) as error:
            raise ValueError(f"Invalid service request JSON: {error}") from error
        if not isinstance(request_values, dict):
            raise ValueError("Service request JSON must be an object")

        response_future = futures.Future()
        try:
            self.safe_service_requests.put_nowait(
                (
                    service_name,
                    service_type,
                    request_values,
                    timeout_ms,
                    response_future,
                )
            )
        except queue.Full as error:
            raise RuntimeError("Safe service request queue is full") from error
        try:
            return response_future.result(timeout=timeout_ms / 1000.0 + 1.0)
        except futures.TimeoutError as error:
            response_future.cancel()
            raise TimeoutError("Safe ROS service request timed out") from error

    def _process_safe_service_requests(self):
        while True:
            try:
                (
                    service_name,
                    service_type,
                    request_values,
                    timeout_ms,
                    response_future,
                ) = self.safe_service_requests.get_nowait()
            except queue.Empty:
                self._expire_safe_service_calls()
                return
            if not response_future.set_running_or_notify_cancel():
                continue
            try:
                self._begin_safe_service_call(
                    service_name,
                    service_type,
                    request_values,
                    timeout_ms,
                    response_future,
                )
            except Exception as error:
                response_future.set_exception(error)

    def _begin_safe_service_call(
        self, service_name, service_type, request_values, timeout_ms, response_future
    ):
        if len(self.pending_safe_service_calls) >= MAX_SAFE_SERVICE_CALLS:
            raise RuntimeError("Too many safe ROS service calls are in flight")
        discovered_types = dict(self.get_service_names_and_types()).get(
            service_name, []
        )
        if service_type not in discovered_types:
            raise LookupError(
                f"ROS service {service_name} with type {service_type} is unavailable"
            )
        service_class = get_service(service_type)
        request = service_class.Request()
        allowed_fields = set(request.get_fields_and_field_types())
        unknown_fields = sorted(set(request_values) - allowed_fields)
        if unknown_fields:
            raise ValueError(
                "Unknown service request fields: " + ", ".join(unknown_fields)
            )
        try:
            set_message_fields(request, request_values)
        except (AssertionError, AttributeError, TypeError, ValueError) as error:
            raise ValueError(f"Service request does not match its ROS schema: {error}") from error

        client = self.create_client(service_class, service_name)
        if not client.wait_for_service(timeout_sec=0.0):
            self.destroy_client(client)
            raise LookupError(f"ROS service {service_name} is not ready")
        call_id = id(response_future)
        try:
            ros_future = client.call_async(request)
        except Exception:
            self.destroy_client(client)
            raise
        self.pending_safe_service_calls[call_id] = {
            "client": client,
            "ros_future": ros_future,
            "response_future": response_future,
            "deadline": time.monotonic() + timeout_ms / 1000.0,
            "service_name": service_name,
            "service_type": service_type,
        }
        ros_future.add_done_callback(
            lambda completed, key=call_id: self._finish_safe_service_call(key, completed)
        )

    def _finish_safe_service_call(self, call_id, ros_future):
        call = self.pending_safe_service_calls.pop(call_id, None)
        if call is None:
            return
        self.destroy_client(call["client"])
        if ros_future.cancelled():
            if not call["response_future"].done():
                call["response_future"].set_exception(
                    TimeoutError("Safe ROS service request was cancelled")
                )
            return
        error = ros_future.exception()
        if error is not None:
            call["response_future"].set_exception(
                RuntimeError(f"ROS service call failed: {error}")
            )
            return
        try:
            payload, truncated = _serialize_inspection_message(ros_future.result())
        except (TypeError, ValueError, OverflowError) as error:
            call["response_future"].set_exception(
                RuntimeError(f"Could not serialize ROS service response: {error}")
            )
            return
        call["response_future"].set_result(
            {"response_json": payload, "truncated": truncated}
        )

    def _expire_safe_service_calls(self):
        now = time.monotonic()
        expired = [
            call_id
            for call_id, call in self.pending_safe_service_calls.items()
            if call["deadline"] <= now
        ]
        for call_id in expired:
            call = self.pending_safe_service_calls.pop(call_id)
            call["ros_future"].cancel()
            self.destroy_client(call["client"])
            if not call["response_future"].done():
                call["response_future"].set_exception(
                    TimeoutError("ROS service response timed out")
                )

    def submit_safe_action_goal(self, action_name, action_type, goal_json):
        if not re.fullmatch(
            r"/[A-Za-z_][A-Za-z0-9_]*(/[A-Za-z_][A-Za-z0-9_]*)*",
            action_name,
        ):
            raise ValueError("Action name must be a fully qualified ROS name")
        if action_type not in SAFE_NON_MOTION_ACTION_TYPES:
            raise PermissionError("This ROS action type is not on the non-motion allowlist")
        if len(goal_json.encode("utf-8")) > MAX_SAFE_SERVICE_REQUEST_BYTES:
            raise ValueError("Action goal exceeds the 16 KiB limit")
        try:
            goal_values = json.loads(
                goal_json,
                object_pairs_hook=self._json_object_without_duplicate_keys,
                parse_constant=self._reject_json_constant,
            )
        except (json.JSONDecodeError, RecursionError, ValueError) as error:
            raise ValueError(f"Invalid action goal JSON: {error}") from error
        if not isinstance(goal_values, dict):
            raise ValueError("Action goal JSON must be an object")

        action_class = get_action(action_type)
        goal = action_class.Goal()
        unknown_fields = sorted(
            set(goal_values) - set(goal.get_fields_and_field_types())
        )
        if unknown_fields:
            raise ValueError("Unknown action goal fields: " + ", ".join(unknown_fields))
        try:
            set_message_fields(goal, goal_values)
        except (AssertionError, AttributeError, TypeError, ValueError) as error:
            raise ValueError(f"Action goal does not match its ROS schema: {error}") from error
        if action_type == "action_tutorials_interfaces/action/Fibonacci":
            if not isinstance(goal.order, int) or isinstance(goal.order, bool):
                raise ValueError("Fibonacci order must be an integer")
            if not 1 <= goal.order <= 20:
                raise ValueError("Fibonacci order must be between 1 and 20")

        goal_id = str(uuid.uuid4())
        try:
            self.safe_action_requests.put_nowait(
                ("start", goal_id, action_name, action_type, goal, None)
            )
        except queue.Full as error:
            raise RuntimeError("Safe ROS action request queue is full") from error
        self._publish_action_goal(
            goal_id, action_name, action_type, status="submitted"
        )
        return goal_id

    def cancel_safe_action_goal(self, goal_id):
        try:
            normalized_id = str(uuid.UUID(goal_id))
        except (AttributeError, TypeError, ValueError) as error:
            raise ValueError("Invalid action goal identifier") from error
        response_future = futures.Future()
        try:
            self.safe_action_requests.put_nowait(
                ("cancel", normalized_id, "", "", None, response_future)
            )
        except queue.Full as error:
            raise RuntimeError("Safe ROS action request queue is full") from error
        try:
            return response_future.result(timeout=5.0)
        except futures.TimeoutError as error:
            response_future.cancel()
            raise TimeoutError("Timed out waiting for action cancellation acknowledgement") from error

    def _publish_action_goal(
        self,
        goal_id,
        action_name,
        action_type,
        status,
        feedback_json="",
        result_json="",
        error="",
        terminal=False,
        truncated=False,
    ):
        event = ros_engine_pb2.WorkspaceEvent(
            action_goal_update=ros_engine_pb2.ActionGoalUpdate(
                goal_id=goal_id,
                action_name=action_name,
                action_type=action_type,
                status=status,
                feedback_json=feedback_json,
                result_json=result_json,
                error=error,
                terminal=terminal,
                truncated=truncated,
            ),
            timestamp_ns=time.time_ns(),
        )
        if status == "feedback":
            self.event_hub.publish(
                event,
                priority="telemetry",
                key=("action_feedback", goal_id),
            )
        else:
            self.event_hub.publish(event, key=("action_goal", goal_id))

    def _process_safe_action_requests(self):
        while True:
            try:
                action, goal_id, name, action_type, goal, response_future = (
                    self.safe_action_requests.get_nowait()
                )
            except queue.Empty:
                self._expire_safe_action_goals()
                return
            try:
                if action == "start":
                    self._begin_safe_action_goal(
                        goal_id, name, action_type, goal
                    )
                else:
                    self._begin_safe_action_cancel(goal_id, response_future)
            except Exception as error:
                if response_future is not None and not response_future.done():
                    response_future.set_exception(error)
                else:
                    self._publish_action_goal(
                        goal_id,
                        name,
                        action_type,
                        status="failed",
                        error=str(error),
                        terminal=True,
                    )

    def _begin_safe_action_goal(self, goal_id, action_name, action_type, goal):
        if len(self.safe_action_goals) >= MAX_SAFE_ACTION_GOALS:
            self._publish_action_goal(
                goal_id,
                action_name,
                action_type,
                status="failed",
                error=f"At most {MAX_SAFE_ACTION_GOALS} safe goals can run at once",
                terminal=True,
            )
            return
        action_class = get_action(action_type)
        client = ActionClient(self, action_class, action_name)
        if not client.wait_for_server(timeout_sec=0.0):
            self.destroy_client(client)
            self._publish_action_goal(
                goal_id,
                action_name,
                action_type,
                status="unavailable",
                error=f"ROS action server {action_name} is not available",
                terminal=True,
            )
            return
        record = {
            "action_name": action_name,
            "action_type": action_type,
            "client": client,
            "goal_handle": None,
            "deadline": time.monotonic() + MAX_SAFE_ACTION_DURATION_SECONDS,
            "timed_out": False,
            "cancel_requested": False,
            "last_feedback_at": 0.0,
        }
        self.safe_action_goals[goal_id] = record
        try:
            send_future = client.send_goal_async(
                goal,
                feedback_callback=lambda feedback, key=goal_id: (
                    self._safe_action_feedback(key, feedback)
                ),
            )
        except Exception:
            self.safe_action_goals.pop(goal_id, None)
            self.destroy_client(client)
            raise
        send_future.add_done_callback(
            lambda completed, key=goal_id: self._safe_action_goal_response(
                key, completed
            )
        )

    def _safe_action_goal_response(self, goal_id, completed):
        record = self.safe_action_goals.get(goal_id)
        if record is None:
            return
        try:
            goal_handle = completed.result()
        except Exception as error:
            self._finish_safe_action_goal(
                goal_id, "failed", error=str(error)
            )
            return
        if not goal_handle.accepted:
            self._finish_safe_action_goal(
                goal_id,
                "rejected",
                error="ROS action server rejected the goal",
            )
            return
        record["goal_handle"] = goal_handle
        self._publish_action_goal(
            goal_id,
            record["action_name"],
            record["action_type"],
            status="accepted",
        )
        result_future = goal_handle.get_result_async()
        result_future.add_done_callback(
            lambda result, key=goal_id: self._safe_action_result(key, result)
        )
        if record["timed_out"]:
            self._request_action_cancel(record, goal_id)

    def _safe_action_feedback(self, goal_id, feedback_message):
        record = self.safe_action_goals.get(goal_id)
        if record is None:
            return
        now = time.monotonic()
        if now - record["last_feedback_at"] < 1.0 / MAX_SAFE_ACTION_FEEDBACK_RATE_HZ:
            return
        record["last_feedback_at"] = now
        try:
            payload, truncated = _serialize_inspection_message(
                feedback_message.feedback
            )
        except (TypeError, ValueError, OverflowError):
            LOGGER.exception("Could not serialize feedback for action goal %s", goal_id)
            self._publish_action_goal(
                goal_id,
                record["action_name"],
                record["action_type"],
                status="feedback_error",
                error="Action feedback could not be serialized",
            )
            return
        self._publish_action_goal(
            goal_id,
            record["action_name"],
            record["action_type"],
            status="feedback",
            feedback_json=payload,
            truncated=truncated,
        )

    def _safe_action_result(self, goal_id, completed):
        record = self.safe_action_goals.get(goal_id)
        if record is None:
            return
        try:
            response = completed.result()
            result_json, truncated = _serialize_inspection_message(response.result)
        except Exception as error:
            self._finish_safe_action_goal(
                goal_id, "failed", error=f"Could not retrieve action result: {error}"
            )
            return
        status = {
            4: "succeeded",
            5: "canceled",
            6: "aborted",
        }.get(response.status, f"terminal_{response.status}")
        if record["timed_out"]:
            status = "timed_out"
        self._finish_safe_action_goal(
            goal_id,
            status,
            result_json=result_json,
            truncated=truncated,
            error=(
                "Action exceeded its 60-second execution limit"
                if record["timed_out"]
                else ""
            ),
        )

    def _finish_safe_action_goal(
        self,
        goal_id,
        status,
        result_json="",
        error="",
        truncated=False,
    ):
        record = self.safe_action_goals.pop(goal_id, None)
        if record is None:
            return
        self.destroy_client(record["client"])
        self._publish_action_goal(
            goal_id,
            record["action_name"],
            record["action_type"],
            status=status,
            result_json=result_json,
            error=error,
            terminal=True,
            truncated=truncated,
        )

    def _begin_safe_action_cancel(self, goal_id, response_future):
        if not response_future.set_running_or_notify_cancel():
            return
        record = self.safe_action_goals.get(goal_id)
        if record is None:
            response_future.set_exception(LookupError("Action goal is not active"))
            return
        if record["goal_handle"] is None:
            response_future.set_exception(
                RuntimeError("Action goal has not yet been accepted by its server")
            )
            return
        record["cancel_requested"] = True
        self._request_action_cancel(
            record, goal_id, response_future=response_future
        )

    def _request_action_cancel(self, record, goal_id, response_future=None):
        if record["goal_handle"] is None:
            return
        record["cancel_requested"] = True
        self._publish_action_goal(
            goal_id,
            record["action_name"],
            record["action_type"],
            status="cancel_requested",
            error=(
                "Execution time limit reached; awaiting confirmed cancellation"
                if record["timed_out"]
                else ""
            ),
        )
        try:
            cancel_future = record["goal_handle"].cancel_goal_async()
        except Exception as error:
            if response_future is not None and not response_future.done():
                response_future.set_exception(error)
            else:
                LOGGER.exception("Could not request cancellation for action goal %s", goal_id)
            record["cancel_requested"] = False
            return

        def cancellation_done(completed):
            try:
                response = completed.result()
                accepted = bool(response.goals_canceling)
            except Exception as error:
                if response_future is not None and not response_future.done():
                    response_future.set_exception(error)
                self._publish_action_goal(
                    goal_id,
                    record["action_name"],
                    record["action_type"],
                    status="cancel_failed",
                    error=f"Action cancellation failed: {error}",
                )
                return
            if response_future is not None and not response_future.done():
                response_future.set_result(accepted)
            if not accepted:
                record["cancel_requested"] = False
                self._publish_action_goal(
                    goal_id,
                    record["action_name"],
                    record["action_type"],
                    status="cancel_rejected",
                    error="ROS action server did not accept cancellation",
                )

        cancel_future.add_done_callback(cancellation_done)

    def _expire_safe_action_goals(self):
        now = time.monotonic()
        for goal_id, record in tuple(self.safe_action_goals.items()):
            if record["deadline"] <= now and not record["timed_out"]:
                record["timed_out"] = True
                self._request_action_cancel(record, goal_id)

    def _process_topic_inspection_requests(self):
        while True:
            try:
                action, topic_name, future = (
                    self.topic_inspection_requests.get_nowait()
                )
            except queue.Empty:
                return
            if not future.set_running_or_notify_cancel():
                continue
            try:
                if action == "start":
                    result = self._start_topic_inspection(topic_name)
                else:
                    result = self._stop_topic_inspection(topic_name)
            except Exception as error:
                future.set_exception(error)
            else:
                future.set_result(result)

    def _publish_topic_inspection(
        self, topic_name, message_type, active, payload="", truncated=False, error=""
    ):
        event = ros_engine_pb2.WorkspaceEvent(
            topic_inspection_update=ros_engine_pb2.TopicInspectionUpdate(
                topic_name=topic_name,
                message_type=message_type,
                json_payload=payload,
                active=active,
                truncated=truncated,
                error=error,
            ),
            timestamp_ns=time.time_ns(),
        )
        self.event_hub.publish(
            event,
            priority="telemetry",
            key=("topic_inspection", topic_name),
        )

    def _start_topic_inspection(self, topic_name):
        existing = self.topic_inspection_subscriptions.get(topic_name)
        if existing is not None:
            return existing["message_type"]
        if len(self.topic_inspection_subscriptions) >= MAX_INSPECTED_TOPICS:
            raise RuntimeError(
                f"At most {MAX_INSPECTED_TOPICS} topics can be inspected at once"
            )

        topic_types = dict(self.get_topic_names_and_types()).get(topic_name, [])
        if not topic_types:
            raise LookupError(f"ROS topic {topic_name} is not currently available")
        if len(topic_types) != 1:
            raise ValueError(f"ROS topic {topic_name} has ambiguous message types")

        message_type_name = topic_types[0]
        try:
            message_class = get_message(message_type_name)
        except (AttributeError, ModuleNotFoundError, ValueError) as error:
            raise LookupError(
                f"Message type {message_type_name} is not available in this ROS environment"
            ) from error

        qos = QoSProfile(
            depth=5,
            reliability=ReliabilityPolicy.BEST_EFFORT,
            durability=DurabilityPolicy.VOLATILE,
        )
        subscription = self.create_subscription(
            message_class,
            topic_name,
            lambda message, name=topic_name, type_name=message_type_name: (
                self._topic_inspection_callback(message, name, type_name)
            ),
            qos,
        )
        self.topic_inspection_subscriptions[topic_name] = {
            "subscription": subscription,
            "message_type": message_type_name,
            "last_sent_at": 0.0,
        }
        self._publish_topic_inspection(
            topic_name, message_type_name, active=True
        )
        return message_type_name

    def _stop_topic_inspection(self, topic_name):
        inspection = self.topic_inspection_subscriptions.pop(topic_name, None)
        if inspection is not None:
            self.destroy_subscription(inspection["subscription"])
            message_type = inspection["message_type"]
        else:
            message_type = ""
        self._publish_topic_inspection(
            topic_name, message_type, active=False
        )
        return message_type

    def _topic_inspection_callback(self, message, topic_name, message_type):
        inspection = self.topic_inspection_subscriptions.get(topic_name)
        if inspection is None:
            return
        now = time.monotonic()
        if now - inspection["last_sent_at"] < 1.0 / MAX_INSPECTION_RATE_HZ:
            return
        inspection["last_sent_at"] = now
        try:
            payload, truncated = _serialize_inspection_message(message)
        except (TypeError, ValueError, OverflowError):
            LOGGER.exception("Could not serialize inspected topic %s", topic_name)
            self._publish_topic_inspection(
                topic_name,
                message_type,
                active=True,
                error="Message could not be serialized",
            )
            return
        self._publish_topic_inspection(
            topic_name,
            message_type,
            active=True,
            payload=payload,
            truncated=truncated,
        )

    def tick(self):
        try:
            self._tick_impl()
        except EventPersistenceError:
            LOGGER.exception("Could not persist a critical ROS event")
            health = self.event_hub.health()
            health_event = ros_engine_pb2.WorkspaceEvent(
                event_health=ros_engine_pb2.EventHealth(**health),
                timestamp_ns=time.time_ns(),
            )
            try:
                self.event_hub.publish(
                    health_event,
                    priority="telemetry",
                    key="event_health",
                    snapshot=True,
                )
            except Exception:
                LOGGER.exception("Could not publish event persistence health")
        except Exception as e:
            self.get_logger().error(f"tick() failed, skipping this cycle: {e}")

    def _sync_automatic_topic_subscriptions(self, topics):
        current_topic_names = {name for name, _ in topics}

        stale_inspections = (
            set(self.topic_inspection_subscriptions) - current_topic_names
        )
        for name in stale_inspections:
            inspection = self.topic_inspection_subscriptions.pop(name)
            self.destroy_subscription(inspection["subscription"])
            self._publish_topic_inspection(
                name,
                inspection["message_type"],
                active=False,
                error="Topic is no longer available",
            )

        stale_odom = set(self.odom_subscribers) - current_topic_names
        for name in stale_odom:
            self.destroy_subscription(self.odom_subscribers.pop(name))

        stale_tf = set(self.tf_subscribers) - current_topic_names
        for name in stale_tf:
            self.destroy_subscription(self.tf_subscribers.pop(name))

        stale_joint_state = set(self.joint_state_subscribers) - current_topic_names
        for name in stale_joint_state:
            self.destroy_subscription(self.joint_state_subscribers.pop(name))

        for name, types in topics:
            if 'nav_msgs/msg/Odometry' in types and name not in self.odom_subscribers:
                sub = self.create_subscription(
                    Odometry, name,
                    lambda msg, n=name: self.odom_callback(msg, n),
                    10,
                )
                self.odom_subscribers[name] = sub
                self.get_logger().info(f'Subscribed to odometry topic: {name}')

            if 'tf2_msgs/msg/TFMessage' in types and name not in self.tf_subscribers:
                qos = (
                    QoSProfile(
                        depth=100,
                        reliability=ReliabilityPolicy.RELIABLE,
                        durability=DurabilityPolicy.TRANSIENT_LOCAL,
                    )
                    if name.endswith("/tf_static")
                    else 10
                )
                sub = self.create_subscription(
                    TFMessage, name,
                    lambda msg, n=name: self.tf_callback(msg, n),
                    qos,
                )
                self.tf_subscribers[name] = sub
                self.get_logger().info(f'Subscribed to TF topic: {name}')

            if (
                "sensor_msgs/msg/JointState" in types
                and name not in self.joint_state_subscribers
            ):
                subscription = self.create_subscription(
                    JointState,
                    name,
                    lambda msg, topic=name: self.joint_state_callback(msg, topic),
                    10,
                )
                self.joint_state_subscribers[name] = subscription
                self.get_logger().info(f"Subscribed to joint state topic: {name}")

    def _build_graph_snapshot(self, topics):
        with self._graph_refresh_lock:
            node_entries = sorted(self.get_node_names_and_namespaces())
            truncated = len(node_entries) > MAX_GRAPH_NODES
            node_entries = node_entries[:MAX_GRAPH_NODES]
            service_roles = {}
            for node_name, node_namespace in node_entries:
                node_id = f"{node_namespace.rstrip('/')}/{node_name}"
                for service_name, _ in self.get_service_names_and_types_by_node(
                    node_name, node_namespace
                ):
                    service_roles.setdefault(service_name, {"servers": [], "clients": []})[
                        "servers"
                    ].append(node_id)
                for service_name, _ in self.get_client_names_and_types_by_node(
                    node_name, node_namespace
                ):
                    service_roles.setdefault(service_name, {"servers": [], "clients": []})[
                        "clients"
                    ].append(node_id)

            graph = ros_engine_pb2.RosGraphSnapshot()
            graph.nodes.extend(
                ros_engine_pb2.GraphNode(name=name, namespace=namespace)
                for name, namespace in node_entries
            )

            endpoint_count = 0
            topic_entries = sorted(topics)
            if len(topic_entries) > MAX_GRAPH_TOPICS:
                truncated = True
            for topic_name, types in topic_entries[:MAX_GRAPH_TOPICS]:
                topic = ros_engine_pb2.GraphTopic(name=topic_name, types=sorted(types))
                for field, query in (
                    ("publishers", self.get_publishers_info_by_topic),
                    ("subscribers", self.get_subscriptions_info_by_topic),
                ):
                    remaining = MAX_GRAPH_ENDPOINTS - endpoint_count
                    if remaining <= 0:
                        truncated = True
                        break
                    endpoints = sorted(
                        query(topic_name),
                        key=lambda endpoint: (
                            endpoint.node_namespace,
                            endpoint.node_name,
                            endpoint.topic_type,
                        ),
                    )
                    if len(endpoints) > remaining:
                        truncated = True
                    selected = endpoints[:remaining]
                    endpoint_count += len(selected)
                    getattr(topic, field).extend(
                        ros_engine_pb2.GraphEndpoint(
                            node_name=endpoint.node_name,
                            node_namespace=endpoint.node_namespace,
                            topic_type=endpoint.topic_type,
                            reliability=_qos_policy_name(
                                endpoint.qos_profile.reliability
                            ),
                            durability=_qos_policy_name(
                                endpoint.qos_profile.durability
                            ),
                            history=_qos_policy_name(endpoint.qos_profile.history),
                            depth=endpoint.qos_profile.depth,
                        )
                        for endpoint in selected
                    )
                graph.topics.append(topic)

            service_entries = sorted(self.get_service_names_and_types())
            if len(service_entries) > MAX_GRAPH_SERVICES:
                truncated = True
            action_services = {}
            for service_name, types in service_entries:
                role = service_roles.get(service_name, {"servers": [], "clients": []})
                action_info = None
                for service_type in types:
                    action_info = action_from_send_goal_service(
                        service_name, service_type
                    )
                    if action_info:
                        break
                if action_info:
                    action_name, action_type = action_info
                    action = action_services.setdefault(
                        action_name,
                        {"types": set(), "servers": set(), "clients": set()},
                    )
                    action["types"].add(action_type)
                    action["servers"].update(role["servers"])
                    action["clients"].update(role["clients"])
                    continue
                if "/_action/" in service_name:
                    continue
                graph.services.append(
                    ros_engine_pb2.GraphService(
                        name=service_name,
                        types=sorted(types),
                        server_nodes=sorted(set(role["servers"])),
                        client_nodes=sorted(set(role["clients"])),
                        safe_types=sorted(
                            set(types) & SAFE_READ_ONLY_SERVICE_TYPES
                        ),
                    )
                )

            if len(action_services) > MAX_GRAPH_ACTIONS:
                truncated = True
            for action_name, action in sorted(action_services.items())[:MAX_GRAPH_ACTIONS]:
                graph.actions.append(
                    ros_engine_pb2.GraphAction(
                        name=action_name,
                        types=sorted(action["types"]),
                        server_nodes=sorted(action["servers"]),
                        client_nodes=sorted(action["clients"]),
                        safe_types=sorted(
                            set(action["types"]) & SAFE_NON_MOTION_ACTION_TYPES
                        ),
                    )
                )

            graph.truncated = truncated
            return graph

    def _tick_impl(self):
        nodes = set(self.get_node_names())
        appeared = nodes - self.known_nodes
        disappeared = self.known_nodes - nodes

        for name in appeared:
            evt = ros_engine_pb2.WorkspaceEvent(
                node_started=ros_engine_pb2.NodeStarted(node_name=name),
                timestamp_ns=time.time_ns(),
            )
            self.event_hub.publish(evt, key=("node", name))

        for name in disappeared:
            evt = ros_engine_pb2.WorkspaceEvent(
                node_crashed=ros_engine_pb2.NodeCrashed(node_name=name),
                timestamp_ns=time.time_ns(),
            )
            self.event_hub.publish(evt, key=("node", name))

        self.known_nodes = nodes

        topics = self.get_topic_names_and_types()
        if self._startup_stop_pending:
            if self.safety.snapshot()[0] != "READY":
                self.safety.enforce_latched_stop()
            self._startup_stop_pending = False

        self._sync_automatic_topic_subscriptions(topics)

        topic_msgs = [
            ros_engine_pb2.Topic(name=n, types=list(t)) for n, t in topics
        ]
        evt = ros_engine_pb2.WorkspaceEvent(
            topic_snapshot=ros_engine_pb2.TopicSnapshot(topics=topic_msgs),
            timestamp_ns=time.time_ns(),
        )
        self.event_hub.publish(
            evt,
            key="topic_snapshot",
            snapshot=True,
        )

        node_evt = ros_engine_pb2.WorkspaceEvent(
            node_snapshot=ros_engine_pb2.NodeSnapshot(node_names=list(nodes)),
            timestamp_ns=time.time_ns(),
        )
        self.event_hub.publish(
            node_evt,
            key="node_snapshot",
            snapshot=True,
        )
        health = self.event_hub.health()
        health_event = ros_engine_pb2.WorkspaceEvent(
            event_health=ros_engine_pb2.EventHealth(**health),
            timestamp_ns=time.time_ns(),
        )
        self.event_hub.publish(
            health_event,
            priority="telemetry",
            key="event_health",
            snapshot=True,
        )
        self.publish_graph_snapshot(topics)

    def publish_graph_snapshot(self, topics):
        graph = self._build_graph_snapshot(topics)
        self.event_hub.publish(
            ros_engine_pb2.WorkspaceEvent(
                ros_graph_snapshot=graph,
                timestamp_ns=time.time_ns(),
            ),
            priority="telemetry",
            key="ros_graph_snapshot",
            snapshot=True,
        )

    def publish_twist(self, request):
        def publish():
            config = validate_twist_request(
                request,
                config_registry=self.robot_configs,
                record=False,
            )
            with self._command_lock:
                if request.topic_name not in self.twist_publishers:
                    self.twist_publishers[request.topic_name] = self.create_publisher(
                        Twist, request.topic_name, 10
                    )
                publisher = self.twist_publishers[request.topic_name]
                msg = Twist()
                msg.linear.x = request.linear_x
                msg.linear.y = request.linear_y
                msg.linear.z = request.linear_z
                msg.angular.x = request.angular_x
                msg.angular.y = request.angular_y
                msg.angular.z = request.angular_z
                publisher.publish(msg)
                self.robot_configs.record_command(request)
            return config

        config = self.safety.run_if_motion_permitted(publish)
        self.command_leases.refresh(
            request.topic_name,
            timeout_seconds=config.command_lease_ms / 1000.0,
        )

    def stop_expired_commands(self):
        with self._command_lock:
            for topic_name in self.command_leases.take_expired():
                publisher = self.twist_publishers.get(topic_name)
                if publisher is not None:
                    publisher.publish(Twist())
                    self.robot_configs.reset_command(topic_name)

class RosEngineServicer(ros_engine_pb2_grpc.RosEngineServicer):
    def __init__(self, event_hub: EventHub, ros_node, auth_token, auth_expires_at):
        self.event_hub = event_hub
        self.ros_node = ros_node
        self.auth_token = auth_token
        self.auth_expires_at = auth_expires_at

    def _require_authentication(self, context):
        provided = dict(context.invocation_metadata()).get(
            "x-robotics-studio-token"
        )
        if not is_valid_auth_token(provided, self.auth_token, self.auth_expires_at):
            context.abort(
                grpc.StatusCode.UNAUTHENTICATED,
                "A valid Robotics Studio session token is required",
            )
        return provided

    def PublishTwist(self, request, context):
        self._require_authentication(context)
        try:
            self.ros_node.publish_twist(request)
        except ValueError as error:
            context.abort(grpc.StatusCode.INVALID_ARGUMENT, str(error))
        except Exception as error:
            LOGGER.exception("Failed to publish Twist command")
            context.abort(grpc.StatusCode.INTERNAL, "Failed to publish Twist command")
        return ros_engine_pb2.Empty()

    def StartTopicInspection(self, request, context):
        self._require_authentication(context)
        try:
            self.ros_node.request_topic_inspection("start", request.topic_name)
        except ValueError as error:
            context.abort(grpc.StatusCode.INVALID_ARGUMENT, str(error))
        except LookupError as error:
            context.abort(grpc.StatusCode.NOT_FOUND, str(error))
        except TimeoutError as error:
            context.abort(grpc.StatusCode.DEADLINE_EXCEEDED, str(error))
        except RuntimeError as error:
            context.abort(grpc.StatusCode.RESOURCE_EXHAUSTED, str(error))
        except Exception:
            LOGGER.exception("Failed to start topic inspection for %s", request.topic_name)
            context.abort(
                grpc.StatusCode.INTERNAL, "Failed to start topic inspection"
            )
        return ros_engine_pb2.Empty()

    def StopTopicInspection(self, request, context):
        self._require_authentication(context)
        try:
            self.ros_node.request_topic_inspection("stop", request.topic_name)
        except ValueError as error:
            context.abort(grpc.StatusCode.INVALID_ARGUMENT, str(error))
        except TimeoutError as error:
            context.abort(grpc.StatusCode.DEADLINE_EXCEEDED, str(error))
        except RuntimeError as error:
            context.abort(grpc.StatusCode.RESOURCE_EXHAUSTED, str(error))
        except Exception:
            LOGGER.exception("Failed to stop topic inspection for %s", request.topic_name)
            context.abort(
                grpc.StatusCode.INTERNAL, "Failed to stop topic inspection"
            )
        return ros_engine_pb2.Empty()

    def InvokeSafeService(self, request, context):
        self._require_authentication(context)
        try:
            response = self.ros_node.invoke_safe_service(
                request.service_name,
                request.service_type,
                request.request_json,
                request.timeout_ms,
            )
        except PermissionError as error:
            context.abort(grpc.StatusCode.PERMISSION_DENIED, str(error))
        except ValueError as error:
            context.abort(grpc.StatusCode.INVALID_ARGUMENT, str(error))
        except LookupError as error:
            context.abort(grpc.StatusCode.NOT_FOUND, str(error))
        except TimeoutError as error:
            context.abort(grpc.StatusCode.DEADLINE_EXCEEDED, str(error))
        except RuntimeError as error:
            context.abort(grpc.StatusCode.RESOURCE_EXHAUSTED, str(error))
        except Exception:
            LOGGER.exception(
                "Safe ROS service invocation failed for %s", request.service_name
            )
            context.abort(
                grpc.StatusCode.INTERNAL, "Safe ROS service invocation failed"
            )
        return ros_engine_pb2.SafeServiceResponse(
            service_name=request.service_name,
            service_type=request.service_type,
            response_json=response["response_json"],
            truncated=response["truncated"],
        )

    def SendSafeActionGoal(self, request, context):
        self._require_authentication(context)
        try:
            goal_id = self.ros_node.submit_safe_action_goal(
                request.action_name,
                request.action_type,
                request.goal_json,
            )
        except PermissionError as error:
            context.abort(grpc.StatusCode.PERMISSION_DENIED, str(error))
        except ValueError as error:
            context.abort(grpc.StatusCode.INVALID_ARGUMENT, str(error))
        except RuntimeError as error:
            context.abort(grpc.StatusCode.RESOURCE_EXHAUSTED, str(error))
        except Exception:
            LOGGER.exception(
                "Could not submit safe ROS action goal for %s", request.action_name
            )
            context.abort(grpc.StatusCode.INTERNAL, "Could not submit ROS action goal")
        return ros_engine_pb2.ActionGoalResponse(goal_id=goal_id)

    def CancelActionGoal(self, request, context):
        self._require_authentication(context)
        try:
            requested = self.ros_node.cancel_safe_action_goal(request.goal_id)
        except ValueError as error:
            context.abort(grpc.StatusCode.INVALID_ARGUMENT, str(error))
        except LookupError as error:
            context.abort(grpc.StatusCode.NOT_FOUND, str(error))
        except TimeoutError as error:
            context.abort(grpc.StatusCode.DEADLINE_EXCEEDED, str(error))
        except RuntimeError as error:
            context.abort(grpc.StatusCode.FAILED_PRECONDITION, str(error))
        except Exception:
            LOGGER.exception("Could not request cancellation for action goal %s", request.goal_id)
            context.abort(
                grpc.StatusCode.INTERNAL, "Could not cancel ROS action goal"
            )
        return ros_engine_pb2.ActionCancelResponse(requested=requested)

    def _safety_state_message(self):
        state, detail, updated_at_ns = self.ros_node.safety.snapshot()
        state_value = getattr(ros_engine_pb2.SafetyState, state)
        return ros_engine_pb2.SafetyState(
            state=state_value,
            detail=detail,
            updated_at_ns=updated_at_ns,
        )

    def _publish_safety_state(self):
        event = ros_engine_pb2.WorkspaceEvent(
            safety_state=self._safety_state_message(),
            timestamp_ns=time.time_ns(),
        )
        self.event_hub.publish(
            event,
            key="safety_state",
            snapshot=True,
            durable=True,
        )
        return event.safety_state

    def SetEmergencyStop(self, request, context):
        self._require_authentication(context)
        try:
            if request.action == ros_engine_pb2.EmergencyStopRequest.ACTIVATE:
                self.ros_node.safety.activate()
            elif request.action == ros_engine_pb2.EmergencyStopRequest.RESET:
                self.ros_node.safety.reset()
            else:
                context.abort(
                    grpc.StatusCode.INVALID_ARGUMENT,
                    "Emergency-stop action must be ACTIVATE or RESET",
                )
            return self._publish_safety_state()
        except EventPersistenceError as error:
            LOGGER.error("Emergency-stop transition could not be durably reported: %s", error)
            try:
                self.ros_node.safety.latch_fault(
                    "Emergency-stop state could not be written to durable event history"
                )
            except Exception:
                LOGGER.exception("Could not persist fail-closed emergency-stop state")
            context.abort(
                grpc.StatusCode.INTERNAL,
                "Emergency-stop event history failed; motion has been fault-latched",
            )
        except ValueError as error:
            context.abort(grpc.StatusCode.FAILED_PRECONDITION, str(error))
        except Exception:
            LOGGER.exception("Emergency-stop transition failed")
            try:
                self._publish_safety_state()
            except EventPersistenceError:
                LOGGER.exception("Emergency-stop state could not be durably reported")
                try:
                    self.ros_node.safety.latch_fault(
                        "Emergency-stop state could not be written to durable event history"
                    )
                except Exception:
                    LOGGER.exception("Could not persist fail-closed emergency-stop state")
            context.abort(
                grpc.StatusCode.INTERNAL,
                "Emergency-stop transition failed; inspect safety state",
            )

    def GetSafetyState(self, request, context):
        self._require_authentication(context)
        return self._safety_state_message()

    def GetEventHealth(self, request, context):
        self._require_authentication(context)
        return ros_engine_pb2.EventHealth(**self.event_hub.health())

    @staticmethod
    def _config_message(config):
        return ros_engine_pb2.RobotConfig(
            topic_name=config.topic_name,
            max_linear_velocity=config.max_linear_velocity,
            max_angular_velocity=config.max_angular_velocity,
            max_linear_acceleration=config.max_linear_acceleration,
            max_angular_acceleration=config.max_angular_acceleration,
            command_lease_ms=config.command_lease_ms,
            command_timeout_ms=config.command_timeout_ms,
            permitted_motion_modes=list(config.permitted_motion_modes),
        )

    def GetRobotConfig(self, request, context):
        self._require_authentication(context)
        try:
            config = self.ros_node.robot_configs.get(request.topic_name)
            return self._config_message(config)
        except ValueError as error:
            context.abort(grpc.StatusCode.INVALID_ARGUMENT, str(error))

    def SetRobotConfig(self, request, context):
        self._require_authentication(context)
        try:
            if request.reset_to_defaults:
                config = self.ros_node.robot_configs.reset(request.topic_name)
            else:
                config = RobotMotionConfig(
                    topic_name=request.topic_name,
                    max_linear_velocity=request.max_linear_velocity,
                    max_angular_velocity=request.max_angular_velocity,
                    max_linear_acceleration=request.max_linear_acceleration,
                    max_angular_acceleration=request.max_angular_acceleration,
                    command_lease_ms=request.command_lease_ms,
                    command_timeout_ms=request.command_timeout_ms,
                    permitted_motion_modes=tuple(request.permitted_motion_modes),
                )
                self.ros_node.robot_configs.set(config)
            return self._config_message(config)
        except ValueError as error:
            context.abort(grpc.StatusCode.INVALID_ARGUMENT, str(error))

    def StreamEvents(self, request, context):
        provided = self._require_authentication(context)
        subscriber = self.event_hub.subscribe()
        try:
            yield from self._stream_subscriber(subscriber, provided, context)
        finally:
            self.event_hub.unsubscribe(subscriber)

    def ReplayEvents(self, request, context):
        provided = self._require_authentication(context)
        after_sequence = request.after_sequence if request.has_after_sequence else None
        subscriber, recovery, initial_events = self.event_hub.subscribe_with_replay(
            after_sequence
        )
        try:
            yield ros_engine_pb2.WorkspaceEvent(
                event_recovery=ros_engine_pb2.EventRecovery(**recovery),
                timestamp_ns=time.time_ns(),
            )
            for event in initial_events:
                if not context.is_active():
                    return
                if not is_valid_auth_token(
                    provided,
                    self.auth_token,
                    self.auth_expires_at,
                ):
                    context.abort(
                        grpc.StatusCode.UNAUTHENTICATED,
                        "The Robotics Studio session token has expired",
                    )
                yield event
            yield from self._stream_subscriber(subscriber, provided, context)
        finally:
            self.event_hub.unsubscribe(subscriber)

    def _stream_subscriber(self, subscriber, provided, context):
        LOGGER.info("ROS event stream opened")
        while context.is_active():
            if not is_valid_auth_token(
                provided,
                self.auth_token,
                self.auth_expires_at,
            ):
                context.abort(
                    grpc.StatusCode.UNAUTHENTICATED,
                    "The Robotics Studio session token has expired",
                )
            try:
                yield subscriber.get(timeout=1.0)
            except EventOverflow:
                context.abort(
                    grpc.StatusCode.RESOURCE_EXHAUSTED,
                    "Critical events overflowed; reconnect to resynchronize from snapshots",
                )
            except queue.Empty:
                continue
        LOGGER.info("ROS event stream closed")


def ros_spin_thread(node):
    rclpy.spin(node)


def main():
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s %(levelname)s %(name)s: %(message)s",
    )
    credential_path = os.environ.get(
        "ROBOTICS_STUDIO_AUTH_FILE",
        "/run/robotics-studio/grpc-auth.json",
    )
    auth_token, auth_expires_at = load_auth_credentials(credential_path)
    rclpy.init()
    queue_capacity = int(os.environ.get("ROBOTICS_STUDIO_EVENT_QUEUE_CAPACITY", "256"))
    if not 16 <= queue_capacity <= 100_000:
        raise ValueError("ROBOTICS_STUDIO_EVENT_QUEUE_CAPACITY must be between 16 and 100000")
    history_path = os.environ.get(
        "ROBOTICS_STUDIO_EVENT_HISTORY",
        "/workspace/.robotics-studio/event-history.json",
    )
    event_hub = EventHub(
        max_queue_size=queue_capacity,
        history_path=history_path,
        event_encoder=lambda event: event.SerializeToString(),
        event_decoder=ros_engine_pb2.WorkspaceEvent.FromString,
    )
    node = RosIntrospector(event_hub)

    spin_thread = threading.Thread(target=ros_spin_thread, args=(node,), daemon=True)
    spin_thread.start()
    try:
        node.safety.enforce_latched_stop()
    except RuntimeError:
        LOGGER.exception("Emergency stop could not be reasserted during startup")

    grpc_workers = int(os.environ.get("ROBOTICS_STUDIO_GRPC_WORKERS", "16"))
    if not 4 <= grpc_workers <= 128:
        raise ValueError("ROBOTICS_STUDIO_GRPC_WORKERS must be between 4 and 128")
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=grpc_workers))
    servicer = RosEngineServicer(
        event_hub, node, auth_token, auth_expires_at
    )
    servicer._publish_safety_state()
    ros_engine_pb2_grpc.add_RosEngineServicer_to_server(
        servicer, server
    )
    with open(
        os.environ.get(
            "ROBOTICS_STUDIO_TLS_CERTIFICATE",
            "/run/robotics-studio/grpc-server.crt",
        ),
        "rb",
    ) as certificate_file:
        certificate = certificate_file.read()
    with open(
        os.environ.get(
            "ROBOTICS_STUDIO_TLS_PRIVATE_KEY",
            "/run/robotics-studio/grpc-server.key",
        ),
        "rb",
    ) as key_file:
        private_key = key_file.read()
    credentials = grpc.ssl_server_credentials(((private_key, certificate),))
    if server.add_secure_port("[::]:50051", credentials) != 50051:
        raise RuntimeError("Could not bind the authenticated TLS gRPC server")
    server.start()
    LOGGER.info("RosEngine gRPC server running on port 50051")

    try:
        server.wait_for_termination()
    except KeyboardInterrupt:
        LOGGER.info("Shutting down")
    finally:
        stop_event = server.stop(grace=2)
        stop_event.wait(3)
        node.destroy_node()
        if rclpy.ok():
            rclpy.shutdown()
        spin_thread.join(timeout=3)


if __name__ == '__main__':
    main()