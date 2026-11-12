# Complete Git Diff

Base: `e9d7459` on `main`; generated from the current worktree with `COMPLETE_DIFF.md` excluded from its own diff.

Validation: frontend production build and all 11 frontend tests passed; Rust format/check, 43 tests (1 ignored), and strict Clippy passed; Python unittest discovery passed (43 tests); shell syntax and Python syntax validation passed; full ROS 2 Jazzy/Gazebo Harmonic Docker integration passed, including live bounded topic inspection, with no temporary integration directory or container left behind. The macOS x86_64 Tauri release bundle built and its DMG checksum verified. Existing dependency-audit results remain npm audit with 0 vulnerabilities and cargo audit with 13 warnings; no dependency manifests were changed. The v0.3.0 mission remains incomplete and is not release-ready. No commit, push, tag, or release was created.

## Full diff

```diff

diff --git a/CHANGELOG.md b/CHANGELOG.md
index 177bdca..b353d42 100644
--- a/CHANGELOG.md
+++ b/CHANGELOG.md
@@ -16,7 +16,13 @@ This version is still in development and is not release-ready. See
 - Bounded read-only ROS graph snapshots with node namespaces, topic
   publisher/subscriber identities and QoS, service types/endpoints, and action
   server/client discovery, grouped by fully qualified node identity in the UI.
+- Optional bounded live topic inspection with executor-owned subscriptions,
+  authenticated start/stop controls, rate and payload limits, and serialized
+  message previews.
 - Reliable transient-local subscription to `/tf_static`.
+- TF inspection preserves each transform's ROS source stamp separately from
+  receive time; reconnect state shows disconnected last-known data and stale
+  dynamic transforms.
 - Bounded URDF parsing for primitive visuals and connected link trees, with
   safe rejection of malformed or unsupported descriptions.
 - Three.js rendering for box, cylinder, and sphere visuals and supported
@@ -28,7 +34,8 @@ This version is still in development and is not release-ready. See
 - Docker integration coverage for selected-workspace package discovery,
   malformed package manifests, installed executables, workspace isolation,
   live ROS graph endpoints, static and dynamic TF plus `/joint_states`
-  delivery, and valid/invalid SDFormat schema checks.
+  delivery, bounded live topic inspection, and valid/invalid SDFormat schema
+  checks.
 - Build-process state and cancellation feedback that distinguishes a stop
   request from confirmed process termination.

diff --git a/README.md b/README.md
index 4c292aa..57afd49 100644
--- a/README.md
+++ b/README.md
@@ -178,14 +178,18 @@ any of this — the app manages the environment for you.
   a first pass and still needs convention validation.
 - The TF tab builds a live transform forest and warns about multiple roots,
   cycles, multiple parents, stale dynamic transforms, non-finite values, and
-  invalid quaternions. Static `/tf_static` edges use transient-local QoS and
-  are not marked stale. This is diagnostic validation, not a substitute for
-  `tf2` authority or full ROS graph validation.
-- ROS graph snapshots are bounded and read-only. Service invocation and
-  action goal submission/monitoring are not implemented because arbitrary
-  operations could bypass the motion safety path. Package discovery reads
-  source package manifests and workspace install outputs, but does not show
-  every underlay package.
+  invalid quaternions. It displays the ROS source stamp separately from the
+  local receive age, so replayed old data does not appear freshly received.
+  Disconnection is explicit; static `/tf_static` edges use transient-local
+  QoS and are not marked stale. This is diagnostic validation, not a
+  substitute for `tf2` authority or full ROS graph validation.
+- ROS graph snapshots are bounded and read-only, with optional live topic
+  inspection limited to four topics, five samples per second per topic, and
+  bounded serialized payloads. Service invocation and action goal
+  submission/monitoring are not implemented because arbitrary operations
+  could bypass the motion safety path. Package discovery reads source
+  package manifests and workspace install outputs, but does not show every
+  underlay package.
 - SDF imports are structurally checked, and the world is validated against
   the installed SDFormat schema before Gazebo starts. This does not resolve or
   validate every external model/mesh resource.
diff --git a/RELEASE_READINESS.md b/RELEASE_READINESS.md
index be7a3cb..a919edb 100644
--- a/RELEASE_READINESS.md
+++ b/RELEASE_READINESS.md
@@ -2,12 +2,12 @@

 ## Decision

-**Not ready for release.** The worktree is based on `307f3ef` on `main` and
+**Not ready for release.** The worktree is based on `e9d7459` on `main` and
 contains uncommitted ROS graph, TF integration, URDF preview, build lifecycle,
-and package-discovery test changes. Local validation and Docker integration
-pass, but substantial v0.3.0 acceptance criteria remain incomplete. Hosted CI
-has not run against this worktree. No commit, push, tag, or release was
-created.
+package-discovery test changes, and bounded live ROS topic inspection. Local
+validation and Docker integration pass, but substantial v0.3.0 acceptance
+criteria remain incomplete. Hosted CI has not run against this worktree. No
+commit, push, tag, or release was created.

 ## Acceptance checklist

@@ -22,20 +22,20 @@ created.
 | 7 | Workspace selection and isolation | PARTIAL | Selected workspace is persisted and passed to container lookup/build/simulation. Package discovery now filters to source/install trees inside that mounted workspace; full operation/platform matrix remains unverified. |
 | 8 | ROS package explorer | PARTIAL | Selected-workspace package metadata, build type, dependencies, executable outputs, launch/config files, filtering, refresh, and file browsing are implemented and tested. Underlay packages, dependency resolution, and full package content workflows remain out of scope. |
 | 9 | ROS node explorer | PARTIAL | Bounded node names/namespaces and per-topic endpoint node identities are streamed and grouped in the UI by fully qualified node identity, including service/action roles. Full ROS lifecycle-state details remain. |
-| 10 | ROS topic explorer | PARTIAL | Bounded publisher/subscriber identities and QoS policies are streamed and shown. Safe live-message inspection and subscription controls are not implemented. |
+| 10 | ROS topic explorer | PARTIAL | Bounded publisher/subscriber identities and QoS policies are streamed and shown. Authenticated live-message inspection is available with executor-owned subscriptions, four-topic/five-Hz/payload bounds, and stop controls. ROS topic type support depends on installed message packages; broader UX and lifecycle testing remain. |
 | 11 | ROS service explorer | PARTIAL | Bounded service names/types and discovered server/client nodes are shown read-only. Structured invocation, validation, timeout, and cancellation remain absent to avoid bypassing safety controls. |
 | 12 | ROS action explorer | PARTIAL | Action names/types and discovered server/client nodes are shown read-only. Goal submission, feedback/results, and cancellation are not implemented. |
 | 13 | Consistent ROS graph refresh | PARTIAL | Nodes, topics, services, and actions are snapshotted from one service timer and bounded. Automatic client subscriptions and full graph transactionality remain absent. |
 | 14 | TF hierarchy and transform inspection | PARTIAL | A UI forest, translation/quaternion/RPY inspector, multiple-root/multiple-parent/cycle checks, and pure unit tests are implemented. It is not authoritative `tf2` validation. |
-| 15 | TF stale and static-frame behavior | PARTIAL | Dynamic edges become stale after five seconds; `/tf_static` uses reliable transient-local QoS and is exempt. Docker integration publishes and verifies both static and dynamic TF. Missing authorities cannot be inferred from edge data. |
+| 15 | TF stale and static-frame behavior | PARTIAL | Dynamic edges become stale after five seconds using event receive time, independent of the preserved ROS source stamp; `/tf_static` uses reliable transient-local QoS and is exempt. The UI labels last-known data on disconnect. Docker integration verifies static/dynamic delivery and source stamps. Frame disappearance/relationship changes and full TF composition remain unverified. |
 | 16 | URDF parsing and validation | PARTIAL | A bounded parser rejects malformed/unsafe XML, validates link/joint-tree structure, primitive dimensions, joint limits, and mimic references/cycles, and reports unsupported mesh visuals. Full URDF semantics, Xacro, and resource validation remain. |
 | 17 | Xacro expansion | NOT STARTED | No controlled ROS Xacro expansion workflow exists. |
 | 18 | Mesh/resource resolution | NOT STARTED | `package://`, relative model paths, and mesh formats are not resolved for rendering. |
 | 19 | Robot model and joint-state rendering | PARTIAL | Active `.urdf` files render box/cylinder/sphere visuals and supported joint transforms; limits, mimic joints, and a collision-geometry toggle are supported. Bounded `/joint_states` updates are covered by Docker integration. No mesh/Xacro, coordinate-convention verification, or visual desktop test yet; inertial and named robot-level materials are not rendered. |
 | 20 | Process lifecycle manager | PARTIAL | Build states distinguish starting/running/stopping/terminal outcomes; stop requests are only reported as cancellation after process exit. Docker, Gazebo, bridge, and ROS service processes still lack a shared lifecycle manager, generalized timeouts, and application-shutdown cleanup. |
 | 21 | Gazebo and SDF | PARTIAL | Jazzy/Harmonic startup, reset, bridge, and pre-launch `gz sdf --check` schema validation are integration-tested. Comprehensive external resource resolution and validation remain absent. |
-| 22 | Frontend workflow tests/accessibility | PARTIAL | Ten dependency-free TF graph and robot-rendering tests plus production type-check/build pass. React workflow tests, accessibility review, and coverage of workspace/ROS/safety UI remain. |
-| 23 | Cross-platform runtime and release artifacts | BLOCKED | CI has Linux checks and a release workflow matrix for macOS/Linux/Windows, but no current packaged artifacts or runtime verification on every platform were produced. Signing/notarization and upgrade verification remain. |
+| 22 | Frontend workflow tests/accessibility | PARTIAL | Eleven dependency-free TF graph and robot-rendering tests plus production type-check/build pass. React workflow tests, accessibility review, and coverage of workspace/ROS/safety UI remain. |
+| 23 | Cross-platform runtime and release artifacts | PARTIAL | A local macOS x86_64 `.app` and `.dmg` were built successfully. They have not been launch-tested or notarized; macOS Apple Silicon, Linux, and Windows release builds/runtime and v0.2.0 upgrade verification remain. |

 ## Security and safety notes

@@ -69,8 +69,8 @@ created.
 | Check | Result |
 |---|---|
 | `npm ci` | Pass; 91 packages installed. |
-| `npm run build` | Pass; Vite reports a large JavaScript chunk warning (~831 kB). |
-| `npm run test:frontend` | Pass: 10 TF graph and robot-description tests. |
+| `npm run build` | Pass; Vite reports a large JavaScript chunk warning (~832 kB). |
+| `npm run test:frontend` | Pass: 11 TF graph and robot-description tests. |
 | `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Pass. |
 | `cargo check --manifest-path src-tauri/Cargo.toml --locked` | Pass. |
 | `cargo test --manifest-path src-tauri/Cargo.toml --locked` | Pass: 43 passed, 1 ignored (live-service test). |
@@ -78,7 +78,8 @@ created.
 | Python unittest discovery from `src-tauri/resources/devcontainer` | Pass: 43 tests, including inaccessible workspace-tree reporting. |
 | Python `py_compile` for service and tests | Pass. |
 | `bash -n scripts/docker-integration.sh` | Pass. |
-| `scripts/docker-integration.sh` | Pass: selected-workspace package discovery, live graph endpoints, static/dynamic TF and joint-state delivery, SDFormat checks, pinned TLS/auth, stop/reset enforcement, event replay/restart, and Gazebo. Cleanup exited 0; a follow-up check found no integration container or temporary directory. |
+| `scripts/docker-integration.sh` | Pass: selected-workspace package discovery, live graph endpoints, bounded topic inspection and stop, static/dynamic TF and joint-state delivery, SDFormat checks, pinned TLS/auth, stop/reset enforcement, event replay/restart, and Gazebo. Cleanup exited 0; a follow-up check found no integration container or temporary directory. |
+| `npm run tauri build` | Pass: built macOS x86_64 `.app` and `.dmg`; `hdiutil verify` confirms the DMG checksum. Application launch, signing/notarization, and non-macOS bundles were not verified. |
 | `cargo audit --file src-tauri/Cargo.lock` | Pass with 13 allowed warnings: no vulnerability findings; warnings include unmaintained crates and unsound `glib`. |
 | `npm audit --audit-level=moderate` | Pass: 0 vulnerabilities. |
 | Python dependency audit | Pass: no known vulnerabilities in the isolated environment's pinned `grpcio==1.85.1`, `grpcio-tools==1.85.1`, `protobuf==7.36.2`, `setuptools==84.2.1`, and `typing_extensions==4.16.0`. The full system pip inventory could not be resolved because it includes ROS distro packages. |
@@ -100,7 +101,9 @@ created.
 - Complete filesystem race hardening for remaining operations and Windows.
 - Add frontend workflow tests and conduct accessibility review.
 - Run hosted CI on the modified worktree; verify Linux, Windows, macOS Intel,
-  and Apple Silicon packaging/runtime paths and test upgrades from v0.2.0.
+  and Apple Silicon packaging/runtime paths and test upgrades from v0.2.0. Only
+  the local macOS x86_64 bundle build has been produced; it has not been
+  launch-tested or notarized.
 - Complete external SDF model/mesh resource resolution and validation.
 - Re-run current dependency audits and reassess disclosed RustSec advisories.
 - Prepare and verify unsigned/signed release artifacts as applicable; do not
diff --git a/docs/architecture.md b/docs/architecture.md
index 9867025..3015e0b 100644
--- a/docs/architecture.md
+++ b/docs/architecture.md
@@ -98,16 +98,23 @@ selected workspace's source and install trees rather than treating ROS
 underlay packages as workspace packages. It reports bounded metadata,
 installed executables, launch/configuration files, and allows the frontend to
 browse package files through approved-root file commands. The graph explorer
-is read-only: it does not invoke arbitrary ROS services or submit action goals,
-which could otherwise bypass the application's motion safety path.
+supports bounded live topic inspection through subscriptions created and
+destroyed on the ROS executor. Inspection is authenticated and limited to four
+topics, five messages per second per topic, depth-five best-effort queues, and
+16 KiB serialized payloads. It does not invoke arbitrary ROS services or submit
+action goals, which could otherwise bypass the application's motion safety
+path.

 TF updates include their source topic. `/tf_static` uses transient-local
 reliable QoS so late subscribers receive latched static transforms. The
 frontend constructs a transform forest, warns on multiple roots, cycles,
 multiple parents, stale dynamic transforms, non-finite values, and invalid
-quaternions, and provides translation/quaternion/RPY inspection. It cannot
-infer missing transform authorities from edge data alone, and is diagnostic
-validation rather than a replacement for `tf2`.
+quaternions, and provides translation/quaternion/RPY inspection. ROS message
+source stamps are retained independently from local event receive time; stale
+replayed dynamic transforms are not made fresh by reconnect. The UI explicitly
+labels disconnected transform data as last-known. It cannot infer missing
+transform authorities from edge data alone, and is diagnostic validation
+rather than a replacement for `tf2`.

 When an active editor file is a `.urdf`, the native backend parses a bounded
 URDF subset and the frontend renders box, cylinder, and sphere visuals with a
diff --git a/scripts/docker-integration.sh b/scripts/docker-integration.sh
index 31791ab..5526a0c 100755
--- a/scripts/docker-integration.sh
+++ b/scripts/docker-integration.sh
@@ -214,6 +214,7 @@ from geometry_msgs.msg import TransformStamped
 from rclpy.action import ActionClient, ActionServer
 from rclpy.qos import DurabilityPolicy, QoSProfile, ReliabilityPolicy
 from sensor_msgs.msg import JointState
+from std_msgs.msg import String
 from std_srvs.srv import Trigger
 from tf2_msgs.msg import TFMessage

@@ -260,6 +261,9 @@ publisher = node.create_publisher(
 )
 dynamic_publisher = node.create_publisher(TFMessage, "/tf", 10)
 joint_state_publisher = node.create_publisher(JointState, "/joint_states", 10)
+inspection_publisher = node.create_publisher(
+    String, "/integration/inspect", 10
+)
 deadline = time.monotonic() + 8
 while (
     publisher.get_subscription_count() == 0
@@ -275,15 +279,47 @@ assert (
     joint_state_publisher.get_subscription_count() > 0
 ), "ROS service did not subscribe to /joint_states"

+inspection_deadline = time.monotonic() + 8
+while True:
+    try:
+        stub.StartTopicInspection(
+            pb.TopicInspectionRequest(topic_name="/integration/inspect"),
+            metadata=metadata,
+            timeout=3,
+        )
+        break
+    except grpc.RpcError as error:
+        if (
+            error.code() != grpc.StatusCode.NOT_FOUND
+            or time.monotonic() >= inspection_deadline
+        ):
+            raise
+        time.sleep(0.1)
+
+try:
+    stub.StartTopicInspection(
+        pb.TopicInspectionRequest(topic_name="/invalid//topic"),
+        metadata=metadata,
+        timeout=3,
+    )
+except grpc.RpcError as error:
+    assert error.code() == grpc.StatusCode.INVALID_ARGUMENT
+else:
+    raise AssertionError("ROS service accepted an invalid inspection topic name")
+
 transform = TransformStamped()
 transform.header.frame_id = "map"
 transform.child_frame_id = "integration_base"
+transform.header.stamp.sec = 1
+transform.header.stamp.nanosec = 234_000_000
 transform.transform.translation.x = 0.25
 transform.transform.rotation.w = 1.0
 publisher.publish(TFMessage(transforms=[transform]))
 dynamic_transform = TransformStamped()
 dynamic_transform.header.frame_id = "odom"
 dynamic_transform.child_frame_id = "integration_dynamic_base"
+dynamic_transform.header.stamp.sec = 5
+dynamic_transform.header.stamp.nanosec = 678_000_000
 dynamic_transform.transform.translation.x = 1.5
 dynamic_transform.transform.rotation.w = 1.0
 dynamic_publisher.publish(TFMessage(transforms=[dynamic_transform]))
@@ -296,18 +332,25 @@ static_tf_verified = False
 dynamic_tf_verified = False
 graph_verified = False
 joint_state_verified = False
+topic_inspection_verified = False
+topic_inspection_stopped = False
+inspection_data = "x" * 600
 for event in events:
+    if not topic_inspection_verified:
+        inspection_publisher.publish(String(data=inspection_data))
     event_type = event.WhichOneof("event")
     if event_type == "tf_update":
         if event.tf_update.topic_name == "/tf_static":
             assert event.tf_update.parent_frame == "map"
             assert event.tf_update.child_frame == "integration_base"
             assert abs(event.tf_update.x - 0.25) < 1e-9
+            assert event.tf_update.source_timestamp_ms == 1234
             static_tf_verified = True
         elif event.tf_update.topic_name == "/tf":
             assert event.tf_update.parent_frame == "odom"
             assert event.tf_update.child_frame == "integration_dynamic_base"
             assert abs(event.tf_update.x - 1.5) < 1e-9
+            assert event.tf_update.source_timestamp_ms == 5678
             dynamic_tf_verified = True
     elif event_type == "joint_state_update":
         joint_state_verified = (
@@ -315,6 +358,26 @@ for event in events:
             and event.joint_state_update.names == ["integration_joint"]
             and list(event.joint_state_update.positions) == [0.5]
         )
+    elif event_type == "topic_inspection_update":
+        update = event.topic_inspection_update
+        if update.topic_name == "/integration/inspect":
+            if update.json_payload:
+                assert update.active
+                import json
+
+                assert update.message_type == "std_msgs/msg/String"
+                assert json.loads(update.json_payload) == {
+                    "data": inspection_data[:512]
+                }
+                assert update.truncated
+                topic_inspection_verified = True
+                stub.StopTopicInspection(
+                    pb.TopicInspectionRequest(topic_name=update.topic_name),
+                    metadata=metadata,
+                    timeout=3,
+                )
+            elif not update.active:
+                topic_inspection_stopped = True
     elif event_type == "ros_graph_snapshot":
         graph = event.ros_graph_snapshot
         tf_topic = next(
@@ -349,11 +412,13 @@ for event in events:
         and dynamic_tf_verified
         and graph_verified
         and joint_state_verified
+        and topic_inspection_verified
+        and topic_inspection_stopped
     ):
         break
 else:
     raise AssertionError(
-        "ROS service did not stream static/dynamic TF, joint-state, and graph details"
+        "ROS service did not stream TF, joint-state, graph, and inspected topic details"
     )
 if not graph_verified:
     raise AssertionError("ROS graph snapshot omitted the /tf_static publisher endpoint")
@@ -361,11 +426,13 @@ if not joint_state_verified:
     raise AssertionError("ROS service did not stream the joint-state update")
 if not dynamic_tf_verified:
     raise AssertionError("ROS service did not stream the dynamic TF update")
+if not topic_inspection_verified or not topic_inspection_stopped:
+    raise AssertionError("ROS topic inspection did not stream and stop cleanly")

 events.cancel()
 node.destroy_node()
 rclpy.shutdown()
-print("Live ROS static/dynamic TF, joint-state, and graph endpoint discovery passed")
+print("Live ROS TF, joint-state, graph, and bounded topic inspection passed")
 PY

 ROBOTICS_STUDIO_TEST_TLS_CERTIFICATE="$workspace/grpc-ca.crt" \
diff --git a/src-tauri/proto/ros_engine.proto b/src-tauri/proto/ros_engine.proto
index b92cfca..9fe06e9 100644
--- a/src-tauri/proto/ros_engine.proto
+++ b/src-tauri/proto/ros_engine.proto
@@ -12,6 +12,12 @@ service RosEngine {
   rpc GetRobotConfig(RobotConfigRequest) returns (RobotConfig);
   rpc SetRobotConfig(RobotConfig) returns (RobotConfig);
   rpc GetEventHealth(Empty) returns (EventHealth);
+  rpc StartTopicInspection(TopicInspectionRequest) returns (Empty);
+  rpc StopTopicInspection(TopicInspectionRequest) returns (Empty);
+}
+
+message TopicInspectionRequest {
+  string topic_name = 1;
 }

 message TwistCommand {
@@ -96,11 +102,21 @@ message WorkspaceEvent {
     EventRecovery event_recovery = 9;
     RosGraphSnapshot ros_graph_snapshot = 12;
     JointStateUpdate joint_state_update = 13;
+    TopicInspectionUpdate topic_inspection_update = 14;
   }
   int64 timestamp_ns = 10;
   uint64 sequence = 11;
 }

+message TopicInspectionUpdate {
+  string topic_name = 1;
+  string message_type = 2;
+  string json_payload = 3;
+  bool active = 4;
+  bool truncated = 5;
+  string error = 6;
+}
+
 message JointStateUpdate {
   string topic_name = 1;
   repeated string names = 2;
@@ -175,6 +191,7 @@ message TFUpdate {
   double qz = 8;
   double qw = 9;
   string topic_name = 10;
+  int64 source_timestamp_ms = 11;
 }

 message NodeStarted {
diff --git a/src-tauri/resources/devcontainer/proto/ros_engine.proto b/src-tauri/resources/devcontainer/proto/ros_engine.proto
index b92cfca..9fe06e9 100644
--- a/src-tauri/resources/devcontainer/proto/ros_engine.proto
+++ b/src-tauri/resources/devcontainer/proto/ros_engine.proto
@@ -12,6 +12,12 @@ service RosEngine {
   rpc GetRobotConfig(RobotConfigRequest) returns (RobotConfig);
   rpc SetRobotConfig(RobotConfig) returns (RobotConfig);
   rpc GetEventHealth(Empty) returns (EventHealth);
+  rpc StartTopicInspection(TopicInspectionRequest) returns (Empty);
+  rpc StopTopicInspection(TopicInspectionRequest) returns (Empty);
+}
+
+message TopicInspectionRequest {
+  string topic_name = 1;
 }

 message TwistCommand {
@@ -96,11 +102,21 @@ message WorkspaceEvent {
     EventRecovery event_recovery = 9;
     RosGraphSnapshot ros_graph_snapshot = 12;
     JointStateUpdate joint_state_update = 13;
+    TopicInspectionUpdate topic_inspection_update = 14;
   }
   int64 timestamp_ns = 10;
   uint64 sequence = 11;
 }

+message TopicInspectionUpdate {
+  string topic_name = 1;
+  string message_type = 2;
+  string json_payload = 3;
+  bool active = 4;
+  bool truncated = 5;
+  string error = 6;
+}
+
 message JointStateUpdate {
   string topic_name = 1;
   repeated string names = 2;
@@ -175,6 +191,7 @@ message TFUpdate {
   double qz = 8;
   double qw = 9;
   string topic_name = 10;
+  int64 source_timestamp_ms = 11;
 }

 message NodeStarted {
diff --git a/src-tauri/resources/devcontainer/server.py b/src-tauri/resources/devcontainer/server.py
index 44521b7..7d9b52c 100644
--- a/src-tauri/resources/devcontainer/server.py
+++ b/src-tauri/resources/devcontainer/server.py
@@ -3,6 +3,10 @@ import queue
 import time
 import logging
 import os
+import math
+import re
+import json
+from collections.abc import Sequence

 import grpc
 from concurrent import futures
@@ -28,6 +32,7 @@ from tf2_msgs.msg import TFMessage
 from nav_msgs.msg import Odometry
 from geometry_msgs.msg import Twist
 from sensor_msgs.msg import JointState
+from rosidl_runtime_py.utilities import get_message
 from server_core import (
     CommandLeaseManager,
     EventPersistenceError,
@@ -49,6 +54,80 @@ MAX_GRAPH_SERVICES = 512
 MAX_GRAPH_ACTIONS = 128
 MAX_GRAPH_ENDPOINTS = 2048
 MAX_JOINT_STATE_ENTRIES = 512
+MAX_INSPECTED_TOPICS = 4
+MAX_INSPECTION_RATE_HZ = 5.0
+MAX_INSPECTION_MESSAGE_BYTES = 16 * 1024
+MAX_INSPECTION_DEPTH = 5
+MAX_INSPECTION_ITEMS = 256
+MAX_INSPECTION_STRING_LENGTH = 512
+
+
+class _MessageSerializationBudget:
+    def __init__(self):
+        self.items = 0
+        self.truncated = False
+
+
+def _bounded_message_value(value, budget, depth=0):
+    if budget.items >= MAX_INSPECTION_ITEMS:
+        budget.truncated = True
+        return "<truncated>"
+    budget.items += 1
+
+    if value is None or isinstance(value, (bool, int)):
+        return value
+    if isinstance(value, float):
+        return value if math.isfinite(value) else str(value)
+    if isinstance(value, str):
+        if len(value) > MAX_INSPECTION_STRING_LENGTH:
+            budget.truncated = True
+            return value[:MAX_INSPECTION_STRING_LENGTH]
+        return value
+    if isinstance(value, (bytes, bytearray)):
+        if len(value) > 64:
+            budget.truncated = True
+        return {"hex": bytes(value[:64]).hex(), "length": len(value)}
+    if depth >= MAX_INSPECTION_DEPTH:
+        budget.truncated = True
+        return "<maximum depth reached>"
+    if hasattr(value, "get_fields_and_field_types"):
+        result = {}
+        for field_name in value.get_fields_and_field_types():
+            if budget.items >= MAX_INSPECTION_ITEMS:
+                budget.truncated = True
+                result["_truncated"] = True
+                break
+            result[field_name] = _bounded_message_value(
+                getattr(value, field_name), budget, depth + 1
+            )
+        return result
+    if isinstance(value, Sequence):
+        result = []
+        for item in value:
+            if budget.items >= MAX_INSPECTION_ITEMS:
+                budget.truncated = True
+                result.append("<truncated>")
+                break
+            result.append(_bounded_message_value(item, budget, depth + 1))
+        return result
+    return str(value)[:MAX_INSPECTION_STRING_LENGTH]
+
+
+def _serialize_inspection_message(message):
+    budget = _MessageSerializationBudget()
+    value = _bounded_message_value(message, budget)
+    payload = json.dumps(
+        value,
+        ensure_ascii=True,
+        allow_nan=False,
+        separators=(",", ":"),
+    )
+    if len(payload.encode("utf-8")) > MAX_INSPECTION_MESSAGE_BYTES:
+        return (
+            '{"error":"Serialized message exceeds the inspection size limit"}',
+            True,
+        )
+    return payload, budget.truncated


 def _qos_policy_name(value):
@@ -84,6 +163,11 @@ class RosIntrospector(Node):
         )
         self.tf_subscribers = {}
         self.joint_state_subscribers = {}
+        self.topic_inspection_subscriptions = {}
+        self.topic_inspection_requests = queue.Queue(maxsize=16)
+        self.topic_inspection_timer = self.create_timer(
+            0.1, self._process_topic_inspection_requests
+        )

     def _publish_stop_to_all(self):
         with self._command_lock:
@@ -151,6 +235,10 @@ class RosIntrospector(Node):
                     qz=transform.transform.rotation.z,
                     qw=transform.transform.rotation.w,
                     topic_name=topic_name,
+                    source_timestamp_ms=(
+                        transform.header.stamp.sec * 1000
+                        + transform.header.stamp.nanosec // 1_000_000
+                    ),
                 ),
                 timestamp_ns=time.time_ns()
             )
@@ -181,6 +269,147 @@ class RosIntrospector(Node):
             key=("joint_state", topic_name),
         )

+    def request_topic_inspection(self, action, topic_name):
+        if not re.fullmatch(r"/[A-Za-z_][A-Za-z0-9_]*(/[A-Za-z_][A-Za-z0-9_]*)*", topic_name):
+            raise ValueError("Topic name must be a fully qualified ROS topic name")
+        future = futures.Future()
+        try:
+            self.topic_inspection_requests.put_nowait(
+                (action, topic_name, future)
+            )
+        except queue.Full as error:
+            raise RuntimeError("Topic inspection request queue is full") from error
+        try:
+            return future.result(timeout=5.0)
+        except futures.TimeoutError as error:
+            future.cancel()
+            raise TimeoutError("Timed out waiting for the ROS executor") from error
+
+    def _process_topic_inspection_requests(self):
+        while True:
+            try:
+                action, topic_name, future = (
+                    self.topic_inspection_requests.get_nowait()
+                )
+            except queue.Empty:
+                return
+            if not future.set_running_or_notify_cancel():
+                continue
+            try:
+                if action == "start":
+                    result = self._start_topic_inspection(topic_name)
+                else:
+                    result = self._stop_topic_inspection(topic_name)
+            except Exception as error:
+                future.set_exception(error)
+            else:
+                future.set_result(result)
+
+    def _publish_topic_inspection(
+        self, topic_name, message_type, active, payload="", truncated=False, error=""
+    ):
+        event = ros_engine_pb2.WorkspaceEvent(
+            topic_inspection_update=ros_engine_pb2.TopicInspectionUpdate(
+                topic_name=topic_name,
+                message_type=message_type,
+                json_payload=payload,
+                active=active,
+                truncated=truncated,
+                error=error,
+            ),
+            timestamp_ns=time.time_ns(),
+        )
+        self.event_hub.publish(
+            event,
+            priority="telemetry",
+            key=("topic_inspection", topic_name),
+        )
+
+    def _start_topic_inspection(self, topic_name):
+        existing = self.topic_inspection_subscriptions.get(topic_name)
+        if existing is not None:
+            return existing["message_type"]
+        if len(self.topic_inspection_subscriptions) >= MAX_INSPECTED_TOPICS:
+            raise RuntimeError(
+                f"At most {MAX_INSPECTED_TOPICS} topics can be inspected at once"
+            )
+
+        topic_types = dict(self.get_topic_names_and_types()).get(topic_name, [])
+        if not topic_types:
+            raise LookupError(f"ROS topic {topic_name} is not currently available")
+        if len(topic_types) != 1:
+            raise ValueError(f"ROS topic {topic_name} has ambiguous message types")
+
+        message_type_name = topic_types[0]
+        try:
+            message_class = get_message(message_type_name)
+        except (AttributeError, ModuleNotFoundError, ValueError) as error:
+            raise LookupError(
+                f"Message type {message_type_name} is not available in this ROS environment"
+            ) from error
+
+        qos = QoSProfile(
+            depth=5,
+            reliability=ReliabilityPolicy.BEST_EFFORT,
+            durability=DurabilityPolicy.VOLATILE,
+        )
+        subscription = self.create_subscription(
+            message_class,
+            topic_name,
+            lambda message, name=topic_name, type_name=message_type_name: (
+                self._topic_inspection_callback(message, name, type_name)
+            ),
+            qos,
+        )
+        self.topic_inspection_subscriptions[topic_name] = {
+            "subscription": subscription,
+            "message_type": message_type_name,
+            "last_sent_at": 0.0,
+        }
+        self._publish_topic_inspection(
+            topic_name, message_type_name, active=True
+        )
+        return message_type_name
+
+    def _stop_topic_inspection(self, topic_name):
+        inspection = self.topic_inspection_subscriptions.pop(topic_name, None)
+        if inspection is not None:
+            self.destroy_subscription(inspection["subscription"])
+            message_type = inspection["message_type"]
+        else:
+            message_type = ""
+        self._publish_topic_inspection(
+            topic_name, message_type, active=False
+        )
+        return message_type
+
+    def _topic_inspection_callback(self, message, topic_name, message_type):
+        inspection = self.topic_inspection_subscriptions.get(topic_name)
+        if inspection is None:
+            return
+        now = time.monotonic()
+        if now - inspection["last_sent_at"] < 1.0 / MAX_INSPECTION_RATE_HZ:
+            return
+        inspection["last_sent_at"] = now
+        try:
+            payload, truncated = _serialize_inspection_message(message)
+        except (TypeError, ValueError, OverflowError):
+            LOGGER.exception("Could not serialize inspected topic %s", topic_name)
+            self._publish_topic_inspection(
+                topic_name,
+                message_type,
+                active=True,
+                error="Message could not be serialized",
+            )
+            return
+        self._publish_topic_inspection(
+            topic_name,
+            message_type,
+            active=True,
+            payload=payload,
+            truncated=truncated,
+        )
+
     def tick(self):
         try:
             self._tick_impl()
@@ -233,6 +462,18 @@ class RosIntrospector(Node):
         # Clean up subscription tracking for topics that no longer exist,
         # so they can be freshly re-subscribed if they reappear later
         current_topic_names = {name for name, _ in topics}
+        stale_inspections = (
+            set(self.topic_inspection_subscriptions) - current_topic_names
+        )
+        for name in stale_inspections:
+            inspection = self.topic_inspection_subscriptions.pop(name)
+            self.destroy_subscription(inspection["subscription"])
+            self._publish_topic_inspection(
+                name,
+                inspection["message_type"],
+                active=False,
+                error="Topic is no longer available",
+            )
         stale = set(self.odom_subscribers.keys()) - current_topic_names
         for name in stale:
             del self.odom_subscribers[name]
@@ -524,6 +765,42 @@ class RosEngineServicer(ros_engine_pb2_grpc.RosEngineServicer):
             context.abort(grpc.StatusCode.INTERNAL, "Failed to publish Twist command")
         return ros_engine_pb2.Empty()

+    def StartTopicInspection(self, request, context):
+        self._require_authentication(context)
+        try:
+            self.ros_node.request_topic_inspection("start", request.topic_name)
+        except ValueError as error:
+            context.abort(grpc.StatusCode.INVALID_ARGUMENT, str(error))
+        except LookupError as error:
+            context.abort(grpc.StatusCode.NOT_FOUND, str(error))
+        except TimeoutError as error:
+            context.abort(grpc.StatusCode.DEADLINE_EXCEEDED, str(error))
+        except RuntimeError as error:
+            context.abort(grpc.StatusCode.RESOURCE_EXHAUSTED, str(error))
+        except Exception:
+            LOGGER.exception("Failed to start topic inspection for %s", request.topic_name)
+            context.abort(
+                grpc.StatusCode.INTERNAL, "Failed to start topic inspection"
+            )
+        return ros_engine_pb2.Empty()
+
+    def StopTopicInspection(self, request, context):
+        self._require_authentication(context)
+        try:
+            self.ros_node.request_topic_inspection("stop", request.topic_name)
+        except ValueError as error:
+            context.abort(grpc.StatusCode.INVALID_ARGUMENT, str(error))
+        except TimeoutError as error:
+            context.abort(grpc.StatusCode.DEADLINE_EXCEEDED, str(error))
+        except RuntimeError as error:
+            context.abort(grpc.StatusCode.RESOURCE_EXHAUSTED, str(error))
+        except Exception:
+            LOGGER.exception("Failed to stop topic inspection for %s", request.topic_name)
+            context.abort(
+                grpc.StatusCode.INTERNAL, "Failed to stop topic inspection"
+            )
+        return ros_engine_pb2.Empty()
+
     def _safety_state_message(self):
         state, detail, updated_at_ns = self.ros_node.safety.snapshot()
         state_value = getattr(ros_engine_pb2.SafetyState, state)
diff --git a/src-tauri/src/lib.rs b/src-tauri/src/lib.rs
index f6a92f9..0c9fa54 100644
--- a/src-tauri/src/lib.rs
+++ b/src-tauri/src/lib.rs
@@ -1127,6 +1127,8 @@ pub fn run() {
             ask_ai,
             initialize_ros_environment,
             publish_twist,
+            start_topic_inspection,
+            stop_topic_inspection,
             set_emergency_stop,
             get_safety_state,
             get_robot_config,
@@ -1361,6 +1363,22 @@ async fn start_ros_stream(app: AppHandle) -> Result<(), String> {
                                 },
                             );
                         }
+                        if let Some(rosengine::workspace_event::Event::TopicInspectionUpdate(
+                            update,
+                        )) = &evt.event
+                        {
+                            let _ = app.emit(
+                                "topic-inspection-update",
+                                TopicInspectionUpdateResponse {
+                                    topic_name: update.topic_name.clone(),
+                                    message_type: update.message_type.clone(),
+                                    json_payload: update.json_payload.clone(),
+                                    active: update.active,
+                                    truncated: update.truncated,
+                                    error: update.error.clone(),
+                                },
+                            );
+                        }

                         if let Some(rosengine::workspace_event::Event::TfUpdate(tf)) = &evt.event {
                             let _ = app.emit(
@@ -1376,6 +1394,8 @@ async fn start_ros_stream(app: AppHandle) -> Result<(), String> {
                                     qz: tf.qz,
                                     qw: tf.qw,
                                     topic_name: tf.topic_name.clone(),
+                                    source_timestamp_ms: tf.source_timestamp_ms,
+                                    received_at_ms: evt.timestamp_ns.div_euclid(1_000_000),
                                 },
                             );
                         }
@@ -1716,6 +1736,17 @@ struct JointStateUpdateResponse {
     truncated: bool,
 }

+#[derive(Serialize, Clone)]
+#[serde(rename_all = "camelCase")]
+struct TopicInspectionUpdateResponse {
+    topic_name: String,
+    message_type: String,
+    json_payload: String,
+    active: bool,
+    truncated: bool,
+    error: String,
+}
+
 fn ros_graph_snapshot_response(snapshot: &rosengine::RosGraphSnapshot) -> RosGraphSnapshotResponse {
     RosGraphSnapshotResponse {
         nodes: snapshot
@@ -1809,6 +1840,38 @@ async fn connect_ros_client(
     Ok((client, token))
 }

+#[tauri::command]
+async fn start_topic_inspection(app: AppHandle, topic_name: String) -> Result<(), String> {
+    if !is_valid_ros_topic(&topic_name) {
+        return Err("Invalid ROS topic name".to_string());
+    }
+    let (mut client, token) = connect_ros_client(&app).await?;
+    client
+        .start_topic_inspection(authenticated_request(
+            rosengine::TopicInspectionRequest { topic_name },
+            &token,
+        )?)
+        .await
+        .map_err(|error| format!("Could not inspect ROS topic: {}", error.message()))?;
+    Ok(())
+}
+
+#[tauri::command]
+async fn stop_topic_inspection(app: AppHandle, topic_name: String) -> Result<(), String> {
+    if !is_valid_ros_topic(&topic_name) {
+        return Err("Invalid ROS topic name".to_string());
+    }
+    let (mut client, token) = connect_ros_client(&app).await?;
+    client
+        .stop_topic_inspection(authenticated_request(
+            rosengine::TopicInspectionRequest { topic_name },
+            &token,
+        )?)
+        .await
+        .map_err(|error| format!("Could not stop ROS topic inspection: {}", error.message()))?;
+    Ok(())
+}
+
 #[tauri::command]
 async fn set_emergency_stop(app: AppHandle, activate: bool) -> Result<SafetyStateResponse, String> {
     let (mut client, token) = connect_ros_client(&app).await?;
@@ -4458,6 +4521,8 @@ struct TfUpdate {
     qz: f64,
     qw: f64,
     topic_name: String,
+    source_timestamp_ms: i64,
+    received_at_ms: i64,
 }

 #[derive(Deserialize)]
diff --git a/src/App.tsx b/src/App.tsx
index 82edb09..d9e8042 100644
--- a/src/App.tsx
+++ b/src/App.tsx
@@ -5,7 +5,10 @@ import RobotView, { RobotControls } from "./components/RobotView";
 import { listen, emit } from "@tauri-apps/api/event";
 import TFTree from "./components/TFTree";
 import RosGraphExplorer from "./components/RosGraphExplorer";
-import type { RosGraphSnapshot } from "./components/RosGraphExplorer";
+import type {
+  RosGraphSnapshot,
+  TopicInspection,
+} from "./components/RosGraphExplorer";
 import Settings from "./components/Settings";
 import NewProjectModal from "./components/NewProjectModal";
 import {
@@ -285,6 +288,14 @@ function App() {
   const [rosConnected, setRosConnected] = useState(false);
   const [rosPackages, setRosPackages] = useState<RosPackageInfo[]>([]);
   const [rosGraph, setRosGraph] = useState<RosGraphSnapshot | null>(null);
+  const [topicInspection, setTopicInspection] =
+    useState<TopicInspection | null>(null);
+  const [pendingInspectionTopic, setPendingInspectionTopic] = useState<
+    string | null
+  >(null);
+  const [topicInspectionError, setTopicInspectionError] = useState<
+    string | null
+  >(null);
   const [rosPackagesLoading, setRosPackagesLoading] = useState(false);
   const [rosPackagesError, setRosPackagesError] = useState<string | null>(null);
   const [rosPackageFilter, setRosPackageFilter] = useState("");
@@ -323,6 +334,50 @@ function App() {
   const [robotCommandError, setRobotCommandError] = useState<string | null>(
     null,
   );
+
+  const startTopicInspection = async (topicName: string) => {
+    setPendingInspectionTopic(topicName);
+    setTopicInspectionError(null);
+    setTopicInspection((current) => ({
+      topicName,
+      messageType:
+        current?.topicName === topicName ? current.messageType : "",
+      jsonPayload: current?.topicName === topicName ? current.jsonPayload : "",
+      active: false,
+      truncated: current?.topicName === topicName ? current.truncated : false,
+      error: "",
+    }));
+    try {
+      if (topicInspection?.active && topicInspection.topicName !== topicName) {
+        await invoke("stop_topic_inspection", {
+          topicName: topicInspection.topicName,
+        });
+      }
+      await invoke("start_topic_inspection", { topicName });
+      setTopicInspection((current) =>
+        current?.topicName === topicName ? { ...current, active: true } : current,
+      );
+    } catch (error) {
+      setTopicInspectionError(String(error));
+    } finally {
+      setPendingInspectionTopic(null);
+    }
+  };
+
+  const stopTopicInspection = async (topicName: string) => {
+    setPendingInspectionTopic(topicName);
+    setTopicInspectionError(null);
+    try {
+      await invoke("stop_topic_inspection", { topicName });
+      setTopicInspection((current) =>
+        current?.topicName === topicName ? { ...current, active: false } : current,
+      );
+    } catch (error) {
+      setTopicInspectionError(String(error));
+    } finally {
+      setPendingInspectionTopic(null);
+    }
+  };
   const rosWorkspaceReady = useRef(false);
   const authRecoveryStarted = useRef(false);
   const [setupOutput, setSetupOutput] = useState<string[]>([]); // Environment setup logs
@@ -1022,6 +1077,18 @@ function App() {
       "ros-graph-snapshot",
       (event) => setRosGraph(event.payload),
     );
+    const unlistenTopicInspection = listen<TopicInspection>(
+      "topic-inspection-update",
+      (event) => {
+        const update = event.payload;
+        setTopicInspection((current) =>
+          current && !update.active && current.topicName !== update.topicName
+            ? current
+            : update,
+        );
+        if (!update.error) setTopicInspectionError(null);
+      },
+    );

     const unlistenBuildOutput = listen<LogEvent>("build-output", (event) => {
       const { kind, text } = event.payload;
@@ -1072,6 +1139,7 @@ function App() {
       unlistenEventHealth.then((f) => f());
       unlistenEventRecovery.then((f) => f());
       unlistenRosGraph.then((f) => f());
+      unlistenTopicInspection.then((f) => f());
       unlistenBuildOutput.then((f) => f());
       unlistenBuildError.then((f) => f());
       unlistenBuildFinished.then((f) => f());
@@ -1811,7 +1879,19 @@ function App() {
                         )}
                       </>
                     )}
-                  <RosGraphExplorer graph={rosGraph} connected={rosConnected} />
+                  <RosGraphExplorer
+                    graph={rosGraph}
+                    connected={rosConnected}
+                    inspection={topicInspection}
+                    pendingTopic={pendingInspectionTopic}
+                    inspectionError={topicInspectionError}
+                    onStartInspection={(topicName) =>
+                      void startTopicInspection(topicName)
+                    }
+                    onStopInspection={(topicName) =>
+                      void stopTopicInspection(topicName)
+                    }
+                  />
                 </div>
               )}
             </div>
@@ -2474,7 +2554,9 @@ function App() {
                   </div>
                 </div>
               )}
-              {activeTab === "tf" && <TFTree onExplain={explainTfFrame} />}
+              {activeTab === "tf" && (
+                <TFTree connected={rosConnected} onExplain={explainTfFrame} />
+              )}
               {activeTab === "problems" && (
                 <div style={{ padding: 8, fontSize: 12 }}>
                   {eventHealth && (
diff --git a/src/components/RosGraphExplorer.tsx b/src/components/RosGraphExplorer.tsx
index 3ce553c..f2370ca 100644
--- a/src/components/RosGraphExplorer.tsx
+++ b/src/components/RosGraphExplorer.tsx
@@ -14,6 +14,15 @@ export type {
   RosGraphTopic,
 } from "../rosGraph";

+export interface TopicInspection {
+  topicName: string;
+  messageType: string;
+  jsonPayload: string;
+  active: boolean;
+  truncated: boolean;
+  error: string;
+}
+
 function nodePath(nodeName: string, nodeNamespace: string) {
   return `${nodeNamespace === "/" ? "" : nodeNamespace}/${nodeName}`;
 }
@@ -87,9 +96,19 @@ function GraphCard({
 export default function RosGraphExplorer({
   graph,
   connected,
+  inspection,
+  pendingTopic,
+  inspectionError,
+  onStartInspection,
+  onStopInspection,
 }: {
   graph: RosGraphSnapshot | null;
   connected: boolean;
+  inspection: TopicInspection | null;
+  pendingTopic: string | null;
+  inspectionError: string | null;
+  onStartInspection: (topicName: string) => void;
+  onStopInspection: (topicName: string) => void;
 }) {
   if (!graph) {
     return (
@@ -186,8 +205,8 @@ export default function RosGraphExplorer({
     >
       <div style={{ fontWeight: 600, marginBottom: 4 }}>Live ROS Graph</div>
       <div style={{ opacity: 0.65, fontSize: 11, marginBottom: 6 }}>
-        Read-only discovery. This panel does not call services or send action
-        goals.
+        Read-only discovery with bounded live topic inspection. This panel does
+        not call services or send action goals.
       </div>
       {graph.truncated && (
         <div role="status" style={{ color: "#e0b060", marginBottom: 6 }}>
@@ -235,6 +254,70 @@ export default function RosGraphExplorer({
               <>
                 {endpointList("Publishers", topic.publishers)}
                 {endpointList("Subscribers", topic.subscribers)}
+                <button
+                  type="button"
+                  disabled={!connected || pendingTopic !== null}
+                  onClick={() =>
+                    inspection?.topicName === topic.name && inspection.active
+                      ? onStopInspection(topic.name)
+                      : onStartInspection(topic.name)
+                  }
+                  style={{ justifySelf: "start", marginTop: 4 }}
+                >
+                  {pendingTopic === topic.name
+                    ? "Updating…"
+                    : inspection?.topicName === topic.name && inspection.active
+                      ? "Stop inspection"
+                      : "Inspect messages"}
+                </button>
+                {inspection?.topicName === topic.name && (
+                  <div
+                    aria-label={`Inspection for ${topic.name}`}
+                    style={{
+                      display: "grid",
+                      gap: 4,
+                      maxWidth: "100%",
+                      overflow: "hidden",
+                    }}
+                  >
+                    {inspection.messageType && (
+                      <div style={{ opacity: 0.7 }}>
+                        {inspection.messageType}
+                      </div>
+                    )}
+                    {inspectionError && (
+                      <div role="alert" style={{ color: "#e99" }}>
+                        {inspectionError}
+                      </div>
+                    )}
+                    {inspection.error && (
+                      <div role="alert" style={{ color: "#e99" }}>
+                        {inspection.error}
+                      </div>
+                    )}
+                    {inspection.truncated && (
+                      <div role="status" style={{ color: "#e0b060" }}>
+                        Message was truncated to fit inspection limits.
+                      </div>
+                    )}
+                    {inspection.jsonPayload && (
+                      <pre
+                        aria-label="Latest ROS topic message"
+                        style={{
+                          maxHeight: 220,
+                          overflow: "auto",
+                          whiteSpace: "pre-wrap",
+                          overflowWrap: "anywhere",
+                          margin: 0,
+                          padding: 6,
+                          background: "rgba(0,0,0,0.2)",
+                        }}
+                      >
+                        {inspection.jsonPayload}
+                      </pre>
+                    )}
+                  </div>
+                )}
               </>
             }
           />
diff --git a/src/components/TFTree.tsx b/src/components/TFTree.tsx
index 84e7fb7..8ef1634 100644
--- a/src/components/TFTree.tsx
+++ b/src/components/TFTree.tsx
@@ -12,9 +12,12 @@ type TfUpdateWithValidation = TfTransform & {
   validationIssues: string[];
 };

-interface TfEvent extends Omit<TfTransform, "receivedAtMs"> {}
+interface TfEvent extends Omit<TfTransform, "receivedAtMs"> {
+  received_at_ms: number;
+}

 interface Props {
+  connected: boolean;
   onExplain?: (tf: TfUpdateWithValidation) => void;
 }

@@ -114,7 +117,7 @@ function quaternionEulerDegrees(transform: TfTransform) {
   };
 }

-export default function TFTree({ onExplain }: Props) {
+export default function TFTree({ connected, onExplain }: Props) {
   const [transforms, setTransforms] = useState<Record<string, TfTransform>>(
     {},
   );
@@ -129,7 +132,7 @@ export default function TFTree({ onExplain }: Props) {
       if (!active) return;
       const transform: TfTransform = {
         ...event.payload,
-        receivedAtMs: Date.now(),
+        receivedAtMs: event.payload.received_at_ms,
       };
       const key = tfEdgeKey(transform.parent_frame, transform.child_frame);
       transformsRef.current[key] = transform;
@@ -174,6 +177,12 @@ export default function TFTree({ onExplain }: Props) {
       <div style={{ opacity: 0.65, marginBottom: 8 }}>
         {graph.edges.length} transform(s) · {graph.roots.length} root frame(s)
       </div>
+      {!connected && (
+        <div role="status" style={{ color: "#e0b060", marginBottom: 8 }}>
+          ROS disconnected. Displayed transforms are last-known data; dynamic
+          transforms are marked stale by their receive time.
+        </div>
+      )}
       {graph.warnings.map((warning) => (
         <div
           key={warning}
@@ -240,6 +249,14 @@ export default function TFTree({ onExplain }: Props) {
                   Math.floor((nowMs - selected.receivedAtMs) / 1000),
                 )}s ago`}
           </div>
+          <div style={{ opacity: 0.65, marginBottom: 6 }}>
+            ROS source stamp:{" "}
+            {selected.source_timestamp_ms > 0
+              ? `${Math.floor(selected.source_timestamp_ms / 1000)}.${String(
+                  selected.source_timestamp_ms % 1000,
+                ).padStart(3, "0")} s`
+              : "zero or unavailable"}
+          </div>
           <div>
             Translation (m): ({selected.x.toFixed(4)}, {selected.y.toFixed(4)},{" "}
             {selected.z.toFixed(4)})
diff --git a/src/tfGraph.ts b/src/tfGraph.ts
index 68f5709..5ea7069 100644
--- a/src/tfGraph.ts
+++ b/src/tfGraph.ts
@@ -10,6 +10,7 @@ export interface TfTransform {
   qz: number;
   qw: number;
   receivedAtMs: number;
+  source_timestamp_ms: number;
 }

 export interface TfGraphEdge extends TfTransform {
diff --git a/tests/tfGraph.test.mjs b/tests/tfGraph.test.mjs
index 2d8d95c..11f38b0 100644
--- a/tests/tfGraph.test.mjs
+++ b/tests/tfGraph.test.mjs
@@ -15,6 +15,7 @@ function transform(parent, child, overrides = {}) {
     qz: 0,
     qw: 1,
     receivedAtMs: 10000,
+    source_timestamp_ms: 9000,
     ...overrides,
   };
 }
@@ -71,6 +72,20 @@ test("marks old dynamic transforms stale but exempts static transforms", () => {
   assert.ok(!graph.edges[1].issues.includes("Transform is stale"));
 });

+test("uses receive time for staleness and preserves the ROS source timestamp", () => {
+  const graph = buildTfGraph(
+    [
+      transform("map", "replayed", {
+        receivedAtMs: 0,
+        source_timestamp_ms: 9000,
+      }),
+    ],
+    10000,
+  );
+  assert.ok(graph.edges[0].issues.includes("Transform is stale"));
+  assert.equal(graph.edges[0].source_timestamp_ms, 9000);
+});
+
 test("reports invalid quaternions and non-finite transforms", () => {
   const graph = buildTfGraph(
     [
```
