import threading
import queue
import time

import grpc
from concurrent import futures

import rclpy
from rclpy.node import Node

import sys
sys.path.append('generated')
import ros_engine_pb2
import ros_engine_pb2_grpc

from nav_msgs.msg import Odometry
from geometry_msgs.msg import Twist

class RosIntrospector(Node):
    def __init__(self, event_queue: queue.Queue):
        super().__init__('ros_engine_wrapper')
        self.event_queue = event_queue
        self.known_nodes = set()
        self.odom_subscribers = {}
        self.timer = self.create_timer(2.0, self.tick)
        self.twist_publishers = {}

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
        self.event_queue.put(evt)

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
            self.event_queue.put(evt)

        for name in disappeared:
            evt = ros_engine_pb2.WorkspaceEvent(
                node_crashed=ros_engine_pb2.NodeCrashed(node_name=name),
                timestamp_ns=time.time_ns()
            )
            self.event_queue.put(evt)

        self.known_nodes = nodes

        topics = self.get_topic_names_and_types()

        # Clean up subscription tracking for topics that no longer exist,
        # so they can be freshly re-subscribed if they reappear later
        current_topic_names = {name for name, _ in topics}
        stale = set(self.odom_subscribers.keys()) - current_topic_names
        for name in stale:
            del self.odom_subscribers[name]

        for name, types in topics:
            if 'nav_msgs/msg/Odometry' in types and name not in self.odom_subscribers:
                sub = self.create_subscription(
                    Odometry, name,
                    lambda msg, n=name: self.odom_callback(msg, n),
                    10
                )
                self.odom_subscribers[name] = sub
                self.get_logger().info(f'Subscribed to odometry topic: {name}')

        topic_msgs = [
            ros_engine_pb2.Topic(name=n, types=list(t)) for n, t in topics
        ]
        evt = ros_engine_pb2.WorkspaceEvent(
            topic_snapshot=ros_engine_pb2.TopicSnapshot(topics=topic_msgs),
            timestamp_ns=time.time_ns()
        )
        self.event_queue.put(evt)

    def publish_twist(self, topic_name, linear_x, angular_z):
        if topic_name not in self.twist_publishers:
            self.twist_publishers[topic_name] = self.create_publisher(Twist, topic_name, 10)
        msg = Twist()
        msg.linear.x = linear_x
        msg.angular.z = angular_z
        self.twist_publishers[topic_name].publish(msg)

class RosEngineServicer(ros_engine_pb2_grpc.RosEngineServicer):
    def __init__(self, event_queue: queue.Queue, ros_node):
        self.event_queue = event_queue
        self.ros_node = ros_node

    def PublishTwist(self, request, context):
        self.ros_node.publish_twist(request.topic_name, request.linear_x, request.angular_z)
        return ros_engine_pb2.Empty()

    def StreamEvents(self, request, context):
        print("=== Stream opened ===", flush=True)
        while context.is_active():
            try:
                evt = self.event_queue.get(timeout=1.0)
                yield evt
            except queue.Empty:
                continue


def ros_spin_thread(node):
    rclpy.spin(node)


def main():
    rclpy.init()
    event_queue = queue.Queue()
    node = RosIntrospector(event_queue)

    spin_thread = threading.Thread(target=ros_spin_thread, args=(node,), daemon=True)
    spin_thread.start()

    server = grpc.server(futures.ThreadPoolExecutor(max_workers=4))
    ros_engine_pb2_grpc.add_RosEngineServicer_to_server(
        RosEngineServicer(event_queue, node), server
    )
    server.add_insecure_port('[::]:50051')
    server.start()
    print("RosEngine gRPC server running on port 50051...")

    try:
        server.wait_for_termination()
    except KeyboardInterrupt:
        print("Shutting down...")
        stop_event = server.stop(grace=2)
        stop_event.wait(3)
        node.destroy_node()
        rclpy.shutdown()


if __name__ == '__main__':
    main()