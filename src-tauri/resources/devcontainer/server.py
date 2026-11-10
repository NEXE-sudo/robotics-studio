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
sys.path.append('generated')
import ros_engine_pb2
import ros_engine_pb2_grpc
from tf2_msgs.msg import TFMessage

from nav_msgs.msg import Odometry
from geometry_msgs.msg import Twist
from server_core import (
    CommandLeaseManager,
    EventHub,
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
        self.command_leases = CommandLeaseManager()
        self.command_timeout_timer = self.create_timer(
            0.1, self.stop_expired_commands
        )
        self.tf_subscribers = {}

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

    def publish_twist(self, topic_name, linear_x, linear_y=0.0, linear_z=0.0, angular_x=0.0, angular_y=0.0, angular_z=0.0):
        if topic_name not in self.twist_publishers:
            self.twist_publishers[topic_name] = self.create_publisher(Twist, topic_name, 10)
        msg = Twist()
        msg.linear.x = linear_x
        msg.linear.y = linear_y
        msg.linear.z = linear_z
        msg.angular.x = angular_x
        msg.angular.y = angular_y
        msg.angular.z = angular_z
        self.twist_publishers[topic_name].publish(msg)
        self.command_leases.refresh(topic_name)

    def stop_expired_commands(self):
        for topic_name in self.command_leases.take_expired():
            publisher = self.twist_publishers.get(topic_name)
            if publisher is not None:
                publisher.publish(Twist())

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
            validate_twist_request(request)
            self.ros_node.publish_twist(
                request.topic_name,
                request.linear_x,
                request.linear_y,
                request.linear_z,
                request.angular_x,
                request.angular_y,
                request.angular_z,
            )
        except ValueError as error:
            context.abort(grpc.StatusCode.INVALID_ARGUMENT, str(error))
        except Exception as error:
            LOGGER.exception("Failed to publish Twist command")
            context.abort(grpc.StatusCode.INTERNAL, "Failed to publish Twist command")
        return ros_engine_pb2.Empty()

    def StreamEvents(self, request, context):
        provided = self._require_authentication(context)
        subscriber = self.event_hub.subscribe()
        LOGGER.info("ROS event stream opened")
        try:
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
                except queue.Empty:
                    continue
        finally:
            self.event_hub.unsubscribe(subscriber)
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
    event_hub = EventHub()
    node = RosIntrospector(event_hub)

    spin_thread = threading.Thread(target=ros_spin_thread, args=(node,), daemon=True)
    spin_thread.start()

    server = grpc.server(futures.ThreadPoolExecutor(max_workers=4))
    ros_engine_pb2_grpc.add_RosEngineServicer_to_server(
        RosEngineServicer(event_hub, node, auth_token, auth_expires_at), server
    )
    server.add_insecure_port('[::]:50051')
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