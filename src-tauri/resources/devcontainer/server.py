import threading
import queue
import time
import logging
import os

import grpc
from concurrent import futures

import rclpy
from rclpy.node import Node

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
    validate_twist_request,
)

LOGGER = logging.getLogger("ros_engine")

class RosIntrospector(Node):
    def __init__(self, event_hub: EventHub):
        super().__init__('ros_engine_wrapper')
        self.event_hub = event_hub
        self.known_nodes = set()
        self.odom_subscribers = {}
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
                ),
                timestamp_ns=time.time_ns()
            )
            self.event_hub.publish(
                evt,
                priority="telemetry",
                key=("tf", transform.child_frame_id),
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

    def _tick_impl(self):
        nodes = set(self.get_node_names())
        appeared = nodes - self.known_nodes
        disappeared = self.known_nodes - nodes

        for name in appeared:
            evt = ros_engine_pb2.WorkspaceEvent(
                node_started=ros_engine_pb2.NodeStarted(node_name=name),
                timestamp_ns=time.time_ns()
            )
            self.event_hub.publish(evt, key=("node", name))

        for name in disappeared:
            evt = ros_engine_pb2.WorkspaceEvent(
                node_crashed=ros_engine_pb2.NodeCrashed(node_name=name),
                timestamp_ns=time.time_ns()
            )
            self.event_hub.publish(evt, key=("node", name))

        self.known_nodes = nodes

        topics = self.get_topic_names_and_types()
        if self._startup_stop_pending:
            if self.safety.snapshot()[0] != "READY":
                self.safety.enforce_latched_stop()
            self._startup_stop_pending = False

        # Clean up subscription tracking for topics that no longer exist,
        # so they can be freshly re-subscribed if they reappear later
        current_topic_names = {name for name, _ in topics}
        stale = set(self.odom_subscribers.keys()) - current_topic_names
        for name in stale:
            del self.odom_subscribers[name]

        stale_tf = set(self.tf_subscribers.keys()) - current_topic_names
        for name in stale_tf:
            del self.tf_subscribers[name]

        for name, types in topics:
            if 'nav_msgs/msg/Odometry' in types and name not in self.odom_subscribers:
                sub = self.create_subscription(
                    Odometry, name,
                    lambda msg, n=name: self.odom_callback(msg, n),
                    10
                )
                self.odom_subscribers[name] = sub
                self.get_logger().info(f'Subscribed to odometry topic: {name}')

        for name, types in topics:
            if 'tf2_msgs/msg/TFMessage' in types and name not in self.tf_subscribers:
                sub = self.create_subscription(
                    TFMessage, name,
                    lambda msg, n=name: self.tf_callback(msg, n),
                    10
                )
                self.tf_subscribers[name] = sub
                self.get_logger().info(f'Subscribed to TF topic: {name}')

        topic_msgs = [
            ros_engine_pb2.Topic(name=n, types=list(t)) for n, t in topics
        ]
        evt = ros_engine_pb2.WorkspaceEvent(
            topic_snapshot=ros_engine_pb2.TopicSnapshot(topics=topic_msgs),
            timestamp_ns=time.time_ns()
        )
        self.event_hub.publish(
            evt,
            key="topic_snapshot",
            snapshot=True,
        )

        node_evt = ros_engine_pb2.WorkspaceEvent(
            node_snapshot=ros_engine_pb2.NodeSnapshot(node_names=list(nodes)),
            timestamp_ns=time.time_ns()
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