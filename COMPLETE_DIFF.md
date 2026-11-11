# Complete Git Diff

Base: `307f3ef` on `main`; reflects current tracked and untracked worktree changes. This file is excluded from its own diff.

Validation summary: `npm ci`, production build, and all 10 frontend tests passed. Rust formatting, check, 43 tests (1 ignored), and strict Clippy passed. Python tests passed (43), and syntax validation passed. The full Docker ROS 2 Jazzy/Gazebo Harmonic integration passed, including authenticated TLS, safety enforcement, event replay/restart, static/dynamic TF, and cleanup; no temporary integration directory or container remained. The image's pinned Python gRPC environment passed `pip-audit`; its system Python remained available to `colcon`, and a representative `colcon build` passed. `npm audit` reported 0 vulnerabilities; `cargo audit` reported 13 warnings, including an unsound `glib` dependency. Hosted CI and cross-platform packaging have not been verified for this worktree. No commit, push, tag, or release was created.

## Full diff

````diff
diff --git a/CHANGELOG.md b/CHANGELOG.md
index ab7e200..177bdca 100644
--- a/CHANGELOG.md
+++ b/CHANGELOG.md
@@ -18 +18 @@ This version is still in development and is not release-ready. See
-  server/client discovery.
+  server/client discovery, grouped by fully qualified node identity in the UI.
@@ -24 +24,2 @@ This version is still in development and is not release-ready. See
-  streamed to the preview.
+  streamed to the preview. Joint limits and mimic relationships are applied,
+  and supported collision primitives can be toggled in the preview.
@@ -29,2 +30,4 @@ This version is still in development and is not release-ready. See
-  live ROS graph endpoints, `/tf_static` and `/joint_states` delivery, and
-  valid/invalid SDFormat schema checks.
+  live ROS graph endpoints, static and dynamic TF plus `/joint_states`
+  delivery, and valid/invalid SDFormat schema checks.
+- Build-process state and cancellation feedback that distinguishes a stop
+  request from confirmed process termination.
@@ -46,0 +50,5 @@ This version is still in development and is not release-ready. See
+- Workspace package discovery reports inaccessible tree errors rather than
+  silently returning an empty package inventory.
+- The ROS container isolates its pinned Python gRPC runtime and code-generation
+  packages in a virtual environment so ROS/colcon retain their system Python
+  dependencies.
diff --git a/README.md b/README.md
index bbd01a4..4c292aa 100644
--- a/README.md
+++ b/README.md
@@ -174,3 +174,5 @@ any of this — the app manages the environment for you.
-  Floating/planar joints and full URDF semantics are not supported. The ROS
-  to three.js coordinate mapping is a first pass and still needs convention
-  validation.
+  Joint limits and mimic relationships are applied, and supported primitive
+  collision geometry can be toggled. Inertial properties and robot-level
+  named materials are not used in the preview. Floating/planar joints and full
+  URDF semantics are not supported. The ROS to three.js coordinate mapping is
+  a first pass and still needs convention validation.
@@ -209,5 +211,5 @@ any of this — the app manages the environment for you.
-  The Build action can cancel an active streaming `colcon build` by terminating
-  its dedicated process group. Gazebo startup now waits up to 20 seconds for a
-  world-control service and cleans up if startup or bridge setup fails, but
-  generalized process status, timeouts, and application-shutdown cleanup are
-  still incomplete.
+  Build status and cancellation requests are tracked, and cancellation is only
+  reported after process exit is confirmed. Gazebo startup waits up to 20
+  seconds for a world-control service and cleans up if startup or bridge setup
+  fails, but generalized process status, timeouts, and application-shutdown
+  cleanup are still incomplete.
diff --git a/RELEASE_READINESS.md b/RELEASE_READINESS.md
index 7788803..be7a3cb 100644
--- a/RELEASE_READINESS.md
+++ b/RELEASE_READINESS.md
@@ -5,6 +5,6 @@
-**Not ready for release.** The checkout starts at `b907550` on `main`; the
-worktree now contains uncommitted ROS package, TF, graph, SDF-validation, and
-initial URDF-preview improvements.
-The hosted CI run for `b907550` passed all three jobs, but it does not include
-the current uncommitted changes. Several major v0.3.0 acceptance criteria
-remain incomplete. No commit, push, tag, or release was created.
+**Not ready for release.** The worktree is based on `307f3ef` on `main` and
+contains uncommitted ROS graph, TF integration, URDF preview, build lifecycle,
+and package-discovery test changes. Local validation and Docker integration
+pass, but substantial v0.3.0 acceptance criteria remain incomplete. Hosted CI
+has not run against this worktree. No commit, push, tag, or release was
+created.
@@ -19 +19 @@ remain incomplete. No commit, push, tag, or release was created.
-| 4 | Dependency security | PARTIAL | Current local `npm audit` reports no vulnerabilities and `cargo audit` reports no vulnerability findings, with 13 allowed warnings for unmaintained/unsound transitive dependencies. Hosted CI has not run on this worktree. |
+| 4 | Dependency security | PARTIAL | Current local `npm audit` reports no vulnerabilities. `cargo audit` reports no vulnerability findings and 13 warnings, including an unsound `glib` version and unmaintained transitive crates. `pip-audit` reports no known vulnerabilities in the integration image's isolated, pinned PyPI-installed packages; the full image inventory includes ROS distro packages not resolvable from PyPI. Hosted CI has not run on this worktree. |
@@ -24 +24 @@ remain incomplete. No commit, push, tag, or release was created.
-| 9 | ROS node explorer | PARTIAL | Bounded node names/namespaces and per-topic endpoint node identities are streamed and shown. Full node-specific endpoint grouping and lifecycle details remain. |
+| 9 | ROS node explorer | PARTIAL | Bounded node names/namespaces and per-topic endpoint node identities are streamed and grouped in the UI by fully qualified node identity, including service/action roles. Full ROS lifecycle-state details remain. |
@@ -30,2 +30,2 @@ remain incomplete. No commit, push, tag, or release was created.
-| 15 | TF stale and static-frame behavior | PARTIAL | Dynamic edges become stale after five seconds; `/tf_static` uses reliable transient-local QoS and is exempt. A live Docker integration publishes and verifies a static TF edge. Missing authorities cannot be inferred from edge data. |
-| 16 | URDF parsing and validation | PARTIAL | A bounded parser rejects malformed/unsafe XML, validates link/joint-tree structure and primitive dimensions, and reports unsupported mesh visuals. Full URDF semantics, Xacro, and resource validation remain. |
+| 15 | TF stale and static-frame behavior | PARTIAL | Dynamic edges become stale after five seconds; `/tf_static` uses reliable transient-local QoS and is exempt. Docker integration publishes and verifies both static and dynamic TF. Missing authorities cannot be inferred from edge data. |
+| 16 | URDF parsing and validation | PARTIAL | A bounded parser rejects malformed/unsafe XML, validates link/joint-tree structure, primitive dimensions, joint limits, and mimic references/cycles, and reports unsupported mesh visuals. Full URDF semantics, Xacro, and resource validation remain. |
@@ -34,2 +34,2 @@ remain incomplete. No commit, push, tag, or release was created.
-| 19 | Robot model and joint-state rendering | PARTIAL | Active `.urdf` files render box/cylinder/sphere visuals and supported joint transforms; bounded `/joint_states` updates are covered by live Docker integration. No mesh/Xacro, coordinate-convention verification, or visual desktop test yet. |
-| 20 | Process lifecycle manager | PARTIAL | Builds and process-registry data have dedicated support, but build, Docker, Gazebo, bridge, and ROS service processes do not share reliable lifecycle, timeout, cancellation, and shutdown handling. |
+| 19 | Robot model and joint-state rendering | PARTIAL | Active `.urdf` files render box/cylinder/sphere visuals and supported joint transforms; limits, mimic joints, and a collision-geometry toggle are supported. Bounded `/joint_states` updates are covered by Docker integration. No mesh/Xacro, coordinate-convention verification, or visual desktop test yet; inertial and named robot-level materials are not rendered. |
+| 20 | Process lifecycle manager | PARTIAL | Build states distinguish starting/running/stopping/terminal outcomes; stop requests are only reported as cancellation after process exit. Docker, Gazebo, bridge, and ROS service processes still lack a shared lifecycle manager, generalized timeouts, and application-shutdown cleanup. |
@@ -37 +37 @@ remain incomplete. No commit, push, tag, or release was created.
-| 22 | Frontend workflow tests/accessibility | PARTIAL | Dependency-free TF graph and robot-rendering tests plus production type-check/build pass. React workflow tests, accessibility review, and coverage of workspace/ROS/safety UI remain. |
+| 22 | Frontend workflow tests/accessibility | PARTIAL | Ten dependency-free TF graph and robot-rendering tests plus production type-check/build pass. React workflow tests, accessibility review, and coverage of workspace/ROS/safety UI remain. |
@@ -56,0 +57,7 @@ remain incomplete. No commit, push, tag, or release was created.
+- The Rust dependency audit reports an unsound `glib` dependency warning in
+  addition to unmaintained transitive dependencies. `pip-audit` found no known
+  vulnerabilities in `grpcio`, `grpcio-tools`, and `setuptools` as installed
+  in an isolated virtual environment in the integration image. The Dockerfile
+  pins these PyPI packages without replacing ROS/colcon's system Python
+  dependencies. A whole-image pip audit cannot resolve ROS-distributed
+  packages such as `action-msgs` from PyPI.
@@ -64,2 +71,3 @@ remain incomplete. No commit, push, tag, or release was created.
-| `npm run build` | Pass; Vite reports an existing large JavaScript chunk warning (~820 kB). |
-| `npm run test:frontend` | Pass: 7 TF graph and robot-description tests (Node 26.5.0). |
+| `npm ci` | Pass; 91 packages installed. |
+| `npm run build` | Pass; Vite reports a large JavaScript chunk warning (~831 kB). |
+| `npm run test:frontend` | Pass: 10 TF graph and robot-description tests. |
@@ -68 +76 @@ remain incomplete. No commit, push, tag, or release was created.
-| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | Pass: 39 passed, 1 ignored (live-service test). |
+| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | Pass: 43 passed, 1 ignored (live-service test). |
@@ -70 +78 @@ remain incomplete. No commit, push, tag, or release was created.
-| `python3 -B -m unittest -v test_server_core` from `src-tauri/resources/devcontainer` | Pass: 42 tests, including package discovery/isolation and action endpoint parsing. |
+| Python unittest discovery from `src-tauri/resources/devcontainer` | Pass: 43 tests, including inaccessible workspace-tree reporting. |
@@ -73,3 +81,2 @@ remain incomplete. No commit, push, tag, or release was created.
-| `scripts/docker-integration.sh` | Pass: selected-workspace package discovery, live graph node/topic/service/action endpoints, `/tf_static` and `/joint_states` delivery, valid/invalid SDFormat checks, pinned TLS/auth, stop/reset enforcement, replay and safety history after restart, Gazebo readiness/reset and topic bridge. Cleanup exited 0 and left no integration container or temporary workspace. |
-| Protocol-copy comparison | Pass: Rust and container protobuf definitions match. |
-| `cargo audit --file src-tauri/Cargo.lock` | Pass with 13 allowed warnings: no vulnerability findings; remaining advisories are unmaintained/unsound transitive dependencies disclosed in the previous readiness review. Patched Rustix advisory was fixed by updating to 1.1.6. |
+| `scripts/docker-integration.sh` | Pass: selected-workspace package discovery, live graph endpoints, static/dynamic TF and joint-state delivery, SDFormat checks, pinned TLS/auth, stop/reset enforcement, event replay/restart, and Gazebo. Cleanup exited 0; a follow-up check found no integration container or temporary directory. |
+| `cargo audit --file src-tauri/Cargo.lock` | Pass with 13 allowed warnings: no vulnerability findings; warnings include unmaintained crates and unsound `glib`. |
@@ -77 +84,3 @@ remain incomplete. No commit, push, tag, or release was created.
-| Hosted GitHub Actions | Pass for baseline commit `b907550` in run `43087023826` (frontend, Rust/integration, and Python jobs); not run against this worktree's changes. |
+| Python dependency audit | Pass: no known vulnerabilities in the isolated environment's pinned `grpcio==1.85.1`, `grpcio-tools==1.85.1`, `protobuf==7.36.2`, `setuptools==84.2.1`, and `typing_extensions==4.16.0`. The full system pip inventory could not be resolved because it includes ROS distro packages. |
+| ROS/colcon Python compatibility | Pass: gRPC uses the isolated Python 3.12 environment, system Python retains `setuptools==68.1.2`, and a representative `colcon build` completes. |
+| Hosted GitHub Actions | Not run against this uncommitted worktree. |
diff --git a/docs/architecture.md b/docs/architecture.md
index d7a8dc2..9867025 100644
--- a/docs/architecture.md
+++ b/docs/architecture.md
@@ -18,2 +18,5 @@ The ROS service source and generated protobuf modules run from
-mount. The service persists its event journal, safety state, robot limits, and
-workspace data under `/workspace/.robotics-studio`.
+mount. Its gRPC runtime and code-generation dependencies are installed in an
+isolated Python virtual environment with access to ROS system packages; the
+system Python remains available to `colcon`. The service persists its event
+journal, safety state, robot limits, and workspace data under
+`/workspace/.robotics-studio`.
@@ -77,3 +80,6 @@ The `colcon build` operation has a dedicated process group and a cancellation
-path. Gazebo startup waits for a world control service before launching the
-bridge, and failed startup attempts trigger cleanup. Reset targets the active
-world's discovered control service rather than assuming a `default` world.
+path. Build lifecycle state distinguishes starting, running, stopping, and
+terminal outcomes; a stop request is not reported as cancellation until the
+process has exited. Gazebo startup waits for a world control service before
+launching the bridge, and failed startup attempts trigger cleanup. Reset
+targets the active world's discovered control service rather than assuming a
+`default` world.
@@ -109,2 +115,5 @@ connected link tree. Fixed, revolute, continuous, and prismatic joint states
-are updated from bounded `/joint_states` telemetry. Mesh visuals are surfaced
-as warnings and skipped. Xacro expansion, mesh/package resource resolution,
+are updated from bounded `/joint_states` telemetry, with supported joint
+limits and mimic relationships applied. Supported primitive collision
+geometry can be toggled in the preview. Mesh visuals are surfaced as warnings
+and skipped; inertial properties and robot-level named materials are not used
+in rendering. Xacro expansion, mesh/package resource resolution,
diff --git a/scripts/docker-integration.sh b/scripts/docker-integration.sh
index 05852ff..31791ab 100755
--- a/scripts/docker-integration.sh
+++ b/scripts/docker-integration.sh
@@ -139 +139 @@ docker exec "$container" bash -lc \
-  'source /opt/ros/jazzy/setup.bash && python3 -m grpc_tools.protoc -I /opt/robotics-studio/proto --python_out=/opt/robotics-studio/generated --grpc_python_out=/opt/robotics-studio/generated /opt/robotics-studio/proto/ros_engine.proto'
+  'source /opt/ros/jazzy/setup.bash && /opt/robotics-studio/venv/bin/python -m grpc_tools.protoc -I /opt/robotics-studio/proto --python_out=/opt/robotics-studio/generated --grpc_python_out=/opt/robotics-studio/generated /opt/robotics-studio/proto/ros_engine.proto'
@@ -178 +178 @@ start_ros_server() {
-    'source /opt/ros/jazzy/setup.bash && cd /opt/robotics-studio && ROBOTICS_STUDIO_GENERATED_PATH=/opt/robotics-studio/generated exec python3 /opt/robotics-studio/server.py >/tmp/server.log 2>&1'
+    'source /opt/ros/jazzy/setup.bash && cd /opt/robotics-studio && ROBOTICS_STUDIO_GENERATED_PATH=/opt/robotics-studio/generated exec /opt/robotics-studio/venv/bin/python /opt/robotics-studio/server.py >/tmp/server.log 2>&1'
@@ -203 +203 @@ docker exec -i "$container" bash -lc \
-  'source /opt/ros/jazzy/setup.bash && cd /opt/robotics-studio && python3 -' <<'PY'
+  'source /opt/ros/jazzy/setup.bash && cd /opt/robotics-studio && /opt/robotics-studio/venv/bin/python -' <<'PY'
@@ -260,0 +261 @@ publisher = node.create_publisher(
+dynamic_publisher = node.create_publisher(TFMessage, "/tf", 10)
@@ -264,0 +266 @@ while (
+    or dynamic_publisher.get_subscription_count() == 0
@@ -268,0 +271,3 @@ assert publisher.get_subscription_count() > 0, "ROS service did not subscribe to
+assert (
+    dynamic_publisher.get_subscription_count() > 0
+), "ROS service did not subscribe to /tf"
@@ -278,0 +284,6 @@ publisher.publish(TFMessage(transforms=[transform]))
+dynamic_transform = TransformStamped()
+dynamic_transform.header.frame_id = "odom"
+dynamic_transform.child_frame_id = "integration_dynamic_base"
+dynamic_transform.transform.translation.x = 1.5
+dynamic_transform.transform.rotation.w = 1.0
+dynamic_publisher.publish(TFMessage(transforms=[dynamic_transform]))
@@ -284 +295,2 @@ joint_state_publisher.publish(joint_state)
-tf_verified = False
+static_tf_verified = False
+dynamic_tf_verified = False
@@ -290,5 +302,10 @@ for event in events:
-        assert event.tf_update.parent_frame == "map"
-        assert event.tf_update.child_frame == "integration_base"
-        assert event.tf_update.topic_name == "/tf_static"
-        assert abs(event.tf_update.x - 0.25) < 1e-9
-        tf_verified = True
+        if event.tf_update.topic_name == "/tf_static":
+            assert event.tf_update.parent_frame == "map"
+            assert event.tf_update.child_frame == "integration_base"
+            assert abs(event.tf_update.x - 0.25) < 1e-9
+            static_tf_verified = True
+        elif event.tf_update.topic_name == "/tf":
+            assert event.tf_update.parent_frame == "odom"
+            assert event.tf_update.child_frame == "integration_dynamic_base"
+            assert abs(event.tf_update.x - 1.5) < 1e-9
+            dynamic_tf_verified = True
@@ -330 +347,6 @@ for event in events:
-    if tf_verified and graph_verified and joint_state_verified:
+    if (
+        static_tf_verified
+        and dynamic_tf_verified
+        and graph_verified
+        and joint_state_verified
+    ):
@@ -334 +356 @@ else:
-        "ROS service did not stream TF, joint-state, and graph details"
+        "ROS service did not stream static/dynamic TF, joint-state, and graph details"
@@ -339,0 +362,2 @@ if not joint_state_verified:
+if not dynamic_tf_verified:
+    raise AssertionError("ROS service did not stream the dynamic TF update")
@@ -344 +368 @@ rclpy.shutdown()
-print("Live ROS TF, joint-state, and graph endpoint discovery passed")
+print("Live ROS static/dynamic TF, joint-state, and graph endpoint discovery passed")
@@ -353 +377 @@ ROBOTICS_STUDIO_TEST_AUTH_TOKEN="$token" \
-docker exec -i "$container" bash -c 'cd /opt/robotics-studio && python3 -' <<'PY'
+docker exec -i "$container" bash -c 'cd /opt/robotics-studio && /opt/robotics-studio/venv/bin/python -' <<'PY'
@@ -416 +440 @@ docker exec "$container" bash -lc \
-  "pkill -TERM -f '[p]ython3 /opt/robotics-studio/server.py' || true"
+  "pkill -TERM -f '[v]env/bin/python /opt/robotics-studio/server.py' || true"
@@ -418 +442 @@ start_ros_server
-docker exec -i "$container" bash -c 'cd /opt/robotics-studio && python3 -' <<'PY'
+docker exec -i "$container" bash -c 'cd /opt/robotics-studio && /opt/robotics-studio/venv/bin/python -' <<'PY'
diff --git a/src-tauri/resources/devcontainer/Dockerfile b/src-tauri/resources/devcontainer/Dockerfile
index 1ff4dae..78fbc05 100644
--- a/src-tauri/resources/devcontainer/Dockerfile
+++ b/src-tauri/resources/devcontainer/Dockerfile
@@ -4,0 +5 @@ RUN apt-get update && apt-get install -y \
+    python3-venv \
@@ -12 +13,6 @@ RUN apt-get update && apt-get install -y \
-RUN pip3 install --break-system-packages grpcio grpcio-tools
+# Keep gRPC tooling separate from the system Python used by ROS and colcon.
+RUN python3 -m venv --system-site-packages /opt/robotics-studio/venv \
+    && /opt/robotics-studio/venv/bin/pip install --no-cache-dir \
+    grpcio==1.85.1 \
+    grpcio-tools==1.85.1 \
+    setuptools==84.2.1
diff --git a/src-tauri/resources/devcontainer/test_server_core.py b/src-tauri/resources/devcontainer/test_server_core.py
index b969520..beec4a4 100644
--- a/src-tauri/resources/devcontainer/test_server_core.py
+++ b/src-tauri/resources/devcontainer/test_server_core.py
@@ -158,0 +159,12 @@ class WorkspacePackageDiscoveryTests(unittest.TestCase):
+    def test_inaccessible_workspace_tree_is_reported_instead_of_returning_empty(self):
+        with tempfile.TemporaryDirectory() as workspace:
+            os.makedirs(os.path.join(workspace, "src"))
+
+            def inaccessible_walk(*_args, **kwargs):
+                kwargs["onerror"](PermissionError("permission denied"))
+                return iter(())
+
+            with mock.patch("server_core.os.walk", side_effect=inaccessible_walk):
+                with self.assertRaisesRegex(PermissionError, "permission denied"):
+                    discover_workspace_packages(workspace)
+
diff --git a/src-tauri/src/lib.rs b/src-tauri/src/lib.rs
index 9f082ee..f6a92f9 100644
--- a/src-tauri/src/lib.rs
+++ b/src-tauri/src/lib.rs
@@ -2573,0 +2574,2 @@ pub enum ManagedProcessStatus {
+    Created,
+    Starting,
@@ -2574,0 +2577 @@ pub enum ManagedProcessStatus {
+    Stopping,
@@ -2580,0 +2584,7 @@ pub enum ManagedProcessStatus {
+#[derive(Clone, Serialize)]
+#[serde(rename_all = "camelCase")]
+struct BuildFinishedEvent {
+    success: bool,
+    cancelled: bool,
+}
+
@@ -2588,0 +2599 @@ pub struct ManagedProcess {
+    pub completed_at: Option<u64>,
@@ -2616,2 +2627,3 @@ impl ProcessRegistry {
-            started_at: Some(current_unix_seconds()),
-            status: ManagedProcessStatus::Running,
+            started_at: None,
+            completed_at: None,
+            status: ManagedProcessStatus::Created,
@@ -2628,0 +2641,34 @@ impl ProcessRegistry {
+    fn mark_starting(&self, id: &str) -> bool {
+        let mut processes = self
+            .processes
+            .lock()
+            .unwrap_or_else(std::sync::PoisonError::into_inner);
+        let Some(process) = processes.get_mut(id) else {
+            return false;
+        };
+        if process.status != ManagedProcessStatus::Created {
+            return false;
+        }
+        process.status = ManagedProcessStatus::Starting;
+        true
+    }
+
+    fn mark_running(&self, id: &str) -> bool {
+        let mut processes = self
+            .processes
+            .lock()
+            .unwrap_or_else(std::sync::PoisonError::into_inner);
+        let Some(process) = processes.get_mut(id) else {
+            return false;
+        };
+        match process.status {
+            ManagedProcessStatus::Starting => {
+                process.status = ManagedProcessStatus::Running;
+            }
+            ManagedProcessStatus::Stopping => {}
+            _ => return false,
+        }
+        process.started_at = Some(current_unix_seconds());
+        true
+    }
+
@@ -2662,0 +2709 @@ impl ProcessRegistry {
+            process.completed_at = Some(current_unix_seconds());
@@ -2666 +2713 @@ impl ProcessRegistry {
-    fn cancel(&self, id: &str) -> bool {
+    fn request_stop(&self, id: &str) -> bool {
@@ -2674 +2721,4 @@ impl ProcessRegistry {
-        if process.status != ManagedProcessStatus::Running {
+        if !matches!(
+            process.status,
+            ManagedProcessStatus::Starting | ManagedProcessStatus::Running
+        ) {
@@ -2677,2 +2727,2 @@ impl ProcessRegistry {
-        process.status = ManagedProcessStatus::Cancelled;
-        process.error = Some("Cancelled by user".to_string());
+        process.status = ManagedProcessStatus::Stopping;
+        process.error = Some("Cancellation requested; awaiting process exit".to_string());
@@ -2681,0 +2732,13 @@ impl ProcessRegistry {
+    fn restore_running_after_stop_error(&self, id: &str) {
+        let mut processes = self
+            .processes
+            .lock()
+            .unwrap_or_else(std::sync::PoisonError::into_inner);
+        if let Some(process) = processes.get_mut(id) {
+            if process.status == ManagedProcessStatus::Stopping {
+                process.status = ManagedProcessStatus::Running;
+                process.error = None;
+            }
+        }
+    }
+
@@ -2781 +2844 @@ async fn run_colcon_build_streaming(app: AppHandle) -> Result<(), String> {
-    let workspace = Some(get_default_workspace_dir().unwrap_or_else(|_| "".to_string()));
+    let workspace = Some(get_default_workspace_dir()?);
@@ -2795,0 +2859 @@ async fn run_colcon_build_streaming(app: AppHandle) -> Result<(), String> {
+    managed_process_registry().mark_starting(&build_id);
@@ -2820,3 +2884,18 @@ async fn run_colcon_build_streaming(app: AppHandle) -> Result<(), String> {
-            Err(_) => {
-                let _ = app.emit("build-error", docker_lost_connection_message());
-                let _ = app.emit("build-finished", false);
+            Err(error) => {
+                managed_process_registry().mark_completed(
+                    &build_id,
+                    None,
+                    ManagedProcessStatus::Failed,
+                    Some(error.to_string()),
+                );
+                let _ = app.emit(
+                    "build-error",
+                    format!("{}: {error}", docker_lost_connection_message()),
+                );
+                let _ = app.emit(
+                    "build-finished",
+                    BuildFinishedEvent {
+                        success: false,
+                        cancelled: false,
+                    },
+                );
@@ -2825,0 +2905 @@ async fn run_colcon_build_streaming(app: AppHandle) -> Result<(), String> {
+        managed_process_registry().mark_running(&build_id);
@@ -2841,0 +2922,15 @@ async fn run_colcon_build_streaming(app: AppHandle) -> Result<(), String> {
+                let exit_code = status.code();
+                let cancellation_confirmed =
+                    managed_process_registry()
+                        .get(&build_id)
+                        .is_some_and(|process| {
+                            process.status == ManagedProcessStatus::Stopping
+                                && matches!(exit_code, Some(130 | 137 | 143))
+                        });
+                let final_status = if cancellation_confirmed {
+                    ManagedProcessStatus::Cancelled
+                } else if status.success() {
+                    ManagedProcessStatus::Completed
+                } else {
+                    ManagedProcessStatus::Failed
+                };
@@ -2844,6 +2939,2 @@ async fn run_colcon_build_streaming(app: AppHandle) -> Result<(), String> {
-                    Some(status.code().unwrap_or(-1)),
-                    if status.success() {
-                        ManagedProcessStatus::Completed
-                    } else {
-                        ManagedProcessStatus::Failed
-                    },
+                    Some(exit_code.unwrap_or(-1)),
+                    final_status,
@@ -2853 +2944,12 @@ async fn run_colcon_build_streaming(app: AppHandle) -> Result<(), String> {
-                        Some("colcon build exited with a non-zero status".to_string())
+                        Some(if cancellation_confirmed {
+                            "Build cancellation was confirmed by the process exit".to_string()
+                        } else {
+                            "colcon build exited with a non-zero status".to_string()
+                        })
+                    },
+                );
+                let _ = app.emit(
+                    "build-finished",
+                    BuildFinishedEvent {
+                        success: status.success(),
+                        cancelled: cancellation_confirmed,
@@ -2856 +2957,0 @@ async fn run_colcon_build_streaming(app: AppHandle) -> Result<(), String> {
-                let _ = app.emit("build-finished", status.success());
@@ -2866 +2967,7 @@ async fn run_colcon_build_streaming(app: AppHandle) -> Result<(), String> {
-                let _ = app.emit("build-finished", false);
+                let _ = app.emit(
+                    "build-finished",
+                    BuildFinishedEvent {
+                        success: false,
+                        cancelled: false,
+                    },
+                );
@@ -2907 +3014,3 @@ fn cancel_colcon_build() -> Result<(), String> {
-    managed_process_registry().cancel(&build_id);
+    if !managed_process_registry().request_stop(&build_id) {
+        return Err("The build process is no longer running".to_string());
+    }
@@ -2920,0 +3030 @@ fn cancel_colcon_build() -> Result<(), String> {
+        managed_process_registry().restore_running_after_stop_error(&build_id);
@@ -3603 +3713 @@ fn start_ros_engine_server(
-             python3 -m grpc_tools.protoc \
+             /opt/robotics-studio/venv/bin/python -m grpc_tools.protoc \
@@ -3627 +3737 @@ fn start_ros_engine_server(
-            "pkill -f '[p]ython3 server.py'",
+            "pkill -f '[v]env/bin/python /opt/robotics-studio/server.py'",
@@ -3643 +3753,2 @@ fn start_ros_engine_server(
-             exec python3 /opt/robotics-studio/server.py > /tmp/server.log 2>&1",
+             exec /opt/robotics-studio/venv/bin/python \
+               /opt/robotics-studio/server.py > /tmp/server.log 2>&1",
@@ -5500 +5611 @@ mod process_registry_tests {
-    fn process_registry_tracks_completion_and_cancellation() {
+    fn process_registry_confirms_cancellation_only_after_process_exit() {
@@ -5513,2 +5624,5 @@ mod process_registry_tests {
-        assert_eq!(process.status, ManagedProcessStatus::Running);
-        assert!(registry.cancel("test-proc"));
+        assert_eq!(process.status, ManagedProcessStatus::Created);
+        assert!(process.started_at.is_none());
+        assert!(registry.mark_starting("test-proc"));
+        assert!(registry.mark_running("test-proc"));
+        assert!(registry.request_stop("test-proc"));
@@ -5517 +5631 @@ mod process_registry_tests {
-            ManagedProcessStatus::Cancelled
+            ManagedProcessStatus::Stopping
@@ -5518,0 +5633 @@ mod process_registry_tests {
+        assert!(registry.get("test-proc").unwrap().completed_at.is_none());
@@ -5525 +5640,46 @@ mod process_registry_tests {
-        assert_eq!(registry.get("test-proc").unwrap().exit_code, Some(130));
+        let process = registry.get("test-proc").unwrap();
+        assert_eq!(process.exit_code, Some(130));
+        assert!(process.completed_at.is_some());
+        assert_eq!(process.status, ManagedProcessStatus::Cancelled);
+    }
+
+    #[test]
+    fn process_registry_keeps_pre_spawn_cancellation_pending_until_confirmation() {
+        let registry = ProcessRegistry::default();
+        registry.register(
+            "starting".to_string(),
+            "build".to_string(),
+            vec!["docker".to_string()],
+            None,
+            None,
+        );
+        assert!(registry.mark_starting("starting"));
+        assert!(registry.request_stop("starting"));
+        assert!(registry.mark_running("starting"));
+        let process = registry.get("starting").unwrap();
+        assert_eq!(process.status, ManagedProcessStatus::Stopping);
+        assert!(process.started_at.is_some());
+        assert!(process.completed_at.is_none());
+    }
+
+    #[test]
+    fn process_registry_tracks_spawn_failure_without_claiming_running() {
+        let registry = ProcessRegistry::default();
+        registry.register(
+            "spawn-failure".to_string(),
+            "build".to_string(),
+            vec!["missing-command".to_string()],
+            None,
+            None,
+        );
+        assert!(registry.mark_starting("spawn-failure"));
+        registry.mark_completed(
+            "spawn-failure",
+            None,
+            ManagedProcessStatus::Failed,
+            Some("spawn failed".to_string()),
+        );
+        let process = registry.get("spawn-failure").unwrap();
+        assert_eq!(process.status, ManagedProcessStatus::Failed);
+        assert!(process.started_at.is_none());
+        assert!(process.completed_at.is_some());
@@ -5532 +5692 @@ mod process_registry_tests {
-        assert!(!registry.cancel("missing"));
+        assert!(!registry.request_stop("missing"));
@@ -5541 +5701,3 @@ mod process_registry_tests {
-        assert_eq!(process.status, ManagedProcessStatus::Running);
+        assert_eq!(process.status, ManagedProcessStatus::Created);
+        assert!(registry.mark_starting("done"));
+        assert!(registry.mark_running("done"));
@@ -5547 +5709 @@ mod process_registry_tests {
-        assert!(!registry.cancel("done"));
+        assert!(!registry.request_stop("done"));
diff --git a/src-tauri/src/urdf.rs b/src-tauri/src/urdf.rs
index ca4a186..6f738ec 100644
--- a/src-tauri/src/urdf.rs
+++ b/src-tauri/src/urdf.rs
@@ -37,0 +38,18 @@ pub struct RobotLink {
+    pub collisions: Vec<RobotVisual>,
+}
+
+#[derive(Clone, Debug, PartialEq, Serialize)]
+#[serde(rename_all = "camelCase")]
+pub struct RobotJointLimit {
+    pub lower: Option<f64>,
+    pub upper: Option<f64>,
+    pub effort: Option<f64>,
+    pub velocity: Option<f64>,
+}
+
+#[derive(Clone, Debug, PartialEq, Serialize)]
+#[serde(rename_all = "camelCase")]
+pub struct RobotMimic {
+    pub joint: String,
+    pub multiplier: f64,
+    pub offset: f64,
@@ -48,0 +67,2 @@ pub struct RobotJoint {
+    pub limit: Option<RobotJointLimit>,
+    pub mimic: Option<RobotMimic>,
@@ -123,0 +144,42 @@ fn parse_origin(attributes: &HashMap<String, String>) -> Result<RobotOrigin, Str
+fn parse_optional_number(
+    attributes: &HashMap<String, String>,
+    name: &str,
+    description: &str,
+) -> Result<Option<f64>, String> {
+    attributes
+        .get(name)
+        .map(|value| parse_number(value, description))
+        .transpose()
+}
+
+fn parse_joint_limit(attributes: &HashMap<String, String>) -> Result<RobotJointLimit, String> {
+    let limit = RobotJointLimit {
+        lower: parse_optional_number(attributes, "lower", "joint lower limit")?,
+        upper: parse_optional_number(attributes, "upper", "joint upper limit")?,
+        effort: parse_optional_number(attributes, "effort", "joint effort limit")?,
+        velocity: parse_optional_number(attributes, "velocity", "joint velocity limit")?,
+    };
+    if limit
+        .lower
+        .zip(limit.upper)
+        .is_some_and(|(lower, upper)| lower > upper)
+    {
+        return Err("URDF joint lower limit must not exceed its upper limit".to_string());
+    }
+    if limit.effort.is_some_and(|value| value <= 0.0)
+        || limit.velocity.is_some_and(|value| value <= 0.0)
+    {
+        return Err("URDF joint effort and velocity limits must be positive".to_string());
+    }
+    Ok(limit)
+}
+
+fn parse_mimic(attributes: &HashMap<String, String>) -> Result<RobotMimic, String> {
+    Ok(RobotMimic {
+        joint: required_attribute(attributes, "joint")?.to_string(),
+        multiplier: parse_optional_number(attributes, "multiplier", "mimic multiplier")?
+            .unwrap_or(1.0),
+        offset: parse_optional_number(attributes, "offset", "mimic offset")?.unwrap_or(0.0),
+    })
+}
+
@@ -195,0 +258,42 @@ fn validate_robot_tree(links: &[RobotLink], joints: &[RobotJoint]) -> Result<Str
+    let joints_by_name = joints
+        .iter()
+        .map(|joint| (joint.name.as_str(), joint))
+        .collect::<HashMap<_, _>>();
+    for joint in joints {
+        if let Some(mimic) = &joint.mimic {
+            if !joints_by_name.contains_key(mimic.joint.as_str()) {
+                return Err(format!(
+                    "Joint '{}' mimics undefined joint '{}'",
+                    joint.name, mimic.joint
+                ));
+            }
+            if mimic.joint == joint.name {
+                return Err(format!("Joint '{}' cannot mimic itself", joint.name));
+            }
+        }
+    }
+
+    fn visit_mimic(
+        name: &str,
+        joints: &HashMap<&str, &RobotJoint>,
+        visiting: &mut HashSet<String>,
+        visited: &mut HashSet<String>,
+    ) -> Result<(), String> {
+        if visited.contains(name) {
+            return Ok(());
+        }
+        if !visiting.insert(name.to_string()) {
+            return Err("URDF joint mimic relationships contain a cycle".to_string());
+        }
+        if let Some(source) = joints.get(name).and_then(|joint| joint.mimic.as_ref()) {
+            visit_mimic(&source.joint, joints, visiting, visited)?;
+        }
+        visiting.remove(name);
+        visited.insert(name.to_string());
+        Ok(())
+    }
+    let mut visiting = HashSet::new();
+    let mut visited = HashSet::new();
+    for joint in joints {
+        visit_mimic(&joint.name, &joints_by_name, &mut visiting, &mut visited)?;
+    }
@@ -266,0 +371 @@ pub fn parse_robot_description(content: &str) -> Result<RobotDescription, String
+    let mut current_collision = None::<RobotVisual>;
@@ -304,0 +410 @@ pub fn parse_robot_description(content: &str) -> Result<RobotDescription, String
+                            collisions: Vec::new(),
@@ -317,0 +424,2 @@ pub fn parse_robot_description(content: &str) -> Result<RobotDescription, String
+                            limit: None,
+                            mimic: None,
@@ -331,0 +440,12 @@ pub fn parse_robot_description(content: &str) -> Result<RobotDescription, String
+                    "collision" if parent == Some("link") => {
+                        if current_collision.is_some() {
+                            return Err("Nested URDF <collision> elements are invalid".to_string());
+                        }
+                        current_collision = Some(RobotVisual {
+                            origin: RobotOrigin::default(),
+                            geometry: RobotGeometry::Unsupported {
+                                description: String::new(),
+                            },
+                            color: None,
+                        });
+                    }
@@ -336,0 +457,5 @@ pub fn parse_robot_description(content: &str) -> Result<RobotDescription, String
+                    "origin" if parent == Some("collision") => {
+                        if let Some(collision) = current_collision.as_mut() {
+                            collision.origin = parse_origin(&attributes)?;
+                        }
+                    }
@@ -358,0 +484,33 @@ pub fn parse_robot_description(content: &str) -> Result<RobotDescription, String
+                    "limit" if parent == Some("joint") => {
+                        if let Some(joint) = current_joint.as_mut() {
+                            if joint.limit.is_some() {
+                                return Err(format!(
+                                    "Joint '{}' defines more than one limit",
+                                    joint.name
+                                ));
+                            }
+                            joint.limit = Some(parse_joint_limit(&attributes)?);
+                        }
+                    }
+                    "mimic" if parent == Some("joint") => {
+                        if let Some(joint) = current_joint.as_mut() {
+                            if joint.mimic.is_some() {
+                                return Err(format!(
+                                    "Joint '{}' defines more than one mimic relationship",
+                                    joint.name
+                                ));
+                            }
+                            joint.mimic = Some(parse_mimic(&attributes)?);
+                        }
+                    }
+                    "inertial" if parent == Some("link") => {
+                        warnings.push(
+                            "URDF inertial properties are not used in the preview".to_string(),
+                        );
+                    }
+                    "material" if parent == Some("robot") => {
+                        warnings.push(
+                            "Robot-level named materials are not resolved in the preview"
+                                .to_string(),
+                        );
+                    }
@@ -367 +525,2 @@ pub fn parse_robot_description(content: &str) -> Result<RobotDescription, String
-                        if let Some(visual) = current_visual.as_mut() {
+                        if let Some(visual) = current_visual.as_mut().or(current_collision.as_mut())
+                        {
@@ -398,0 +558,3 @@ pub fn parse_robot_description(content: &str) -> Result<RobotDescription, String
+                        "collision" => {
+                            return Err("URDF collision is missing a geometry".to_string());
+                        }
@@ -440,0 +603,29 @@ pub fn parse_robot_description(content: &str) -> Result<RobotDescription, String
+                    "collision" => {
+                        let collision = current_collision
+                            .take()
+                            .ok_or_else(|| "URDF collision state is inconsistent".to_string())?;
+                        if matches!(
+                            collision.geometry,
+                            RobotGeometry::Unsupported { ref description }
+                                if description.is_empty()
+                        ) {
+                            return Err(
+                                "URDF collision is missing a supported geometry".to_string()
+                            );
+                        }
+                        if let RobotGeometry::Unsupported { description } = &collision.geometry {
+                            warnings.push(format!(
+                                "Collision mesh geometry '{description}' is not rendered in this preview"
+                            ));
+                        }
+                        let link = current_link
+                            .as_mut()
+                            .ok_or_else(|| "URDF collision is outside a link".to_string())?;
+                        if link.collisions.len() >= MAX_VISUALS_PER_LINK {
+                            return Err(format!(
+                                "Link '{}' contains too many collision elements",
+                                link.name
+                            ));
+                        }
+                        link.collisions.push(collision);
+                    }
@@ -604,0 +796,40 @@ mod tests {
+
+    #[test]
+    fn parses_joint_limits_mimic_relationships_and_collision_geometry() {
+        let robot = parse_robot_description(
+            r#"<robot name="bounded">
+                <link name="base"/>
+                <link name="follower">
+                    <collision>
+                        <origin xyz="0 0 0.1"/>
+                        <geometry><box size="0.2 0.3 0.4"/></geometry>
+                    </collision>
+                </link>
+                <joint name="driver" type="revolute">
+                    <parent link="base"/><child link="follower"/>
+                    <limit lower="-1" upper="1" effort="4" velocity="2"/>
+                </joint>
+                <joint name="mimic_driver" type="continuous">
+                    <parent link="follower"/><child link="tool"/>
+                    <mimic joint="driver" multiplier="-2" offset="0.25"/>
+                </joint>
+                <link name="tool"/>
+            </robot>"#,
+        )
+        .unwrap();
+        assert_eq!(robot.joints[0].limit.as_ref().unwrap().lower, Some(-1.0));
+        assert_eq!(robot.joints[0].limit.as_ref().unwrap().upper, Some(1.0));
+        assert_eq!(robot.joints[1].mimic.as_ref().unwrap().joint, "driver");
+        assert_eq!(robot.links[1].collisions[0].origin.xyz, [0.0, 0.0, 0.1]);
+        assert_eq!(robot.links[1].collisions.len(), 1);
+    }
+
+    #[test]
+    fn rejects_invalid_joint_limits_and_mimic_relationships() {
+        let invalid_limit = r#"<robot name="test"><link name="a"/><link name="b"/><joint name="j" type="revolute"><parent link="a"/><child link="b"/><limit lower="2" upper="1"/></joint></robot>"#;
+        assert!(parse_robot_description(invalid_limit).is_err());
+        let missing_mimic = r#"<robot name="test"><link name="a"/><link name="b"/><joint name="j" type="revolute"><parent link="a"/><child link="b"/><mimic joint="missing"/></joint></robot>"#;
+        assert!(parse_robot_description(missing_mimic).is_err());
+        let mimic_cycle = r#"<robot name="test"><link name="a"/><link name="b"/><link name="c"/><joint name="one" type="revolute"><parent link="a"/><child link="b"/><mimic joint="two"/></joint><joint name="two" type="revolute"><parent link="b"/><child link="c"/><mimic joint="one"/></joint></robot>"#;
+        assert!(parse_robot_description(mimic_cycle).is_err());
+    }
diff --git a/src/App.tsx b/src/App.tsx
index 32f14b1..82edb09 100644
--- a/src/App.tsx
+++ b/src/App.tsx
@@ -333,0 +334,2 @@ function App() {
+  const [buildCancellationRequested, setBuildCancellationRequested] =
+    useState(false);
@@ -670,0 +673 @@ function App() {
+    setBuildCancellationRequested(false);
@@ -683,0 +687 @@ function App() {
+    setBuildCancellationRequested(true);
@@ -687,0 +692 @@ function App() {
+      setBuildCancellationRequested(false);
@@ -1042 +1047,4 @@ function App() {
-    const unlistenBuildFinished = listen<boolean>("build-finished", (event) => {
+    const unlistenBuildFinished = listen<{
+      success: boolean;
+      cancelled: boolean;
+    }>("build-finished", (event) => {
@@ -1043,0 +1052 @@ function App() {
+      setBuildCancellationRequested(false);
@@ -1047 +1056,5 @@ function App() {
-          event.payload ? "[✓ Build succeeded]" : "[✗ Build failed]",
+          event.payload.success
+            ? "[✓ Build succeeded]"
+            : event.payload.cancelled
+              ? "[✓ Build cancellation confirmed]"
+              : "[✗ Build failed]",
@@ -1270,2 +1283,7 @@ function App() {
-          <button className="ide-btn" onClick={cancelBuild}>
-            Cancel Build
+          <button
+            className="ide-btn"
+            onClick={cancelBuild}
+            disabled={buildCancellationRequested}
+            aria-live="polite"
+          >
+            {buildCancellationRequested ? "Stopping build…" : "Cancel Build"}
diff --git a/src/components/RobotView.tsx b/src/components/RobotView.tsx
index 624f0eb..5b40546 100644
--- a/src/components/RobotView.tsx
+++ b/src/components/RobotView.tsx
@@ -66 +66,5 @@ function buildPlaceholderRobot(node: string): RobotInstance {
-  return { group, updateJoints: () => undefined };
+  return {
+    group,
+    updateJoints: () => undefined,
+    setCollisionVisible: () => undefined,
+  };
@@ -78,0 +83,2 @@ export default function RobotView({
+  const [showCollisions, setShowCollisions] = useState(false);
+  const collisionVisibleRef = useRef(false);
@@ -104,0 +111,7 @@ export default function RobotView({
+  useEffect(() => {
+    collisionVisibleRef.current = showCollisions;
+    Object.values(robotsRef.current).forEach((robot) =>
+      robot.setCollisionVisible(showCollisions),
+    );
+  }, [showCollisions]);
+
@@ -179,0 +193 @@ export default function RobotView({
+      preview.setCollisionVisible(collisionVisibleRef.current);
@@ -202,0 +217 @@ export default function RobotView({
+        preview.setCollisionVisible(collisionVisibleRef.current);
@@ -222,0 +238 @@ export default function RobotView({
+          robot.setCollisionVisible(collisionVisibleRef.current);
@@ -296,0 +313,15 @@ export default function RobotView({
+      {robotDescription && (
+        <button
+          type="button"
+          aria-pressed={showCollisions}
+          onClick={() => setShowCollisions((visible) => !visible)}
+          style={{
+            position: "absolute",
+            zIndex: 1,
+            left: 8,
+            bottom: 8,
+          }}
+        >
+          {showCollisions ? "Hide" : "Show"} collision geometry
+        </button>
+      )}
diff --git a/src/components/RosGraphExplorer.tsx b/src/components/RosGraphExplorer.tsx
index 5c7d09e..3ce553c 100644
--- a/src/components/RosGraphExplorer.tsx
+++ b/src/components/RosGraphExplorer.tsx
@@ -1,35 +1,6 @@
-export interface RosGraphEndpoint {
-  nodeName: string;
-  nodeNamespace: string;
-  topicType: string;
-  reliability: string;
-  durability: string;
-  history: string;
-  depth: number;
-}
-
-export interface RosGraphNode {
-  name: string;
-  namespace: string;
-}
-
-export interface RosGraphTopic {
-  name: string;
-  types: string[];
-  publishers: RosGraphEndpoint[];
-  subscribers: RosGraphEndpoint[];
-}
-
-export interface RosGraphService {
-  name: string;
-  types: string[];
-  serverNodes: string[];
-  clientNodes: string[];
-}
-
-export interface RosGraphAction {
-  name: string;
-  types: string[];
-  serverNodes: string[];
-  clientNodes: string[];
-}
+import { groupRosGraphByNode, rosNodeId } from "../rosGraph";
+import type {
+  RosGraphEndpoint,
+  RosGraphNodeDetails,
+  RosGraphSnapshot,
+} from "../rosGraph";
@@ -37,7 +8,8 @@ export interface RosGraphAction {
-export interface RosGraphSnapshot {
-  nodes: RosGraphNode[];
-  topics: RosGraphTopic[];
-  services: RosGraphService[];
-  actions: RosGraphAction[];
-  truncated: boolean;
-}
+export type {
+  RosGraphAction,
+  RosGraphEndpoint,
+  RosGraphNode,
+  RosGraphService,
+  RosGraphSnapshot,
+  RosGraphTopic,
+} from "../rosGraph";
@@ -172,0 +145,37 @@ export default function RosGraphExplorer({
+  const nodeDetails = groupRosGraphByNode(graph);
+  const nodeTopicList = (
+    title: string,
+    endpoints: RosGraphNodeDetails["publishers"],
+  ) => (
+    <div>
+      <strong>{title} ({endpoints.length})</strong>
+      {endpoints.map((endpoint, index) => (
+        <div
+          key={`${endpoint.topicName}/${endpoint.topicType}/${index}`}
+          style={{ marginTop: 3, paddingLeft: 8, overflowWrap: "anywhere" }}
+        >
+          <div>{endpoint.topicName}</div>
+          <div style={{ opacity: 0.7 }}>
+            {endpoint.topicType} · {endpoint.reliability},{" "}
+            {endpoint.durability}, {endpoint.history}
+            {endpoint.history === "KEEP_LAST" ? ` (${endpoint.depth})` : ""}
+          </div>
+        </div>
+      ))}
+    </div>
+  );
+  const nodeOperationList = (
+    title: string,
+    operations: RosGraphNodeDetails["services"],
+  ) => (
+    <div>
+      <strong>{title} ({operations.length})</strong>
+      {operations.map((operation) => (
+        <div key={operation.name} style={{ marginTop: 3, overflowWrap: "anywhere" }}>
+          {operation.name}
+          <div style={{ opacity: 0.7 }}>{operation.types.join(", ")}</div>
+        </div>
+      ))}
+    </div>
+  );
+
@@ -189,8 +198,29 @@ export default function RosGraphExplorer({
-        {graph.nodes.map((node) => (
-          <div
-            key={`${node.namespace}/${node.name}`}
-            style={{ overflowWrap: "anywhere", fontFamily: "monospace" }}
-          >
-            {nodePath(node.name, node.namespace)}
-          </div>
-        ))}
+        {graph.nodes.map((node) => {
+          const id = rosNodeId(node.name, node.namespace);
+          const details = nodeDetails.get(id)!;
+          return (
+            <details
+              key={id}
+              style={{
+                padding: "5px 7px",
+                background: "rgba(255,255,255,0.025)",
+                border: "1px solid rgba(255,255,255,0.07)",
+                borderRadius: 4,
+              }}
+            >
+              <summary style={{ cursor: "pointer", overflowWrap: "anywhere" }}>
+                {nodePath(node.name, node.namespace)} · {details.publishers.length} pub,{" "}
+                {details.subscribers.length} sub, {details.services.length} services,{" "}
+                {details.actions.length} actions
+              </summary>
+              <div style={{ padding: "6px 0 2px", display: "grid", gap: 5 }}>
+                {nodeTopicList("Publishes", details.publishers)}
+                {nodeTopicList("Subscribes", details.subscribers)}
+                {nodeOperationList("Service servers", details.services)}
+                {nodeOperationList("Service clients", details.serviceClients)}
+                {nodeOperationList("Action servers", details.actions)}
+                {nodeOperationList("Action clients", details.actionClients)}
+              </div>
+            </details>
+          );
+        })}
diff --git a/src/robotDescription.ts b/src/robotDescription.ts
index 820bb28..4a7386e 100644
--- a/src/robotDescription.ts
+++ b/src/robotDescription.ts
@@ -22,0 +23,14 @@ export interface RobotLink {
+  collisions: RobotVisual[];
+}
+
+export interface RobotJointLimit {
+  lower: number | null;
+  upper: number | null;
+  effort: number | null;
+  velocity: number | null;
+}
+
+export interface RobotMimic {
+  joint: string;
+  multiplier: number;
+  offset: number;
@@ -31,0 +46,2 @@ export interface RobotJoint {
+  limit: RobotJointLimit | null;
+  mimic: RobotMimic | null;
@@ -44,0 +61 @@ export interface RobotInstance {
+  setCollisionVisible: (visible: boolean) => void;
@@ -47 +64 @@ export interface RobotInstance {
-function createVisual(visual: RobotVisual) {
+function createVisual(visual: RobotVisual, collision = false) {
@@ -68,0 +86,3 @@ function createVisual(visual: RobotVisual) {
+  const opacity = collision
+    ? Math.min(color?.[3] ?? 0.35, 0.35)
+    : color?.[3] ?? 1;
@@ -70 +90,3 @@ function createVisual(visual: RobotVisual) {
-    color: color
+    color: collision
+      ? new THREE.Color(0xe34b4b)
+      : color
@@ -73,2 +95,3 @@ function createVisual(visual: RobotVisual) {
-    opacity: color?.[3] ?? 1,
-    transparent: (color?.[3] ?? 1) < 1,
+    opacity,
+    transparent: collision || opacity < 1,
+    wireframe: collision,
@@ -102,0 +126,6 @@ export function buildRobotInstance(
+  const collisionObjects: THREE.Object3D[] = [];
+  const limitPosition = (joint: RobotJoint, position: number) =>
+    Math.max(
+      joint.limit?.lower ?? Number.NEGATIVE_INFINITY,
+      Math.min(joint.limit?.upper ?? Number.POSITIVE_INFINITY, position),
+    );
@@ -114,0 +144,8 @@ export function buildRobotInstance(
+    for (const collision of definition.collisions) {
+      const mesh = createVisual(collision, true);
+      if (mesh) {
+        mesh.visible = false;
+        collisionObjects.push(mesh);
+        link.add(mesh);
+      }
+    }
@@ -142,2 +179,34 @@ export function buildRobotInstance(
-      for (const [name, position] of positions) {
-        if (!Number.isFinite(position)) continue;
+      const resolvedPositions = new Map(positions);
+      for (const joint of description.joints) {
+        if (joint.mimic) {
+          resolvedPositions.delete(joint.name);
+        } else {
+          const position = resolvedPositions.get(joint.name);
+          if (position !== undefined && Number.isFinite(position)) {
+            resolvedPositions.set(joint.name, limitPosition(joint, position));
+          }
+        }
+      }
+      for (let pass = 0; pass < description.joints.length; pass += 1) {
+        let changed = false;
+        for (const joint of description.joints) {
+          if (!joint.mimic || resolvedPositions.has(joint.name)) continue;
+          const sourcePosition = resolvedPositions.get(joint.mimic.joint);
+          if (sourcePosition === undefined || !Number.isFinite(sourcePosition)) {
+            continue;
+          }
+          resolvedPositions.set(
+            joint.name,
+            limitPosition(
+              joint,
+              sourcePosition * joint.mimic.multiplier + joint.mimic.offset,
+            ),
+          );
+          changed = true;
+        }
+        if (!changed) break;
+      }
+
+      for (const [name, rawPosition] of resolvedPositions) {
+        const position = Number.isFinite(rawPosition) ? rawPosition : undefined;
+        if (position === undefined) continue;
@@ -160,0 +230,5 @@ export function buildRobotInstance(
+    setCollisionVisible(visible) {
+      collisionObjects.forEach((object) => {
+        object.visible = visible;
+      });
+    },
diff --git a/tests/robotDescription.test.mjs b/tests/robotDescription.test.mjs
index a8a4d3d..7f2e89e 100644
--- a/tests/robotDescription.test.mjs
+++ b/tests/robotDescription.test.mjs
@@ -21,0 +22 @@ test("builds primitive robot links and applies revolute and prismatic joint stat
+        collisions: [],
@@ -23,2 +24,2 @@ test("builds primitive robot links and applies revolute and prismatic joint stat
-      { name: "arm_link", visuals: [] },
-      { name: "slider_link", visuals: [] },
+      { name: "arm_link", visuals: [], collisions: [] },
+      { name: "slider_link", visuals: [], collisions: [] },
@@ -33,0 +35,2 @@ test("builds primitive robot links and applies revolute and prismatic joint stat
+        limit: null,
+        mimic: null,
@@ -41,0 +45,2 @@ test("builds primitive robot links and applies revolute and prismatic joint stat
+        limit: null,
+        mimic: null,
@@ -82,0 +88 @@ test("skips unsupported mesh visuals without failing the whole model", () => {
+        collisions: [],
@@ -90,0 +97,57 @@ test("skips unsupported mesh visuals without failing the whole model", () => {
+
+test("clamps joint limits, resolves mimic joints, and toggles collision geometry", () => {
+  const instance = buildRobotInstance({
+    name: "bounded_robot",
+    rootLink: "base",
+    warnings: [],
+    links: [
+      { name: "base", visuals: [], collisions: [] },
+      { name: "driver_link", visuals: [], collisions: [] },
+      {
+        name: "follower_link",
+        visuals: [],
+        collisions: [
+          {
+            origin,
+            geometry: { kind: "box", size: [1, 1, 1] },
+            color: null,
+          },
+        ],
+      },
+    ],
+    joints: [
+      {
+        name: "driver",
+        jointType: "revolute",
+        parent: "base",
+        child: "driver_link",
+        origin,
+        axis: [0, 0, 1],
+        limit: { lower: -1, upper: 1, effort: 2, velocity: 3 },
+        mimic: null,
+      },
+      {
+        name: "follower",
+        jointType: "revolute",
+        parent: "driver_link",
+        child: "follower_link",
+        origin,
+        axis: [0, 0, 1],
+        limit: null,
+        mimic: { joint: "driver", multiplier: -2, offset: 0.25 },
+      },
+    ],
+  });
+  const driver = instance.group.getObjectByName("driver:motion");
+  const follower = instance.group.getObjectByName("follower:motion");
+  const collision = instance.group.getObjectByName("follower_link").children[0];
+  assert.ok(driver);
+  assert.ok(follower);
+  assert.equal(collision.visible, false);
+
+  instance.updateJoints(new Map([["driver", 2]]));
+  assert.ok(Math.abs(driver.rotation.z - 1) < 1e-8);
+  assert.ok(Math.abs(follower.rotation.z - (-1.75)) < 1e-8);
+  instance.setCollisionVisible(true);
+  assert.equal(collision.visible, true);
+});
diff --git a/src/rosGraph.ts b/src/rosGraph.ts
new file mode 100644
index 0000000..5e208bd
--- /dev/null
+++ b/src/rosGraph.ts
@@ -0,0 +1,119 @@
+export interface RosGraphEndpoint {
+  nodeName: string;
+  nodeNamespace: string;
+  topicType: string;
+  reliability: string;
+  durability: string;
+  history: string;
+  depth: number;
+}
+
+export interface RosGraphNode {
+  name: string;
+  namespace: string;
+}
+
+export interface RosGraphTopic {
+  name: string;
+  types: string[];
+  publishers: RosGraphEndpoint[];
+  subscribers: RosGraphEndpoint[];
+}
+
+export interface RosGraphService {
+  name: string;
+  types: string[];
+  serverNodes: string[];
+  clientNodes: string[];
+}
+
+export interface RosGraphAction {
+  name: string;
+  types: string[];
+  serverNodes: string[];
+  clientNodes: string[];
+}
+
+export interface RosGraphSnapshot {
+  nodes: RosGraphNode[];
+  topics: RosGraphTopic[];
+  services: RosGraphService[];
+  actions: RosGraphAction[];
+  truncated: boolean;
+}
+
+export interface RosGraphNodeTopic extends RosGraphEndpoint {
+  topicName: string;
+}
+
+export interface RosGraphNodeOperation {
+  name: string;
+  types: string[];
+}
+
+export interface RosGraphNodeDetails {
+  publishers: RosGraphNodeTopic[];
+  subscribers: RosGraphNodeTopic[];
+  services: RosGraphNodeOperation[];
+  serviceClients: RosGraphNodeOperation[];
+  actions: RosGraphNodeOperation[];
+  actionClients: RosGraphNodeOperation[];
+}
+
+export function rosNodeId(nodeName: string, nodeNamespace: string): string {
+  const namespace =
+    nodeNamespace === "/" ? "" : nodeNamespace.replace(/\/+$/, "");
+  return `${namespace}/${nodeName}`;
+}
+
+export function groupRosGraphByNode(
+  graph: RosGraphSnapshot,
+): Map<string, RosGraphNodeDetails> {
+  const result = new Map<string, RosGraphNodeDetails>();
+  for (const node of graph.nodes) {
+    result.set(rosNodeId(node.name, node.namespace), {
+      publishers: [],
+      subscribers: [],
+      services: [],
+      serviceClients: [],
+      actions: [],
+      actionClients: [],
+    });
+  }
+
+  for (const topic of graph.topics) {
+    for (const [endpoints, key] of [
+      [topic.publishers, "publishers"],
+      [topic.subscribers, "subscribers"],
+    ] as const) {
+      for (const endpoint of endpoints) {
+        const node = result.get(
+          rosNodeId(endpoint.nodeName, endpoint.nodeNamespace),
+        );
+        if (node) node[key].push({ ...endpoint, topicName: topic.name });
+      }
+    }
+  }
+
+  for (const service of graph.services) {
+    const operation = { name: service.name, types: service.types };
+    for (const nodeId of service.serverNodes) {
+      result.get(nodeId)?.services.push(operation);
+    }
+    for (const nodeId of service.clientNodes) {
+      result.get(nodeId)?.serviceClients.push(operation);
+    }
+  }
+
+  for (const action of graph.actions) {
+    const operation = { name: action.name, types: action.types };
+    for (const nodeId of action.serverNodes) {
+      result.get(nodeId)?.actions.push(operation);
+    }
+    for (const nodeId of action.clientNodes) {
+      result.get(nodeId)?.actionClients.push(operation);
+    }
+  }
+
+  return result;
+}
diff --git a/tests/rosGraph.test.mjs b/tests/rosGraph.test.mjs
new file mode 100644
index 0000000..a341cb9
--- /dev/null
+++ b/tests/rosGraph.test.mjs
@@ -0,0 +1,80 @@
+import assert from "node:assert/strict";
+import test from "node:test";
+import { groupRosGraphByNode, rosNodeId } from "../src/rosGraph.ts";
+
+const endpoint = (nodeName, nodeNamespace, topicType = "std_msgs/msg/String") => ({
+  nodeName,
+  nodeNamespace,
+  topicType,
+  reliability: "RELIABLE",
+  durability: "VOLATILE",
+  history: "KEEP_LAST",
+  depth: 10,
+});
+
+test("groups graph endpoints by fully qualified node identity", () => {
+  const graph = {
+    nodes: [
+      { name: "worker", namespace: "/" },
+      { name: "worker", namespace: "/robot" },
+    ],
+    topics: [
+      {
+        name: "/status",
+        types: ["std_msgs/msg/String"],
+        publishers: [endpoint("worker", "/")],
+        subscribers: [endpoint("worker", "/robot")],
+      },
+    ],
+    services: [
+      {
+        name: "/reset",
+        types: ["std_srvs/srv/Empty"],
+        serverNodes: ["/worker"],
+        clientNodes: ["/robot/worker"],
+      },
+    ],
+    actions: [
+      {
+        name: "/navigate",
+        types: ["nav2_msgs/action/NavigateToPose"],
+        serverNodes: ["/robot/worker"],
+        clientNodes: ["/worker"],
+      },
+    ],
+    truncated: false,
+  };
+
+  const grouped = groupRosGraphByNode(graph);
+  const rootNode = grouped.get(rosNodeId("worker", "/"));
+  const namespacedNode = grouped.get(rosNodeId("worker", "/robot"));
+
+  assert.equal(rootNode.publishers[0].topicName, "/status");
+  assert.deepEqual(rootNode.services.map(({ name }) => name), ["/reset"]);
+  assert.deepEqual(rootNode.actionClients.map(({ name }) => name), ["/navigate"]);
+  assert.equal(namespacedNode.subscribers[0].topicType, "std_msgs/msg/String");
+  assert.deepEqual(namespacedNode.serviceClients.map(({ name }) => name), ["/reset"]);
+  assert.deepEqual(namespacedNode.actions.map(({ name }) => name), ["/navigate"]);
+});
+
+test("ignores endpoint roles for nodes omitted from a bounded snapshot", () => {
+  const graph = {
+    nodes: [],
+    topics: [{
+      name: "/orphan",
+      types: ["std_msgs/msg/String"],
+      publishers: [endpoint("missing", "/")],
+      subscribers: [],
+    }],
+    services: [{
+      name: "/orphan_service",
+      types: [],
+      serverNodes: ["/missing"],
+      clientNodes: [],
+    }],
+    actions: [],
+    truncated: true,
+  };
+
+  assert.equal(groupRosGraphByNode(graph).size, 0);
+});

````
