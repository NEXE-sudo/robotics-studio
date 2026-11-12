#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
workspace="$(mktemp -d /tmp/robotics-studio-integration.XXXXXX)"
container="robotics-studio-integration-$$"
image="robotics-studio-ros2:integration"
container_may_exist=0

cleanup() {
    local exit_code=$?
    local container_removed=1
    local workspace_removed=0
    trap - EXIT

    if [[ "$container_may_exist" == 1 ]]; then
        if ! command -v docker >/dev/null 2>&1 || ! docker info >/dev/null 2>&1; then
            container_removed=0
            echo "WARNING: Docker is unavailable; cannot confirm integration container removal: $container" >&2
        elif ! docker rm -f "$container" >/dev/null 2>&1 &&
            docker inspect "$container" >/dev/null 2>&1; then
            container_removed=0
            echo "WARNING: Could not stop and remove integration container: $container" >&2
        fi
    fi

    if [[ "$container_removed" == 1 && -d "$workspace" ]]; then
        # The integration image runs as root and may create root-owned files
        # in the bind-mounted workspace. Remove those only after its container
        # has been stopped and removed.
        if command -v docker >/dev/null 2>&1 &&
            docker image inspect "$image" >/dev/null 2>&1 &&
            docker run --rm --user 0:0 \
                --mount "type=bind,source=$workspace,target=/cleanup" \
                --entrypoint /bin/bash "$image" -c \
                'find /cleanup -mindepth 1 -maxdepth 1 -exec rm -rf -- {} +' \
                >/dev/null 2>&1 &&
            rm -rf -- "$workspace" 2>/dev/null; then
            workspace_removed=1
        fi

        if [[ "$workspace_removed" == 0 ]] && rm -rf -- "$workspace" 2>/dev/null; then
            workspace_removed=1
        fi

        if [[ "$workspace_removed" == 0 ]] &&
            command -v sudo >/dev/null 2>&1 &&
            sudo -n true >/dev/null 2>&1 &&
            sudo -n rm -rf -- "$workspace" 2>/dev/null; then
            workspace_removed=1
        fi

        if [[ -e "$workspace" ]]; then
            echo "WARNING: Could not completely remove temporary directory: $workspace" >&2
        fi
    elif [[ "$container_removed" == 0 ]]; then
        echo "WARNING: Leaving bind-mounted temporary directory in place because the container may still exist: $workspace" >&2
    fi

    exit "$exit_code"
}

trap cleanup EXIT

docker info >/dev/null
docker build -t "$image" "$repo_root/src-tauri/resources/devcontainer"

cp "$repo_root/src-tauri/resources/devcontainer/diff_drive.sdf" "$workspace/diff_drive.sdf"
printf 'workspace-mount-ok\n' > "$workspace/mount-check"
mkdir -p "$workspace/worlds"
printf '<sdf version="1.9"><world name="invalid"><model name="bad"><link name="bad"><visual name="bad"><geometry><box><size>1</size></box></geometry></visual></link></model></world></sdf>\n' \
  > "$workspace/worlds/invalid-schema.sdf"
package_dir="$workspace/src/robotics_studio_integration_fixture"
mkdir -p "$package_dir/launch" "$package_dir/config"
cat > "$package_dir/package.xml" <<'XML'
<package format="3">
  <name>robotics_studio_integration_fixture</name>
  <version>1.0.0</version>
  <description>Temporary package discovery integration fixture</description>
  <buildtool_depend>ament_cmake</buildtool_depend>
  <depend>rclpy</depend>
</package>
XML
printf '<launch/>\n' > "$package_dir/launch/start.launch.xml"
printf 'enabled: true\n' > "$package_dir/config/settings.yaml"
mkdir -p "$workspace/install/lib/robotics_studio_integration_fixture"
printf '#!/bin/sh\n' > "$workspace/install/lib/robotics_studio_integration_fixture/fixture_node"
chmod +x "$workspace/install/lib/robotics_studio_integration_fixture/fixture_node"
mkdir -p "$workspace/other_workspace/src/not_selected"
printf '<package><name>not_selected</name></package>\n' \
  > "$workspace/other_workspace/src/not_selected/package.xml"
mkdir -p "$workspace/src/malformed_fixture"
printf '<package>\n' > "$workspace/src/malformed_fixture/package.xml"
container_may_exist=1
docker run -d \
  --name "$container" \
  -p 127.0.0.1:50051:50051 \
  -v "$workspace:/workspace" \
  "$image" tail -f /dev/null >/dev/null

docker exec "$container" mkdir -p \
  /opt/robotics-studio/proto \
  /opt/robotics-studio/generated \
  /run/robotics-studio
docker cp "$repo_root/src-tauri/resources/devcontainer/server.py" \
  "$container:/opt/robotics-studio/server.py" >/dev/null
docker cp "$repo_root/src-tauri/resources/devcontainer/server_core.py" \
  "$container:/opt/robotics-studio/server_core.py" >/dev/null
docker cp "$repo_root/src-tauri/resources/devcontainer/proto/ros_engine.proto" \
  "$container:/opt/robotics-studio/proto/ros_engine.proto" >/dev/null

docker exec -i "$container" bash -c 'cd /opt/robotics-studio && python3 -' <<'PY'
import sys

sys.path.insert(0, "/opt/robotics-studio")
from server_core import discover_workspace_packages

packages = {
    package["name"]: package
    for package in discover_workspace_packages("/workspace")
}
assert set(packages) == {
    "malformed_fixture",
    "robotics_studio_integration_fixture",
}, sorted(packages)
fixture = packages["robotics_studio_integration_fixture"]
assert fixture["package_type"] == "ament_cmake"
assert fixture["version"] == "1.0.0"
assert fixture["description"] == "Temporary package discovery integration fixture"
assert fixture["dependencies"] == ["ament_cmake", "rclpy"]
assert fixture["executables"] == ["fixture_node"]
assert fixture["launch_files"] == ["launch/start.launch.xml"]
assert fixture["config_files"] == ["config/settings.yaml"]
assert packages["malformed_fixture"]["metadata_error"]
print("Selected-workspace ROS package discovery passed")
PY

docker exec "$container" bash -lc \
  'source /opt/ros/jazzy/setup.bash && /opt/robotics-studio/venv/bin/python -m grpc_tools.protoc -I /opt/robotics-studio/proto --python_out=/opt/robotics-studio/generated --grpc_python_out=/opt/robotics-studio/generated /opt/robotics-studio/proto/ros_engine.proto'

docker exec "$container" bash -lc '
  set -eu
  chmod 700 /run/robotics-studio
  openssl req -x509 -newkey rsa:2048 -nodes -days 3650 \
    -keyout /run/robotics-studio/grpc-ca.key \
    -out /run/robotics-studio/grpc-ca.crt \
    -subj "/CN=Robotics Studio local gRPC CA" \
    -addext "basicConstraints=critical,CA:TRUE" \
    -addext "keyUsage=critical,keyCertSign,cRLSign" >/dev/null 2>&1
  openssl req -new -newkey rsa:2048 -nodes \
    -keyout /run/robotics-studio/grpc-server.key \
    -out /run/robotics-studio/grpc-server.csr \
    -subj "/CN=127.0.0.1" >/dev/null 2>&1
  printf "%s\n" \
    "basicConstraints=critical,CA:FALSE" \
    "keyUsage=critical,digitalSignature,keyEncipherment" \
    "extendedKeyUsage=serverAuth" \
    "subjectAltName=IP:127.0.0.1,DNS:localhost" \
    > /run/robotics-studio/grpc-server.ext
  openssl x509 -req -in /run/robotics-studio/grpc-server.csr \
    -CA /run/robotics-studio/grpc-ca.crt \
    -CAkey /run/robotics-studio/grpc-ca.key -CAcreateserial \
    -out /run/robotics-studio/grpc-server.crt -days 3650 -sha256 \
    -extfile /run/robotics-studio/grpc-server.ext >/dev/null 2>&1
  chmod 600 /run/robotics-studio/grpc-ca.key /run/robotics-studio/grpc-server.key
  chmod 644 /run/robotics-studio/grpc-ca.crt /run/robotics-studio/grpc-server.crt
'
docker exec "$container" cat /run/robotics-studio/grpc-ca.crt > "$workspace/grpc-ca.crt"

token="$(openssl rand -hex 32)"
expires_at="$(python3 -c 'import time; print(int(time.time()) + 86400)')"
printf '{"token":"%s","expires_at":%s}' "$token" "$expires_at" |
  docker exec -i "$container" bash -c \
    'cat > /run/robotics-studio/grpc-auth.json && chmod 600 /run/robotics-studio/grpc-auth.json'

start_ros_server() {
  docker exec -d "$container" bash -lc \
    'source /opt/ros/jazzy/setup.bash && cd /opt/robotics-studio && ROBOTICS_STUDIO_GENERATED_PATH=/opt/robotics-studio/generated exec /opt/robotics-studio/venv/bin/python /opt/robotics-studio/server.py >/tmp/server.log 2>&1'
  for attempt in $(seq 1 30); do
    if docker exec "$container" bash -lc \
      'grep -q "running on port 50051" /tmp/server.log' 2>/dev/null; then
      return
    fi
    sleep 1
  done
  docker exec "$container" cat /tmp/server.log
  return 1
}

test "$(docker exec "$container" cat /workspace/mount-check)" = workspace-mount-ok
docker exec "$container" bash -lc \
  'source /opt/ros/jazzy/setup.bash && gz sdf --check /workspace/diff_drive.sdf'
if docker exec "$container" bash -lc \
  'source /opt/ros/jazzy/setup.bash && gz sdf --check /workspace/worlds/invalid-schema.sdf' \
  >/dev/null 2>&1; then
  echo "Gazebo accepted an invalid SDF world" >&2
  exit 1
fi
echo "SDFormat schema validation accepted the valid world and rejected invalid geometry"
start_ros_server

docker exec -i "$container" bash -lc \
  'source /opt/ros/jazzy/setup.bash && cd /opt/robotics-studio && /opt/robotics-studio/venv/bin/python -' <<'PY'
import sys
import time

sys.path.insert(0, "/opt/robotics-studio/generated")
import grpc
import rclpy
import ros_engine_pb2 as pb
import ros_engine_pb2_grpc as rpc
from action_tutorials_interfaces.action import Fibonacci
from geometry_msgs.msg import TransformStamped
from rclpy.action import ActionClient, ActionServer
from rclpy.qos import DurabilityPolicy, QoSProfile, ReliabilityPolicy
from sensor_msgs.msg import JointState
from std_msgs.msg import String
from std_srvs.srv import Trigger
from tf2_msgs.msg import TFMessage

certificate = open("/run/robotics-studio/grpc-ca.crt", "rb").read()
token = open("/run/robotics-studio/grpc-auth.json", encoding="utf-8").read()
import json

metadata = (("x-robotics-studio-token", json.loads(token)["token"]),)
stub = rpc.RosEngineStub(
    grpc.secure_channel(
        "127.0.0.1:50051",
        grpc.ssl_channel_credentials(root_certificates=certificate),
    )
)
events = stub.ReplayEvents(
    pb.ReplayEventsRequest(has_after_sequence=True, after_sequence=0),
    metadata=metadata,
    timeout=10,
)

rclpy.init()
node = rclpy.create_node("robotics_studio_tf_integration")
service = node.create_service(
    Trigger,
    "/integration/echo",
    lambda request, response: response,
)
service_client = node.create_client(Trigger, "/integration/echo")
action_server = ActionServer(
    node,
    Fibonacci,
    "/integration/fibonacci",
    lambda goal_handle: Fibonacci.Result(),
)
action_client = ActionClient(node, Fibonacci, "/integration/fibonacci")
publisher = node.create_publisher(
    TFMessage,
    "/tf_static",
    QoSProfile(
        depth=10,
        reliability=ReliabilityPolicy.RELIABLE,
        durability=DurabilityPolicy.TRANSIENT_LOCAL,
    ),
)
dynamic_publisher = node.create_publisher(TFMessage, "/tf", 10)
joint_state_publisher = node.create_publisher(JointState, "/joint_states", 10)
inspection_publisher = node.create_publisher(
    String, "/integration/inspect", 10
)
deadline = time.monotonic() + 8
while (
    publisher.get_subscription_count() == 0
    or dynamic_publisher.get_subscription_count() == 0
    or joint_state_publisher.get_subscription_count() == 0
) and time.monotonic() < deadline:
    rclpy.spin_once(node, timeout_sec=0.1)
assert publisher.get_subscription_count() > 0, "ROS service did not subscribe to /tf_static"
assert (
    dynamic_publisher.get_subscription_count() > 0
), "ROS service did not subscribe to /tf"
assert (
    joint_state_publisher.get_subscription_count() > 0
), "ROS service did not subscribe to /joint_states"

inspection_deadline = time.monotonic() + 8
while True:
    try:
        stub.StartTopicInspection(
            pb.TopicInspectionRequest(topic_name="/integration/inspect"),
            metadata=metadata,
            timeout=3,
        )
        break
    except grpc.RpcError as error:
        if (
            error.code() != grpc.StatusCode.NOT_FOUND
            or time.monotonic() >= inspection_deadline
        ):
            raise
        time.sleep(0.1)

try:
    stub.StartTopicInspection(
        pb.TopicInspectionRequest(topic_name="/invalid//topic"),
        metadata=metadata,
        timeout=3,
    )
except grpc.RpcError as error:
    assert error.code() == grpc.StatusCode.INVALID_ARGUMENT
else:
    raise AssertionError("ROS service accepted an invalid inspection topic name")

transform = TransformStamped()
transform.header.frame_id = "map"
transform.child_frame_id = "integration_base"
transform.header.stamp.sec = 1
transform.header.stamp.nanosec = 234_000_000
transform.transform.translation.x = 0.25
transform.transform.rotation.w = 1.0
publisher.publish(TFMessage(transforms=[transform]))
dynamic_transform = TransformStamped()
dynamic_transform.header.frame_id = "odom"
dynamic_transform.child_frame_id = "integration_dynamic_base"
dynamic_transform.header.stamp.sec = 5
dynamic_transform.header.stamp.nanosec = 678_000_000
dynamic_transform.transform.translation.x = 1.5
dynamic_transform.transform.rotation.w = 1.0
dynamic_publisher.publish(TFMessage(transforms=[dynamic_transform]))
joint_state = JointState()
joint_state.name = ["integration_joint"]
joint_state.position = [0.5]
joint_state_publisher.publish(joint_state)

static_tf_verified = False
dynamic_tf_verified = False
graph_verified = False
joint_state_verified = False
topic_inspection_verified = False
topic_inspection_stopped = False
inspection_data = "x" * 600
for event in events:
    if not topic_inspection_verified:
        inspection_publisher.publish(String(data=inspection_data))
    event_type = event.WhichOneof("event")
    if event_type == "tf_update":
        if event.tf_update.topic_name == "/tf_static":
            assert event.tf_update.parent_frame == "map"
            assert event.tf_update.child_frame == "integration_base"
            assert abs(event.tf_update.x - 0.25) < 1e-9
            assert event.tf_update.source_timestamp_ms == 1234
            static_tf_verified = True
        elif event.tf_update.topic_name == "/tf":
            assert event.tf_update.parent_frame == "odom"
            assert event.tf_update.child_frame == "integration_dynamic_base"
            assert abs(event.tf_update.x - 1.5) < 1e-9
            assert event.tf_update.source_timestamp_ms == 5678
            dynamic_tf_verified = True
    elif event_type == "joint_state_update":
        joint_state_verified = (
            event.joint_state_update.topic_name == "/joint_states"
            and event.joint_state_update.names == ["integration_joint"]
            and list(event.joint_state_update.positions) == [0.5]
        )
    elif event_type == "topic_inspection_update":
        update = event.topic_inspection_update
        if update.topic_name == "/integration/inspect":
            if update.json_payload:
                assert update.active
                import json

                assert update.message_type == "std_msgs/msg/String"
                assert json.loads(update.json_payload) == {
                    "data": inspection_data[:512]
                }
                assert update.truncated
                topic_inspection_verified = True
                stub.StopTopicInspection(
                    pb.TopicInspectionRequest(topic_name=update.topic_name),
                    metadata=metadata,
                    timeout=3,
                )
            elif not update.active:
                topic_inspection_stopped = True
    elif event_type == "ros_graph_snapshot":
        graph = event.ros_graph_snapshot
        tf_topic = next(
            (topic for topic in graph.topics if topic.name == "/tf_static"),
            None,
        )
        service_info = next(
            (service for service in graph.services if service.name == "/integration/echo"),
            None,
        )
        action_info = next(
            (action for action in graph.actions if action.name == "/integration/fibonacci"),
            None,
        )
        if tf_topic and service_info and action_info:
            graph_verified = (
                any(
                endpoint.node_name == "robotics_studio_tf_integration"
                and endpoint.reliability == "RELIABLE"
                and endpoint.durability == "TRANSIENT_LOCAL"
                for endpoint in tf_topic.publishers
                )
                and "/robotics_studio_tf_integration" in service_info.server_nodes
                and "/robotics_studio_tf_integration" in service_info.client_nodes
                and action_info.types
                == ["action_tutorials_interfaces/action/Fibonacci"]
                and "/robotics_studio_tf_integration" in action_info.server_nodes
                and "/robotics_studio_tf_integration" in action_info.client_nodes
            )
    if (
        static_tf_verified
        and dynamic_tf_verified
        and graph_verified
        and joint_state_verified
        and topic_inspection_verified
        and topic_inspection_stopped
    ):
        break
else:
    raise AssertionError(
        "ROS service did not stream TF, joint-state, graph, and inspected topic details"
    )
if not graph_verified:
    raise AssertionError("ROS graph snapshot omitted the /tf_static publisher endpoint")
if not joint_state_verified:
    raise AssertionError("ROS service did not stream the joint-state update")
if not dynamic_tf_verified:
    raise AssertionError("ROS service did not stream the dynamic TF update")
if not topic_inspection_verified or not topic_inspection_stopped:
    raise AssertionError("ROS topic inspection did not stream and stop cleanly")

events.cancel()
node.destroy_node()
rclpy.shutdown()
print("Live ROS TF, joint-state, graph, and bounded topic inspection passed")
PY

ROBOTICS_STUDIO_TEST_TLS_CERTIFICATE="$workspace/grpc-ca.crt" \
ROBOTICS_STUDIO_TEST_AUTH_TOKEN="$token" \
  cargo test --manifest-path "$repo_root/src-tauri/Cargo.toml" --locked -- \
    --ignored --exact \
    ros_docker_integration_tests::pinned_tls_client_authenticates_to_docker_ros_service

docker exec -i "$container" bash -c 'cd /opt/robotics-studio && /opt/robotics-studio/venv/bin/python -' <<'PY'
import json
import sys
import time

sys.path.insert(0, "/opt/robotics-studio/generated")
import grpc
import ros_engine_pb2 as pb
import ros_engine_pb2_grpc as rpc

certificate = open("/run/robotics-studio/grpc-ca.crt", "rb").read()
token = json.load(open("/run/robotics-studio/grpc-auth.json"))["token"]
stub = rpc.RosEngineStub(
    grpc.secure_channel(
        "127.0.0.1:50051",
        grpc.ssl_channel_credentials(root_certificates=certificate),
    )
)
metadata = (("x-robotics-studio-token", token),)
try:
    stub.GetSafetyState(pb.Empty(), timeout=3)
    raise AssertionError("unauthenticated request was accepted")
except grpc.RpcError as error:
    assert error.code() == grpc.StatusCode.UNAUTHENTICATED, error

assert stub.GetSafetyState(pb.Empty(), metadata=metadata, timeout=3).state == pb.SafetyState.READY
activated = stub.SetEmergencyStop(
    pb.EmergencyStopRequest(action=pb.EmergencyStopRequest.ACTIVATE),
    metadata=metadata,
    timeout=3,
)
assert activated.state == pb.SafetyState.ACTIVE
try:
    stub.PublishTwist(
        pb.TwistCommand(
            topic_name="/audit/cmd_vel",
            linear_x=0.1,
            timestamp_ns=time.time_ns(),
        ),
        metadata=metadata,
        timeout=3,
    )
    raise AssertionError("motion command was accepted while latched")
except grpc.RpcError as error:
    assert error.code() == grpc.StatusCode.INVALID_ARGUMENT, error
assert stub.SetEmergencyStop(
    pb.EmergencyStopRequest(action=pb.EmergencyStopRequest.RESET),
    metadata=metadata,
    timeout=3,
).state == pb.SafetyState.READY

events = stub.ReplayEvents(
    pb.ReplayEventsRequest(has_after_sequence=True, after_sequence=0),
    metadata=metadata,
    timeout=5,
)
assert next(events).WhichOneof("event") == "event_recovery"
assert next(events).WhichOneof("event") == "safety_state"
events.cancel()
print("Authenticated TLS, stop/reset enforcement and event replay passed")
PY

docker exec "$container" bash -lc \
  "pkill -TERM -f '[v]env/bin/python /opt/robotics-studio/server.py' || true"
start_ros_server
docker exec -i "$container" bash -c 'cd /opt/robotics-studio && /opt/robotics-studio/venv/bin/python -' <<'PY'
import json
import sys

sys.path.insert(0, "/opt/robotics-studio/generated")
import grpc
import ros_engine_pb2 as pb
import ros_engine_pb2_grpc as rpc

root = open("/run/robotics-studio/grpc-ca.crt", "rb").read()
token = json.load(open("/run/robotics-studio/grpc-auth.json"))["token"]
stub = rpc.RosEngineStub(
    grpc.secure_channel(
        "127.0.0.1:50051",
        grpc.ssl_channel_credentials(root_certificates=root),
    )
)
events = stub.ReplayEvents(
    pb.ReplayEventsRequest(has_after_sequence=True, after_sequence=0),
    metadata=(("x-robotics-studio-token", token),),
    timeout=5,
)
assert next(events).WhichOneof("event") == "event_recovery"
event = next(events)
assert event.WhichOneof("event") == "safety_state" and event.sequence > 0
events.cancel()
print("Critical safety history survived a ROS service restart")
PY

docker exec -d "$container" bash -lc \
  'source /opt/ros/jazzy/setup.bash && gz sim -s -r --headless-rendering /workspace/diff_drive.sdf >/tmp/gz.log 2>&1'
world_services=""
for attempt in $(seq 1 30); do
  if world_services="$(docker exec "$container" bash -lc \
    'source /opt/ros/jazzy/setup.bash && gz service -l' 2>/dev/null)" &&
    printf '%s\n' "$world_services" | grep -Fx '/world/diff_drive/control' >/dev/null; then
    break
  fi
  sleep 1
done
if ! printf '%s\n' "$world_services" | grep -Fx '/world/diff_drive/control' >/dev/null; then
  docker exec "$container" cat /tmp/gz.log
  echo "Gazebo did not advertise /world/diff_drive/control" >&2
  exit 1
fi
world_control='/world/diff_drive/control'
docker exec "$container" bash -lc \
  'source /opt/ros/jazzy/setup.bash && gz topic -l' |
  grep -F '/model/vehicle_blue' >/dev/null
docker exec "$container" bash -c \
  'source /opt/ros/jazzy/setup.bash && gz service -s "$1" --reqtype gz.msgs.WorldControl --reptype gz.msgs.Boolean --timeout 2000 --req "reset: {all: true}"' \
  robotics-studio-integration "$world_control"

docker exec -d "$container" bash -lc \
  'source /opt/ros/jazzy/setup.bash && ros2 run ros_gz_bridge parameter_bridge /model/vehicle_blue/cmd_vel@geometry_msgs/msg/Twist@gz.msgs.Twist /model/vehicle_blue/odometry@nav_msgs/msg/Odometry@gz.msgs.Odometry /model/vehicle_blue/tf@tf2_msgs/msg/TFMessage@gz.msgs.Pose_V >/tmp/bridge.log 2>&1'
for attempt in $(seq 1 20); do
  if topics="$(docker exec "$container" bash -lc \
    'source /opt/ros/jazzy/setup.bash && ros2 topic list' 2>/dev/null)" &&
    printf '%s\n' "$topics" | grep -Fx '/model/vehicle_blue/cmd_vel' >/dev/null; then
    break
  fi
  sleep 1
done
docker exec "$container" bash -lc \
  'source /opt/ros/jazzy/setup.bash && ros2 topic list' |
  grep -Fx '/model/vehicle_blue/cmd_vel' >/dev/null
docker exec "$container" bash -lc \
  "pkill -TERM -f '[g]z sim -s' || true; pkill -TERM -f '[p]arameter_bridge' || true"

echo "Docker ROS 2 Jazzy and Gazebo Harmonic integration checks passed"
