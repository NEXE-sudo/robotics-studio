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

docker exec "$container" bash -lc \
  'source /opt/ros/jazzy/setup.bash && python3 -m grpc_tools.protoc -I /opt/robotics-studio/proto --python_out=/opt/robotics-studio/generated --grpc_python_out=/opt/robotics-studio/generated /opt/robotics-studio/proto/ros_engine.proto'

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
    'source /opt/ros/jazzy/setup.bash && cd /opt/robotics-studio && ROBOTICS_STUDIO_GENERATED_PATH=/opt/robotics-studio/generated exec python3 /opt/robotics-studio/server.py >/tmp/server.log 2>&1'
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
start_ros_server

ROBOTICS_STUDIO_TEST_TLS_CERTIFICATE="$workspace/grpc-ca.crt" \
ROBOTICS_STUDIO_TEST_AUTH_TOKEN="$token" \
  cargo test --manifest-path "$repo_root/src-tauri/Cargo.toml" --locked -- \
    --ignored --exact \
    ros_docker_integration_tests::pinned_tls_client_authenticates_to_docker_ros_service

docker exec -i "$container" bash -c 'cd /opt/robotics-studio && python3 -' <<'PY'
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
  "pkill -TERM -f '[p]ython3 /opt/robotics-studio/server.py' || true"
start_ros_server
docker exec -i "$container" bash -c 'cd /opt/robotics-studio && python3 -' <<'PY'
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
