# Complete Git Diff

Base: `59d8467ed2c358894fa529425a999ff930fd6984` on `main`. This generated report includes tracked and untracked worktree files and excludes itself.

**Current repository state**: this worktree reflects the current v0.3.0 implementation pass and
release-readiness updates. The project remains not release-ready; the verification summary below
tracks the current status of the in-progress work rather than a completed release candidate.

Diff lines that would otherwise end in whitespace show an explicit marker so this report remains clean under whitespace checks.

## Verification summary

- `npm ci`: passed; 90 packages installed.
- `npm run build`: passed; TypeScript and Vite production build succeeded (943.86 kB chunk warning remains).
- `npm run test:frontend`: passed; 16 tests, including TF composition, parent changes, invalid data, and stale/disappearing transforms.
- Rust format, locked tests, and strict Clippy: passed; 51 Rust tests passed and 1 live-service test was ignored.
- Python service-core tests and module compilation: passed; 44 tests.
- `scripts/docker-integration.sh`: passed on ROS 2 Jazzy/Gazebo Harmonic, including package/underlay discovery, safe service/action workflows, TF/topic inspection, TLS, safety recovery, and Gazebo; no temporary directory or container remained.
- macOS x86_64 `npm run tauri build`: passed; `hdiutil verify` confirmed the DMG checksum. App launch/signing and other platforms remain unverified.
- `npm audit`: 0 vulnerabilities. `cargo audit`: no vulnerability advisories, 13 unsound/unmaintained warnings. Pinned service Python dependencies audit clean; the full ROS image reports 103 advisory matches whose distro backport status is unresolved.
- Latest known hosted CI passed on the base commit; it has not run on this worktree. The v0.3.0 candidate remains not release-ready. No commit, push, tag, or release was created.

## Full diff

````diff
diff --git a/CHANGELOG.md b/CHANGELOG.md
index b353d42..ebdd012 100644
--- a/CHANGELOG.md
+++ b/CHANGELOG.md
@@ -9,13 +9,19 @@ This version is still in development and is not release-ready. See
[trailing space]
 - Selected-workspace ROS package discovery with package build type, version,
   description, dependencies, installed executables, launch/configuration file
-  paths, filtering, refresh, and approved-root package file browsing.
+  paths, filtering, refresh, approved-root package file browsing, and bounded
+  Jazzy underlay visibility with dependency-source resolution.
 - TF parent-child forest and transform inspector with multiple-root,
-  multiple-parent, cycle, stale dynamic-transform, quaternion, and finite-value
-  diagnostics.
+  multiple-parent, cycle, stale/disappearing dynamic-transform, quaternion, and
+  finite-value diagnostics; unambiguous root-to-frame poses are composed.
 - Bounded read-only ROS graph snapshots with node namespaces, topic
   publisher/subscriber identities and QoS, service types/endpoints, and action
   server/client discovery, grouped by fully qualified node identity in the UI.
+- Safe service inspection for allowlisted lifecycle and read-only parameter
+  APIs, with structured JSON schema validation, bounded requests/responses,
+  timeouts, and explicit failures.
+- Safe non-motion Fibonacci action goals with feedback, results, server status,
+  cancellation requests, and terminal updates only after ROS confirms results.
 - Optional bounded live topic inspection with executor-owned subscriptions,
   authenticated start/stop controls, rate and payload limits, and serialized
   message previews.
@@ -23,19 +29,22 @@ This version is still in development and is not release-ready. See
 - TF inspection preserves each transform's ROS source stamp separately from
   receive time; reconnect state shows disconnected last-known data and stale
   dynamic transforms.
-- Bounded URDF parsing for primitive visuals and connected link trees, with
-  safe rejection of malformed or unsupported descriptions.
-- Three.js rendering for box, cylinder, and sphere visuals and supported
-  fixed/revolute/continuous/prismatic joints; live `/joint_states` updates are
-  streamed to the preview. Joint limits and mimic relationships are applied,
-  and supported collision primitives can be toggled in the preview.
+- Bounded URDF parsing and non-executing Xacro subset expansion, with
+  workspace-confined package/relative resources, STL and texture-free DAE
+  geometry, named materials, inertial data, and explicit rejection of
+  unsupported or external resources.
+- Three.js rendering for supported primitive and mesh visuals, fixed,
+  revolute, continuous, and prismatic joints, joint limits, mimic
+  relationships, collision geometry, and inertial frames; live
+  `/joint_states` updates are streamed to the preview.
 - Dependency-free frontend unit tests for TF graph construction and robot
   geometry/joint transforms.
 - Docker integration coverage for selected-workspace package discovery,
   malformed package manifests, installed executables, workspace isolation,
   live ROS graph endpoints, static and dynamic TF plus `/joint_states`
-  delivery, bounded live topic inspection, and valid/invalid SDFormat schema
-  checks.
+  delivery, bounded live topic inspection, safe lifecycle service calls,
+  denied/invalid service and action requests, Fibonacci action feedback,
+  results and confirmed cancellation, and valid/invalid SDFormat schema checks.
 - Build-process state and cancellation feedback that distinguishes a stop
   request from confirmed process termination.
[trailing space]
@@ -50,8 +59,9 @@ This version is still in development and is not release-ready. See
   frontend can inspect them.
 - ROS package discovery no longer interpolates the host workspace path into a
   shell command.
-- Service calls and action goals are intentionally not exposed by the graph
-  explorer, avoiding a bypass of the backend's bounded motion/safety path.
+- Service and action requests are backend-allowlisted and schema-validated:
+  arbitrary or mutating service types and motion-producing actions are not
+  exposed by the graph explorer.
 - Gazebo validates the selected SDF against the installed SDFormat schema
   before startup; external model/mesh resource validation remains incomplete.
 - Workspace package discovery reports inaccessible tree errors rather than
@@ -66,7 +76,8 @@ This version is still in development and is not release-ready. See
 ### Changed
[trailing space]
 - The ROS sidebar distinguishes source packages in the selected workspace
-  from packages available in the ROS underlay.
+  from packages available in the ROS underlay and resolves dependencies
+  against both inventories.
 - The TF panel displays hierarchy and transform values instead of only a
   flat/grouped edge list.
 - Documentation now identifies package, TF, cross-platform, and release
@@ -81,19 +92,21 @@ This version is still in development and is not release-ready. See
 ### Breaking changes
[trailing space]
 - No application-facing breaking change has been identified in this
-  development pass. New protobuf graph and joint-state event fields are
-  additive for protobuf readers; matching bundled protocol definitions must
-  still be deployed together.
+  development pass. New protobuf service/action RPCs and graph/action event
+  fields are additive; matching bundled protocol definitions and service code
+  must still be deployed together.
[trailing space]
 ### Known limitations
[trailing space]
-- Structured service/action invocation and monitoring are not implemented;
-  graph discovery remains read-only.
+- Service invocation is restricted to read-only lifecycle and parameter APIs;
+  action submission is restricted to Fibonacci. Broader service/action type
+  support and frontend workflow testing remain incomplete.
 - TF diagnostics are not a substitute for `tf2` authority validation; missing
   frame authorities cannot be determined from received edges alone.
-- URDF support is a bounded subset: Xacro expansion, safe mesh/model
-  resolution, floating/planar joints, and full URDF semantics remain
-  outstanding. Coordinate-convention validation remains incomplete.
+- Xacro expansion is a safe non-executing subset rather than ROS's full Xacro
+  implementation. Textured/external DAE resources, floating/planar joints,
+  full URDF semantics, and broader coordinate-convention validation remain
+  outstanding.
 - Build, simulation, and service processes do not yet share a complete
   lifecycle manager.
 - Windows filesystem race resistance, full SDF schema/resource validation,
diff --git a/README.md b/README.md
index 57afd49..9cb4429 100644
--- a/README.md
+++ b/README.md
@@ -18,15 +18,20 @@ upfront about that than pretend otherwise.
 - **Live ROS status** — node/topic snapshots and TF updates are streamed over
   authenticated gRPC while the system runs. The ROS sidebar also discovers
   source packages in the selected workspace, shows package metadata and
-  resources, and lets you inspect package files. The live graph shows topic
-  publisher/subscriber QoS, service endpoints, and discovered action servers
-  and clients. Service invocation and action goal monitoring are not yet
-  available.
-- **3D visualization** — preview primitive URDF visuals and articulated
-  joints in the three.js viewport when a `.urdf` file is active; streamed
-  `/joint_states` update supported joints. Without an active URDF, the view
-  keeps its placeholder robot. In-app controls publish bounded `cmd_vel`
-  commands directly (no manual container terminal needed for basic teleop).
+  resources, and lets you inspect package files. Underlay packages are
+  identified separately from workspace packages, with dependency sources
+  reported as workspace, underlay, or missing. The live graph shows topic
+  publisher/subscriber QoS and bounded message inspection. It also exposes
+  read-only lifecycle/parameter service calls and the allowlisted non-motion
+  Fibonacci action workflow; arbitrary service types and motion-producing
+  actions are not enabled.
+- **3D visualization** — preview supported URDF and Xacro descriptions,
+  primitive and workspace-confined STL/texture-free DAE visuals, supported
+  joints, inertial frames, and collision geometry in the three.js viewport.
+  Streamed `/joint_states` update supported joints. Without an active robot
+  description, the view keeps its placeholder robot. In-app controls publish
+  bounded `cmd_vel` commands directly (no manual container terminal needed
+  for basic teleop).
 - **Custom Gazebo world support** — load your own world file instead of
   being limited to the bundled demo.
 - **AI assistant with real context** — ask questions about your open file,
@@ -168,28 +173,36 @@ any of this — the app manages the environment for you.
[trailing space]
 ## Known limitations (please break things and tell me what you find)
[trailing space]
-- The URDF preview supports box, cylinder, and sphere visuals plus
-  fixed/revolute/continuous/prismatic joints. It does not expand Xacro or
-  resolve mesh/package resources; mesh visuals are reported and skipped.
-  Joint limits and mimic relationships are applied, and supported primitive
-  collision geometry can be toggled. Inertial properties and robot-level
-  named materials are not used in the preview. Floating/planar joints and full
-  URDF semantics are not supported. The ROS to three.js coordinate mapping is
-  a first pass and still needs convention validation.
+- The URDF preview supports box, cylinder, sphere, STL, and texture-free DAE
+  visuals plus fixed/revolute/continuous/prismatic joints. Xacro uses a
+  non-executing subset (arguments, properties, arithmetic, macros, workspace-
+  local includes, and boolean conditionals); unsupported constructs fail
+  closed. `package://` and relative mesh paths are confined to the selected
+  workspace and its discovered packages. DAE image/texture and external
+  resource references are rejected. Joint limits, mimic relationships,
+  named material colors, inertial origins, and supported primitive collision
+  geometry are represented in the preview. Inertial frames can be shown with
+  a scalar radius-of-gyration marker. Floating/planar joints and full URDF
+  semantics are not supported. ROS fixed-axis RPY rotations and the Z-up to
+  Three.js coordinate mapping are applied.
 - The TF tab builds a live transform forest and warns about multiple roots,
   cycles, multiple parents, stale dynamic transforms, non-finite values, and
   invalid quaternions. It displays the ROS source stamp separately from the
   local receive age, so replayed old data does not appear freshly received.
+  Stale dynamic edges are labeled after five seconds and reported as having
+  no recent updates after 30 seconds; an old dynamic parent is replaced after
+  a child changes parent and the old edge has gone stale. The inspector
+  composes root-to-frame poses when the path is unambiguous and valid.
   Disconnection is explicit; static `/tf_static` edges use transient-local
   QoS and are not marked stale. This is diagnostic validation, not a
   substitute for `tf2` authority or full ROS graph validation.
-- ROS graph snapshots are bounded and read-only, with optional live topic
-  inspection limited to four topics, five samples per second per topic, and
-  bounded serialized payloads. Service invocation and action goal
-  submission/monitoring are not implemented because arbitrary operations
-  could bypass the motion safety path. Package discovery reads source
-  package manifests and workspace install outputs, but does not show every
-  underlay package.
+- ROS graph snapshots are bounded. Live topic inspection is limited to four
+  topics, five samples per second per topic, and bounded serialized payloads.
+  Service invocation is restricted to read-only lifecycle/parameter types;
+  action goals are restricted to the non-motion Fibonacci type. Arbitrary
+  service types and motion-producing actions are not enabled. Package
+  discovery reports bounded workspace and Jazzy-underlay packages, not every
+  package installed in the container.
 - SDF imports are structurally checked, and the world is validated against
   the installed SDFormat schema before Gazebo starts. This does not resolve or
   validate every external model/mesh resource.
@@ -315,16 +328,18 @@ and dependency audit, and isolated Python service-core tests from the repository
 root. Install `cargo-audit` 0.23.0 if it is not already available.
[trailing space]
 ```sh
+npm ci
 cargo install cargo-audit --version 0.23.0 --locked
 npm run build
-npm audit --audit-level=moderate
+npm run test:frontend
+npm audit
 cargo audit --file src-tauri/Cargo.lock
 cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
 cargo check --manifest-path src-tauri/Cargo.toml --locked
 cargo test --manifest-path src-tauri/Cargo.toml --locked
-cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features --locked -- -D warnings
+cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
 python3 -m unittest discover -s src-tauri/resources/devcontainer -p 'test_*.py' -v
-npm run test:frontend
+python3 -m py_compile src-tauri/resources/devcontainer/server.py src-tauri/resources/devcontainer/server_core.py
 ```
[trailing space]
 For the live container integration (Docker Desktop or Docker Engine must be
@@ -335,12 +350,13 @@ running), execute:
 ```
[trailing space]
 The script builds the supported ROS 2 Jazzy/Gazebo Harmonic image, mounts a
-temporary workspace, verifies selected-workspace package discovery, the
-pinned-TLS Rust client, gRPC authentication, emergency-stop gating, event
-replay after service restart, live `/tf_static` delivery, Gazebo world/model
-readiness, reset, and ROS-Gazebo topic bridging. It performs no robot movement
-and removes its temporary container and workspace on exit. CI runs the same
-script on its Linux runner.
+temporary workspace, verifies workspace/underlay package discovery, safe
+lifecycle service calls, rejected unsafe requests, Fibonacci action feedback,
+results and confirmed cancellation, bounded topic inspection, static/dynamic
+TF and joint states, pinned-TLS authentication, emergency-stop gating, event
+replay after service restart, Gazebo readiness/reset and ROS-Gazebo topic
+bridging. It performs no robot movement and removes its temporary container
+and workspace on exit. CI runs the same script on its Linux runner.
[trailing space]
 Unit/build checks are not a substitute for live Docker/ROS 2/Gazebo checks.
 They require Docker Desktop running for an integration environment; if it is
diff --git a/RELEASE_READINESS.md b/RELEASE_READINESS.md
index a919edb..c5cbc5f 100644
--- a/RELEASE_READINESS.md
+++ b/RELEASE_READINESS.md
@@ -2,12 +2,13 @@
[trailing space]
 ## Decision
[trailing space]
-**Not ready for release.** The worktree is based on `e9d7459` on `main` and
-contains uncommitted ROS graph, TF integration, URDF preview, build lifecycle,
-package-discovery test changes, and bounded live ROS topic inspection. Local
-validation and Docker integration pass, but substantial v0.3.0 acceptance
-criteria remain incomplete. Hosted CI has not run against this worktree. No
-commit, push, tag, or release was created.
+**Not ready for release.** The worktree is based on `59d8467` on `main` and
+contains uncommitted ROS package, service/action, Xacro/mesh rendering, TF,
+filesystem, and process-related changes. Local validation and the complete
+Jazzy/Harmonic Docker integration pass, but substantial v0.3.0 acceptance
+criteria remain incomplete. The latest known hosted CI passed on the base
+commit; CI has not run against this worktree. No commit, push, tag, or release
+was created.
[trailing space]
 ## Acceptance checklist
[trailing space]
@@ -16,25 +17,25 @@ commit, push, tag, or release was created.
 | 1 | gRPC authentication and TLS | PARTIAL | Local Docker integration confirms the pinned-TLS authenticated client and rejects unauthenticated calls. Cross-platform and packaged-app transport testing remains. |
 | 2 | Trusted ROS service installation | PARTIAL | Service code and protocol are installed outside the writable workspace mount and exercised in Docker. Upgrade/migration paths need broader platform testing. |
 | 3 | Workspace filesystem boundaries | PARTIAL | Unix descriptor-rooted file operations and package-path containment have regression tests. Windows race resistance and several other operations remain incomplete. |
-| 4 | Dependency security | PARTIAL | Current local `npm audit` reports no vulnerabilities. `cargo audit` reports no vulnerability findings and 13 warnings, including an unsound `glib` version and unmaintained transitive crates. `pip-audit` reports no known vulnerabilities in the integration image's isolated, pinned PyPI-installed packages; the full image inventory includes ROS distro packages not resolvable from PyPI. Hosted CI has not run on this worktree. |
+| 4 | Dependency security | PARTIAL | `npm audit` reports 0 vulnerabilities. `cargo audit` reports no vulnerability findings but 13 warnings, including unsound Linux-target `glib` 0.18.5 and unmaintained transitive crates; current GTK/WebKit dependencies constrain the Linux stack. The three pinned PyPI service packages audit clean. A whole-image `pip-audit` finds 103 advisory matches across 10 Python distributions in the ROS base image; their Ubuntu/ROS backport status needs upstream package-level review. Hosted CI has not run on this worktree. |
 | 5 | Critical ROS event recovery | PARTIAL | Journal/replay, gap handling, and service-restart recovery are tested. The UI does not acknowledge event application and telemetry is best-effort. |
 | 6 | Motion bounds and emergency stop | PARTIAL | Rust/Python limits, stop latch, persistence, and restart behavior have tests and integration checks. This is software-only and not safety-rated. |
 | 7 | Workspace selection and isolation | PARTIAL | Selected workspace is persisted and passed to container lookup/build/simulation. Package discovery now filters to source/install trees inside that mounted workspace; full operation/platform matrix remains unverified. |
-| 8 | ROS package explorer | PARTIAL | Selected-workspace package metadata, build type, dependencies, executable outputs, launch/config files, filtering, refresh, and file browsing are implemented and tested. Underlay packages, dependency resolution, and full package content workflows remain out of scope. |
-| 9 | ROS node explorer | PARTIAL | Bounded node names/namespaces and per-topic endpoint node identities are streamed and grouped in the UI by fully qualified node identity, including service/action roles. Full ROS lifecycle-state details remain. |
+| 8 | ROS package explorer | PARTIAL | Selected-workspace packages and a bounded Jazzy underlay inventory are shown with workspace-over-underlay precedence and dependency sources (`workspace`, `underlay`, or `missing`). Metadata, executables, launch/config paths, filtering, refresh, and confined workspace file browsing are tested. Full ROS package-index/underlay coverage and all content workflows remain. |
+| 9 | ROS node explorer | PARTIAL | Bounded node names/namespaces and per-topic endpoint node identities are streamed and grouped by fully qualified identity, including service/action roles. Available lifecycle service state can be queried through the safe service flow; automatic lifecycle state discovery and refresh remain incomplete. |
 | 10 | ROS topic explorer | PARTIAL | Bounded publisher/subscriber identities and QoS policies are streamed and shown. Authenticated live-message inspection is available with executor-owned subscriptions, four-topic/five-Hz/payload bounds, and stop controls. ROS topic type support depends on installed message packages; broader UX and lifecycle testing remain. |
-| 11 | ROS service explorer | PARTIAL | Bounded service names/types and discovered server/client nodes are shown read-only. Structured invocation, validation, timeout, and cancellation remain absent to avoid bypassing safety controls. |
-| 12 | ROS action explorer | PARTIAL | Action names/types and discovered server/client nodes are shown read-only. Goal submission, feedback/results, and cancellation are not implemented. |
+| 11 | ROS service explorer | PARTIAL | Allowlisted lifecycle and read-only parameter service types can be invoked with structured JSON/schema validation, bounded inputs/responses, timeout and explicit error reporting. Docker verifies lifecycle `GetState`, denied mutating types, and invalid fields. Service cancellation and broader workflow testing remain. |
+| 12 | ROS action explorer | PARTIAL | The non-motion Fibonacci allowlist supports schema-checked goal submission, feedback, result, server-unavailable/error states, cancellation acknowledgement, and terminal status only after a ROS result. Docker verifies success and cancellation; broader action types, reconnect behavior, and timeout recovery remain. |
 | 13 | Consistent ROS graph refresh | PARTIAL | Nodes, topics, services, and actions are snapshotted from one service timer and bounded. Automatic client subscriptions and full graph transactionality remain absent. |
-| 14 | TF hierarchy and transform inspection | PARTIAL | A UI forest, translation/quaternion/RPY inspector, multiple-root/multiple-parent/cycle checks, and pure unit tests are implemented. It is not authoritative `tf2` validation. |
-| 15 | TF stale and static-frame behavior | PARTIAL | Dynamic edges become stale after five seconds using event receive time, independent of the preserved ROS source stamp; `/tf_static` uses reliable transient-local QoS and is exempt. The UI labels last-known data on disconnect. Docker integration verifies static/dynamic delivery and source stamps. Frame disappearance/relationship changes and full TF composition remain unverified. |
-| 16 | URDF parsing and validation | PARTIAL | A bounded parser rejects malformed/unsafe XML, validates link/joint-tree structure, primitive dimensions, joint limits, and mimic references/cycles, and reports unsupported mesh visuals. Full URDF semantics, Xacro, and resource validation remain. |
-| 17 | Xacro expansion | NOT STARTED | No controlled ROS Xacro expansion workflow exists. |
-| 18 | Mesh/resource resolution | NOT STARTED | `package://`, relative model paths, and mesh formats are not resolved for rendering. |
-| 19 | Robot model and joint-state rendering | PARTIAL | Active `.urdf` files render box/cylinder/sphere visuals and supported joint transforms; limits, mimic joints, and a collision-geometry toggle are supported. Bounded `/joint_states` updates are covered by Docker integration. No mesh/Xacro, coordinate-convention verification, or visual desktop test yet; inertial and named robot-level materials are not rendered. |
+| 14 | TF hierarchy and transform inspection | PARTIAL | UI forest, multiple-root/multiple-parent/cycle validation, quaternion/RPY inspection, and root-relative pose composition for unambiguous valid chains are covered by unit tests. It is not authoritative `tf2` validation. |
+| 15 | TF stale and static-frame behavior | PARTIAL | Dynamic edges become stale after five seconds and are flagged as having no updates after 30 seconds; stale dynamic parent edges are replaced after an observed parent change. `/tf_static` uses reliable transient-local QoS and is exempt. Docker verifies static/dynamic delivery and source stamps; live reconnection and frame-authority behavior remain incomplete. |
+| 16 | URDF parsing and validation | PARTIAL | Bounded parsing validates XML, link/joint trees, supported joint types/limits, mimic references/cycles, primitive/mesh geometry, materials, and inertial data. Floating/planar joints and full URDF semantics remain unsupported. |
+| 17 | Xacro expansion | PARTIAL | A bounded non-executing subset supports arguments, properties, arithmetic, macros, workspace-local includes, and boolean conditionals, with cycle/expansion limits and fail-closed unsupported expressions. This is not controlled ROS `xacro` tooling or full Xacro compatibility. |
+| 18 | Mesh/resource resolution | PARTIAL | Workspace/package-confined relative and `package://` STL and texture-free DAE meshes are loaded with file/size bounds. External DAE textures/references and broader resource semantics fail closed; visual desktop verification remains. |
+| 19 | Robot model and joint-state rendering | PARTIAL | URDF/Xacro visuals, supported joints, live `/joint_states`, limits, mimic, collision geometry, materials, inertial frames, mesh transforms, and scene disposal are implemented and covered by focused tests. Native preview and broader coordinate-convention verification remain unperformed. |
 | 20 | Process lifecycle manager | PARTIAL | Build states distinguish starting/running/stopping/terminal outcomes; stop requests are only reported as cancellation after process exit. Docker, Gazebo, bridge, and ROS service processes still lack a shared lifecycle manager, generalized timeouts, and application-shutdown cleanup. |
 | 21 | Gazebo and SDF | PARTIAL | Jazzy/Harmonic startup, reset, bridge, and pre-launch `gz sdf --check` schema validation are integration-tested. Comprehensive external resource resolution and validation remain absent. |
-| 22 | Frontend workflow tests/accessibility | PARTIAL | Eleven dependency-free TF graph and robot-rendering tests plus production type-check/build pass. React workflow tests, accessibility review, and coverage of workspace/ROS/safety UI remain. |
+| 22 | Frontend workflow tests/accessibility | PARTIAL | Twelve dependency-free TF graph and robot-rendering tests plus production type-check/build pass. React workflow tests, accessibility review, and coverage of workspace/ROS/safety UI remain. |
 | 23 | Cross-platform runtime and release artifacts | PARTIAL | A local macOS x86_64 `.app` and `.dmg` were built successfully. They have not been launch-tested or notarized; macOS Apple Silicon, Linux, and Windows release builds/runtime and v0.2.0 upgrade verification remain. |
[trailing space]
 ## Security and safety notes
@@ -52,15 +53,16 @@ commit, push, tag, or release was created.
 - TF validation is diagnostic. It does not replace ROS `tf2`, identify
   transform authorities, or make motion-safety guarantees.
 - Existing TLS/authentication, emergency-stop, and filesystem controls were
-  not intentionally weakened. No new independent security audit was performed
-  for this implementation pass.
-- The Rust dependency audit reports an unsound `glib` dependency warning in
-  addition to unmaintained transitive dependencies. `pip-audit` found no known
-  vulnerabilities in `grpcio`, `grpcio-tools`, and `setuptools` as installed
-  in an isolated virtual environment in the integration image. The Dockerfile
-  pins these PyPI packages without replacing ROS/colcon's system Python
-  dependencies. A whole-image pip audit cannot resolve ROS-distributed
-  packages such as `action-msgs` from PyPI.
+  not intentionally weakened. No independent application security review was
+  performed for this implementation pass.
+- `cargo audit` reports no vulnerability advisories but 13 warnings, including
+  unsound Linux-target `glib` 0.18.5 and unmaintained transitive crates. The
+  `glib` dependency is part of the GTK/WebKit Linux Tauri stack. The pinned
+  service dependencies `grpcio==1.85.1`, `grpcio-tools==1.85.1`, and
+  `setuptools==84.2.1` audit clean in isolation. Whole-image `pip-audit`
+  reports 103 advisory matches across ten Python distributions, including
+  ROS/Ubuntu-provided packages; PyPI auditing does not determine whether distro
+  maintainers backported fixes.
 - The software emergency stop cannot guarantee physical braking or replace a
   safety-rated controller or physical emergency-stop circuit.
[trailing space]
@@ -68,43 +70,45 @@ commit, push, tag, or release was created.
[trailing space]
 | Check | Result |
 |---|---|
-| `npm ci` | Pass; 91 packages installed. |
-| `npm run build` | Pass; Vite reports a large JavaScript chunk warning (~832 kB). |
-| `npm run test:frontend` | Pass: 11 TF graph and robot-description tests. |
+| `npm ci` | Pass; 90 packages installed and 0 npm vulnerabilities reported. |
+| `npm run build` | Pass; TypeScript check and Vite production build succeed; 941.5 kB JavaScript chunk warning remains. |
+| `npm run test:frontend` | Pass: 16 dependency-free TF graph and robot-rendering tests. |
 | `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Pass. |
-| `cargo check --manifest-path src-tauri/Cargo.toml --locked` | Pass. |
-| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | Pass: 43 passed, 1 ignored (live-service test). |
-| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features --locked -- -D warnings` | Pass. |
-| Python unittest discovery from `src-tauri/resources/devcontainer` | Pass: 43 tests, including inaccessible workspace-tree reporting. |
-| Python `py_compile` for service and tests | Pass. |
+| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | Pass: 51 passed, 1 ignored (live-service test). |
+| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | Pass. |
+| Python unittest discovery from `src-tauri/resources/devcontainer` | Pass: 44 tests. |
+| Python `py_compile` for ROS service modules | Pass. |
 | `bash -n scripts/docker-integration.sh` | Pass. |
-| `scripts/docker-integration.sh` | Pass: selected-workspace package discovery, live graph endpoints, bounded topic inspection and stop, static/dynamic TF and joint-state delivery, SDFormat checks, pinned TLS/auth, stop/reset enforcement, event replay/restart, and Gazebo. Cleanup exited 0; a follow-up check found no integration container or temporary directory. |
-| `npm run tauri build` | Pass: built macOS x86_64 `.app` and `.dmg`; `hdiutil verify` confirms the DMG checksum. Application launch, signing/notarization, and non-macOS bundles were not verified. |
-| `cargo audit --file src-tauri/Cargo.lock` | Pass with 13 allowed warnings: no vulnerability findings; warnings include unmaintained crates and unsound `glib`. |
-| `npm audit --audit-level=moderate` | Pass: 0 vulnerabilities. |
-| Python dependency audit | Pass: no known vulnerabilities in the isolated environment's pinned `grpcio==1.85.1`, `grpcio-tools==1.85.1`, `protobuf==7.36.2`, `setuptools==84.2.1`, and `typing_extensions==4.16.0`. The full system pip inventory could not be resolved because it includes ROS distro packages. |
-| ROS/colcon Python compatibility | Pass: gRPC uses the isolated Python 3.12 environment, system Python retains `setuptools==68.1.2`, and a representative `colcon build` completes. |
+| `scripts/docker-integration.sh` | Pass: selected-workspace/underlay discovery; safe lifecycle service calls and denied/invalid requests; Fibonacci action feedback, result, unavailable-server reporting and confirmed cancellation; bounded topic inspection; static/dynamic TF and joint-state delivery; SDFormat; pinned TLS/auth; stop/reset; event replay/restart; and Gazebo. Exit 0; no integration container or temporary directory remains. |
+| `npm run tauri build` and `hdiutil verify` | Pass: built the macOS x86_64 `.app` and `.dmg`; DMG checksum is valid. Launch, signing/notarization, and non-macOS bundles remain unverified. |
+| `cd src-tauri && cargo audit` | Pass with 13 warnings: no vulnerability findings; unsound `glib` and unmaintained transitive dependencies remain. |
+| `npm audit` | Pass: 0 vulnerabilities. |
+| `pip-audit -r` with the three pinned service packages | Pass: no known vulnerabilities. |
+| Full integration-image `pip-audit` | Reports 103 advisory matches across ten distributions and ROS packages not resolvable from PyPI; distro backport status remains unverified. |
 | Hosted GitHub Actions | Not run against this uncommitted worktree. |
[trailing space]
 ## Release blockers
[trailing space]
-- Complete live ROS node/topic endpoint details and implement service/action
-  safe structured invocation/goal monitoring without bypassing the motion
-  safety controls. Bounded read-only graph discovery is implemented.
-- Validate TF reconnect/disappearance behavior against a live ROS publisher;
-  static `/tf_static` delivery is covered by the Docker integration test.
-- Complete URDF semantics and coordinate validation; implement Xacro expansion
-  and safe package/model resource resolution. The current bounded parser,
-  primitive preview, and live joint-state updates are partial support only.
+- Complete automatic lifecycle-state inspection, graph refresh/subscription
+  recovery, TF frame disappearance/relationship updates, and transform
+  composition/validation against live ROS behavior.
+- Expand the intentionally narrow safe service/action allowlists only with
+  reviewed non-motion APIs; add service cancellation and frontend workflow
+  tests.
+- Integrate controlled ROS Xacro tooling or document and approve the current
+  non-executing subset as intended product behavior. Complete remaining URDF
+  semantics and coordinate-convention verification; textured/external mesh
+  resources remain rejected.
 - Unify application-managed process state, failure reporting, timeouts,
   cancellation confirmation, and shutdown cleanup across supported platforms.
 - Complete filesystem race hardening for remaining operations and Windows.
 - Add frontend workflow tests and conduct accessibility review.
 - Run hosted CI on the modified worktree; verify Linux, Windows, macOS Intel,
-  and Apple Silicon packaging/runtime paths and test upgrades from v0.2.0. Only
-  the local macOS x86_64 bundle build has been produced; it has not been
-  launch-tested or notarized.
+  and Apple Silicon packaging/runtime paths and test upgrades from v0.2.0. The
+  local macOS x86_64 bundle builds, but has not been launch-tested or notarized.
 - Complete external SDF model/mesh resource resolution and validation.
-- Re-run current dependency audits and reassess disclosed RustSec advisories.
+- Resolve or document the RustSec warnings and establish whether the ROS base
+  image's Python advisory findings are fixed by distro backports; do not
+  suppress these findings without evidence.
 - Prepare and verify unsigned/signed release artifacts as applicable; do not
   publish a tag or release until explicitly approved.
diff --git a/docs/architecture.md b/docs/architecture.md
index 3015e0b..32f9dcb 100644
--- a/docs/architecture.md
+++ b/docs/architecture.md
@@ -94,16 +94,22 @@ coverage remain incomplete.
 The current service streams bounded node, topic endpoint/QoS, service, and
 action snapshots alongside transform and odometry updates, safety state, and
 event-recovery metadata. The package explorer scans package manifests in the
-selected workspace's source and install trees rather than treating ROS
-underlay packages as workspace packages. It reports bounded metadata,
-installed executables, launch/configuration files, and allows the frontend to
-browse package files through approved-root file commands. The graph explorer
-supports bounded live topic inspection through subscriptions created and
-destroyed on the ROS executor. Inspection is authenticated and limited to four
-topics, five messages per second per topic, depth-five best-effort queues, and
-16 KiB serialized payloads. It does not invoke arbitrary ROS services or submit
-action goals, which could otherwise bypass the application's motion safety
-path.
+selected workspace's source and install trees and a bounded Jazzy underlay
+inventory. Workspace packages take precedence; dependencies are annotated as
+workspace, underlay, or missing. It reports bounded metadata, installed
+executables, launch/configuration files, and allows the frontend to browse
+workspace package files through approved-root file commands.
+
+The graph explorer supports bounded live topic inspection through
+subscriptions created and destroyed on the ROS executor. Inspection is
+authenticated and limited to four topics, five messages per second per topic,
+depth-five best-effort queues, and 16 KiB serialized payloads. Structured
+service calls are restricted to read-only lifecycle and parameter types,
+validated against ROS request schemas, and bounded by request, response, and
+timeout limits. Action goals are restricted to the non-motion Fibonacci
+action; feedback, results, server errors, cancellation acknowledgement, and
+confirmed terminal status are streamed. These are narrow allowlists, not
+arbitrary service/action access or a replacement for motion-safety controls.
[trailing space]
 TF updates include their source topic. `/tf_static` uses transient-local
 reliable QoS so late subscribers receive latched static transforms. The
@@ -112,20 +118,26 @@ multiple parents, stale dynamic transforms, non-finite values, and invalid
 quaternions, and provides translation/quaternion/RPY inspection. ROS message
 source stamps are retained independently from local event receive time; stale
 replayed dynamic transforms are not made fresh by reconnect. The UI explicitly
-labels disconnected transform data as last-known. It cannot infer missing
-transform authorities from edge data alone, and is diagnostic validation
-rather than a replacement for `tf2`.
-
-When an active editor file is a `.urdf`, the native backend parses a bounded
-URDF subset and the frontend renders box, cylinder, and sphere visuals with a
-connected link tree. Fixed, revolute, continuous, and prismatic joint states
-are updated from bounded `/joint_states` telemetry, with supported joint
-limits and mimic relationships applied. Supported primitive collision
-geometry can be toggled in the preview. Mesh visuals are surfaced as warnings
-and skipped; inertial properties and robot-level named materials are not used
-in rendering. Xacro expansion, mesh/package resource resolution,
-floating/planar joints, and full URDF semantics are not implemented. The
-ROS-to-three.js coordinate mapping still needs broader convention validation.
+labels disconnected transform data as last-known, marks dynamic edges stale
+after five seconds, and calls out edges without updates after 30 seconds. A
+stale dynamic parent is replaced after a later parent change; simultaneously
+conflicting parents remain visible as an error. Unambiguous valid chains are
+composed into a root-relative pose. The UI cannot infer missing transform
+authorities from edge data alone, and is diagnostic validation rather than a
+replacement for `tf2`.
+
+When an active editor file is a `.urdf` or `.xacro`, the native backend parses
+bounded XML and performs a non-executing Xacro subset expansion. Workspace-
+confined relative and `package://` STL and texture-free DAE resources are
+loaded into the Three.js preview alongside supported primitives. Fixed,
+revolute, continuous, and prismatic joint states are updated from bounded
+`/joint_states` telemetry; supported limits, mimic relationships, named
+material colors, inertial origins, and collision geometry are represented.
+Meshes and scene resources are disposed when the model changes. This is not
+the full ROS Xacro implementation: unsupported macros/expressions fail closed,
+external DAE resources are rejected, floating/planar joints and full URDF
+semantics remain unsupported, and the coordinate mapping still needs broader
+visual convention validation.
[trailing space]
 SDF import checks XML well-formedness, an SDF root, a world element, supported
 declared versions, and rejects DTDs. Before launching Gazebo, the container's
@@ -137,9 +149,12 @@ validated.
[trailing space]
 Run unit/build checks with the commands in the README. The live integration
 script `scripts/docker-integration.sh` builds the container image and exercises
-the real ROS service, selected-workspace package discovery, pinned TLS, event
-replay after restart, live `/tf_static` and `/joint_states` delivery, Gazebo
-world readiness/reset, and ROS-Gazebo topic bridging. Unit tests cover TF graph
-validation, URDF parsing, primitive/joint rendering, and package metadata
-discovery. This does not substitute for visual desktop validation on every
-target operating system or testing a physical robot.
+the real ROS service, selected-workspace and underlay package discovery,
+authenticated safe lifecycle service calls, denied/invalid service and action
+requests, Fibonacci feedback/result/cancellation, pinned TLS, event replay
+after restart, bounded topic inspection, live `/tf_static` and `/joint_states`
+delivery, Gazebo world readiness/reset, and ROS-Gazebo topic bridging. Unit
+tests cover TF graph validation, URDF parsing, Xacro expansion, mesh resolution,
+rendering, and package metadata discovery. This does not substitute for native
+visual validation on each target OS, broader ROS action types, or testing a
+physical robot.
diff --git a/scripts/docker-integration.sh b/scripts/docker-integration.sh
index 5526a0c..c616e9d 100755
--- a/scripts/docker-integration.sh
+++ b/scripts/docker-integration.sh
@@ -113,25 +113,34 @@ docker exec -i "$container" bash -c 'cd /opt/robotics-studio && python3 -' <<'PY
 import sys
[trailing space]
 sys.path.insert(0, "/opt/robotics-studio")
-from server_core import discover_workspace_packages
+from server_core import discover_ros_packages
[trailing space]
 packages = {
     package["name"]: package
-    for package in discover_workspace_packages("/workspace")
+    for package in discover_ros_packages("/workspace")
 }
-assert set(packages) == {
+workspace_packages = {
+    name for name, package in packages.items() if package["source"] == "workspace"
+}
+assert workspace_packages == {
     "malformed_fixture",
     "robotics_studio_integration_fixture",
-}, sorted(packages)
+}, sorted(workspace_packages)
 fixture = packages["robotics_studio_integration_fixture"]
 assert fixture["package_type"] == "ament_cmake"
 assert fixture["version"] == "1.0.0"
 assert fixture["description"] == "Temporary package discovery integration fixture"
 assert fixture["dependencies"] == ["ament_cmake", "rclpy"]
+assert fixture["dependency_sources"] == {
+    "ament_cmake": "underlay",
+    "rclpy": "underlay",
+}
 assert fixture["executables"] == ["fixture_node"]
 assert fixture["launch_files"] == ["launch/start.launch.xml"]
 assert fixture["config_files"] == ["config/settings.yaml"]
 assert packages["malformed_fixture"]["metadata_error"]
+assert packages["rclpy"]["source"] == "underlay"
+assert packages["rclpy"]["path"] == ""
 print("Selected-workspace ROS package discovery passed")
 PY
[trailing space]
@@ -203,6 +212,7 @@ docker exec -i "$container" bash -lc \
   'source /opt/ros/jazzy/setup.bash && cd /opt/robotics-studio && /opt/robotics-studio/venv/bin/python -' <<'PY'
 import sys
 import time
+from threading import Thread
[trailing space]
 sys.path.insert(0, "/opt/robotics-studio/generated")
 import grpc
@@ -211,10 +221,13 @@ import ros_engine_pb2 as pb
 import ros_engine_pb2_grpc as rpc
 from action_tutorials_interfaces.action import Fibonacci
 from geometry_msgs.msg import TransformStamped
-from rclpy.action import ActionClient, ActionServer
+from rclpy.action import ActionClient, ActionServer, CancelResponse
+from rclpy.callback_groups import ReentrantCallbackGroup
+from rclpy.executors import MultiThreadedExecutor
 from rclpy.qos import DurabilityPolicy, QoSProfile, ReliabilityPolicy
 from sensor_msgs.msg import JointState
 from std_msgs.msg import String
+from lifecycle_msgs.srv import GetState
 from std_srvs.srv import Trigger
 from tf2_msgs.msg import TFMessage
[trailing space]
@@ -232,7 +245,7 @@ stub = rpc.RosEngineStub(
 events = stub.ReplayEvents(
     pb.ReplayEventsRequest(has_after_sequence=True, after_sequence=0),
     metadata=metadata,
-    timeout=10,
+    timeout=30,
 )
[trailing space]
 rclpy.init()
@@ -242,12 +255,48 @@ service = node.create_service(
     "/integration/echo",
     lambda request, response: response,
 )
+
+def get_integration_lifecycle_state(request, response):
+    response.current_state.id = 2
+    response.current_state.label = "active"
+    return response
+
+lifecycle_service = node.create_service(
+    GetState,
+    "/integration/lifecycle/get_state",
+    get_integration_lifecycle_state,
+)
 service_client = node.create_client(Trigger, "/integration/echo")
+
+def execute_fibonacci(goal_handle):
+    sequence = [0, 1]
+    feedback = Fibonacci.Feedback()
+    result = Fibonacci.Result()
+    feedback.partial_sequence = sequence
+    goal_handle.publish_feedback(feedback)
+    for index in range(1, goal_handle.request.order):
+        if goal_handle.is_cancel_requested:
+            goal_handle.canceled()
+            result.sequence = sequence
+            return result
+        sequence.append(sequence[index] + sequence[index - 1])
+        feedback.partial_sequence = sequence
+        goal_handle.publish_feedback(feedback)
+        time.sleep(0.1)
+    if goal_handle.is_cancel_requested:
+        goal_handle.canceled()
+    else:
+        goal_handle.succeed()
+    result.sequence = sequence
+    return result
+
 action_server = ActionServer(
     node,
     Fibonacci,
     "/integration/fibonacci",
-    lambda goal_handle: Fibonacci.Result(),
+    execute_fibonacci,
+    callback_group=ReentrantCallbackGroup(),
+    cancel_callback=lambda _goal_handle: CancelResponse.ACCEPT,
 )
 action_client = ActionClient(node, Fibonacci, "/integration/fibonacci")
 publisher = node.create_publisher(
@@ -307,6 +356,99 @@ except grpc.RpcError as error:
 else:
     raise AssertionError("ROS service accepted an invalid inspection topic name")
[trailing space]
+executor = MultiThreadedExecutor(num_threads=4)
+executor.add_node(node)
+executor_thread = Thread(target=executor.spin, daemon=True)
+executor_thread.start()
+
+safe_service_request = pb.SafeServiceRequest(
+    service_name="/integration/lifecycle/get_state",
+    service_type="lifecycle_msgs/srv/GetState",
+    request_json="{}",
+    timeout_ms=1000,
+)
+service_deadline = time.monotonic() + 8
+while True:
+    try:
+        safe_service_response = stub.InvokeSafeService(
+            safe_service_request,
+            metadata=metadata,
+            timeout=3,
+        )
+        break
+    except grpc.RpcError as error:
+        if (
+            error.code() != grpc.StatusCode.NOT_FOUND
+            or time.monotonic() >= service_deadline
+        ):
+            raise
+        time.sleep(0.1)
+import json
+
+assert safe_service_response.service_type == "lifecycle_msgs/srv/GetState"
+assert json.loads(safe_service_response.response_json) == {
+    "current_state": {"id": 2, "label": "active"}
+}
+try:
+    stub.InvokeSafeService(
+        pb.SafeServiceRequest(
+            service_name="/integration/echo",
+            service_type="std_srvs/srv/Trigger",
+            request_json="{}",
+            timeout_ms=1000,
+        ),
+        metadata=metadata,
+        timeout=3,
+    )
+except grpc.RpcError as error:
+    assert error.code() == grpc.StatusCode.PERMISSION_DENIED
+else:
+    raise AssertionError("ROS service invoked a type outside the read-only allowlist")
+try:
+    stub.InvokeSafeService(
+        pb.SafeServiceRequest(
+            service_name="/integration/lifecycle/get_state",
+            service_type="lifecycle_msgs/srv/GetState",
+            request_json='{"unknown":true}',
+            timeout_ms=1000,
+        ),
+        metadata=metadata,
+        timeout=3,
+    )
+except grpc.RpcError as error:
+    assert error.code() == grpc.StatusCode.INVALID_ARGUMENT
+else:
+    raise AssertionError("ROS service accepted a request outside its schema")
+
+try:
+    stub.SendSafeActionGoal(
+        pb.SafeActionGoalRequest(
+            action_name="/integration/fibonacci",
+            action_type="example_interfaces/action/Fibonacci",
+            goal_json='{"order":3}',
+        ),
+        metadata=metadata,
+        timeout=3,
+    )
+except grpc.RpcError as error:
+    assert error.code() == grpc.StatusCode.PERMISSION_DENIED
+else:
+    raise AssertionError("ROS action accepted a type outside the non-motion allowlist")
+try:
+    stub.SendSafeActionGoal(
+        pb.SafeActionGoalRequest(
+            action_name="/integration/fibonacci",
+            action_type="action_tutorials_interfaces/action/Fibonacci",
+            goal_json='{"order":0}',
+        ),
+        metadata=metadata,
+        timeout=3,
+    )
+except grpc.RpcError as error:
+    assert error.code() == grpc.StatusCode.INVALID_ARGUMENT
+else:
+    raise AssertionError("ROS action accepted a goal outside its safe bounds")
+
 transform = TransformStamped()
 transform.header.frame_id = "map"
 transform.child_frame_id = "integration_base"
@@ -388,11 +530,19 @@ for event in events:
             (service for service in graph.services if service.name == "/integration/echo"),
             None,
         )
+        lifecycle_service_info = next(
+            (
+                service
+                for service in graph.services
+                if service.name == "/integration/lifecycle/get_state"
+            ),
+            None,
+        )
         action_info = next(
             (action for action in graph.actions if action.name == "/integration/fibonacci"),
             None,
         )
-        if tf_topic and service_info and action_info:
+        if tf_topic and service_info and lifecycle_service_info and action_info:
             graph_verified = (
                 any(
                 endpoint.node_name == "robotics_studio_tf_integration"
@@ -402,6 +552,8 @@ for event in events:
                 )
                 and "/robotics_studio_tf_integration" in service_info.server_nodes
                 and "/robotics_studio_tf_integration" in service_info.client_nodes
+                and lifecycle_service_info.safe_types
+                == ["lifecycle_msgs/srv/GetState"]
                 and action_info.types
                 == ["action_tutorials_interfaces/action/Fibonacci"]
                 and "/robotics_studio_tf_integration" in action_info.server_nodes
@@ -429,10 +581,94 @@ if not dynamic_tf_verified:
 if not topic_inspection_verified or not topic_inspection_stopped:
     raise AssertionError("ROS topic inspection did not stream and stop cleanly")
[trailing space]
+action_response = stub.SendSafeActionGoal(
+    pb.SafeActionGoalRequest(
+        action_name="/integration/fibonacci",
+        action_type="action_tutorials_interfaces/action/Fibonacci",
+        goal_json='{"order":3}',
+    ),
+    metadata=metadata,
+    timeout=3,
+)
+action_updates = []
+for event in events:
+    if event.WhichOneof("event") != "action_goal_update":
+        continue
+    update = event.action_goal_update
+    if update.goal_id != action_response.goal_id:
+        continue
+    action_updates.append(update)
+    if update.terminal:
+        break
+else:
+    raise AssertionError("ROS action goal did not reach a terminal state")
+assert any(update.status == "feedback" for update in action_updates), (
+    "ROS action goal did not stream feedback"
+)
+terminal_update = action_updates[-1]
+assert terminal_update.status == "succeeded", terminal_update
+assert json.loads(terminal_update.result_json)["sequence"] == [0, 1, 1, 2]
+
+missing_action = stub.SendSafeActionGoal(
+    pb.SafeActionGoalRequest(
+        action_name="/integration/missing_fibonacci",
+        action_type="action_tutorials_interfaces/action/Fibonacci",
+        goal_json='{"order":3}',
+    ),
+    metadata=metadata,
+    timeout=3,
+)
+for event in events:
+    if event.WhichOneof("event") == "action_goal_update":
+        update = event.action_goal_update
+        if update.goal_id == missing_action.goal_id and update.terminal:
+            assert update.status == "unavailable"
+            assert "not available" in update.error
+            break
+else:
+    raise AssertionError("Unavailable ROS action server was not reported")
+
+cancellable_goal = stub.SendSafeActionGoal(
+    pb.SafeActionGoalRequest(
+        action_name="/integration/fibonacci",
+        action_type="action_tutorials_interfaces/action/Fibonacci",
+        goal_json='{"order":20}',
+    ),
+    metadata=metadata,
+    timeout=3,
+)
+cancel_requested = False
+for event in events:
+    if event.WhichOneof("event") != "action_goal_update":
+        continue
+    update = event.action_goal_update
+    if update.goal_id != cancellable_goal.goal_id:
+        continue
+    if update.status == "feedback" and not cancel_requested:
+        cancellation = stub.CancelActionGoal(
+            pb.ActionGoalIdRequest(goal_id=cancellable_goal.goal_id),
+            metadata=metadata,
+            timeout=3,
+        )
+        assert cancellation.requested, "ROS action server rejected cancellation"
+        cancel_requested = True
+    if update.terminal:
+        assert cancel_requested, "ROS action goal completed before cancellation"
+        assert update.status == "canceled", update
+        break
+else:
+    raise AssertionError("Canceled ROS action did not report a terminal result")
+
 events.cancel()
+executor.shutdown()
+executor_thread.join(timeout=3)
+assert not executor_thread.is_alive(), "ROS integration fixture executor did not stop"
+executor.remove_node(node)
 node.destroy_node()
 rclpy.shutdown()
 print("Live ROS TF, joint-state, graph, and bounded topic inspection passed")
+print("Allowlisted lifecycle state service invocation passed")
+print("Safe ROS action feedback, result, cancellation, and terminal status passed")
 PY
[trailing space]
 ROBOTICS_STUDIO_TEST_TLS_CERTIFICATE="$workspace/grpc-ca.crt" \
diff --git a/src-tauri/Cargo.lock b/src-tauri/Cargo.lock
index 6ceb3ad..86d5d6d 100644
--- a/src-tauri/Cargo.lock
+++ b/src-tauri/Cargo.lock
@@ -3469,6 +3469,7 @@ dependencies = [
 name = "robotics-studio"
 version = "0.3.0"
 dependencies = [
+ "base64 0.22.1",
  "getrandom 0.3.4",
  "keyring",
  "prost",
diff --git a/src-tauri/Cargo.toml b/src-tauri/Cargo.toml
index 1e9b7db..0b916c3 100644
--- a/src-tauri/Cargo.toml
+++ b/src-tauri/Cargo.toml
@@ -24,6 +24,7 @@ tauri = { version = "2", features = [] }
 tauri-plugin-opener = "2"
 serde = { version = "1", features = ["derive"] }
 serde_json = "1"
+base64 = "0.22"
 tauri-plugin-dialog = "2.7.2"
 tonic = { version = "0.14.6", features = ["tls-ring"] }
 prost = "0.14.4"
diff --git a/src-tauri/proto/ros_engine.proto b/src-tauri/proto/ros_engine.proto
index 9fe06e9..a89bf65 100644
--- a/src-tauri/proto/ros_engine.proto
+++ b/src-tauri/proto/ros_engine.proto
@@ -14,6 +14,41 @@ service RosEngine {
   rpc GetEventHealth(Empty) returns (EventHealth);
   rpc StartTopicInspection(TopicInspectionRequest) returns (Empty);
   rpc StopTopicInspection(TopicInspectionRequest) returns (Empty);
+  rpc InvokeSafeService(SafeServiceRequest) returns (SafeServiceResponse);
+  rpc SendSafeActionGoal(SafeActionGoalRequest) returns (ActionGoalResponse);
+  rpc CancelActionGoal(ActionGoalIdRequest) returns (ActionCancelResponse);
+}
+
+message SafeServiceRequest {
+  string service_name = 1;
+  string service_type = 2;
+  string request_json = 3;
+  uint32 timeout_ms = 4;
+}
+
+message SafeServiceResponse {
+  string service_name = 1;
+  string service_type = 2;
+  string response_json = 3;
+  bool truncated = 4;
+}
+
+message SafeActionGoalRequest {
+  string action_name = 1;
+  string action_type = 2;
+  string goal_json = 3;
+}
+
+message ActionGoalResponse {
+  string goal_id = 1;
+}
+
+message ActionGoalIdRequest {
+  string goal_id = 1;
+}
+
+message ActionCancelResponse {
+  bool requested = 1;
 }
[trailing space]
 message TopicInspectionRequest {
@@ -103,6 +138,7 @@ message WorkspaceEvent {
     RosGraphSnapshot ros_graph_snapshot = 12;
     JointStateUpdate joint_state_update = 13;
     TopicInspectionUpdate topic_inspection_update = 14;
+    ActionGoalUpdate action_goal_update = 15;
   }
   int64 timestamp_ns = 10;
   uint64 sequence = 11;
@@ -159,6 +195,7 @@ message GraphService {
   repeated string types = 2;
   repeated string server_nodes = 3;
   repeated string client_nodes = 4;
+  repeated string safe_types = 5;
 }
[trailing space]
 message GraphAction {
@@ -166,6 +203,19 @@ message GraphAction {
   repeated string types = 2;
   repeated string server_nodes = 3;
   repeated string client_nodes = 4;
+  repeated string safe_types = 5;
+}
+
+message ActionGoalUpdate {
+  string goal_id = 1;
+  string action_name = 2;
+  string action_type = 3;
+  string status = 4;
+  string feedback_json = 5;
+  string result_json = 6;
+  string error = 7;
+  bool terminal = 8;
+  bool truncated = 9;
 }
[trailing space]
 message EventRecovery {
diff --git a/src-tauri/resources/devcontainer/proto/ros_engine.proto b/src-tauri/resources/devcontainer/proto/ros_engine.proto
index 9fe06e9..a89bf65 100644
--- a/src-tauri/resources/devcontainer/proto/ros_engine.proto
+++ b/src-tauri/resources/devcontainer/proto/ros_engine.proto
@@ -14,6 +14,41 @@ service RosEngine {
   rpc GetEventHealth(Empty) returns (EventHealth);
   rpc StartTopicInspection(TopicInspectionRequest) returns (Empty);
   rpc StopTopicInspection(TopicInspectionRequest) returns (Empty);
+  rpc InvokeSafeService(SafeServiceRequest) returns (SafeServiceResponse);
+  rpc SendSafeActionGoal(SafeActionGoalRequest) returns (ActionGoalResponse);
+  rpc CancelActionGoal(ActionGoalIdRequest) returns (ActionCancelResponse);
+}
+
+message SafeServiceRequest {
+  string service_name = 1;
+  string service_type = 2;
+  string request_json = 3;
+  uint32 timeout_ms = 4;
+}
+
+message SafeServiceResponse {
+  string service_name = 1;
+  string service_type = 2;
+  string response_json = 3;
+  bool truncated = 4;
+}
+
+message SafeActionGoalRequest {
+  string action_name = 1;
+  string action_type = 2;
+  string goal_json = 3;
+}
+
+message ActionGoalResponse {
+  string goal_id = 1;
+}
+
+message ActionGoalIdRequest {
+  string goal_id = 1;
+}
+
+message ActionCancelResponse {
+  bool requested = 1;
 }
[trailing space]
 message TopicInspectionRequest {
@@ -103,6 +138,7 @@ message WorkspaceEvent {
     RosGraphSnapshot ros_graph_snapshot = 12;
     JointStateUpdate joint_state_update = 13;
     TopicInspectionUpdate topic_inspection_update = 14;
+    ActionGoalUpdate action_goal_update = 15;
   }
   int64 timestamp_ns = 10;
   uint64 sequence = 11;
@@ -159,6 +195,7 @@ message GraphService {
   repeated string types = 2;
   repeated string server_nodes = 3;
   repeated string client_nodes = 4;
+  repeated string safe_types = 5;
 }
[trailing space]
 message GraphAction {
@@ -166,6 +203,19 @@ message GraphAction {
   repeated string types = 2;
   repeated string server_nodes = 3;
   repeated string client_nodes = 4;
+  repeated string safe_types = 5;
+}
+
+message ActionGoalUpdate {
+  string goal_id = 1;
+  string action_name = 2;
+  string action_type = 3;
+  string status = 4;
+  string feedback_json = 5;
+  string result_json = 6;
+  string error = 7;
+  bool terminal = 8;
+  bool truncated = 9;
 }
[trailing space]
 message EventRecovery {
diff --git a/src-tauri/resources/devcontainer/server.py b/src-tauri/resources/devcontainer/server.py
index 7d9b52c..526fcd6 100644
--- a/src-tauri/resources/devcontainer/server.py
+++ b/src-tauri/resources/devcontainer/server.py
@@ -6,6 +6,7 @@ import os
 import math
 import re
 import json
+import uuid
 from collections.abc import Sequence
[trailing space]
 import grpc
@@ -14,6 +15,7 @@ from concurrent import futures
 import rclpy
 from rclpy.node import Node
 from rclpy.qos import DurabilityPolicy, QoSProfile, ReliabilityPolicy
+from rclpy.action import ActionClient
[trailing space]
 AUTH_PROTOCOL_VERSION = 1
[trailing space]
@@ -32,7 +34,8 @@ from tf2_msgs.msg import TFMessage
 from nav_msgs.msg import Odometry
 from geometry_msgs.msg import Twist
 from sensor_msgs.msg import JointState
-from rosidl_runtime_py.utilities import get_message
+from rosidl_runtime_py.set_message import set_message_fields
+from rosidl_runtime_py.utilities import get_action, get_message, get_service
 from server_core import (
     CommandLeaseManager,
     EventPersistenceError,
@@ -60,6 +63,26 @@ MAX_INSPECTION_MESSAGE_BYTES = 16 * 1024
 MAX_INSPECTION_DEPTH = 5
 MAX_INSPECTION_ITEMS = 256
 MAX_INSPECTION_STRING_LENGTH = 512
+MAX_SAFE_SERVICE_REQUEST_BYTES = 16 * 1024
+MAX_SAFE_SERVICE_CALLS = 8
+SAFE_READ_ONLY_SERVICE_TYPES = frozenset(
+    {
+        "lifecycle_msgs/srv/GetAvailableStates",
+        "lifecycle_msgs/srv/GetAvailableTransitions",
+        "lifecycle_msgs/srv/GetState",
+        "lifecycle_msgs/srv/GetTransitionGraph",
+        "rcl_interfaces/srv/DescribeParameters",
+        "rcl_interfaces/srv/GetParameterTypes",
+        "rcl_interfaces/srv/GetParameters",
+        "rcl_interfaces/srv/ListParameters",
+    }
+)
+SAFE_NON_MOTION_ACTION_TYPES = frozenset(
+    {"action_tutorials_interfaces/action/Fibonacci"}
+)
+MAX_SAFE_ACTION_GOALS = 4
+MAX_SAFE_ACTION_DURATION_SECONDS = 60.0
+MAX_SAFE_ACTION_FEEDBACK_RATE_HZ = 5.0
[trailing space]
[trailing space]
 class _MessageSerializationBudget:
@@ -168,6 +191,16 @@ class RosIntrospector(Node):
         self.topic_inspection_timer = self.create_timer(
             0.1, self._process_topic_inspection_requests
         )
+        self.safe_service_requests = queue.Queue(maxsize=16)
+        self.pending_safe_service_calls = {}
+        self.safe_service_timer = self.create_timer(
+            0.1, self._process_safe_service_requests
+        )
+        self.safe_action_requests = queue.Queue(maxsize=16)
+        self.safe_action_goals = {}
+        self.safe_action_timer = self.create_timer(
+            0.1, self._process_safe_action_requests
+        )
[trailing space]
     def _publish_stop_to_all(self):
         with self._command_lock:
@@ -285,6 +318,561 @@ class RosIntrospector(Node):
             future.cancel()
             raise TimeoutError("Timed out waiting for the ROS executor") from error
[trailing space]
+    @staticmethod
+    def _json_object_without_duplicate_keys(pairs):
+        result = {}
+        for key, value in pairs:
+            if key in result:
+                raise ValueError(f"Duplicate JSON field: {key}")
+            result[key] = value
+        return result
+
+    @staticmethod
+    def _reject_json_constant(value):
+        raise ValueError(f"Invalid JSON number: {value}")
+
+    def invoke_safe_service(self, service_name, service_type, request_json, timeout_ms):
+        if not re.fullmatch(
+            r"/[A-Za-z_][A-Za-z0-9_]*(/[A-Za-z_][A-Za-z0-9_]*)*",
+            service_name,
+        ):
+            raise ValueError("Service name must be a fully qualified ROS name")
+        if service_type not in SAFE_READ_ONLY_SERVICE_TYPES:
+            raise PermissionError("This ROS service type is not on the read-only allowlist")
+        if not 100 <= timeout_ms <= 5000:
+            raise ValueError("Service timeout must be between 100 and 5000 milliseconds")
+        try:
+            request_size = len(request_json.encode("utf-8"))
+        except UnicodeEncodeError as error:
+            raise ValueError("Service request is not valid UTF-8") from error
+        if request_size > MAX_SAFE_SERVICE_REQUEST_BYTES:
+            raise ValueError("Service request exceeds the 16 KiB limit")
+        try:
+            request_values = json.loads(
+                request_json,
+                object_pairs_hook=self._json_object_without_duplicate_keys,
+                parse_constant=self._reject_json_constant,
+            )
+        except (json.JSONDecodeError, RecursionError, ValueError) as error:
+            raise ValueError(f"Invalid service request JSON: {error}") from error
+        if not isinstance(request_values, dict):
+            raise ValueError("Service request JSON must be an object")
+
+        response_future = futures.Future()
+        try:
+            self.safe_service_requests.put_nowait(
+                (
+                    service_name,
+                    service_type,
+                    request_values,
+                    timeout_ms,
+                    response_future,
+                )
+            )
+        except queue.Full as error:
+            raise RuntimeError("Safe service request queue is full") from error
+        try:
+            return response_future.result(timeout=timeout_ms / 1000.0 + 1.0)
+        except futures.TimeoutError as error:
+            response_future.cancel()
+            raise TimeoutError("Safe ROS service request timed out") from error
+
+    def _process_safe_service_requests(self):
+        while True:
+            try:
+                (
+                    service_name,
+                    service_type,
+                    request_values,
+                    timeout_ms,
+                    response_future,
+                ) = self.safe_service_requests.get_nowait()
+            except queue.Empty:
+                self._expire_safe_service_calls()
+                return
+            if not response_future.set_running_or_notify_cancel():
+                continue
+            try:
+                self._begin_safe_service_call(
+                    service_name,
+                    service_type,
+                    request_values,
+                    timeout_ms,
+                    response_future,
+                )
+            except Exception as error:
+                response_future.set_exception(error)
+
+    def _begin_safe_service_call(
+        self, service_name, service_type, request_values, timeout_ms, response_future
+    ):
+        if len(self.pending_safe_service_calls) >= MAX_SAFE_SERVICE_CALLS:
+            raise RuntimeError("Too many safe ROS service calls are in flight")
+        discovered_types = dict(self.get_service_names_and_types()).get(
+            service_name, []
+        )
+        if service_type not in discovered_types:
+            raise LookupError(
+                f"ROS service {service_name} with type {service_type} is unavailable"
+            )
+        service_class = get_service(service_type)
+        request = service_class.Request()
+        allowed_fields = set(request.get_fields_and_field_types())
+        unknown_fields = sorted(set(request_values) - allowed_fields)
+        if unknown_fields:
+            raise ValueError(
+                "Unknown service request fields: " + ", ".join(unknown_fields)
+            )
+        try:
+            set_message_fields(request, request_values)
+        except (AssertionError, AttributeError, TypeError, ValueError) as error:
+            raise ValueError(f"Service request does not match its ROS schema: {error}") from error
+
+        client = self.create_client(service_class, service_name)
+        if not client.wait_for_service(timeout_sec=0.0):
+            self.destroy_client(client)
+            raise LookupError(f"ROS service {service_name} is not ready")
+        call_id = id(response_future)
+        try:
+            ros_future = client.call_async(request)
+        except Exception:
+            self.destroy_client(client)
+            raise
+        self.pending_safe_service_calls[call_id] = {
+            "client": client,
+            "ros_future": ros_future,
+            "response_future": response_future,
+            "deadline": time.monotonic() + timeout_ms / 1000.0,
+            "service_name": service_name,
+            "service_type": service_type,
+        }
+        ros_future.add_done_callback(
+            lambda completed, key=call_id: self._finish_safe_service_call(key, completed)
+        )
+
+    def _finish_safe_service_call(self, call_id, ros_future):
+        call = self.pending_safe_service_calls.pop(call_id, None)
+        if call is None:
+            return
+        self.destroy_client(call["client"])
+        if ros_future.cancelled():
+            if not call["response_future"].done():
+                call["response_future"].set_exception(
+                    TimeoutError("Safe ROS service request was cancelled")
+                )
+            return
+        error = ros_future.exception()
+        if error is not None:
+            call["response_future"].set_exception(
+                RuntimeError(f"ROS service call failed: {error}")
+            )
+            return
+        try:
+            payload, truncated = _serialize_inspection_message(ros_future.result())
+        except (TypeError, ValueError, OverflowError) as error:
+            call["response_future"].set_exception(
+                RuntimeError(f"Could not serialize ROS service response: {error}")
+            )
+            return
+        call["response_future"].set_result(
+            {"response_json": payload, "truncated": truncated}
+        )
+
+    def _expire_safe_service_calls(self):
+        now = time.monotonic()
+        expired = [
+            call_id
+            for call_id, call in self.pending_safe_service_calls.items()
+            if call["deadline"] <= now
+        ]
+        for call_id in expired:
+            call = self.pending_safe_service_calls.pop(call_id)
+            call["ros_future"].cancel()
+            self.destroy_client(call["client"])
+            if not call["response_future"].done():
+                call["response_future"].set_exception(
+                    TimeoutError("ROS service response timed out")
+                )
+
+    def submit_safe_action_goal(self, action_name, action_type, goal_json):
+        if not re.fullmatch(
+            r"/[A-Za-z_][A-Za-z0-9_]*(/[A-Za-z_][A-Za-z0-9_]*)*",
+            action_name,
+        ):
+            raise ValueError("Action name must be a fully qualified ROS name")
+        if action_type not in SAFE_NON_MOTION_ACTION_TYPES:
+            raise PermissionError("This ROS action type is not on the non-motion allowlist")
+        if len(goal_json.encode("utf-8")) > MAX_SAFE_SERVICE_REQUEST_BYTES:
+            raise ValueError("Action goal exceeds the 16 KiB limit")
+        try:
+            goal_values = json.loads(
+                goal_json,
+                object_pairs_hook=self._json_object_without_duplicate_keys,
+                parse_constant=self._reject_json_constant,
+            )
+        except (json.JSONDecodeError, RecursionError, ValueError) as error:
+            raise ValueError(f"Invalid action goal JSON: {error}") from error
+        if not isinstance(goal_values, dict):
+            raise ValueError("Action goal JSON must be an object")
+
+        action_class = get_action(action_type)
+        goal = action_class.Goal()
+        unknown_fields = sorted(
+            set(goal_values) - set(goal.get_fields_and_field_types())
+        )
+        if unknown_fields:
+            raise ValueError("Unknown action goal fields: " + ", ".join(unknown_fields))
+        try:
+            set_message_fields(goal, goal_values)
+        except (AssertionError, AttributeError, TypeError, ValueError) as error:
+            raise ValueError(f"Action goal does not match its ROS schema: {error}") from error
+        if action_type == "action_tutorials_interfaces/action/Fibonacci":
+            if not isinstance(goal.order, int) or isinstance(goal.order, bool):
+                raise ValueError("Fibonacci order must be an integer")
+            if not 1 <= goal.order <= 20:
+                raise ValueError("Fibonacci order must be between 1 and 20")
+
+        goal_id = str(uuid.uuid4())
+        try:
+            self.safe_action_requests.put_nowait(
+                ("start", goal_id, action_name, action_type, goal, None)
+            )
+        except queue.Full as error:
+            raise RuntimeError("Safe ROS action request queue is full") from error
+        self._publish_action_goal(
+            goal_id, action_name, action_type, status="submitted"
+        )
+        return goal_id
+
+    def cancel_safe_action_goal(self, goal_id):
+        try:
+            normalized_id = str(uuid.UUID(goal_id))
+        except (AttributeError, TypeError, ValueError) as error:
+            raise ValueError("Invalid action goal identifier") from error
+        response_future = futures.Future()
+        try:
+            self.safe_action_requests.put_nowait(
+                ("cancel", normalized_id, "", "", None, response_future)
+            )
+        except queue.Full as error:
+            raise RuntimeError("Safe ROS action request queue is full") from error
+        try:
+            return response_future.result(timeout=5.0)
+        except futures.TimeoutError as error:
+            response_future.cancel()
+            raise TimeoutError("Timed out waiting for action cancellation acknowledgement") from error
+
+    def _publish_action_goal(
+        self,
+        goal_id,
+        action_name,
+        action_type,
+        status,
+        feedback_json="",
+        result_json="",
+        error="",
+        terminal=False,
+        truncated=False,
+    ):
+        event = ros_engine_pb2.WorkspaceEvent(
+            action_goal_update=ros_engine_pb2.ActionGoalUpdate(
+                goal_id=goal_id,
+                action_name=action_name,
+                action_type=action_type,
+                status=status,
+                feedback_json=feedback_json,
+                result_json=result_json,
+                error=error,
+                terminal=terminal,
+                truncated=truncated,
+            ),
+            timestamp_ns=time.time_ns(),
+        )
+        if status == "feedback":
+            self.event_hub.publish(
+                event,
+                priority="telemetry",
+                key=("action_feedback", goal_id),
+            )
+        else:
+            self.event_hub.publish(event, key=("action_goal", goal_id))
+
+    def _process_safe_action_requests(self):
+        while True:
+            try:
+                action, goal_id, name, action_type, goal, response_future = (
+                    self.safe_action_requests.get_nowait()
+                )
+            except queue.Empty:
+                self._expire_safe_action_goals()
+                return
+            try:
+                if action == "start":
+                    self._begin_safe_action_goal(
+                        goal_id, name, action_type, goal
+                    )
+                else:
+                    self._begin_safe_action_cancel(goal_id, response_future)
+            except Exception as error:
+                if response_future is not None and not response_future.done():
+                    response_future.set_exception(error)
+                else:
+                    self._publish_action_goal(
+                        goal_id,
+                        name,
+                        action_type,
+                        status="failed",
+                        error=str(error),
+                        terminal=True,
+                    )
+
+    def _begin_safe_action_goal(self, goal_id, action_name, action_type, goal):
+        if len(self.safe_action_goals) >= MAX_SAFE_ACTION_GOALS:
+            self._publish_action_goal(
+                goal_id,
+                action_name,
+                action_type,
+                status="failed",
+                error=f"At most {MAX_SAFE_ACTION_GOALS} safe goals can run at once",
+                terminal=True,
+            )
+            return
+        action_class = get_action(action_type)
+        client = ActionClient(self, action_class, action_name)
+        if not client.wait_for_server(timeout_sec=0.0):
+            self.destroy_client(client)
+            self._publish_action_goal(
+                goal_id,
+                action_name,
+                action_type,
+                status="unavailable",
+                error=f"ROS action server {action_name} is not available",
+                terminal=True,
+            )
+            return
+        record = {
+            "action_name": action_name,
+            "action_type": action_type,
+            "client": client,
+            "goal_handle": None,
+            "deadline": time.monotonic() + MAX_SAFE_ACTION_DURATION_SECONDS,
+            "timed_out": False,
+            "cancel_requested": False,
+            "last_feedback_at": 0.0,
+        }
+        self.safe_action_goals[goal_id] = record
+        try:
+            send_future = client.send_goal_async(
+                goal,
+                feedback_callback=lambda feedback, key=goal_id: (
+                    self._safe_action_feedback(key, feedback)
+                ),
+            )
+        except Exception:
+            self.safe_action_goals.pop(goal_id, None)
+            self.destroy_client(client)
+            raise
+        send_future.add_done_callback(
+            lambda completed, key=goal_id: self._safe_action_goal_response(
+                key, completed
+            )
+        )
+
+    def _safe_action_goal_response(self, goal_id, completed):
+        record = self.safe_action_goals.get(goal_id)
+        if record is None:
+            return
+        try:
+            goal_handle = completed.result()
+        except Exception as error:
+            self._finish_safe_action_goal(
+                goal_id, "failed", error=str(error)
+            )
+            return
+        if not goal_handle.accepted:
+            self._finish_safe_action_goal(
+                goal_id,
+                "rejected",
+                error="ROS action server rejected the goal",
+            )
+            return
+        record["goal_handle"] = goal_handle
+        self._publish_action_goal(
+            goal_id,
+            record["action_name"],
+            record["action_type"],
+            status="accepted",
+        )
+        result_future = goal_handle.get_result_async()
+        result_future.add_done_callback(
+            lambda result, key=goal_id: self._safe_action_result(key, result)
+        )
+        if record["timed_out"]:
+            self._request_action_cancel(record, goal_id)
+
+    def _safe_action_feedback(self, goal_id, feedback_message):
+        record = self.safe_action_goals.get(goal_id)
+        if record is None:
+            return
+        now = time.monotonic()
+        if now - record["last_feedback_at"] < 1.0 / MAX_SAFE_ACTION_FEEDBACK_RATE_HZ:
+            return
+        record["last_feedback_at"] = now
+        try:
+            payload, truncated = _serialize_inspection_message(
+                feedback_message.feedback
+            )
+        except (TypeError, ValueError, OverflowError):
+            LOGGER.exception("Could not serialize feedback for action goal %s", goal_id)
+            self._publish_action_goal(
+                goal_id,
+                record["action_name"],
+                record["action_type"],
+                status="feedback_error",
+                error="Action feedback could not be serialized",
+            )
+            return
+        self._publish_action_goal(
+            goal_id,
+            record["action_name"],
+            record["action_type"],
+            status="feedback",
+            feedback_json=payload,
+            truncated=truncated,
+        )
+
+    def _safe_action_result(self, goal_id, completed):
+        record = self.safe_action_goals.get(goal_id)
+        if record is None:
+            return
+        try:
+            response = completed.result()
+            result_json, truncated = _serialize_inspection_message(response.result)
+        except Exception as error:
+            self._finish_safe_action_goal(
+                goal_id, "failed", error=f"Could not retrieve action result: {error}"
+            )
+            return
+        status = {
+            4: "succeeded",
+            5: "canceled",
+            6: "aborted",
+        }.get(response.status, f"terminal_{response.status}")
+        if record["timed_out"]:
+            status = "timed_out"
+        self._finish_safe_action_goal(
+            goal_id,
+            status,
+            result_json=result_json,
+            truncated=truncated,
+            error=(
+                "Action exceeded its 60-second execution limit"
+                if record["timed_out"]
+                else ""
+            ),
+        )
+
+    def _finish_safe_action_goal(
+        self,
+        goal_id,
+        status,
+        result_json="",
+        error="",
+        truncated=False,
+    ):
+        record = self.safe_action_goals.pop(goal_id, None)
+        if record is None:
+            return
+        self.destroy_client(record["client"])
+        self._publish_action_goal(
+            goal_id,
+            record["action_name"],
+            record["action_type"],
+            status=status,
+            result_json=result_json,
+            error=error,
+            terminal=True,
+            truncated=truncated,
+        )
+
+    def _begin_safe_action_cancel(self, goal_id, response_future):
+        if not response_future.set_running_or_notify_cancel():
+            return
+        record = self.safe_action_goals.get(goal_id)
+        if record is None:
+            response_future.set_exception(LookupError("Action goal is not active"))
+            return
+        if record["goal_handle"] is None:
+            response_future.set_exception(
+                RuntimeError("Action goal has not yet been accepted by its server")
+            )
+            return
+        record["cancel_requested"] = True
+        self._request_action_cancel(
+            record, goal_id, response_future=response_future
+        )
+
+    def _request_action_cancel(self, record, goal_id, response_future=None):
+        if record["goal_handle"] is None:
+            return
+        record["cancel_requested"] = True
+        self._publish_action_goal(
+            goal_id,
+            record["action_name"],
+            record["action_type"],
+            status="cancel_requested",
+            error=(
+                "Execution time limit reached; awaiting confirmed cancellation"
+                if record["timed_out"]
+                else ""
+            ),
+        )
+        try:
+            cancel_future = record["goal_handle"].cancel_goal_async()
+        except Exception as error:
+            if response_future is not None and not response_future.done():
+                response_future.set_exception(error)
+            else:
+                LOGGER.exception("Could not request cancellation for action goal %s", goal_id)
+            record["cancel_requested"] = False
+            return
+
+        def cancellation_done(completed):
+            try:
+                response = completed.result()
+                accepted = bool(response.goals_canceling)
+            except Exception as error:
+                if response_future is not None and not response_future.done():
+                    response_future.set_exception(error)
+                self._publish_action_goal(
+                    goal_id,
+                    record["action_name"],
+                    record["action_type"],
+                    status="cancel_failed",
+                    error=f"Action cancellation failed: {error}",
+                )
+                return
+            if response_future is not None and not response_future.done():
+                response_future.set_result(accepted)
+            if not accepted:
+                record["cancel_requested"] = False
+                self._publish_action_goal(
+                    goal_id,
+                    record["action_name"],
+                    record["action_type"],
+                    status="cancel_rejected",
+                    error="ROS action server did not accept cancellation",
+                )
+
+        cancel_future.add_done_callback(cancellation_done)
+
+    def _expire_safe_action_goals(self):
+        now = time.monotonic()
+        for goal_id, record in tuple(self.safe_action_goals.items()):
+            if record["deadline"] <= now and not record["timed_out"]:
+                record["timed_out"] = True
+                self._request_action_cancel(record, goal_id)
+
     def _process_topic_inspection_requests(self):
         while True:
             try:
@@ -670,6 +1258,9 @@ class RosIntrospector(Node):
                     types=sorted(types),
                     server_nodes=sorted(set(role["servers"])),
                     client_nodes=sorted(set(role["clients"])),
+                    safe_types=sorted(
+                        set(types) & SAFE_READ_ONLY_SERVICE_TYPES
+                    ),
                 )
             )
[trailing space]
@@ -684,6 +1275,9 @@ class RosIntrospector(Node):
                     types=sorted(action["types"]),
                     server_nodes=sorted(action["servers"]),
                     client_nodes=sorted(action["clients"]),
+                    safe_types=sorted(
+                        set(action["types"]) & SAFE_NON_MOTION_ACTION_TYPES
+                    ),
                 )
             )
[trailing space]
@@ -801,6 +1395,79 @@ class RosEngineServicer(ros_engine_pb2_grpc.RosEngineServicer):
             )
         return ros_engine_pb2.Empty()
[trailing space]
+    def InvokeSafeService(self, request, context):
+        self._require_authentication(context)
+        try:
+            response = self.ros_node.invoke_safe_service(
+                request.service_name,
+                request.service_type,
+                request.request_json,
+                request.timeout_ms,
+            )
+        except PermissionError as error:
+            context.abort(grpc.StatusCode.PERMISSION_DENIED, str(error))
+        except ValueError as error:
+            context.abort(grpc.StatusCode.INVALID_ARGUMENT, str(error))
+        except LookupError as error:
+            context.abort(grpc.StatusCode.NOT_FOUND, str(error))
+        except TimeoutError as error:
+            context.abort(grpc.StatusCode.DEADLINE_EXCEEDED, str(error))
+        except RuntimeError as error:
+            context.abort(grpc.StatusCode.RESOURCE_EXHAUSTED, str(error))
+        except Exception:
+            LOGGER.exception(
+                "Safe ROS service invocation failed for %s", request.service_name
+            )
+            context.abort(
+                grpc.StatusCode.INTERNAL, "Safe ROS service invocation failed"
+            )
+        return ros_engine_pb2.SafeServiceResponse(
+            service_name=request.service_name,
+            service_type=request.service_type,
+            response_json=response["response_json"],
+            truncated=response["truncated"],
+        )
+
+    def SendSafeActionGoal(self, request, context):
+        self._require_authentication(context)
+        try:
+            goal_id = self.ros_node.submit_safe_action_goal(
+                request.action_name,
+                request.action_type,
+                request.goal_json,
+            )
+        except PermissionError as error:
+            context.abort(grpc.StatusCode.PERMISSION_DENIED, str(error))
+        except ValueError as error:
+            context.abort(grpc.StatusCode.INVALID_ARGUMENT, str(error))
+        except RuntimeError as error:
+            context.abort(grpc.StatusCode.RESOURCE_EXHAUSTED, str(error))
+        except Exception:
+            LOGGER.exception(
+                "Could not submit safe ROS action goal for %s", request.action_name
+            )
+            context.abort(grpc.StatusCode.INTERNAL, "Could not submit ROS action goal")
+        return ros_engine_pb2.ActionGoalResponse(goal_id=goal_id)
+
+    def CancelActionGoal(self, request, context):
+        self._require_authentication(context)
+        try:
+            requested = self.ros_node.cancel_safe_action_goal(request.goal_id)
+        except ValueError as error:
+            context.abort(grpc.StatusCode.INVALID_ARGUMENT, str(error))
+        except LookupError as error:
+            context.abort(grpc.StatusCode.NOT_FOUND, str(error))
+        except TimeoutError as error:
+            context.abort(grpc.StatusCode.DEADLINE_EXCEEDED, str(error))
+        except RuntimeError as error:
+            context.abort(grpc.StatusCode.FAILED_PRECONDITION, str(error))
+        except Exception:
+            LOGGER.exception("Could not request cancellation for action goal %s", request.goal_id)
+            context.abort(
+                grpc.StatusCode.INTERNAL, "Could not cancel ROS action goal"
+            )
+        return ros_engine_pb2.ActionCancelResponse(requested=requested)
+
     def _safety_state_message(self):
         state, detail, updated_at_ns = self.ros_node.safety.snapshot()
         state_value = getattr(ros_engine_pb2.SafetyState, state)
diff --git a/src-tauri/resources/devcontainer/server_core.py b/src-tauri/resources/devcontainer/server_core.py
index a291f2b..99293d3 100644
--- a/src-tauri/resources/devcontainer/server_core.py
+++ b/src-tauri/resources/devcontainer/server_core.py
@@ -29,6 +29,7 @@ MAX_CONFIG_LINEAR_ACCELERATION = 2.0
 MAX_CONFIG_ANGULAR_ACCELERATION = 4.0
 ALLOWED_MOTION_MODES = frozenset(("planar", "full_twist"))
 MAX_WORKSPACE_PACKAGES = 1000
+MAX_UNDERLAY_PACKAGES = 2000
 MAX_PACKAGE_SCAN_FILES = 5000
 MAX_PACKAGE_DISCOVERY_FILES = 250
 ACTION_SEND_GOAL_SUFFIX = "/_action/send_goal"
@@ -298,6 +299,79 @@ def discover_workspace_packages(workspace="/workspace"):
     ]
[trailing space]
[trailing space]
+def _discover_underlay_packages(underlay_roots):
+    packages = {}
+    for prefix in underlay_roots:
+        root = Path(prefix)
+        if root.is_symlink() or not root.is_dir():
+            raise ValueError(f"ROS underlay prefix is missing or unsafe: {root}")
+        root = root.resolve(strict=True)
+        share = root / "share"
+        if not share.is_dir() or share.is_symlink():
+            raise ValueError(f"ROS underlay package index is missing or unsafe: {share}")
+        try:
+            package_directories = sorted(share.iterdir())
+        except OSError as error:
+            raise ValueError(f"Could not inspect ROS underlay {share}: {error}") from error
+        for package_directory in package_directories:
+            manifest = package_directory / "package.xml"
+            if (
+                not package_directory.is_dir()
+                or package_directory.is_symlink()
+                or manifest.is_symlink()
+                or not manifest.is_file()
+                or not _package_directory_is_within(root, package_directory)
+            ):
+                continue
+            package = _parse_package_manifest(package_directory)
+            name = package["name"]
+            if not name or name in packages:
+                continue
+            package.update(
+                {
+                    "path": "",
+                    "executables": [],
+                    "launch_files": [],
+                    "config_files": [],
+                    "source": "underlay",
+                }
+            )
+            packages[name] = package
+            if len(packages) > MAX_UNDERLAY_PACKAGES:
+                raise ValueError(
+                    f"ROS underlay contains more than {MAX_UNDERLAY_PACKAGES} packages"
+                )
+    return packages
+
+
+def discover_ros_packages(
+    workspace="/workspace", underlay_roots=("/opt/ros/jazzy",)
+):
+    """Return bounded workspace and ROS underlay packages with dependency resolution."""
+    workspace_packages = {
+        package["name"]: {**package, "source": "workspace"}
+        for package in discover_workspace_packages(workspace)
+    }
+    underlay_packages = _discover_underlay_packages(underlay_roots)
+    package_sources = {
+        name: "workspace" for name in workspace_packages
+    }
+    package_sources.update(
+        {
+            name: "underlay"
+            for name in underlay_packages
+            if name not in workspace_packages
+        }
+    )
+    resolved_packages = {**underlay_packages, **workspace_packages}
+    for package in resolved_packages.values():
+        package["dependency_sources"] = {
+            dependency: package_sources.get(dependency, "missing")
+            for dependency in package["dependencies"]
+        }
+    return [resolved_packages[name] for name in sorted(resolved_packages)]
+
+
 def action_from_send_goal_service(service_name, service_type):
     """Return an action name/type only for canonical ROS 2 send-goal endpoints."""
     if not service_name.endswith(ACTION_SEND_GOAL_SUFFIX):
diff --git a/src-tauri/resources/devcontainer/test_server_core.py b/src-tauri/resources/devcontainer/test_server_core.py
index beec4a4..a0245fa 100644
--- a/src-tauri/resources/devcontainer/test_server_core.py
+++ b/src-tauri/resources/devcontainer/test_server_core.py
@@ -17,6 +17,7 @@ from server_core import (
     SafetyController,
     action_from_send_goal_service,
     discover_workspace_packages,
+    discover_ros_packages,
     is_valid_auth_token,
     load_auth_credentials,
     validate_twist_request,
@@ -181,6 +182,60 @@ class WorkspacePackageDiscoveryTests(unittest.TestCase):
[trailing space]
             self.assertEqual(discover_workspace_packages(workspace), [])
[trailing space]
+    def test_resolves_workspace_dependencies_against_workspace_and_underlay(self):
+        with tempfile.TemporaryDirectory() as parent:
+            workspace = os.path.join(parent, "workspace")
+            underlay = os.path.join(parent, "underlay")
+            os.makedirs(workspace)
+            os.makedirs(underlay)
+            self.create_package(
+                workspace,
+                "src/application",
+                """<package format="3"><name>application</name>
+                  <version>1.0.0</version><description>application</description>
+                  <depend>workspace_util</depend><depend>rclpy</depend>
+                  <depend>missing_ros_package</depend></package>""",
+            )
+            self.create_package(
+                workspace,
+                "src/workspace_util",
+                """<package format="3"><name>workspace_util</name>
+                  <version>1.0.0</version><description>workspace utility</description>
+                </package>""",
+            )
+            self.create_package(
+                underlay,
+                "share/rclpy",
+                """<package format="3"><name>rclpy</name>
+                  <version>1.0.0</version><description>ROS client library</description>
+                </package>""",
+            )
+            self.create_package(
+                underlay,
+                "share/workspace_util",
+                """<package format="3"><name>workspace_util</name>
+                  <version>0.1.0</version><description>underlay shadow</description>
+                </package>""",
+            )
+
+            packages = {
+                package["name"]: package
+                for package in discover_ros_packages(workspace, (underlay,))
+            }
+
+            self.assertEqual(
+                packages["application"]["dependency_sources"],
+                {
+                    "workspace_util": "workspace",
+                    "rclpy": "underlay",
+                    "missing_ros_package": "missing",
+                },
+            )
+            self.assertEqual(packages["rclpy"]["source"], "underlay")
+            self.assertEqual(packages["rclpy"]["path"], "")
+            self.assertEqual(packages["workspace_util"]["source"], "workspace")
+            self.assertEqual(packages["workspace_util"]["description"], "workspace utility")
+
[trailing space]
 class EventHubTests(unittest.TestCase):
     def test_subscribers_receive_independent_events(self):
diff --git a/src-tauri/src/lib.rs b/src-tauri/src/lib.rs
index 0c9fa54..91bb2c0 100644
--- a/src-tauri/src/lib.rs
+++ b/src-tauri/src/lib.rs
@@ -4,6 +4,7 @@ use std::path::{Component, Path, PathBuf};
 use std::sync::{Arc, Mutex, OnceLock};
[trailing space]
 mod urdf;
+mod xacro;
[trailing space]
 const MAX_FILE_SIZE: u64 = 8 * 1024 * 1024;
 const MAX_WORLD_SIZE: u64 = 10 * 1024 * 1024;
@@ -17,6 +18,41 @@ struct FsAccess {
     root_descriptors: Mutex<HashMap<PathBuf, Arc<rustix::fd::OwnedFd>>>,
 }
[trailing space]
+#[derive(Serialize, Clone)]
+#[serde(rename_all = "camelCase")]
+struct SafeServiceResponse {
+    service_name: String,
+    service_type: String,
+    response_json: String,
+    truncated: bool,
+}
+
+#[derive(Serialize, Clone)]
+#[serde(rename_all = "camelCase")]
+struct SafeActionGoalUpdateResponse {
+    goal_id: String,
+    action_name: String,
+    action_type: String,
+    status: String,
+    feedback_json: String,
+    result_json: String,
+    error: String,
+    terminal: bool,
+    truncated: bool,
+}
+
+#[derive(Serialize)]
+#[serde(rename_all = "camelCase")]
+struct SafeActionGoalResponse {
+    goal_id: String,
+}
+
+#[derive(Serialize)]
+#[serde(rename_all = "camelCase")]
+struct SafeActionCancelResponse {
+    requested: bool,
+}
+
 impl FsAccess {
     fn approve_directory(&self, path: &Path) -> Result<PathBuf, String> {
         let canonical = path
@@ -982,9 +1018,457 @@ fn validate_sdf_file(path: &Path, content: &str) -> Result<(), String> {
     Ok(())
 }
[trailing space]
+const MAX_XACRO_TOTAL_BYTES: usize = 8 * 1024 * 1024;
+const MAX_MESH_COUNT: usize = 128;
+const MAX_TOTAL_MESH_BYTES: usize = 16 * 1024 * 1024;
+
+fn package_name_from_manifest(content: &str) -> Result<String, String> {
+    let mut reader = quick_xml::Reader::from_str(content);
+    reader.config_mut().trim_text(true);
+    let mut package_root = false;
+    let mut in_name = false;
+    let mut name = None;
+    loop {
+        match reader.read_event() {
+            Ok(quick_xml::events::Event::Start(element)) => {
+                let local = element.local_name();
+                if !package_root {
+                    if local.as_ref() != b"package" {
+                        return Err("ROS package manifest root must be <package>".to_string());
+                    }
+                    package_root = true;
+                } else if local.as_ref() == b"name" && name.is_none() {
+                    in_name = true;
+                }
+            }
+            Ok(quick_xml::events::Event::Text(text)) if in_name => {
+                name = Some(
+                    text.decode()
+                        .map_err(|error| format!("Malformed ROS package name: {error}"))?
+                        .trim()
+                        .to_string(),
+                );
+            }
+            Ok(quick_xml::events::Event::End(element))
+                if element.local_name().as_ref() == b"name" =>
+            {
+                in_name = false;
+            }
+            Ok(quick_xml::events::Event::DocType(_)) => {
+                return Err("ROS package manifests must not contain a DTD".to_string());
+            }
+            Ok(quick_xml::events::Event::Eof) => break,
+            Ok(_) => {}
+            Err(error) => return Err(format!("Malformed ROS package manifest: {error}")),
+        }
+    }
+    let name = name.ok_or_else(|| "ROS package manifest is missing its name".to_string())?;
+    if name.is_empty()
+        || !name.chars().all(|character| {
+            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
+        })
+        || name.starts_with('_')
+    {
+        return Err(format!("Invalid ROS package name '{name}'"));
+    }
+    Ok(name)
+}
+
+fn find_ros_packages(
+    access: &FsAccess,
+    workspace: &Path,
+) -> Result<HashMap<String, PathBuf>, String> {
+    const MAX_SCANNED_ENTRIES: usize = 20_000;
+    const MAX_PACKAGE_DEPTH: usize = 12;
+    let mut packages = HashMap::new();
+    let mut pending = vec![(workspace.to_path_buf(), 0usize)];
+    let mut scanned = 0usize;
+    while let Some((directory, depth)) = pending.pop() {
+        let manifest = directory.join("package.xml");
+        if fs::symlink_metadata(&manifest)
+            .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
+        {
+            if fs::metadata(&manifest)
+                .map_err(|error| format!("Could not inspect ROS package manifest: {error}"))?
+                .len()
+                > 64 * 1024
+            {
+                return Err("ROS package manifest exceeds the 64 KiB limit".to_string());
+            }
+            let content = read_approved_file(access, &manifest)?;
+            if content.len() > 64 * 1024 {
+                return Err("ROS package manifest exceeds the 64 KiB limit".to_string());
+            }
+            let name = package_name_from_manifest(&content)?;
+            if packages.insert(name.clone(), directory.clone()).is_some() {
+                return Err(format!(
+                    "ROS package name '{name}' is ambiguous in the selected workspace"
+                ));
+            }
+            continue;
+        }
+        if depth >= MAX_PACKAGE_DEPTH {
+            continue;
+        }
+        let entries = match fs::read_dir(&directory) {
+            Ok(entries) => entries,
+            Err(_) => continue,
+        };
+        for entry in entries {
+            scanned += 1;
+            if scanned > MAX_SCANNED_ENTRIES {
+                return Err(format!(
+                    "ROS package discovery exceeds the {MAX_SCANNED_ENTRIES}-entry limit"
+                ));
+            }
+            let entry = entry.map_err(|error| format!("Could not scan workspace: {error}"))?;
+            let file_type = entry
+                .file_type()
+                .map_err(|error| format!("Could not inspect workspace entry: {error}"))?;
+            let name = entry.file_name();
+            if file_type.is_dir()
+                && !file_type.is_symlink()
+                && !matches!(
+                    name.to_str(),
+                    Some(".git" | "node_modules" | "build" | "install" | "log" | "target")
+                )
+            {
+                pending.push((entry.path(), depth + 1));
+            }
+        }
+    }
+    Ok(packages)
+}
+
+fn package_boundary_for(
+    source: &Path,
+    packages: &HashMap<String, PathBuf>,
+    workspace: &Path,
+) -> PathBuf {
+    packages
+        .values()
+        .filter(|root| source.starts_with(root))
+        .max_by_key(|root| root.components().count())
+        .cloned()
+        .unwrap_or_else(|| workspace.to_path_buf())
+}
+
+fn resolve_resource_file(base: &Path, boundary: &Path, filename: &str) -> Result<PathBuf, String> {
+    if filename.is_empty()
+        || filename.contains("://")
+        || filename.contains('\\')
+        || filename.contains('\0')
+    {
+        return Err("Robot resource URI must be a local relative path".to_string());
+    }
+    let relative = Path::new(filename);
+    if relative.is_absolute()
+        || relative
+            .components()
+            .any(|component| matches!(component, Component::RootDir | Component::Prefix(_)))
+    {
+        return Err("Robot resource path must be relative".to_string());
+    }
+    let resolved = base
+        .join(relative)
+        .canonicalize()
+        .map_err(|error| format!("Could not resolve robot resource '{filename}': {error}"))?;
+    if !resolved.starts_with(boundary) {
+        return Err("Robot resource resolves outside its approved workspace/package root".into());
+    }
+    let metadata = fs::symlink_metadata(&resolved)
+        .map_err(|error| format!("Could not inspect robot resource: {error}"))?;
+    if !metadata.is_file() || metadata.file_type().is_symlink() {
+        return Err("Robot resource must be a regular file".to_string());
+    }
+    Ok(resolved)
+}
+
+fn resolve_mesh_path(
+    filename: &str,
+    description_path: &Path,
+    workspace: &Path,
+    packages: &HashMap<String, PathBuf>,
+) -> Result<PathBuf, String> {
+    if let Some(package_uri) = filename.strip_prefix("package://") {
+        let (package_name, relative) = package_uri.split_once('/').ok_or_else(|| {
+            "package:// mesh URI must include a package and file path".to_string()
+        })?;
+        if package_name.is_empty()
+            || !package_name.chars().all(|character| {
+                character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
+            })
+        {
+            return Err("package:// URI contains an invalid package name".to_string());
+        }
+        let package_root = packages.get(package_name).ok_or_else(|| {
+            format!("ROS package '{package_name}' is not in the selected workspace")
+        })?;
+        if Path::new(relative)
+            .components()
+            .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
+        {
+            return Err(
+                "package:// mesh path cannot traverse outside its package root".to_string(),
+            );
+        }
+        resolve_resource_file(package_root, package_root, relative)
+    } else {
+        let boundary = package_boundary_for(description_path, packages, workspace);
+        let base = description_path
+            .parent()
+            .ok_or_else(|| "Robot description has no parent directory".to_string())?;
+        resolve_resource_file(base, &boundary, filename)
+    }
+}
+
+fn render_robot_description(
+    access: &FsAccess,
+    workspace_root: &str,
+    path: &str,
+    content: &str,
+) -> Result<urdf::RobotDescription, String> {
+    let workspace = Path::new(workspace_root)
+        .canonicalize()
+        .map_err(|error| format!("Could not resolve selected workspace: {error}"))?;
+    if !workspace.is_dir()
+        || !access
+            .approved_roots
+            .lock()
+            .map_err(|_| "Filesystem access state is unavailable".to_string())?
+            .contains(&workspace)
+    {
+        return Err("Robot resources are restricted to the approved selected workspace".into());
+    }
+    let source_path = access.resolve_existing(Path::new(path))?;
+    if !source_path.starts_with(&workspace) {
+        return Err("Robot description is outside the selected workspace".to_string());
+    }
+    if content.len() > 2 * 1024 * 1024 {
+        return Err("Robot description exceeds the 2 MiB source limit".to_string());
+    }
+    let packages = find_ros_packages(access, &workspace)?;
+    let is_xacro = source_path
+        .extension()
+        .and_then(|extension| extension.to_str())
+        .is_some_and(|extension| extension.eq_ignore_ascii_case("xacro"));
+    let expanded = if is_xacro {
+        let mut total_include_bytes = content.len();
+        let mut include_loader = |including_file: &Path,
+                                  filename: &str|
+         -> Result<(PathBuf, String), String> {
+            if filename.contains('\\') {
+                return Err("Xacro includes must use workspace-local relative paths".to_string());
+            }
+            let include_path = if let Some(package_uri) = filename.strip_prefix("package://") {
+                let (package_name, relative) = package_uri.split_once('/').ok_or_else(|| {
+                    "Xacro package include must name a package and file".to_string()
+                })?;
+                let package_root = packages.get(package_name).ok_or_else(|| {
+                    format!("ROS package '{package_name}' is not in the selected workspace")
+                })?;
+                if Path::new(relative)
+                    .components()
+                    .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
+                {
+                    return Err(
+                        "Xacro package include cannot traverse its package root".to_string()
+                    );
+                }
+                resolve_resource_file(package_root, package_root, relative)?
+            } else {
+                if Path::new(filename).is_absolute() || filename.contains("://") {
+                    return Err(
+                        "Xacro includes must use workspace-local relative paths".to_string()
+                    );
+                }
+                let include_boundary = package_boundary_for(including_file, &packages, &workspace);
+                let include_base = including_file
+                    .parent()
+                    .ok_or_else(|| "Xacro source has no parent directory".to_string())?;
+                resolve_resource_file(include_base, &include_boundary, filename)?
+            };
+            if fs::metadata(&include_path)
+                .map_err(|error| format!("Could not inspect Xacro include: {error}"))?
+                .len()
+                > 2 * 1024 * 1024
+            {
+                return Err("Xacro include exceeds the 2 MiB source limit".to_string());
+            }
+            let include_content = read_approved_file(access, &include_path)?;
+            if include_content.len() > 2 * 1024 * 1024 {
+                return Err("Xacro include exceeds the 2 MiB source limit".to_string());
+            }
+            total_include_bytes = total_include_bytes.saturating_add(include_content.len());
+            if total_include_bytes > MAX_XACRO_TOTAL_BYTES {
+                return Err("Xacro source and includes exceed the 8 MiB total limit".into());
+            }
+            Ok((include_path, include_content))
+        };
+        xacro::expand(content, &source_path, &mut include_loader)?
+    } else {
+        content.to_string()
+    };
+
+    let mut robot = urdf::parse_robot_description(&expanded)?;
+    let mut mesh_count = 0usize;
+    let mut mesh_bytes = 0usize;
+    for link in &mut robot.links {
+        for visual in link.visuals.iter_mut().chain(link.collisions.iter_mut()) {
+            let urdf::RobotGeometry::Mesh {
+                filename,
+                format,
+                data,
+                ..
+            } = &mut visual.geometry
+            else {
+                continue;
+            };
+            mesh_count += 1;
+            if mesh_count > MAX_MESH_COUNT {
+                return Err(format!("Robot mesh count exceeds {MAX_MESH_COUNT}"));
+            }
+            let mesh_path = resolve_mesh_path(filename, &source_path, &workspace, &packages)?;
+            let mesh_content = access
+                .read_file_if_present(&mesh_path)?
+                .ok_or_else(|| "Robot mesh disappeared while loading".to_string())?;
+            mesh_bytes = mesh_bytes.saturating_add(mesh_content.len());
+            if mesh_bytes > MAX_TOTAL_MESH_BYTES {
+                return Err("Robot meshes exceed the 16 MiB total byte limit".to_string());
+            }
+            urdf::validate_mesh_bytes(format, &mesh_content)?;
+            *data = Some(base64::Engine::encode(
+                &base64::engine::general_purpose::STANDARD,
+                mesh_content,
+            ));
+        }
+    }
+    Ok(robot)
+}
+
 #[tauri::command]
-fn parse_robot_description(content: String) -> Result<urdf::RobotDescription, String> {
-    urdf::parse_robot_description(&content)
+fn parse_robot_description(
+    access: tauri::State<'_, FsAccess>,
+    workspace_root: String,
+    path: String,
+    content: String,
+) -> Result<urdf::RobotDescription, String> {
+    render_robot_description(&access, &workspace_root, &path, &content)
+}
+
+#[cfg(test)]
+mod urdf_rendering_tests {
+    use super::{render_robot_description, FsAccess};
+    use crate::urdf::RobotGeometry;
+    use std::fs;
+
+    fn fixture_workspace() -> std::path::PathBuf {
+        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
+            .join("../tests/fixtures/urdf_workspace")
+            .canonicalize()
+            .unwrap()
+    }
+
+    #[test]
+    fn expands_xacro_and_loads_only_workspace_package_meshes() {
+        let access = FsAccess::default();
+        let workspace = fixture_workspace();
+        access.approve_directory(&workspace).unwrap();
+        let description_path = workspace
+            .join("src/demo_robot/urdf/robot.xacro")
+            .canonicalize()
+            .unwrap();
+        let content = fs::read_to_string(&description_path).unwrap();
+        let robot = render_robot_description(
+            &access,
+            workspace.to_str().unwrap(),
+            description_path.to_str().unwrap(),
+            &content,
+        )
+        .unwrap();
+
+        assert_eq!(robot.name, "demo_robot");
+        assert_eq!(robot.root_link, "base");
+        assert_eq!(robot.links[0].visuals[0].color, Some([0.1, 0.3, 0.8, 1.0]));
+        assert_eq!(robot.links[0].inertial.as_ref().unwrap().mass, 1.5);
+        assert_eq!(robot.links[0].collisions.len(), 1);
+        let RobotGeometry::Mesh {
+            filename,
+            scale,
+            format,
+            data,
+        } = &robot.links[0].visuals[0].geometry
+        else {
+            panic!("expected resolved STL mesh");
+        };
+        assert_eq!(filename, "package://demo_robot/meshes/triangle.stl");
+        assert_eq!(*scale, [1.0, 2.0, 1.0]);
+        assert_eq!(format, "stl");
+        assert!(data.as_ref().is_some_and(|mesh| !mesh.is_empty()));
+    }
+
+    #[test]
+    fn rejects_unapproved_roots_and_package_traversal_in_mesh_uris() {
+        let workspace = fixture_workspace();
+        let access = FsAccess::default();
+        let description_path = workspace.join("src/demo_robot/urdf/robot.xacro");
+        let content = fs::read_to_string(&description_path).unwrap();
+
+        assert!(render_robot_description(
+            &access,
+            workspace.to_str().unwrap(),
+            description_path.to_str().unwrap(),
+            &content,
+        )
+        .unwrap_err()
+        .contains("approved selected workspace"));
+
+        access.approve_directory(&workspace).unwrap();
+        let traversal = content.replace(
+            "package://demo_robot/meshes/triangle.stl",
+            "package://demo_robot/../../outside.stl",
+        );
+        assert!(render_robot_description(
+            &access,
+            workspace.to_str().unwrap(),
+            description_path.to_str().unwrap(),
+            &traversal,
+        )
+        .unwrap_err()
+        .contains("cannot traverse"));
+    }
+
+    #[test]
+    fn resolves_and_validates_texture_free_collada_meshes() {
+        let access = FsAccess::default();
+        let workspace = fixture_workspace();
+        access.approve_directory(&workspace).unwrap();
+        let description_path = workspace
+            .join("src/demo_robot/urdf/robot.xacro")
+            .canonicalize()
+            .unwrap();
+        let content = r#"<robot name="dae_test"><link name="base"><visual><geometry><mesh filename="package://demo_robot/meshes/triangle.dae"/></geometry></visual></link></robot>"#;
+        let robot = render_robot_description(
+            &access,
+            workspace.to_str().unwrap(),
+            description_path.to_str().unwrap(),
+            content,
+        )
+        .unwrap();
+        let RobotGeometry::Mesh {
+            format,
+            data: Some(data),
+            ..
+        } = &robot.links[0].visuals[0].geometry
+        else {
+            panic!("expected resolved DAE mesh");
+        };
+        assert_eq!(format, "dae");
+        let decoded =
+            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data).unwrap();
+        let original = fs::read(workspace.join("src/demo_robot/meshes/triangle.dae")).unwrap();
+        assert_eq!(decoded, original);
+    }
 }
[trailing space]
 fn import_world_contents(
@@ -1127,6 +1611,9 @@ pub fn run() {
             ask_ai,
             initialize_ros_environment,
             publish_twist,
+            invoke_safe_ros_service,
+            send_safe_ros_action_goal,
+            cancel_safe_ros_action_goal,
             start_topic_inspection,
             stop_topic_inspection,
             set_emergency_stop,
@@ -1379,6 +1866,24 @@ async fn start_ros_stream(app: AppHandle) -> Result<(), String> {
                                 },
                             );
                         }
+                        if let Some(rosengine::workspace_event::Event::ActionGoalUpdate(update)) =
+                            &evt.event
+                        {
+                            let _ = app.emit(
+                                "safe-action-goal-update",
+                                SafeActionGoalUpdateResponse {
+                                    goal_id: update.goal_id.clone(),
+                                    action_name: update.action_name.clone(),
+                                    action_type: update.action_type.clone(),
+                                    status: update.status.clone(),
+                                    feedback_json: update.feedback_json.clone(),
+                                    result_json: update.result_json.clone(),
+                                    error: update.error.clone(),
+                                    terminal: update.terminal,
+                                    truncated: update.truncated,
+                                },
+                            );
+                        }
[trailing space]
                         if let Some(rosengine::workspace_event::Event::TfUpdate(tf)) = &evt.event {
                             let _ = app.emit(
@@ -1706,6 +2211,7 @@ struct RosGraphServiceResponse {
     types: Vec<String>,
     server_nodes: Vec<String>,
     client_nodes: Vec<String>,
+    safe_types: Vec<String>,
 }
[trailing space]
 #[derive(Serialize, Clone)]
@@ -1715,6 +2221,7 @@ struct RosGraphActionResponse {
     types: Vec<String>,
     server_nodes: Vec<String>,
     client_nodes: Vec<String>,
+    safe_types: Vec<String>,
 }
[trailing space]
 #[derive(Serialize, Clone)]
@@ -1799,6 +2306,7 @@ fn ros_graph_snapshot_response(snapshot: &rosengine::RosGraphSnapshot) -> RosGra
                 types: service.types.clone(),
                 server_nodes: service.server_nodes.clone(),
                 client_nodes: service.client_nodes.clone(),
+                safe_types: service.safe_types.clone(),
             })
             .collect(),
         actions: snapshot
@@ -1809,6 +2317,7 @@ fn ros_graph_snapshot_response(snapshot: &rosengine::RosGraphSnapshot) -> RosGra
                 types: action.types.clone(),
                 server_nodes: action.server_nodes.clone(),
                 client_nodes: action.client_nodes.clone(),
+                safe_types: action.safe_types.clone(),
             })
             .collect(),
         truncated: snapshot.truncated,
@@ -1840,6 +2349,126 @@ async fn connect_ros_client(
     Ok((client, token))
 }
[trailing space]
+const SAFE_READ_ONLY_SERVICE_TYPES: &[&str] = &[
+    "lifecycle_msgs/srv/GetAvailableStates",
+    "lifecycle_msgs/srv/GetAvailableTransitions",
+    "lifecycle_msgs/srv/GetState",
+    "lifecycle_msgs/srv/GetTransitionGraph",
+    "rcl_interfaces/srv/DescribeParameters",
+    "rcl_interfaces/srv/GetParameterTypes",
+    "rcl_interfaces/srv/GetParameters",
+    "rcl_interfaces/srv/ListParameters",
+];
+const SAFE_NON_MOTION_ACTION_TYPES: &[&str] = &["action_tutorials_interfaces/action/Fibonacci"];
+
+#[tauri::command]
+async fn invoke_safe_ros_service(
+    app: AppHandle,
+    service_name: String,
+    service_type: String,
+    request_json: String,
+    timeout_ms: u32,
+) -> Result<SafeServiceResponse, String> {
+    if !is_valid_ros_topic(&service_name) {
+        return Err("Invalid ROS service name".to_string());
+    }
+    if !SAFE_READ_ONLY_SERVICE_TYPES.contains(&service_type.as_str()) {
+        return Err("This ROS service type is not on the read-only allowlist".to_string());
+    }
+    if !(100..=5_000).contains(&timeout_ms) {
+        return Err("Service timeout must be between 100 and 5000 milliseconds".to_string());
+    }
+    if request_json.len() > 16 * 1024
+        || !serde_json::from_str::<serde_json::Value>(&request_json)
+            .is_ok_and(|value| value.is_object())
+    {
+        return Err("Service request must be a JSON object no larger than 16 KiB".to_string());
+    }
+    let (mut client, token) = connect_ros_client(&app).await?;
+    let response = client
+        .invoke_safe_service(authenticated_request(
+            rosengine::SafeServiceRequest {
+                service_name,
+                service_type,
+                request_json,
+                timeout_ms,
+            },
+            &token,
+        )?)
+        .await
+        .map_err(|error| format!("Safe ROS service call failed: {}", error.message()))?
+        .into_inner();
+    Ok(SafeServiceResponse {
+        service_name: response.service_name,
+        service_type: response.service_type,
+        response_json: response.response_json,
+        truncated: response.truncated,
+    })
+}
+
+#[tauri::command]
+async fn send_safe_ros_action_goal(
+    app: AppHandle,
+    action_name: String,
+    action_type: String,
+    goal_json: String,
+) -> Result<SafeActionGoalResponse, String> {
+    if !is_valid_ros_topic(&action_name) {
+        return Err("Invalid ROS action name".to_string());
+    }
+    if !SAFE_NON_MOTION_ACTION_TYPES.contains(&action_type.as_str()) {
+        return Err("This ROS action type is not on the non-motion allowlist".to_string());
+    }
+    if goal_json.len() > 16 * 1024
+        || !serde_json::from_str::<serde_json::Value>(&goal_json)
+            .is_ok_and(|value| value.is_object())
+    {
+        return Err("Action goal must be a JSON object no larger than 16 KiB".to_string());
+    }
+    let (mut client, token) = connect_ros_client(&app).await?;
+    let response = client
+        .send_safe_action_goal(authenticated_request(
+            rosengine::SafeActionGoalRequest {
+                action_name,
+                action_type,
+                goal_json,
+            },
+            &token,
+        )?)
+        .await
+        .map_err(|error| format!("Could not submit ROS action goal: {}", error.message()))?
+        .into_inner();
+    Ok(SafeActionGoalResponse {
+        goal_id: response.goal_id,
+    })
+}
+
+#[tauri::command]
+async fn cancel_safe_ros_action_goal(
+    app: AppHandle,
+    goal_id: String,
+) -> Result<SafeActionCancelResponse, String> {
+    if goal_id.len() != 36
+        || !goal_id
+            .chars()
+            .all(|ch| ch.is_ascii_hexdigit() || ch == '-')
+    {
+        return Err("Invalid action goal identifier".to_string());
+    }
+    let (mut client, token) = connect_ros_client(&app).await?;
+    let response = client
+        .cancel_action_goal(authenticated_request(
+            rosengine::ActionGoalIdRequest { goal_id },
+            &token,
+        )?)
+        .await
+        .map_err(|error| format!("Could not cancel ROS action goal: {}", error.message()))?
+        .into_inner();
+    Ok(SafeActionCancelResponse {
+        requested: response.requested,
+    })
+}
+
 #[tauri::command]
 async fn start_topic_inspection(app: AppHandle, topic_name: String) -> Result<(), String> {
     if !is_valid_ros_topic(&topic_name) {
@@ -2311,10 +2940,12 @@ fn stop_workspace_container(workspace: &Path) -> Result<(), String> {
 struct RosPackageInfo {
     name: String,
     path: String,
+    source: String,
     package_type: String,
     version: Option<String>,
     description: Option<String>,
     dependencies: Vec<String>,
+    dependency_sources: HashMap<String, String>,
     executables: Vec<String>,
     launch_files: Vec<String>,
     config_files: Vec<String>,
@@ -2370,7 +3001,7 @@ fn list_ros_packages() -> Result<Vec<RosPackageInfo>, String> {
             &container_name,
             "bash",
             "-lc",
-            "cd /opt/robotics-studio && python3 -c \"import json; from server_core import discover_workspace_packages; print(json.dumps(discover_workspace_packages('/workspace')))\"",
+            "cd /opt/robotics-studio && python3 -c \"import json; from server_core import discover_ros_packages; print(json.dumps(discover_ros_packages('/workspace')))\"",
         ])
         .output()
         .map_err(|error| format!("Could not inspect ROS packages: {error}"))?;
@@ -2384,9 +3015,28 @@ fn list_ros_packages() -> Result<Vec<RosPackageInfo>, String> {
     let mut parsed: Vec<RosPackageInfo> = serde_json::from_str(stdout.trim())
         .map_err(|error| format!("Could not parse ROS package metadata: {error}"))?;
     for package in &mut parsed {
-        package.path = resolve_workspace_package_path(&workspace, &package.name, &package.path)?
-            .to_string_lossy()
-            .into_owned();
+        match package.source.as_str() {
+            "workspace" => {
+                package.path =
+                    resolve_workspace_package_path(&workspace, &package.name, &package.path)?
+                        .to_string_lossy()
+                        .into_owned();
+            }
+            "underlay" => {
+                if !package.path.is_empty() {
+                    return Err(format!(
+                        "ROS underlay package {} unexpectedly returned a workspace path",
+                        package.name
+                    ));
+                }
+            }
+            _ => {
+                return Err(format!(
+                    "ROS package {} returned an unknown package source",
+                    package.name
+                ));
+            }
+        }
     }
     Ok(parsed)
 }
diff --git a/src-tauri/src/urdf.rs b/src-tauri/src/urdf.rs
index 6f738ec..c624e17 100644
--- a/src-tauri/src/urdf.rs
+++ b/src-tauri/src/urdf.rs
@@ -1,10 +1,14 @@
 use serde::Serialize;
 use std::collections::{HashMap, HashSet};
[trailing space]
-const MAX_DESCRIPTION_BYTES: usize = 2 * 1024 * 1024;
+const MAX_DESCRIPTION_BYTES: usize = 4 * 1024 * 1024;
+const MAX_MESH_BYTES: usize = 8 * 1024 * 1024;
 const MAX_LINKS: usize = 512;
 const MAX_JOINTS: usize = 512;
 const MAX_VISUALS_PER_LINK: usize = 64;
+const MAX_MESH_TRIANGLES: usize = 250_000;
+const MAX_XML_ELEMENTS: usize = 100_000;
+const MAX_XML_DEPTH: usize = 128;
[trailing space]
 #[derive(Clone, Debug, Default, PartialEq, Serialize)]
 #[serde(rename_all = "camelCase")]
@@ -16,10 +20,25 @@ pub struct RobotOrigin {
 #[derive(Clone, Debug, PartialEq, Serialize)]
 #[serde(tag = "kind", rename_all = "camelCase")]
 pub enum RobotGeometry {
-    Box { size: [f64; 3] },
-    Cylinder { radius: f64, length: f64 },
-    Sphere { radius: f64 },
-    Unsupported { description: String },
+    Box {
+        size: [f64; 3],
+    },
+    Cylinder {
+        radius: f64,
+        length: f64,
+    },
+    Sphere {
+        radius: f64,
+    },
+    Mesh {
+        filename: String,
+        scale: [f64; 3],
+        format: String,
+        data: Option<String>,
+    },
+    Unsupported {
+        description: String,
+    },
 }
[trailing space]
 #[derive(Clone, Debug, PartialEq, Serialize)]
@@ -28,6 +47,22 @@ pub struct RobotVisual {
     pub origin: RobotOrigin,
     pub geometry: RobotGeometry,
     pub color: Option<[f64; 4]>,
+    pub material_name: Option<String>,
+}
+
+#[derive(Clone, Debug, PartialEq, Serialize)]
+#[serde(rename_all = "camelCase")]
+pub struct RobotInertial {
+    pub origin: RobotOrigin,
+    pub mass: f64,
+    pub inertia: [f64; 6],
+}
+
+#[derive(Default)]
+struct InertialBuilder {
+    origin: RobotOrigin,
+    mass: Option<f64>,
+    inertia: Option<[f64; 6]>,
 }
[trailing space]
 #[derive(Clone, Debug, PartialEq, Serialize)]
@@ -36,6 +71,7 @@ pub struct RobotLink {
     pub name: String,
     pub visuals: Vec<RobotVisual>,
     pub collisions: Vec<RobotVisual>,
+    pub inertial: Option<RobotInertial>,
 }
[trailing space]
 #[derive(Clone, Debug, PartialEq, Serialize)]
@@ -194,6 +230,163 @@ fn parse_color(value: &str) -> Result<[f64; 4], String> {
     Ok(color)
 }
[trailing space]
+pub fn validate_mesh_bytes(format: &str, bytes: &[u8]) -> Result<(), String> {
+    if bytes.is_empty() || bytes.len() > MAX_MESH_BYTES {
+        return Err(format!(
+            "Mesh size must be between 1 byte and {MAX_MESH_BYTES} bytes"
+        ));
+    }
+
+    match format {
+        "stl" => {
+            let triangle_count = if bytes.len() >= 84 {
+                let count = u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize;
+                let expected_size = 84usize.saturating_add(count.saturating_mul(50));
+                (expected_size == bytes.len()).then_some(count)
+            } else {
+                None
+            };
+            let triangle_count = match triangle_count {
+                Some(count) => count,
+                None => {
+                    let text = std::str::from_utf8(bytes).map_err(|_| {
+                        "STL mesh is neither valid binary nor UTF-8 ASCII".to_string()
+                    })?;
+                    if !text.trim_start().starts_with("solid")
+                        || !text
+                            .trim_end()
+                            .lines()
+                            .last()
+                            .is_some_and(|line| line.trim_start().starts_with("endsolid"))
+                    {
+                        return Err("Malformed ASCII STL mesh".to_string());
+                    }
+                    text.match_indices("facet normal").count()
+                }
+            };
+            if triangle_count == 0 || triangle_count > MAX_MESH_TRIANGLES {
+                return Err(format!(
+                    "STL triangle count must be between 1 and {MAX_MESH_TRIANGLES}"
+                ));
+            }
+        }
+        "dae" => {
+            let text =
+                std::str::from_utf8(bytes).map_err(|_| "DAE mesh must be UTF-8 XML".to_string())?;
+            if text.len() > MAX_MESH_BYTES {
+                return Err("DAE mesh exceeds its byte limit".to_string());
+            }
+            let mut reader = quick_xml::Reader::from_str(text);
+            reader.config_mut().trim_text(true);
+            let mut root_seen = false;
+            let mut triangle_count = 0usize;
+            let mut node_count = 0usize;
+            let mut depth = 0usize;
+            loop {
+                match reader.read_event() {
+                    Ok(
+                        event @ (quick_xml::events::Event::Start(_)
+                        | quick_xml::events::Event::Empty(_)),
+                    ) => {
+                        let (element, is_empty) = match event {
+                            quick_xml::events::Event::Start(element) => (element, false),
+                            quick_xml::events::Event::Empty(element) => (element, true),
+                            _ => unreachable!(),
+                        };
+                        node_count += 1;
+                        if node_count > 100_000 || depth > 128 {
+                            return Err("DAE XML exceeds the node or nesting limit".to_string());
+                        }
+                        let name =
+                            String::from_utf8_lossy(element.local_name().as_ref()).into_owned();
+                        if !root_seen {
+                            if name != "COLLADA" {
+                                return Err("DAE mesh root element must be <COLLADA>".to_string());
+                            }
+                            root_seen = true;
+                        }
+                        if name == "image" {
+                            return Err(
+                                "DAE meshes with external image/texture references are not supported"
+                                    .to_string(),
+                            );
+                        }
+                        for attribute in element.attributes().with_checks(true) {
+                            let attribute = attribute
+                                .map_err(|error| format!("Malformed DAE attribute: {error}"))?;
+                            if String::from_utf8_lossy(attribute.key.as_ref())
+                                .rsplit(':')
+                                .next()
+                                == Some("url")
+                            {
+                                let value = attribute
+                                    .decoded_and_normalized_value(
+                                        quick_xml::XmlVersion::Implicit1_0,
+                                        reader.decoder(),
+                                    )
+                                    .map_err(|error| {
+                                        format!("Malformed DAE URL attribute: {error}")
+                                    })?;
+                                if !value.starts_with('#') || value.len() < 2 {
+                                    return Err("DAE meshes cannot reference external resources"
+                                        .to_string());
+                                }
+                            }
+                        }
+                        if matches!(name.as_ref(), "triangles" | "polylist") {
+                            let mut count = None;
+                            for attribute in element.attributes().with_checks(true) {
+                                let attribute = attribute
+                                    .map_err(|error| format!("Malformed DAE attribute: {error}"))?;
+                                if attribute.key.as_ref() == b"count" {
+                                    let value = attribute
+                                        .decoded_and_normalized_value(
+                                            quick_xml::XmlVersion::Implicit1_0,
+                                            reader.decoder(),
+                                        )
+                                        .map_err(|error| {
+                                            format!("Malformed DAE count attribute: {error}")
+                                        })?;
+                                    count =
+                                        Some(value.parse::<usize>().map_err(|_| {
+                                            "Invalid DAE primitive count".to_string()
+                                        })?);
+                                }
+                            }
+                            triangle_count =
+                                triangle_count.saturating_add(count.ok_or_else(|| {
+                                    "DAE primitive is missing its count attribute".to_string()
+                                })?);
+                            if triangle_count > MAX_MESH_TRIANGLES {
+                                return Err(format!(
+                                    "DAE triangle count exceeds {MAX_MESH_TRIANGLES}"
+                                ));
+                            }
+                        }
+                        if !is_empty {
+                            depth += 1;
+                        }
+                    }
+                    Ok(quick_xml::events::Event::End(_)) => {
+                        depth = depth.saturating_sub(1);
+                    }
+                    Ok(quick_xml::events::Event::DocType(_)) => {
+                        return Err("DAE documents must not contain a DTD".to_string());
+                    }
+                    Ok(quick_xml::events::Event::Eof) => break,
+                    Ok(_) => {}
+                    Err(error) => return Err(format!("Malformed DAE XML: {error}")),
+                }
+            }
+            if !root_seen || triangle_count == 0 {
+                return Err("DAE mesh contains no supported triangle geometry".to_string());
+            }
+        }
+        _ => return Err(format!("Unsupported mesh format '{format}'")),
+    }
+    Ok(())
+}
+
 fn element_name(element: &quick_xml::events::BytesStart<'_>) -> String {
     String::from_utf8_lossy(element.local_name().as_ref()).into_owned()
 }
@@ -227,12 +420,35 @@ fn parse_geometry(
             }
             Ok(RobotGeometry::Sphere { radius })
         }
-        "mesh" => Ok(RobotGeometry::Unsupported {
-            description: attributes
-                .get("filename")
-                .cloned()
-                .unwrap_or_else(|| "mesh with no filename".to_string()),
-        }),
+        "mesh" => {
+            let filename = required_attribute(attributes, "filename")?.to_string();
+            let format = filename
+                .rsplit_once('.')
+                .map(|(_, extension)| extension.to_ascii_lowercase())
+                .ok_or_else(|| format!("Unsupported URDF mesh format: '{filename}'"))?;
+            if !matches!(format.as_str(), "stl" | "dae") {
+                return Err(format!(
+                    "Unsupported URDF mesh format '{format}'; only STL and DAE are supported"
+                ));
+            }
+            let scale = attributes
+                .get("scale")
+                .map(|value| parse_vector(value, "mesh scale"))
+                .transpose()?
+                .unwrap_or([1.0; 3]);
+            if scale
+                .iter()
+                .any(|component| component.abs() <= f64::EPSILON)
+            {
+                return Err("URDF mesh scale components must be non-zero".to_string());
+            }
+            Ok(RobotGeometry::Mesh {
+                filename,
+                scale,
+                format,
+                data: None,
+            })
+        }
         _ => Err(format!("Unsupported URDF geometry element: {element_name}")),
     }
 }
@@ -369,15 +585,23 @@ pub fn parse_robot_description(content: &str) -> Result<RobotDescription, String
     let mut current_joint = None::<RobotJoint>;
     let mut current_visual = None::<RobotVisual>;
     let mut current_collision = None::<RobotVisual>;
+    let mut current_inertial = None::<InertialBuilder>;
+    let mut current_global_material = None::<(String, Option<[f64; 4]>)>;
+    let mut global_materials = HashMap::<String, Option<[f64; 4]>>::new();
     let mut links = Vec::new();
     let mut joints = Vec::new();
     let mut warnings = Vec::new();
+    let mut element_count = 0usize;
[trailing space]
     loop {
         match reader.read_event() {
             Ok(
                 event @ (quick_xml::events::Event::Start(_) | quick_xml::events::Event::Empty(_)),
             ) => {
+                element_count += 1;
+                if element_count > MAX_XML_ELEMENTS || stack.len() >= MAX_XML_DEPTH {
+                    return Err("URDF XML exceeds the element or nesting limit".to_string());
+                }
                 let (element, is_empty) = match event {
                     quick_xml::events::Event::Start(element) => (element, false),
                     quick_xml::events::Event::Empty(element) => (element, true),
@@ -408,6 +632,7 @@ pub fn parse_robot_description(content: &str) -> Result<RobotDescription, String
                             name: required_attribute(&attributes, "name")?.to_string(),
                             visuals: Vec::new(),
                             collisions: Vec::new(),
+                            inertial: None,
                         });
                     }
                     "joint" if parent == Some("robot") => {
@@ -435,6 +660,7 @@ pub fn parse_robot_description(content: &str) -> Result<RobotDescription, String
                                 description: String::new(),
                             },
                             color: None,
+                            material_name: None,
                         });
                     }
                     "collision" if parent == Some("link") => {
@@ -447,6 +673,7 @@ pub fn parse_robot_description(content: &str) -> Result<RobotDescription, String
                                 description: String::new(),
                             },
                             color: None,
+                            material_name: None,
                         });
                     }
                     "origin" if parent == Some("visual") => {
@@ -464,6 +691,11 @@ pub fn parse_robot_description(content: &str) -> Result<RobotDescription, String
                             joint.origin = parse_origin(&attributes)?;
                         }
                     }
+                    "origin" if parent == Some("inertial") => {
+                        if let Some(inertial) = current_inertial.as_mut() {
+                            inertial.origin = parse_origin(&attributes)?;
+                        }
+                    }
                     "parent" if parent == Some("joint") => {
                         if let Some(joint) = current_joint.as_mut() {
                             joint.parent = required_attribute(&attributes, "link")?.to_string();
@@ -504,21 +736,87 @@ pub fn parse_robot_description(content: &str) -> Result<RobotDescription, String
                         }
                     }
                     "inertial" if parent == Some("link") => {
-                        warnings.push(
-                            "URDF inertial properties are not used in the preview".to_string(),
-                        );
+                        if current_inertial.is_some() {
+                            return Err("Link contains more than one inertial element".to_string());
+                        }
+                        current_inertial = Some(InertialBuilder::default());
+                    }
+                    "mass" if parent == Some("inertial") => {
+                        if let Some(inertial) = current_inertial.as_mut() {
+                            if inertial.mass.is_some() {
+                                return Err("Inertial element defines mass more than once".into());
+                            }
+                            let mass = parse_number(
+                                required_attribute(&attributes, "value")?,
+                                "inertial mass",
+                            )?;
+                            if mass < 0.0 {
+                                return Err("URDF inertial mass must not be negative".to_string());
+                            }
+                            inertial.mass = Some(mass);
+                        }
+                    }
+                    "inertia" if parent == Some("inertial") => {
+                        if let Some(inertial) = current_inertial.as_mut() {
+                            if inertial.inertia.is_some() {
+                                return Err(
+                                    "Inertial element defines inertia more than once".into()
+                                );
+                            }
+                            let names = ["ixx", "ixy", "ixz", "iyy", "iyz", "izz"];
+                            let mut values = [0.0; 6];
+                            for (index, name) in names.iter().enumerate() {
+                                values[index] = parse_number(
+                                    required_attribute(&attributes, name)?,
+                                    "inertia tensor",
+                                )?;
+                            }
+                            let [ixx, ixy, ixz, iyy, iyz, izz] = values;
+                            let minor_xy = ixx * iyy - ixy * ixy;
+                            let minor_xz = ixx * izz - ixz * ixz;
+                            let minor_yz = iyy * izz - iyz * iyz;
+                            let determinant = ixx * (iyy * izz - iyz * iyz)
+                                - ixy * (ixy * izz - iyz * ixz)
+                                + ixz * (ixy * iyz - iyy * ixz);
+                            if !minor_xy.is_finite()
+                                || !minor_xz.is_finite()
+                                || !minor_yz.is_finite()
+                                || !determinant.is_finite()
+                                || ixx < 0.0
+                                || iyy < 0.0
+                                || izz < 0.0
+                                || minor_xy < -f64::EPSILON
+                                || minor_xz < -f64::EPSILON
+                                || minor_yz < -f64::EPSILON
+                                || determinant < -f64::EPSILON
+                            {
+                                return Err(
+                                    "URDF inertia tensor must be positive semidefinite".to_string()
+                                );
+                            }
+                            inertial.inertia = Some(values);
+                        }
                     }
                     "material" if parent == Some("robot") => {
-                        warnings.push(
-                            "Robot-level named materials are not resolved in the preview"
-                                .to_string(),
-                        );
+                        let name = required_attribute(&attributes, "name")?.to_string();
+                        if current_global_material.is_some() {
+                            return Err("Nested URDF material elements are invalid".into());
+                        }
+                        current_global_material = Some((name, None));
+                    }
+                    "material" if parent == Some("visual") => {
+                        if let Some(visual) = current_visual.as_mut() {
+                            visual.material_name = attributes.get("name").cloned();
+                        }
                     }
                     "color" if parent == Some("material") => {
-                        if let (Some(visual), Some(color)) =
-                            (current_visual.as_mut(), attributes.get("rgba"))
-                        {
-                            visual.color = Some(parse_color(color)?);
+                        if let Some(color) = attributes.get("rgba") {
+                            let color = parse_color(color)?;
+                            if let Some((_, material_color)) = current_global_material.as_mut() {
+                                *material_color = Some(color);
+                            } else if let Some(visual) = current_visual.as_mut() {
+                                visual.color = Some(color);
+                            }
                         }
                     }
                     "box" | "cylinder" | "sphere" | "mesh" if parent == Some("geometry") => {
@@ -552,6 +850,36 @@ pub fn parse_robot_description(content: &str) -> Result<RobotDescription, String
                                     .ok_or_else(|| "URDF link state is inconsistent".to_string())?,
                             );
                         }
+                        "inertial" => {
+                            let inertial = current_inertial
+                                .take()
+                                .ok_or_else(|| "URDF inertial state is inconsistent".to_string())?;
+                            let mass = inertial
+                                .mass
+                                .ok_or_else(|| "URDF inertial is missing mass".to_string())?;
+                            let inertia = inertial.inertia.ok_or_else(|| {
+                                "URDF inertial is missing its inertia tensor".to_string()
+                            })?;
+                            if let Some(link) = current_link.as_mut() {
+                                link.inertial = Some(RobotInertial {
+                                    origin: inertial.origin,
+                                    mass,
+                                    inertia,
+                                });
+                            } else {
+                                return Err("URDF inertial is outside a link".to_string());
+                            }
+                        }
+                        "material" if current_global_material.is_some() => {
+                            let (name, color) = current_global_material
+                                .take()
+                                .ok_or_else(|| "URDF material state is inconsistent".to_string())?;
+                            if global_materials.insert(name.clone(), color).is_some() {
+                                return Err(format!(
+                                    "URDF material '{name}' is defined more than once"
+                                ));
+                            }
+                        }
                         "visual" => {
                             return Err("URDF visual is missing a geometry".to_string());
                         }
@@ -629,6 +957,35 @@ pub fn parse_robot_description(content: &str) -> Result<RobotDescription, String
                         }
                         link.collisions.push(collision);
                     }
+                    "inertial" => {
+                        let inertial = current_inertial
+                            .take()
+                            .ok_or_else(|| "URDF inertial state is inconsistent".to_string())?;
+                        let mass = inertial
+                            .mass
+                            .ok_or_else(|| "URDF inertial is missing mass".to_string())?;
+                        let inertia = inertial.inertia.ok_or_else(|| {
+                            "URDF inertial is missing its inertia tensor".to_string()
+                        })?;
+                        let link = current_link
+                            .as_mut()
+                            .ok_or_else(|| "URDF inertial is outside a link".to_string())?;
+                        link.inertial = Some(RobotInertial {
+                            origin: inertial.origin,
+                            mass,
+                            inertia,
+                        });
+                    }
+                    "material" if current_global_material.is_some() => {
+                        let (name, color) = current_global_material
+                            .take()
+                            .ok_or_else(|| "URDF material state is inconsistent".to_string())?;
+                        if global_materials.insert(name.clone(), color).is_some() {
+                            return Err(format!(
+                                "URDF material '{name}' is defined more than once"
+                            ));
+                        }
+                    }
                     "link" => {
                         if links.len() >= MAX_LINKS {
                             return Err(format!("URDF contains more than {MAX_LINKS} links"));
@@ -700,6 +1057,21 @@ pub fn parse_robot_description(content: &str) -> Result<RobotDescription, String
     if !root_seen || !root_closed || !stack.is_empty() {
         return Err("URDF document is incomplete".to_string());
     }
+    for link in &mut links {
+        for visual in &mut link.visuals {
+            if visual.color.is_none() {
+                if let Some(name) = visual.material_name.as_ref() {
+                    match global_materials.get(name) {
+                        Some(Some(color)) => visual.color = Some(*color),
+                        Some(None) => {}
+                        None => {
+                            warnings.push(format!("Visual references undefined material '{name}'"))
+                        }
+                    }
+                }
+            }
+        }
+    }
     let root_link = validate_robot_tree(&links, &joints)?;
     let name = robot_name.ok_or_else(|| "URDF robot name is missing".to_string())?;
     Ok(RobotDescription {
@@ -713,7 +1085,7 @@ pub fn parse_robot_description(content: &str) -> Result<RobotDescription, String
[trailing space]
 #[cfg(test)]
 mod tests {
-    use super::{parse_robot_description, RobotGeometry};
+    use super::{parse_robot_description, validate_mesh_bytes, RobotGeometry};
[trailing space]
     #[test]
     fn parses_primitive_visuals_and_normalizes_joint_axes() {
@@ -780,18 +1152,65 @@ mod tests {
     }
[trailing space]
     #[test]
-    fn reports_mesh_geometry_as_unsupported_instead_of_silently_rendering_it() {
+    fn parses_stl_mesh_references_and_scale_for_safe_resource_resolution() {
         let robot = parse_robot_description(
-            r#"<robot name="test"><link name="base"><visual><geometry><mesh filename="package://robot/meshes/base.stl"/></geometry></visual></link></robot>"#,
+            r#"<robot name="test"><link name="base"><visual><geometry><mesh filename="package://robot/meshes/base.stl" scale="2 1 0.5"/></geometry></visual></link></robot>"#,
         )
         .unwrap();
[trailing space]
-        assert!(matches!(
-            robot.links[0].visuals[0].geometry,
-            RobotGeometry::Unsupported { .. }
-        ));
-        assert_eq!(robot.warnings.len(), 1);
-        assert!(robot.warnings[0].contains("base.stl"));
+        let RobotGeometry::Mesh {
+            filename,
+            scale,
+            format,
+            data,
+        } = &robot.links[0].visuals[0].geometry
+        else {
+            panic!("expected STL mesh geometry");
+        };
+        assert_eq!(filename, "package://robot/meshes/base.stl");
+        assert_eq!(*scale, [2.0, 1.0, 0.5]);
+        assert_eq!(format, "stl");
+        assert!(data.is_none());
+    }
+
+    #[test]
+    fn resolves_named_materials_and_preserves_inertial_frames_and_tensors() {
+        let robot = parse_robot_description(
+            r#"<robot name="test">
+                <material name="blue"><color rgba="0.1 0.2 0.8 1"/></material>
+                <link name="base">
+                    <inertial><origin xyz="0 0 0.2" rpy="0.1 0.2 0.3"/>
+                        <mass value="2"/><inertia ixx="1" ixy="0" ixz="0" iyy="2" iyz="0" izz="3"/>
+                    </inertial>
+                    <visual><geometry><box size="1 1 1"/></geometry><material name="blue"/></visual>
+                </link>
+            </robot>"#,
+        )
+        .unwrap();
+        assert_eq!(robot.links[0].visuals[0].color, Some([0.1, 0.2, 0.8, 1.0]));
+        let inertial = robot.links[0].inertial.as_ref().unwrap();
+        assert_eq!(inertial.mass, 2.0);
+        assert_eq!(inertial.origin.xyz, [0.0, 0.0, 0.2]);
+        assert_eq!(inertial.inertia, [1.0, 0.0, 0.0, 2.0, 0.0, 3.0]);
+    }
+
+    #[test]
+    fn rejects_invalid_inertia_and_external_dae_textures_and_bounds_meshes() {
+        let invalid_inertia = r#"<robot name="test"><link name="base"><inertial><mass value="1"/><inertia ixx="1" ixy="2" ixz="0" iyy="1" iyz="0" izz="1"/></inertial></link></robot>"#;
+        assert!(parse_robot_description(invalid_inertia)
+            .unwrap_err()
+            .contains("positive semidefinite"));
+        let dae = br#"<COLLADA><library_images><image id="texture"><init_from>https://example.test/texture.png</init_from></image></library_images><library_geometries><triangles count="1"/></library_geometries></COLLADA>"#;
+        assert!(validate_mesh_bytes("dae", dae)
+            .unwrap_err()
+            .contains("external image"));
+        let external_url =
+            br#"<COLLADA><instance_geometry url="https://example.test/geometry.dae"/><triangles count="1"/></COLLADA>"#;
+        assert!(validate_mesh_bytes("dae", external_url)
+            .unwrap_err()
+            .contains("external resources"));
+        assert!(validate_mesh_bytes("stl", b"invalid").is_err());
+        assert!(validate_mesh_bytes("stl", &vec![0; 8 * 1024 * 1024 + 1]).is_err());
     }
[trailing space]
     #[test]
diff --git a/src/App.tsx b/src/App.tsx
index d9e8042..ce255a3 100644
--- a/src/App.tsx
+++ b/src/App.tsx
@@ -7,6 +7,8 @@ import TFTree from "./components/TFTree";
 import RosGraphExplorer from "./components/RosGraphExplorer";
 import type {
   RosGraphSnapshot,
+  SafeActionGoal,
+  SafeServiceInvocation,
   TopicInspection,
 } from "./components/RosGraphExplorer";
 import Settings from "./components/Settings";
@@ -58,10 +60,12 @@ interface RecentProject {
 interface RosPackageInfo {
   name: string;
   path: string;
+  source: "workspace" | "underlay";
   package_type: string;
   version: string | null;
   description: string | null;
   dependencies: string[];
+  dependency_sources: Record<string, "workspace" | "underlay" | "missing">;
   executables: string[];
   launch_files: string[];
   config_files: string[];
@@ -296,6 +300,11 @@ function App() {
   const [topicInspectionError, setTopicInspectionError] = useState<
     string | null
   >(null);
+  const [safeServiceInvocation, setSafeServiceInvocation] =
+    useState<SafeServiceInvocation | null>(null);
+  const [safeActionGoal, setSafeActionGoal] = useState<SafeActionGoal | null>(
+    null,
+  );
   const [rosPackagesLoading, setRosPackagesLoading] = useState(false);
   const [rosPackagesError, setRosPackagesError] = useState<string | null>(null);
   const [rosPackageFilter, setRosPackageFilter] = useState("");
@@ -378,6 +387,188 @@ function App() {
       setPendingInspectionTopic(null);
     }
   };
+
+  const invokeSafeRosService = async (
+    serviceName: string,
+    serviceType: string,
+    requestJson: string,
+  ) => {
+    let requestValue: unknown;
+    try {
+      if (new TextEncoder().encode(requestJson).length > 16 * 1024) {
+        throw new Error("Request exceeds the 16 KiB limit.");
+      }
+      requestValue = JSON.parse(requestJson);
+      if (
+        requestValue === null ||
+        typeof requestValue !== "object" ||
+        Array.isArray(requestValue)
+      ) {
+        throw new Error("Request must be a JSON object.");
+      }
+    } catch (error) {
+      setSafeServiceInvocation({
+        serviceName,
+        serviceType,
+        responseJson: "",
+        error: String(error),
+        truncated: false,
+        pending: false,
+      });
+      return;
+    }
+
+    setSafeServiceInvocation({
+      serviceName,
+      serviceType,
+      responseJson: "",
+      error: "",
+      truncated: false,
+      pending: true,
+    });
+    try {
+      const response = await invoke<{
+        responseJson: string;
+        truncated: boolean;
+      }>("invoke_safe_ros_service", {
+        serviceName,
+        serviceType,
+        requestJson,
+        timeoutMs: 3000,
+      });
+      setSafeServiceInvocation({
+        serviceName,
+        serviceType,
+        responseJson: response.responseJson,
+        error: "",
+        truncated: response.truncated,
+        pending: false,
+      });
+    } catch (error) {
+      setSafeServiceInvocation({
+        serviceName,
+        serviceType,
+        responseJson: "",
+        error: String(error),
+        truncated: false,
+        pending: false,
+      });
+    }
+  };
+
+  const sendSafeActionGoal = async (
+    actionName: string,
+    actionType: string,
+    goalJson: string,
+  ) => {
+    let goalValue: unknown;
+    try {
+      if (new TextEncoder().encode(goalJson).length > 16 * 1024) {
+        throw new Error("Goal exceeds the 16 KiB limit.");
+      }
+      goalValue = JSON.parse(goalJson);
+      if (goalValue === null || typeof goalValue !== "object" || Array.isArray(goalValue)) {
+        throw new Error("Goal must be a JSON object.");
+      }
+    } catch (error) {
+      setSafeActionGoal({
+        goalId: "",
+        actionName,
+        actionType,
+        status: "invalid_goal",
+        feedbackJson: "",
+        resultJson: "",
+        error: String(error),
+        terminal: true,
+        truncated: false,
+        cancelPending: false,
+      });
+      return;
+    }
+
+    setSafeActionGoal({
+      goalId: "",
+      actionName,
+      actionType,
+      status: "submitting",
+      feedbackJson: "",
+      resultJson: "",
+      error: "",
+      terminal: false,
+      truncated: false,
+      cancelPending: false,
+    });
+    try {
+      const response = await invoke<{ goalId: string }>(
+        "send_safe_ros_action_goal",
+        {
+          actionName,
+          actionType,
+          goalJson,
+        },
+      );
+      setSafeActionGoal((current) =>
+        current?.actionName === actionName &&
+        current.actionType === actionType &&
+        (!current.goalId || current.goalId === response.goalId)
+          ? {
+              ...current,
+              goalId: response.goalId,
+              status:
+                current.status === "submitting" ? "submitted" : current.status,
+            }
+          : current,
+      );
+    } catch (error) {
+      setSafeActionGoal((current) =>
+        current?.actionName === actionName &&
+        current.actionType === actionType
+          ? {
+              ...current,
+              status: "failed",
+              error: String(error),
+              terminal: true,
+              cancelPending: false,
+            }
+          : current,
+      );
+    }
+  };
+
+  const cancelSafeActionGoal = async (goalId: string) => {
+    setSafeActionGoal((current) =>
+      current?.goalId === goalId
+        ? { ...current, cancelPending: true, error: "" }
+        : current,
+    );
+    try {
+      const response = await invoke<{ requested: boolean }>(
+        "cancel_safe_ros_action_goal",
+        { goalId },
+      );
+      if (!response.requested) {
+        setSafeActionGoal((current) =>
+          current?.goalId === goalId
+            ? {
+                ...current,
+                cancelPending: false,
+                error: "The ROS action server did not accept cancellation.",
+              }
+            : current,
+        );
+      }
+    } catch (error) {
+      setSafeActionGoal((current) =>
+        current?.goalId === goalId
+          ? {
+              ...current,
+              cancelPending: false,
+              error: String(error),
+            }
+          : current,
+      );
+    }
+  };
   const rosWorkspaceReady = useRef(false);
   const authRecoveryStarted = useRef(false);
   const [setupOutput, setSetupOutput] = useState<string[]>([]); // Environment setup logs
@@ -407,6 +598,7 @@ function App() {
       [
         pkg.name,
         pkg.description ?? "",
+        pkg.source,
         pkg.package_type,
         ...pkg.dependencies,
       ]
@@ -1089,6 +1281,30 @@ function App() {
         if (!update.error) setTopicInspectionError(null);
       },
     );
+    const unlistenActionGoal = listen<Omit<SafeActionGoal, "cancelPending">>(
+      "safe-action-goal-update",
+      (event) => {
+        const update = event.payload;
+        setSafeActionGoal((current) => {
+          if (!current) return current;
+          if (current.goalId && current.goalId !== update.goalId) return current;
+          if (
+            current.actionName !== update.actionName ||
+            current.actionType !== update.actionType
+          ) {
+            return current;
+          }
+          return {
+            ...current,
+            ...update,
+            feedbackJson: update.feedbackJson || current.feedbackJson,
+            resultJson: update.resultJson || current.resultJson,
+            error: update.error || current.error,
+            cancelPending: update.status === "cancel_requested",
+          };
+        });
+      },
+    );
[trailing space]
     const unlistenBuildOutput = listen<LogEvent>("build-output", (event) => {
       const { kind, text } = event.payload;
@@ -1140,6 +1356,7 @@ function App() {
       unlistenEventRecovery.then((f) => f());
       unlistenRosGraph.then((f) => f());
       unlistenTopicInspection.then((f) => f());
+      unlistenActionGoal.then((f) => f());
       unlistenBuildOutput.then((f) => f());
       unlistenBuildError.then((f) => f());
       unlistenBuildFinished.then((f) => f());
@@ -1659,6 +1876,12 @@ function App() {
                     !rosPackagesError &&
                     rosPackages.length > 0 && (
                       <>
+                        <div style={{ opacity: 0.65, marginBottom: 8 }}>
+                          {rosPackages.filter((pkg) => pkg.source === "workspace").length}{" "}
+                          workspace ·{" "}
+                          {rosPackages.filter((pkg) => pkg.source === "underlay").length}{" "}
+                          underlay packages
+                        </div>
                         <input
                           type="search"
                           aria-label="Filter ROS packages"
@@ -1709,6 +1932,22 @@ function App() {
                             >
                               {pkg.name}
                             </div>
+                            <span
+                              style={{
+                                display: "inline-block",
+                                background:
+                                  pkg.source === "workspace"
+                                    ? "rgba(124,217,146,0.12)"
+                                    : "rgba(79,195,247,0.12)",
+                                borderRadius: 999,
+                                padding: "2px 6px",
+                                marginBottom: 5,
+                              }}
+                            >
+                              {pkg.source === "workspace"
+                                ? "Selected workspace"
+                                : "ROS 2 underlay"}
+                            </span>
                             {pkg.description && (
                               <div
                                 style={{
@@ -1727,7 +1966,7 @@ function App() {
                                 marginBottom: 4,
                               }}
                             >
-                              {pkg.path}
+                              {pkg.source === "workspace" ? pkg.path : "Installed ROS prefix"}
                             </div>
                             <div
                               style={{
@@ -1787,15 +2026,27 @@ function App() {
                                   {pkg.dependencies.slice(0, 8).map((dep) => (
                                     <span
                                       key={`${pkg.name}-${dep}`}
+                                      title={`Resolved from ${pkg.dependency_sources[dep] ?? "missing"}`}
                                       style={{
-                                        background: "rgba(255,255,255,0.04)",
+                                        background:
+                                          pkg.dependency_sources[dep] === "missing"
+                                            ? "rgba(224,96,96,0.18)"
+                                            : "rgba(255,255,255,0.04)",
                                         borderRadius: 999,
                                         padding: "2px 6px",
                                       }}
                                     >
                                       {dep}
+                                      {pkg.dependency_sources[dep] === "missing"
+                                        ? " (missing)"
+                                        : ""}
                                     </span>
                                   ))}
+                                  {pkg.dependencies.length > 8 && (
+                                    <span style={{ opacity: 0.6 }}>
+                                      +{pkg.dependencies.length - 8} more
+                                    </span>
+                                  )}
                                 </div>
                               </div>
                             )}
@@ -1827,17 +2078,19 @@ function App() {
                                 )}
                               </div>
                             )}
-                            <button
-                              className="ide-btn"
-                              type="button"
-                              aria-expanded={expandedRosPackage === pkg.path}
-                              onClick={() => void inspectRosPackage(pkg.path)}
-                              style={{ marginTop: 8 }}
-                            >
-                              {expandedRosPackage === pkg.path
-                                ? "Hide contents"
-                                : "Inspect contents"}
-                            </button>
+                            {pkg.source === "workspace" && (
+                              <button
+                                className="ide-btn"
+                                type="button"
+                                aria-expanded={expandedRosPackage === pkg.path}
+                                onClick={() => void inspectRosPackage(pkg.path)}
+                                style={{ marginTop: 8 }}
+                              >
+                                {expandedRosPackage === pkg.path
+                                  ? "Hide contents"
+                                  : "Inspect contents"}
+                              </button>
+                            )}
                             {expandedRosPackage === pkg.path &&
                               rosPackageContentsError && (
                                 <div
@@ -1885,12 +2138,27 @@ function App() {
                     inspection={topicInspection}
                     pendingTopic={pendingInspectionTopic}
                     inspectionError={topicInspectionError}
+                    safeServiceInvocation={safeServiceInvocation}
+                    actionGoal={safeActionGoal}
                     onStartInspection={(topicName) =>
                       void startTopicInspection(topicName)
                     }
                     onStopInspection={(topicName) =>
                       void stopTopicInspection(topicName)
                     }
+                    onInvokeSafeService={(serviceName, serviceType, requestJson) =>
+                      void invokeSafeRosService(
+                        serviceName,
+                        serviceType,
+                        requestJson,
+                      )
+                    }
+                    onSendSafeActionGoal={(actionName, actionType, goalJson) =>
+                      void sendSafeActionGoal(actionName, actionType, goalJson)
+                    }
+                    onCancelSafeActionGoal={(goalId) =>
+                      void cancelSafeActionGoal(goalId)
+                    }
                   />
                 </div>
               )}
@@ -2532,10 +2800,13 @@ function App() {
                   <div style={{ flex: 1 }}>
                     <RobotView
                       descriptionContent={
-                        activeFile?.name.toLowerCase().endsWith(".urdf")
+                        activeFile &&
+                        /\.(urdf|xacro)$/i.test(activeFile.name)
                           ? activeFile.content
                           : null
                       }
+                      descriptionPath={activeFile?.path ?? null}
+                      workspaceRoot={currentPath}
                     />
                   </div>
                   <div style={{ width: 200, borderLeft: "1px solid #333" }}>
diff --git a/src/components/RobotView.tsx b/src/components/RobotView.tsx
index 5b40546..26ae08f 100644
--- a/src/components/RobotView.tsx
+++ b/src/components/RobotView.tsx
@@ -67,13 +67,29 @@ function buildPlaceholderRobot(node: string): RobotInstance {
     group,
     updateJoints: () => undefined,
     setCollisionVisible: () => undefined,
+    setInertialVisible: () => undefined,
+    dispose: () => {
+      group.traverse((child) => {
+        if (child instanceof THREE.Mesh) {
+          child.geometry.dispose();
+          const materials = Array.isArray(child.material)
+            ? child.material
+            : [child.material];
+          materials.forEach((material) => material.dispose());
+        }
+      });
+    },
   };
 }
[trailing space]
 export default function RobotView({
   descriptionContent,
+  descriptionPath,
+  workspaceRoot,
 }: {
   descriptionContent: string | null;
+  descriptionPath: string | null;
+  workspaceRoot: string | null;
 }) {
   const containerRef = useRef<HTMLDivElement>(null);
   const robotsRef = useRef<Record<string, RobotInstance>>({});
@@ -81,10 +97,12 @@ export default function RobotView({
     useState<RobotDescription | null>(null);
   const [descriptionError, setDescriptionError] = useState<string | null>(null);
   const [showCollisions, setShowCollisions] = useState(false);
+  const [showInertial, setShowInertial] = useState(false);
   const collisionVisibleRef = useRef(false);
+  const inertialVisibleRef = useRef(false);
[trailing space]
   useEffect(() => {
-    if (!descriptionContent) {
+    if (!descriptionContent || !descriptionPath || !workspaceRoot) {
       setRobotDescription(null);
       setDescriptionError(null);
       return;
@@ -94,6 +112,8 @@ export default function RobotView({
     setRobotDescription(null);
     setDescriptionError(null);
     invoke<RobotDescription>("parse_robot_description", {
+      path: descriptionPath,
+      workspaceRoot,
       content: descriptionContent,
     })
       .then((description) => {
@@ -106,7 +126,7 @@ export default function RobotView({
     return () => {
       active = false;
     };
-  }, [descriptionContent]);
+  }, [descriptionContent, descriptionPath, workspaceRoot]);
[trailing space]
   useEffect(() => {
     collisionVisibleRef.current = showCollisions;
@@ -115,6 +135,13 @@ export default function RobotView({
     );
   }, [showCollisions]);
[trailing space]
+  useEffect(() => {
+    inertialVisibleRef.current = showInertial;
+    Object.values(robotsRef.current).forEach((robot) =>
+      robot.setInertialVisible(showInertial),
+    );
+  }, [showInertial]);
+
   useEffect(() => {
     if (!containerRef.current) return;
[trailing space]
@@ -191,6 +218,7 @@ export default function RobotView({
     if (robotDescription) {
       const preview = buildRobotInstance(robotDescription);
       preview.setCollisionVisible(collisionVisibleRef.current);
+      preview.setInertialVisible(inertialVisibleRef.current);
       scene.add(preview.group);
       robotsRef.current["__description_preview__"] = preview;
     }
@@ -208,13 +236,15 @@ export default function RobotView({
     const unlistenClear = listen("clear-robots", () => {
       if (!isActive) return;
[trailing space]
-      Object.values(robotsRef.current).forEach((robot) =>
-        scene.remove(robot.group),
-      );
+      Object.values(robotsRef.current).forEach((robot) => {
+        scene.remove(robot.group);
+        robot.dispose();
+      });
       robotsRef.current = {};
       if (robotDescription) {
         const preview = buildRobotInstance(robotDescription);
         preview.setCollisionVisible(collisionVisibleRef.current);
+        preview.setInertialVisible(inertialVisibleRef.current);
         scene.add(preview.group);
         robotsRef.current["__description_preview__"] = preview;
       }
@@ -236,6 +266,7 @@ export default function RobotView({
               ? buildRobotInstance(robotDescription)
               : buildPlaceholderRobot(node));
           robot.setCollisionVisible(collisionVisibleRef.current);
+          robot.setInertialVisible(inertialVisibleRef.current);
           if (robotsRef.current["__description_preview__"]) {
             delete robotsRef.current["__description_preview__"];
           } else {
@@ -276,16 +307,7 @@ export default function RobotView({
[trailing space]
       Object.values(robotsRef.current).forEach((robot) => {
         scene.remove(robot.group);
-        robot.group.traverse((child) => {
-          if (child instanceof THREE.Mesh) {
-            child.geometry.dispose();
-            if (Array.isArray(child.material)) {
-              child.material.forEach((m) => m.dispose());
-            } else {
-              child.material.dispose();
-            }
-          }
-        });
+        robot.dispose();
       });
[trailing space]
       controls.dispose();
@@ -311,19 +333,34 @@ export default function RobotView({
       }}
     >
       {robotDescription && (
-        <button
-          type="button"
-          aria-pressed={showCollisions}
-          onClick={() => setShowCollisions((visible) => !visible)}
+        <div
           style={{
             position: "absolute",
             zIndex: 1,
             left: 8,
             bottom: 8,
+            display: "flex",
+            gap: 8,
           }}
         >
-          {showCollisions ? "Hide" : "Show"} collision geometry
-        </button>
+          <button
+            type="button"
+            aria-pressed={showCollisions}
+            onClick={() => setShowCollisions((visible) => !visible)}
+          >
+            {showCollisions ? "Hide" : "Show"} collision geometry
+          </button>
+          {robotDescription.links.some((link) => link.inertial) && (
+            <button
+              type="button"
+              aria-pressed={showInertial}
+              title="Shows each inertial origin and a scalar radius-of-gyration marker."
+              onClick={() => setShowInertial((visible) => !visible)}
+            >
+              {showInertial ? "Hide" : "Show"} inertial frames
+            </button>
+          )}
+        </div>
       )}
       {(descriptionError || robotDescription?.warnings.length) && (
         <div
@@ -360,7 +397,8 @@ export default function RobotView({
           Validating robot description...
         </div>
       )}
-      {!descriptionContent && !descriptionError && (
+      {(!descriptionContent || !descriptionPath || !workspaceRoot) &&
+        !descriptionError && (
         <div
           role="status"
           style={{
@@ -373,7 +411,7 @@ export default function RobotView({
             fontSize: 12,
           }}
         >
-          Open a URDF file to preview its geometry; a placeholder robot is shown.
+          Open a URDF or Xacro file inside the selected workspace to preview it.
         </div>
       )}
     </div>
diff --git a/src/components/RosGraphExplorer.tsx b/src/components/RosGraphExplorer.tsx
index f2370ca..e9ccc45 100644
--- a/src/components/RosGraphExplorer.tsx
+++ b/src/components/RosGraphExplorer.tsx
@@ -1,3 +1,4 @@
+import { useState } from "react";
 import { groupRosGraphByNode, rosNodeId } from "../rosGraph";
 import type {
   RosGraphEndpoint,
@@ -23,6 +24,28 @@ export interface TopicInspection {
   error: string;
 }
[trailing space]
+export interface SafeServiceInvocation {
+  serviceName: string;
+  serviceType: string;
+  responseJson: string;
+  error: string;
+  truncated: boolean;
+  pending: boolean;
+}
+
+export interface SafeActionGoal {
+  goalId: string;
+  actionName: string;
+  actionType: string;
+  status: string;
+  feedbackJson: string;
+  resultJson: string;
+  error: string;
+  terminal: boolean;
+  truncated: boolean;
+  cancelPending: boolean;
+}
+
 function nodePath(nodeName: string, nodeNamespace: string) {
   return `${nodeNamespace === "/" ? "" : nodeNamespace}/${nodeName}`;
 }
@@ -93,22 +116,265 @@ function GraphCard({
   );
 }
[trailing space]
+function defaultServiceRequest(serviceType: string) {
+  if (
+    serviceType === "rcl_interfaces/srv/GetParameters" ||
+    serviceType === "rcl_interfaces/srv/GetParameterTypes" ||
+    serviceType === "rcl_interfaces/srv/DescribeParameters"
+  ) {
+    return '{"names":[]}';
+  }
+  if (serviceType === "rcl_interfaces/srv/ListParameters") {
+    return '{"prefixes":[],"depth":0}';
+  }
+  return "{}";
+}
+
+function SafeServiceForm({
+  serviceName,
+  serviceType,
+  invocation,
+  onInvoke,
+}: {
+  serviceName: string;
+  serviceType: string;
+  invocation: SafeServiceInvocation | null;
+  onInvoke: (serviceName: string, serviceType: string, requestJson: string) => void;
+}) {
+  const [requestJson, setRequestJson] = useState(() =>
+    defaultServiceRequest(serviceType),
+  );
+  const selected =
+    invocation?.serviceName === serviceName &&
+    invocation.serviceType === serviceType;
+
+  return (
+    <form
+      aria-label={`Read-only request for ${serviceName}`}
+      onSubmit={(event) => {
+        event.preventDefault();
+        onInvoke(serviceName, serviceType, requestJson);
+      }}
+      style={{ display: "grid", gap: 5, marginTop: 6 }}
+    >
+      <label style={{ opacity: 0.75 }} htmlFor={`request-${serviceName}`}>
+        Read-only request · {serviceType}
+      </label>
+      <textarea
+        id={`request-${serviceName}`}
+        aria-label={`JSON request for ${serviceName}`}
+        value={requestJson}
+        onChange={(event) => setRequestJson(event.target.value)}
+        rows={3}
+        spellCheck={false}
+        style={{
+          boxSizing: "border-box",
+          width: "100%",
+          resize: "vertical",
+          color: "#ddd",
+          background: "#1e1e1e",
+          border: "1px solid #444",
+          borderRadius: 4,
+          padding: 6,
+          fontFamily: "monospace",
+        }}
+      />
+      <button
+        type="submit"
+        disabled={invocation?.pending ?? false}
+        style={{ justifySelf: "start" }}
+      >
+        {selected && invocation.pending ? "Calling…" : "Call read-only service"}
+      </button>
+      {selected && invocation.error && (
+        <div role="alert" style={{ color: "#e99", overflowWrap: "anywhere" }}>
+          {invocation.error}
+        </div>
+      )}
+      {selected && invocation.truncated && (
+        <div role="status" style={{ color: "#e0b060" }}>
+          Response was truncated to fit inspection limits.
+        </div>
+      )}
+      {selected && invocation.responseJson && (
+        <pre
+          aria-label={`Response from ${serviceName}`}
+          style={{
+            maxHeight: 220,
+            overflow: "auto",
+            whiteSpace: "pre-wrap",
+            overflowWrap: "anywhere",
+            margin: 0,
+            padding: 6,
+            background: "rgba(0,0,0,0.2)",
+          }}
+        >
+          {invocation.responseJson}
+        </pre>
+      )}
+    </form>
+  );
+}
+
+function SafeActionForm({
+  actionName,
+  actionType,
+  goal,
+  disabled,
+  onSend,
+  onCancel,
+}: {
+  actionName: string;
+  actionType: string;
+  goal: SafeActionGoal | null;
+  disabled: boolean;
+  onSend: (
+    actionName: string,
+    actionType: string,
+    goalJson: string,
+  ) => void;
+  onCancel: (goalId: string) => void;
+}) {
+  const [goalJson, setGoalJson] = useState('{"order":5}');
+  const selectedGoal =
+    goal?.actionName === actionName && goal.actionType === actionType
+      ? goal
+      : null;
+  return (
+    <form
+      aria-label={`Safe action goal for ${actionName}`}
+      onSubmit={(event) => {
+        event.preventDefault();
+        onSend(actionName, actionType, goalJson);
+      }}
+      style={{ display: "grid", gap: 5, marginTop: 6 }}
+    >
+      <label style={{ opacity: 0.75 }} htmlFor={`goal-${actionName}`}>
+        Allowlisted non-motion action · {actionType}
+      </label>
+      <textarea
+        id={`goal-${actionName}`}
+        aria-label={`JSON goal for ${actionName}`}
+        value={goalJson}
+        onChange={(event) => setGoalJson(event.target.value)}
+        rows={3}
+        spellCheck={false}
+        style={{
+          boxSizing: "border-box",
+          width: "100%",
+          resize: "vertical",
+          color: "#ddd",
+          background: "#1e1e1e",
+          border: "1px solid #444",
+          borderRadius: 4,
+          padding: 6,
+          fontFamily: "monospace",
+        }}
+      />
+      <button
+        type="submit"
+        disabled={disabled || Boolean(goal && !goal.terminal)}
+        style={{ justifySelf: "start" }}
+      >
+        {selectedGoal?.status === "submitting"
+          ? "Submitting…"
+          : "Send safe action goal"}
+      </button>
+      {selectedGoal && (
+        <div role="status" aria-live="polite">
+          Status: {selectedGoal.status}
+        </div>
+      )}
+      {selectedGoal?.error && (
+        <div role="alert" style={{ color: "#e99", overflowWrap: "anywhere" }}>
+          {selectedGoal.error}
+        </div>
+      )}
+      {selectedGoal?.feedbackJson && (
+        <pre
+          aria-label={`Feedback from ${actionName}`}
+          style={{
+            maxHeight: 180,
+            overflow: "auto",
+            whiteSpace: "pre-wrap",
+            overflowWrap: "anywhere",
+            margin: 0,
+            padding: 6,
+            background: "rgba(0,0,0,0.2)",
+          }}
+        >
+          Feedback: {selectedGoal.feedbackJson}
+        </pre>
+      )}
+      {selectedGoal?.resultJson && (
+        <pre
+          aria-label={`Result from ${actionName}`}
+          style={{
+            maxHeight: 180,
+            overflow: "auto",
+            whiteSpace: "pre-wrap",
+            overflowWrap: "anywhere",
+            margin: 0,
+            padding: 6,
+            background: "rgba(0,0,0,0.2)",
+          }}
+        >
+          Result: {selectedGoal.resultJson}
+        </pre>
+      )}
+      {selectedGoal?.truncated && (
+        <div role="status" style={{ color: "#e0b060" }}>
+          Action output was truncated to fit inspection limits.
+        </div>
+      )}
+      {selectedGoal?.goalId && !selectedGoal.terminal && (
+        <button
+          type="button"
+          disabled={selectedGoal.cancelPending}
+          onClick={() => onCancel(selectedGoal.goalId)}
+          style={{ justifySelf: "start" }}
+        >
+          {selectedGoal.cancelPending ? "Waiting for cancellation…" : "Cancel goal"}
+        </button>
+      )}
+    </form>
+  );
+}
+
 export default function RosGraphExplorer({
   graph,
   connected,
   inspection,
   pendingTopic,
   inspectionError,
+  safeServiceInvocation,
+  actionGoal,
   onStartInspection,
   onStopInspection,
+  onInvokeSafeService,
+  onSendSafeActionGoal,
+  onCancelSafeActionGoal,
 }: {
   graph: RosGraphSnapshot | null;
   connected: boolean;
   inspection: TopicInspection | null;
   pendingTopic: string | null;
   inspectionError: string | null;
+  safeServiceInvocation: SafeServiceInvocation | null;
+  actionGoal: SafeActionGoal | null;
   onStartInspection: (topicName: string) => void;
   onStopInspection: (topicName: string) => void;
+  onInvokeSafeService: (
+    serviceName: string,
+    serviceType: string,
+    requestJson: string,
+  ) => void;
+  onSendSafeActionGoal: (
+    actionName: string,
+    actionType: string,
+    goalJson: string,
+  ) => void;
+  onCancelSafeActionGoal: (goalId: string) => void;
 }) {
   if (!graph) {
     return (
@@ -206,7 +472,8 @@ export default function RosGraphExplorer({
       <div style={{ fontWeight: 600, marginBottom: 4 }}>Live ROS Graph</div>
       <div style={{ opacity: 0.65, fontSize: 11, marginBottom: 6 }}>
         Read-only discovery with bounded live topic inspection. This panel does
-        not call services or send action goals.
+        not invoke arbitrary services or actions. Service calls and action goals
+        are restricted to backend allowlists.
       </div>
       {graph.truncated && (
         <div role="status" style={{ color: "#e0b060", marginBottom: 6 }}>
@@ -329,7 +596,20 @@ export default function RosGraphExplorer({
             key={service.name}
             name={service.name}
             types={service.types}
-            details={endpointsFor(service.serverNodes, service.clientNodes)}
+            details={
+              <>
+                {endpointsFor(service.serverNodes, service.clientNodes)}
+                {service.safeTypes.map((serviceType) => (
+                  <SafeServiceForm
+                    key={`${service.name}/${serviceType}`}
+                    serviceName={service.name}
+                    serviceType={serviceType}
+                    invocation={safeServiceInvocation}
+                    onInvoke={onInvokeSafeService}
+                  />
+                ))}
+              </>
+            }
           />
         ))}
       </GraphSection>
@@ -339,7 +619,22 @@ export default function RosGraphExplorer({
             key={action.name}
             name={action.name}
             types={action.types}
-            details={endpointsFor(action.serverNodes, action.clientNodes)}
+            details={
+              <>
+                {endpointsFor(action.serverNodes, action.clientNodes)}
+                {action.safeTypes.map((actionType) => (
+                  <SafeActionForm
+                    key={`${action.name}/${actionType}`}
+                    actionName={action.name}
+                    actionType={actionType}
+                    goal={actionGoal}
+                    disabled={Boolean(actionGoal && !actionGoal.terminal)}
+                    onSend={onSendSafeActionGoal}
+                    onCancel={onCancelSafeActionGoal}
+                  />
+                ))}
+              </>
+            }
           />
         ))}
       </GraphSection>
diff --git a/src/components/TFTree.tsx b/src/components/TFTree.tsx
index 8ef1634..f448adb 100644
--- a/src/components/TFTree.tsx
+++ b/src/components/TFTree.tsx
@@ -3,7 +3,8 @@ import { listen } from "@tauri-apps/api/event";
 import { AlertTriangle, Sparkles } from "lucide-react";
 import {
   buildTfGraph,
-  tfEdgeKey,
+  composeTfTransform,
+  mergeTfTransform,
   type TfTransform,
   type TfTreeNode,
 } from "../tfGraph";
@@ -134,8 +135,10 @@ export default function TFTree({ connected, onExplain }: Props) {
         ...event.payload,
         receivedAtMs: event.payload.received_at_ms,
       };
-      const key = tfEdgeKey(transform.parent_frame, transform.child_frame);
-      transformsRef.current[key] = transform;
+      transformsRef.current = mergeTfTransform(
+        transformsRef.current,
+        transform,
+      );
       if (updateFrame === null) {
         updateFrame = window.requestAnimationFrame(() => {
           updateFrame = null;
@@ -162,6 +165,9 @@ export default function TFTree({ connected, onExplain }: Props) {
   const selected =
     graph.edges.find((edge) => edge.key === selectedEdgeKey) ?? graph.edges[0];
   const euler = selected ? quaternionEulerDegrees(selected) : null;
+  const composed = selected
+    ? composeTfTransform(graph.edges, selected.child_frame)
+    : null;
[trailing space]
   return (
     <div
@@ -257,6 +263,25 @@ export default function TFTree({ connected, onExplain }: Props) {
                 ).padStart(3, "0")} s`
               : "zero or unavailable"}
           </div>
+          {composed?.ok ? (
+            <div
+              aria-label={`Composed transform from ${composed.pose.rootFrame} to ${composed.pose.frame}`}
+              style={{ marginBottom: 6 }}
+            >
+              Pose in {composed.pose.rootFrame}: ({composed.pose.x.toFixed(4)},{" "}
+              {composed.pose.y.toFixed(4)}, {composed.pose.z.toFixed(4)}) m
+              {composed.pose.stale ? " · includes stale data" : ""}
+              <div>
+                Composed quaternion: ({composed.pose.qx.toFixed(4)},{" "}
+                {composed.pose.qy.toFixed(4)}, {composed.pose.qz.toFixed(4)},{" "}
+                {composed.pose.qw.toFixed(4)})
+              </div>
+            </div>
+          ) : composed ? (
+            <div role="alert" style={{ color: "#e0b060", marginBottom: 6 }}>
+              Composed pose unavailable: {composed.reason}
+            </div>
+          ) : null}
           <div>
             Translation (m): ({selected.x.toFixed(4)}, {selected.y.toFixed(4)},{" "}
             {selected.z.toFixed(4)})
diff --git a/src/robotDescription.ts b/src/robotDescription.ts
index 4a7386e..c6f0976 100644
--- a/src/robotDescription.ts
+++ b/src/robotDescription.ts
@@ -1,4 +1,6 @@
 import * as THREE from "three";
+import { ColladaLoader } from "three/examples/jsm/loaders/ColladaLoader.js";
+import { STLLoader } from "three/examples/jsm/loaders/STLLoader.js";
[trailing space]
 export interface RobotOrigin {
   xyz: [number, number, number];
@@ -9,18 +11,33 @@ export type RobotGeometry =
   | { kind: "box"; size: [number, number, number] }
   | { kind: "cylinder"; radius: number; length: number }
   | { kind: "sphere"; radius: number }
+  | {
+      kind: "mesh";
+      filename: string;
+      scale: [number, number, number];
+      format: "stl" | "dae";
+      data: string | null;
+    }
   | { kind: "unsupported"; description: string };
[trailing space]
 export interface RobotVisual {
   origin: RobotOrigin;
   geometry: RobotGeometry;
   color: [number, number, number, number] | null;
+  materialName?: string | null;
+}
+
+export interface RobotInertial {
+  origin: RobotOrigin;
+  mass: number;
+  inertia: [number, number, number, number, number, number];
 }
[trailing space]
 export interface RobotLink {
   name: string;
   visuals: RobotVisual[];
   collisions: RobotVisual[];
+  inertial?: RobotInertial | null;
 }
[trailing space]
 export interface RobotJointLimit {
@@ -59,14 +76,39 @@ export interface RobotInstance {
   group: THREE.Group;
   updateJoints: (positions: ReadonlyMap<string, number>) => void;
   setCollisionVisible: (visible: boolean) => void;
+  setInertialVisible: (visible: boolean) => void;
+  dispose: () => void;
+}
+
+function bytesFromBase64(data: string): ArrayBuffer {
+  const binary = atob(data);
+  const bytes = new Uint8Array(binary.length);
+  for (let index = 0; index < binary.length; index += 1) {
+    bytes[index] = binary.charCodeAt(index);
+  }
+  return bytes.buffer;
 }
[trailing space]
 function createVisual(visual: RobotVisual, collision = false) {
   const { geometry, color } = visual;
+  let contents: THREE.Object3D;
   let shape: THREE.BufferGeometry;
   switch (geometry.kind) {
     case "box":
       shape = new THREE.BoxGeometry(...geometry.size);
+      contents = new THREE.Mesh(
+        shape,
+        new THREE.MeshStandardMaterial({
+          color: collision
+            ? 0xe34b4b
+            : color
+              ? new THREE.Color(color[0], color[1], color[2])
+              : 0x80aaff,
+          opacity: collision ? Math.min(color?.[3] ?? 0.35, 0.35) : (color?.[3] ?? 1),
+          transparent: collision || (color?.[3] ?? 1) < 1,
+          wireframe: collision,
+        }),
+      );
       break;
     case "cylinder":
       shape = new THREE.CylinderGeometry(
@@ -75,36 +117,109 @@ function createVisual(visual: RobotVisual, collision = false) {
         geometry.length,
         24,
       );
+      contents = new THREE.Mesh(
+        shape,
+        new THREE.MeshStandardMaterial({
+          color: collision
+            ? 0xe34b4b
+            : color
+              ? new THREE.Color(color[0], color[1], color[2])
+              : 0x80aaff,
+          opacity: collision ? Math.min(color?.[3] ?? 0.35, 0.35) : (color?.[3] ?? 1),
+          transparent: collision || (color?.[3] ?? 1) < 1,
+          wireframe: collision,
+        }),
+      );
+      (contents as THREE.Mesh).rotation.x = Math.PI / 2;
       break;
     case "sphere":
       shape = new THREE.SphereGeometry(geometry.radius, 24, 16);
+      contents = new THREE.Mesh(
+        shape,
+        new THREE.MeshStandardMaterial({
+          color: collision
+            ? 0xe34b4b
+            : color
+              ? new THREE.Color(color[0], color[1], color[2])
+              : 0x80aaff,
+          opacity: collision ? Math.min(color?.[3] ?? 0.35, 0.35) : (color?.[3] ?? 1),
+          transparent: collision || (color?.[3] ?? 1) < 1,
+          wireframe: collision,
+        }),
+      );
+      break;
+    case "mesh": {
+      if (!geometry.data) {
+        throw new Error(`Mesh data for '${geometry.filename}' was not loaded`);
+      }
+      const buffer = bytesFromBase64(geometry.data);
+      if (geometry.format === "stl") {
+        const meshGeometry = new STLLoader().parse(buffer);
+        const material = new THREE.MeshStandardMaterial({
+          color: collision
+            ? 0xe34b4b
+            : color
+              ? new THREE.Color(color[0], color[1], color[2])
+              : 0x80aaff,
+          opacity: collision ? Math.min(color?.[3] ?? 0.35, 0.35) : (color?.[3] ?? 1),
+          transparent: collision || (color?.[3] ?? 1) < 1,
+          wireframe: collision,
+        });
+        contents = new THREE.Mesh(meshGeometry, material);
+      } else {
+        const daeContent = new TextDecoder().decode(buffer);
+        const dae = new ColladaLoader().parse(daeContent, "");
+        if (!dae) {
+          throw new Error(`Could not parse DAE mesh '${geometry.filename}'`);
+        }
+        contents = dae.scene;
+        const upAxis =
+          /<up_axis>\s*([XYZ]_UP)\s*<\/up_axis>/i.exec(daeContent)?.[1]?.toUpperCase() ??
+          "Y_UP";
+        contents.rotation.set(0, 0, 0);
+        if (upAxis === "Y_UP") contents.rotation.x = Math.PI / 2;
+        if (upAxis === "X_UP") contents.rotation.y = -Math.PI / 2;
+        if (color || collision) {
+          contents.traverse((child) => {
+            if (!(child instanceof THREE.Mesh)) return;
+            const existing = Array.isArray(child.material)
+              ? child.material
+              : [child.material];
+            for (const material of existing) {
+              if ("color" in material && material.color instanceof THREE.Color) {
+                material.color.set(
+                  collision
+                    ? 0xe34b4b
+                    : new THREE.Color(color![0], color![1], color![2]),
+                );
+              }
+              material.opacity = collision
+                ? Math.min(color?.[3] ?? 0.35, 0.35)
+                : (color?.[3] ?? material.opacity);
+              material.transparent = collision || material.opacity < 1;
+              material.wireframe = collision;
+            }
+          });
+        }
+      }
+      contents.scale.set(...geometry.scale);
       break;
+    }
     case "unsupported":
       return null;
   }
[trailing space]
-  const opacity = collision
-    ? Math.min(color?.[3] ?? 0.35, 0.35)
-    : color?.[3] ?? 1;
-  const material = new THREE.MeshStandardMaterial({
-    color: collision
-      ? new THREE.Color(0xe34b4b)
-      : color
-      ? new THREE.Color(color[0], color[1], color[2])
-      : new THREE.Color(0x80aaff),
-    opacity,
-    transparent: collision || opacity < 1,
-    wireframe: collision,
+  contents.traverse((child) => {
+    if (child instanceof THREE.Mesh) {
+      child.castShadow = true;
+      child.receiveShadow = true;
+    }
   });
-  const mesh = new THREE.Mesh(shape, material);
-  if (geometry.kind === "cylinder") mesh.rotation.x = Math.PI / 2;
-  mesh.castShadow = true;
-  mesh.receiveShadow = true;
[trailing space]
   const visualGroup = new THREE.Group();
   visualGroup.position.set(...visual.origin.xyz);
-  visualGroup.rotation.set(...visual.origin.rpy, "XYZ");
-  visualGroup.add(mesh);
+  visualGroup.rotation.set(...visual.origin.rpy, "ZYX");
+  visualGroup.add(contents);
   return visualGroup;
 }
[trailing space]
@@ -124,6 +239,7 @@ export function buildRobotInstance(
     { joint: RobotJoint; motion: THREE.Group }
   >();
   const collisionObjects: THREE.Object3D[] = [];
+  const inertialObjects: THREE.Object3D[] = [];
   const limitPosition = (joint: RobotJoint, position: number) =>
     Math.max(
       joint.limit?.lower ?? Number.NEGATIVE_INFINITY,
@@ -149,12 +265,48 @@ export function buildRobotInstance(
         link.add(mesh);
       }
     }
+    if (definition.inertial) {
+      const inertial = new THREE.Group();
+      inertial.name = `${name}:inertial`;
+      inertial.position.set(...definition.inertial.origin.xyz);
+      inertial.rotation.set(...definition.inertial.origin.rpy, "ZYX");
+      const axes = new THREE.AxesHelper(0.12);
+      inertial.add(axes);
+      const radiusOfGyration =
+        definition.inertial.mass > 0
+          ? Math.sqrt(
+              Math.max(
+                0,
+                (definition.inertial.inertia[0] +
+                  definition.inertial.inertia[3] +
+                  definition.inertial.inertia[5]) /
+                  (3 * definition.inertial.mass),
+              ),
+            )
+          : 0;
+      if (radiusOfGyration > 0) {
+        const marker = new THREE.Mesh(
+          new THREE.SphereGeometry(1, 12, 8),
+          new THREE.MeshBasicMaterial({
+            color: 0xffaa44,
+            wireframe: true,
+            transparent: true,
+            opacity: 0.55,
+          }),
+        );
+        marker.scale.setScalar(Math.min(radiusOfGyration, 1));
+        inertial.add(marker);
+      }
+      inertial.visible = false;
+      inertialObjects.push(inertial);
+      link.add(inertial);
+    }
[trailing space]
     for (const joint of jointsByParent.get(name) ?? []) {
       const origin = new THREE.Group();
       origin.name = `${joint.name}:origin`;
       origin.position.set(...joint.origin.xyz);
-      origin.rotation.set(...joint.origin.rpy, "XYZ");
+      origin.rotation.set(...joint.origin.rpy, "ZYX");
       const motion = new THREE.Group();
       motion.name = `${joint.name}:motion`;
       origin.add(motion);
@@ -232,5 +384,38 @@ export function buildRobotInstance(
         object.visible = visible;
       });
     },
+    setInertialVisible(visible) {
+      inertialObjects.forEach((object) => {
+        object.visible = visible;
+      });
+    },
+    dispose() {
+      const geometries = new Set<THREE.BufferGeometry>();
+      const materials = new Set<THREE.Material>();
+      const textures = new Set<THREE.Texture>();
+      group.traverse((child) => {
+        if (child instanceof THREE.Mesh) {
+          geometries.add(child.geometry);
+          const meshMaterials = Array.isArray(child.material)
+            ? child.material
+            : [child.material];
+          for (const material of meshMaterials) {
+            materials.add(material);
+            for (const value of Object.values(material)) {
+              if (value instanceof THREE.Texture) textures.add(value);
+            }
+          }
+        } else if (child instanceof THREE.AxesHelper) {
+          geometries.add(child.geometry);
+          const axesMaterials = Array.isArray(child.material)
+            ? child.material
+            : [child.material];
+          axesMaterials.forEach((material) => materials.add(material));
+        }
+      });
+      geometries.forEach((geometry) => geometry.dispose());
+      materials.forEach((material) => material.dispose());
+      textures.forEach((texture) => texture.dispose());
+    },
   };
 }
diff --git a/src/rosGraph.ts b/src/rosGraph.ts
index 5e208bd..6a38355 100644
--- a/src/rosGraph.ts
+++ b/src/rosGraph.ts
@@ -25,6 +25,7 @@ export interface RosGraphService {
   types: string[];
   serverNodes: string[];
   clientNodes: string[];
+  safeTypes: string[];
 }
[trailing space]
 export interface RosGraphAction {
@@ -32,6 +33,7 @@ export interface RosGraphAction {
   types: string[];
   serverNodes: string[];
   clientNodes: string[];
+  safeTypes: string[];
 }
[trailing space]
 export interface RosGraphSnapshot {
diff --git a/src/tfGraph.ts b/src/tfGraph.ts
index 5ea7069..537de4f 100644
--- a/src/tfGraph.ts
+++ b/src/tfGraph.ts
@@ -31,13 +31,175 @@ export interface TfGraph {
   warnings: string[];
 }
[trailing space]
+export interface TfComposedPose {
+  rootFrame: string;
+  frame: string;
+  x: number;
+  y: number;
+  z: number;
+  qx: number;
+  qy: number;
+  qz: number;
+  qw: number;
+  stale: boolean;
+}
+
+export type TfComposition =
+  | { ok: true; pose: TfComposedPose }
+  | { ok: false; reason: string };
+
 const QUATERNION_TOLERANCE = 0.01;
 const TRANSLATION_WARNING_THRESHOLD = 1000;
+const SOURCE_DISAPPEARED_AFTER_MS = 30_000;
[trailing space]
 export function tfEdgeKey(parentFrame: string, childFrame: string) {
   return JSON.stringify([parentFrame, childFrame]);
 }
[trailing space]
+export function mergeTfTransform(
+  existing: Record<string, TfTransform>,
+  incoming: TfTransform,
+  staleAfterMs = 5000,
+): Record<string, TfTransform> {
+  const updated = { ...existing };
+  for (const [key, transform] of Object.entries(updated)) {
+    if (
+      transform.child_frame === incoming.child_frame &&
+      transform.parent_frame !== incoming.parent_frame &&
+      !transform.topic_name.endsWith("/tf_static") &&
+      incoming.receivedAtMs - transform.receivedAtMs > staleAfterMs
+    ) {
+      delete updated[key];
+    }
+  }
+  updated[tfEdgeKey(incoming.parent_frame, incoming.child_frame)] = incoming;
+  return updated;
+}
+
+function normalizeQuaternion(
+  quaternion: [number, number, number, number],
+): [number, number, number, number] {
+  const norm = Math.hypot(...quaternion);
+  return [
+    quaternion[0] / norm,
+    quaternion[1] / norm,
+    quaternion[2] / norm,
+    quaternion[3] / norm,
+  ];
+}
+
+function multiplyQuaternions(
+  first: [number, number, number, number],
+  second: [number, number, number, number],
+): [number, number, number, number] {
+  const [x1, y1, z1, w1] = first;
+  const [x2, y2, z2, w2] = second;
+  return [
+    w1 * x2 + x1 * w2 + y1 * z2 - z1 * y2,
+    w1 * y2 - x1 * z2 + y1 * w2 + z1 * x2,
+    w1 * z2 + x1 * y2 - y1 * x2 + z1 * w2,
+    w1 * w2 - x1 * x2 - y1 * y2 - z1 * z2,
+  ];
+}
+
+function rotateVector(
+  quaternion: [number, number, number, number],
+  vector: [number, number, number],
+): [number, number, number] {
+  const [x, y, z, w] = quaternion;
+  const [vx, vy, vz] = vector;
+  const tx = 2 * (y * vz - z * vy);
+  const ty = 2 * (z * vx - x * vz);
+  const tz = 2 * (x * vy - y * vx);
+  return [
+    vx + w * tx + y * tz - z * ty,
+    vy + w * ty + z * tx - x * tz,
+    vz + w * tz + x * ty - y * tx,
+  ];
+}
+
+export function composeTfTransform(
+  edges: TfGraphEdge[],
+  targetFrame: string,
+): TfComposition {
+  if (!targetFrame) return { ok: false, reason: "Target frame is empty" };
+  const path: TfGraphEdge[] = [];
+  const visited = new Set<string>();
+  const parentsByChild = new Map<string, TfGraphEdge[]>();
+  for (const edge of edges) {
+    const parents = parentsByChild.get(edge.child_frame) ?? [];
+    parents.push(edge);
+    parentsByChild.set(edge.child_frame, parents);
+  }
+  let frame = targetFrame;
+
+  while (true) {
+    if (visited.has(frame)) {
+      return { ok: false, reason: "Transform path contains a cycle" };
+    }
+    visited.add(frame);
+    const parents = parentsByChild.get(frame) ?? [];
+    if (parents.length > 1) {
+      return { ok: false, reason: `Frame ${frame} has multiple parents` };
+    }
+    if (parents.length === 0) break;
+
+    const edge = parents[0];
+    const blockingIssue = edge.issues.find(
+      (issue) =>
+        issue !== "Transform is stale" &&
+        issue !== "No transform update received for 30 seconds" &&
+        issue !== "Large translation magnitude",
+    );
+    if (blockingIssue) {
+      return { ok: false, reason: `${edge.child_frame}: ${blockingIssue}` };
+    }
+    path.push(edge);
+    frame = edge.parent_frame;
+  }
+
+  let translation: [number, number, number] = [0, 0, 0];
+  let rotation: [number, number, number, number] = [0, 0, 0, 1];
+  let stale = false;
+  for (const edge of path.reverse()) {
+    const edgeRotation = normalizeQuaternion([
+      edge.qx,
+      edge.qy,
+      edge.qz,
+      edge.qw,
+    ]);
+    const rotatedTranslation = rotateVector(rotation, [
+      edge.x,
+      edge.y,
+      edge.z,
+    ]);
+    translation = [
+      translation[0] + rotatedTranslation[0],
+      translation[1] + rotatedTranslation[1],
+      translation[2] + rotatedTranslation[2],
+    ];
+    rotation = normalizeQuaternion(multiplyQuaternions(rotation, edgeRotation));
+    stale ||= edge.issues.includes("Transform is stale") ||
+      edge.issues.includes("No transform update received for 30 seconds");
+  }
+
+  return {
+    ok: true,
+    pose: {
+      rootFrame: frame,
+      frame: targetFrame,
+      x: translation[0],
+      y: translation[1],
+      z: translation[2],
+      qx: rotation[0],
+      qy: rotation[1],
+      qz: rotation[2],
+      qw: rotation[3],
+      stale,
+    },
+  };
+}
+
 export function buildTfGraph(
   transforms: TfTransform[],
   nowMs = Date.now(),
@@ -62,6 +224,12 @@ export function buildTfGraph(
     if (values.some((value) => !Number.isFinite(value))) {
       issues.push("Contains NaN or Infinity");
     }
+    if (
+      !Number.isFinite(transform.receivedAtMs) ||
+      !Number.isFinite(transform.source_timestamp_ms)
+    ) {
+      issues.push("Contains an invalid timestamp");
+    }
[trailing space]
     const quaternionNorm = Math.hypot(
       transform.qx,
@@ -83,11 +251,12 @@ export function buildTfGraph(
       issues.push("Large translation magnitude");
     }
[trailing space]
-    if (
-      !transform.topic_name.endsWith("/tf_static") &&
-      nowMs - transform.receivedAtMs > staleAfterMs
-    ) {
-      issues.push("Transform is stale");
+    if (!transform.topic_name.endsWith("/tf_static")) {
+      const age = nowMs - transform.receivedAtMs;
+      if (age > staleAfterMs) issues.push("Transform is stale");
+      if (age > SOURCE_DISAPPEARED_AFTER_MS) {
+        issues.push("No transform update received for 30 seconds");
+      }
     }
[trailing space]
     return { ...transform, key, issues };
diff --git a/tests/robotDescription.test.mjs b/tests/robotDescription.test.mjs
index 7f2e89e..da182d0 100644
--- a/tests/robotDescription.test.mjs
+++ b/tests/robotDescription.test.mjs
@@ -1,5 +1,6 @@
 import assert from "node:assert/strict";
 import test from "node:test";
+import * as THREE from "three";
 import { buildRobotInstance } from "../src/robotDescription.ts";
[trailing space]
 const origin = { xyz: [0, 0, 0], rpy: [0, 0, 0] };
@@ -151,3 +152,81 @@ test("clamps joint limits, resolves mimic joints, and toggles collision geometry
   instance.setCollisionVisible(true);
   assert.equal(collision.visible, true);
 });
+
+test("loads STL meshes, applies URDF transforms, shows inertial frames, and disposes resources", () => {
+  const stl = `solid triangle
+facet normal 0 0 1
+outer loop
+vertex 0 0 0
+vertex 1 0 0
+vertex 0 1 0
+endloop
+endfacet
+endsolid triangle
+`;
+  const instance = buildRobotInstance({
+    name: "mesh_robot",
+    rootLink: "base",
+    warnings: [],
+    links: [
+      {
+        name: "base",
+        visuals: [
+          {
+            origin: { xyz: [0, 0, 0.2], rpy: [0.2, 0.3, 0.4] },
+            geometry: {
+              kind: "mesh",
+              filename: "package://demo/meshes/triangle.stl",
+              scale: [1, 2, 3],
+              format: "stl",
+              data: Buffer.from(stl).toString("base64"),
+            },
+            color: [0.1, 0.3, 0.8, 1],
+            materialName: "blue",
+          },
+        ],
+        collisions: [],
+        inertial: {
+          origin: { xyz: [0, 0, 0.1], rpy: [0, 0, 0] },
+          mass: 2,
+          inertia: [0.2, 0, 0, 0.3, 0, 0.4],
+        },
+      },
+    ],
+    joints: [],
+  });
+
+  const base = instance.group.getObjectByName("base");
+  assert.ok(base);
+  const visual = base.children.find(
+    (child) => child instanceof THREE.Group && child.position.z === 0.2,
+  );
+  assert.ok(visual);
+  assert.ok(
+    visual.quaternion.angleTo(
+      new THREE.Quaternion().setFromEuler(
+        new THREE.Euler(0.2, 0.3, 0.4, "ZYX"),
+      ),
+    ) < 1e-7,
+  );
+  const mesh = visual.children[0];
+  assert.ok(mesh instanceof THREE.Mesh);
+  assert.equal(mesh.scale.y, 2);
+  assert.equal(mesh.scale.z, 3);
+  assert.equal(mesh.geometry.getAttribute("position").count, 3);
+
+  const inertial = instance.group.getObjectByName("base:inertial");
+  assert.ok(inertial);
+  assert.equal(inertial.visible, false);
+  instance.setInertialVisible(true);
+  assert.equal(inertial.visible, true);
+
+  let geometryDisposed = false;
+  const disposeGeometry = mesh.geometry.dispose.bind(mesh.geometry);
+  mesh.geometry.dispose = () => {
+    geometryDisposed = true;
+    disposeGeometry();
+  };
+  instance.dispose();
+  assert.equal(geometryDisposed, true);
+});
diff --git a/tests/tfGraph.test.mjs b/tests/tfGraph.test.mjs
index 11f38b0..c29d7e0 100644
--- a/tests/tfGraph.test.mjs
+++ b/tests/tfGraph.test.mjs
@@ -1,6 +1,10 @@
 import assert from "node:assert/strict";
 import test from "node:test";
-import { buildTfGraph } from "../src/tfGraph.ts";
+import {
+  buildTfGraph,
+  composeTfTransform,
+  mergeTfTransform,
+} from "../src/tfGraph.ts";
[trailing space]
 function transform(parent, child, overrides = {}) {
   return {
@@ -86,14 +90,84 @@ test("uses receive time for staleness and preserves the ROS source timestamp", (
   assert.equal(graph.edges[0].source_timestamp_ms, 9000);
 });
[trailing space]
+test("marks dynamic frames with no recent updates as disappeared", () => {
+  const graph = buildTfGraph(
+    [transform("map", "lost_frame", { receivedAtMs: 0 })],
+    40000,
+  );
+  assert.ok(graph.edges[0].issues.includes("Transform is stale"));
+  assert.ok(
+    graph.edges[0].issues.includes(
+      "No transform update received for 30 seconds",
+    ),
+  );
+});
+
+test("replaces a stale dynamic parent when a child changes parent", () => {
+  const previous = transform("map", "base_link", { receivedAtMs: 1000 });
+  const changed = transform("odom", "base_link", { receivedAtMs: 7000 });
+  const merged = mergeTfTransform(
+    { [JSON.stringify(["map", "base_link"])]: previous },
+    changed,
+  );
+  assert.deepEqual(Object.values(merged), [changed]);
+
+  const conflict = mergeTfTransform(
+    { [JSON.stringify(["map", "base_link"])]: previous },
+    transform("odom", "base_link", { receivedAtMs: 2000 }),
+  );
+  assert.equal(Object.keys(conflict).length, 2);
+});
+
+test("composes chained transforms with parent rotation", () => {
+  const halfSqrt = Math.SQRT1_2;
+  const graph = buildTfGraph(
+    [
+      transform("map", "base_link", {
+        x: 1,
+        qz: halfSqrt,
+        qw: halfSqrt,
+      }),
+      transform("base_link", "laser", { x: 1 }),
+    ],
+    10000,
+  );
+  const composition = composeTfTransform(graph.edges, "laser");
+  assert.equal(composition.ok, true);
+  if (!composition.ok) return;
+  assert.equal(composition.pose.rootFrame, "map");
+  assert.ok(Math.abs(composition.pose.x - 1) < 1e-9);
+  assert.ok(Math.abs(composition.pose.y - 1) < 1e-9);
+  assert.ok(Math.abs(composition.pose.qz - halfSqrt) < 1e-9);
+  assert.ok(Math.abs(composition.pose.qw - halfSqrt) < 1e-9);
+});
+
+test("refuses composed poses with ambiguous or invalid edges", () => {
+  const ambiguous = buildTfGraph(
+    [transform("map_a", "base"), transform("map_b", "base")],
+    10000,
+  );
+  const ambiguousResult = composeTfTransform(ambiguous.edges, "base");
+  assert.equal(ambiguousResult.ok, false);
+
+  const invalid = buildTfGraph(
+    [transform("map", "base", { qx: 0, qy: 0, qz: 0, qw: 0 })],
+    10000,
+  );
+  const invalidResult = composeTfTransform(invalid.edges, "base");
+  assert.equal(invalidResult.ok, false);
+});
+
 test("reports invalid quaternions and non-finite transforms", () => {
   const graph = buildTfGraph(
     [
       transform("map", "zero", { qx: 0, qy: 0, qz: 0, qw: 0 }),
       transform("map", "invalid", { x: Number.NaN }),
+      transform("map", "bad_timestamp", { receivedAtMs: Number.NaN }),
     ],
     10000,
   );
   assert.ok(graph.edges[0].issues.includes("Zero quaternion — likely uninitialized"));
   assert.ok(graph.edges[1].issues.includes("Contains NaN or Infinity"));
+  assert.ok(graph.edges[2].issues.includes("Contains an invalid timestamp"));
 });
diff --git a/src-tauri/src/xacro.rs b/src-tauri/src/xacro.rs
new file mode 100644
index 0000000..7e530d4
--- /dev/null
+++ b/src-tauri/src/xacro.rs
@@ -0,0 +1,762 @@
+use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
+use quick_xml::{Reader, Writer};
+use std::collections::{HashMap, HashSet};
+use std::io::Cursor;
+use std::path::{Path, PathBuf};
+use std::time::{Duration, Instant};
+
+const MAX_SOURCE_BYTES: usize = 2 * 1024 * 1024;
+const MAX_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
+const MAX_EXPANDED_NODES: usize = 50_000;
+const MAX_INCLUDE_FILES: usize = 32;
+const MAX_DEPTH: usize = 32;
+const MAX_EXPANSION_TIME: Duration = Duration::from_secs(2);
+
+#[derive(Clone, Debug)]
+struct XmlNode {
+    name: String,
+    attributes: Vec<(String, String)>,
+    children: Vec<XmlNode>,
+    text: String,
+}
+
+impl XmlNode {
+    fn attribute(&self, name: &str) -> Option<&str> {
+        self.attributes
+            .iter()
+            .find_map(|(key, value)| (key == name).then_some(value.as_str()))
+    }
+}
+
+fn parse_document(content: &str, deadline: Instant) -> Result<XmlNode, String> {
+    if content.len() > MAX_SOURCE_BYTES {
+        return Err(format!(
+            "Xacro source exceeds the {MAX_SOURCE_BYTES}-byte limit"
+        ));
+    }
+    if content.contains('\0') {
+        return Err("Xacro source contains a NUL character".to_string());
+    }
+
+    let mut reader = Reader::from_str(content);
+    reader.config_mut().trim_text(true);
+    let mut stack = Vec::<XmlNode>::new();
+    let mut root = None;
+    let mut node_count = 0usize;
+
+    loop {
+        if Instant::now() >= deadline {
+            return Err("Xacro parsing exceeded its 2-second time limit".to_string());
+        }
+        match reader.read_event() {
+            Ok(event @ (Event::Start(_) | Event::Empty(_))) => {
+                let (element, is_empty) = match event {
+                    Event::Start(element) => (element, false),
+                    Event::Empty(element) => (element, true),
+                    _ => unreachable!(),
+                };
+                node_count += 1;
+                if node_count > MAX_EXPANDED_NODES || stack.len() >= MAX_DEPTH {
+                    return Err("Xacro XML exceeds the node or nesting limit".to_string());
+                }
+                let mut attributes = Vec::new();
+                for attribute in element.attributes().with_checks(true) {
+                    let attribute =
+                        attribute.map_err(|error| format!("Malformed Xacro attribute: {error}"))?;
+                    let name = String::from_utf8_lossy(attribute.key.as_ref()).into_owned();
+                    let value = attribute
+                        .decoded_and_normalized_value(
+                            quick_xml::XmlVersion::Implicit1_0,
+                            reader.decoder(),
+                        )
+                        .map_err(|error| format!("Malformed Xacro attribute value: {error}"))?
+                        .into_owned();
+                    attributes.push((name, value));
+                }
+                let node = XmlNode {
+                    name: String::from_utf8_lossy(element.name().as_ref()).into_owned(),
+                    attributes,
+                    children: Vec::new(),
+                    text: String::new(),
+                };
+                if is_empty {
+                    if let Some(parent) = stack.last_mut() {
+                        parent.children.push(node);
+                    } else if root.replace(node).is_some() {
+                        return Err("Xacro document must have exactly one root element".to_string());
+                    }
+                } else {
+                    stack.push(node);
+                }
+            }
+            Ok(Event::End(element)) => {
+                let node = stack
+                    .pop()
+                    .ok_or_else(|| "Xacro XML elements are improperly nested".to_string())?;
+                if element.name().as_ref() != node.name.as_bytes() {
+                    return Err("Xacro XML elements are improperly nested".to_string());
+                }
+                if let Some(parent) = stack.last_mut() {
+                    parent.children.push(node);
+                } else if root.replace(node).is_some() {
+                    return Err("Xacro document must have exactly one root element".to_string());
+                }
+            }
+            Ok(Event::Text(text)) => {
+                if let Some(parent) = stack.last_mut() {
+                    let decoded = text
+                        .decode()
+                        .map_err(|error| format!("Malformed Xacro text: {error}"))?;
+                    parent.text.push_str(&decoded);
+                } else {
+                    let text_bytes: &[u8] = text.as_ref();
+                    if !text_bytes.iter().all(u8::is_ascii_whitespace) {
+                        return Err("Xacro contains text outside its root element".to_string());
+                    }
+                }
+            }
+            Ok(Event::CData(text)) => {
+                if let Some(parent) = stack.last_mut() {
+                    parent.text.push_str(
+                        &text
+                            .decode()
+                            .map_err(|error| format!("Malformed Xacro text: {error}"))?,
+                    );
+                } else {
+                    return Err("Xacro contains data outside its root element".to_string());
+                }
+            }
+            Ok(Event::DocType(_)) => return Err("Xacro documents must not contain a DTD".into()),
+            Ok(Event::Eof) => break,
+            Ok(_) => {}
+            Err(error) => return Err(format!("Malformed Xacro XML: {error}")),
+        }
+    }
+
+    if !stack.is_empty() {
+        return Err("Xacro document is incomplete".to_string());
+    }
+    root.ok_or_else(|| "Xacro document has no root element".to_string())
+}
+
+fn local_name(name: &str) -> &str {
+    name.rsplit(':').next().unwrap_or(name)
+}
+
+fn is_xacro(node: &XmlNode, name: &str) -> bool {
+    node.name.starts_with("xacro:") && local_name(&node.name) == name
+}
+
+#[derive(Clone)]
+struct Context {
+    properties: HashMap<String, String>,
+    macros: HashMap<String, XmlNode>,
+    include_stack: HashSet<PathBuf>,
+    expanded_nodes: usize,
+    expanded_bytes: usize,
+    include_count: usize,
+    deadline: Instant,
+}
+
+fn valid_identifier(value: &str) -> bool {
+    let mut characters = value.chars();
+    characters
+        .next()
+        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
+        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
+}
+
+struct ExpressionParser<'a> {
+    source: &'a str,
+    position: usize,
+    depth: usize,
+    properties: &'a HashMap<String, String>,
+}
+
+impl ExpressionParser<'_> {
+    fn skip_whitespace(&mut self) {
+        while self
+            .source
+            .as_bytes()
+            .get(self.position)
+            .is_some_and(u8::is_ascii_whitespace)
+        {
+            self.position += 1;
+        }
+    }
+
+    fn parse_sum(&mut self) -> Result<f64, String> {
+        let mut value = self.parse_product()?;
+        loop {
+            self.skip_whitespace();
+            match self.source.as_bytes().get(self.position) {
+                Some(b'+') => {
+                    self.position += 1;
+                    value += self.parse_product()?;
+                }
+                Some(b'-') => {
+                    self.position += 1;
+                    value -= self.parse_product()?;
+                }
+                _ => return Ok(value),
+            }
+            if !value.is_finite() {
+                return Err("Xacro numeric expression is not finite".to_string());
+            }
+        }
+    }
+
+    fn parse_product(&mut self) -> Result<f64, String> {
+        let mut value = self.parse_atom()?;
+        loop {
+            self.skip_whitespace();
+            match self.source.as_bytes().get(self.position) {
+                Some(b'*') => {
+                    self.position += 1;
+                    value *= self.parse_atom()?;
+                }
+                Some(b'/') => {
+                    self.position += 1;
+                    let divisor = self.parse_atom()?;
+                    if divisor == 0.0 {
+                        return Err("Xacro numeric expression divides by zero".to_string());
+                    }
+                    value /= divisor;
+                }
+                _ => return Ok(value),
+            }
+            if !value.is_finite() {
+                return Err("Xacro numeric expression is not finite".to_string());
+            }
+        }
+    }
+
+    fn parse_atom(&mut self) -> Result<f64, String> {
+        self.skip_whitespace();
+        let bytes = self.source.as_bytes();
+        if bytes.get(self.position) == Some(&b'(') {
+            self.depth += 1;
+            if self.depth > MAX_DEPTH {
+                return Err("Xacro numeric expression exceeds the nesting limit".to_string());
+            }
+            self.position += 1;
+            let value = self.parse_sum()?;
+            self.skip_whitespace();
+            if bytes.get(self.position) != Some(&b')') {
+                return Err("Xacro numeric expression has unbalanced parentheses".to_string());
+            }
+            self.position += 1;
+            self.depth -= 1;
+            return Ok(value);
+        }
+        if matches!(bytes.get(self.position), Some(b'+') | Some(b'-')) {
+            self.depth += 1;
+            if self.depth > MAX_DEPTH {
+                return Err("Xacro numeric expression exceeds the nesting limit".to_string());
+            }
+            let negative = bytes[self.position] == b'-';
+            self.position += 1;
+            let value = self.parse_atom()?;
+            self.depth -= 1;
+            return Ok(if negative { -value } else { value });
+        }
+        let start = self.position;
+        if bytes
+            .get(self.position)
+            .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
+        {
+            self.position += 1;
+            while bytes
+                .get(self.position)
+                .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
+            {
+                self.position += 1;
+            }
+            let identifier = &self.source[start..self.position];
+            if identifier == "pi" {
+                return Ok(std::f64::consts::PI);
+            }
+            let value = self
+                .properties
+                .get(identifier)
+                .ok_or_else(|| format!("Xacro property '{identifier}' is not defined"))?
+                .parse::<f64>()
+                .map_err(|_| {
+                    format!("Xacro property '{identifier}' is not numeric in this expression")
+                })?;
+            return if value.is_finite() {
+                Ok(value)
+            } else {
+                Err("Xacro numeric expression is not finite".to_string())
+            };
+        }
+
+        while bytes
+            .get(self.position)
+            .is_some_and(|byte| byte.is_ascii_digit() || matches!(byte, b'.' | b'e' | b'E'))
+        {
+            self.position += 1;
+            if matches!(bytes[self.position - 1], b'e' | b'E')
+                && matches!(bytes.get(self.position), Some(b'+') | Some(b'-'))
+            {
+                self.position += 1;
+            }
+        }
+        if start == self.position {
+            return Err("Unsupported Xacro numeric expression".to_string());
+        }
+        self.source[start..self.position]
+            .parse::<f64>()
+            .ok()
+            .filter(|value| value.is_finite())
+            .ok_or_else(|| "Invalid Xacro numeric expression".to_string())
+    }
+}
+
+fn evaluate_expression(
+    expression: &str,
+    properties: &HashMap<String, String>,
+) -> Result<f64, String> {
+    let mut parser = ExpressionParser {
+        source: expression,
+        position: 0,
+        depth: 0,
+        properties,
+    };
+    let value = parser.parse_sum()?;
+    parser.skip_whitespace();
+    if parser.position != expression.len() {
+        return Err("Unsupported Xacro numeric expression".to_string());
+    }
+    Ok(value)
+}
+
+fn substitute(source: &str, properties: &HashMap<String, String>) -> Result<String, String> {
+    let mut output = String::with_capacity(source.len());
+    let mut rest = source;
+    while let Some(start) = ["${", "$(arg ", "$(find "]
+        .iter()
+        .filter_map(|needle| rest.find(needle))
+        .min()
+    {
+        output.push_str(&rest[..start]);
+        let is_arg = rest[start..].starts_with("$(arg ");
+        let is_find = rest[start..].starts_with("$(find ");
+        let (prefix_length, suffix) = if is_arg {
+            (6, ')')
+        } else if is_find {
+            (7, ')')
+        } else {
+            (2, '}')
+        };
+        let value_start = start + prefix_length;
+        let end = rest[value_start..]
+            .find(suffix)
+            .map(|index| value_start + index)
+            .ok_or_else(|| "Xacro substitution is not terminated".to_string())?;
+        let expression = rest[value_start..end].trim();
+        if is_arg {
+            if !valid_identifier(expression) {
+                return Err(format!("Invalid Xacro argument '{expression}'"));
+            }
+            let value = properties
+                .get(expression)
+                .ok_or_else(|| format!("Xacro argument '{expression}' is not defined"))?;
+            output.push_str(value);
+        } else if is_find {
+            if !valid_identifier(expression) {
+                return Err(format!("Invalid Xacro package name '{expression}'"));
+            }
+            output.push_str("package://");
+            output.push_str(expression);
+        } else if valid_identifier(expression) {
+            let value = properties.get(expression).ok_or_else(|| {
+                format!("Xacro property or argument '{expression}' is not defined")
+            })?;
+            output.push_str(value);
+        } else {
+            output.push_str(&evaluate_expression(expression, properties)?.to_string());
+        }
+        rest = &rest[end + 1..];
+    }
+    output.push_str(rest);
+    if output.contains("${") || output.contains("$(") {
+        return Err("Unsupported Xacro substitution syntax".to_string());
+    }
+    Ok(output)
+}
+
+fn parse_macro_params(value: &str) -> Result<Vec<(String, Option<String>)>, String> {
+    value
+        .split_whitespace()
+        .map(|parameter| {
+            if parameter.starts_with('*') || parameter.starts_with('^') {
+                return Err("Xacro block parameters are not supported".to_string());
+            }
+            let (name, default) = parameter
+                .split_once(":=")
+                .map_or((parameter, None), |(name, default)| {
+                    (name, Some(default.to_string()))
+                });
+            if !valid_identifier(name) {
+                return Err(format!("Invalid Xacro macro parameter '{name}'"));
+            }
+            Ok((name.to_string(), default))
+        })
+        .collect()
+}
+
+fn condition(value: &str) -> Result<bool, String> {
+    match value.trim() {
+        "true" | "1" => Ok(true),
+        "false" | "0" => Ok(false),
+        _ => value
+            .trim()
+            .parse::<f64>()
+            .ok()
+            .filter(|number| number.is_finite())
+            .map(|number| number != 0.0)
+            .ok_or_else(|| {
+                format!("Unsupported Xacro condition '{value}'; use a boolean or number")
+            }),
+    }
+}
+
+fn expand_nodes<F>(
+    nodes: &[XmlNode],
+    source_path: &Path,
+    context: &mut Context,
+    macro_depth: usize,
+    include_loader: &mut F,
+) -> Result<Vec<XmlNode>, String>
+where
+    F: FnMut(&Path, &str) -> Result<(PathBuf, String), String>,
+{
+    if macro_depth > MAX_DEPTH {
+        return Err("Xacro macro recursion exceeds the nesting limit".to_string());
+    }
+    let mut output = Vec::new();
+    for node in nodes {
+        if Instant::now() >= context.deadline {
+            return Err("Xacro expansion exceeded its 2-second time limit".to_string());
+        }
+        context.expanded_nodes += 1;
+        if context.expanded_nodes > MAX_EXPANDED_NODES {
+            return Err("Xacro expansion exceeds the node limit".to_string());
+        }
+        if is_xacro(node, "property") {
+            let name = node
+                .attribute("name")
+                .ok_or_else(|| "Xacro property is missing its name".to_string())?;
+            if !valid_identifier(name) {
+                return Err(format!("Invalid Xacro property name '{name}'"));
+            }
+            let value = node
+                .attribute("value")
+                .ok_or_else(|| format!("Xacro property '{name}' is missing its value"))?;
+            let value = substitute(value, &context.properties)?;
+            if context.properties.len() >= 1024 && !context.properties.contains_key(name) {
+                return Err("Xacro expansion exceeds the 1024-property limit".to_string());
+            }
+            context.properties.insert(name.to_string(), value);
+            continue;
+        }
+        if is_xacro(node, "arg") {
+            let name = node
+                .attribute("name")
+                .ok_or_else(|| "Xacro argument is missing its name".to_string())?;
+            if !valid_identifier(name) {
+                return Err(format!("Invalid Xacro argument name '{name}'"));
+            }
+            let default = node
+                .attribute("default")
+                .ok_or_else(|| format!("Xacro argument '{name}' needs a default value"))?;
+            let value = substitute(default, &context.properties)?;
+            context.properties.entry(name.to_string()).or_insert(value);
+            continue;
+        }
+        if is_xacro(node, "macro") {
+            let name = node
+                .attribute("name")
+                .ok_or_else(|| "Xacro macro is missing its name".to_string())?;
+            if !valid_identifier(name) {
+                return Err(format!("Invalid Xacro macro name '{name}'"));
+            }
+            context.macros.insert(name.to_string(), node.clone());
+            continue;
+        }
+        if is_xacro(node, "include") {
+            context.include_count += 1;
+            if context.include_count > MAX_INCLUDE_FILES {
+                return Err(format!(
+                    "Xacro includes exceed the {MAX_INCLUDE_FILES}-file limit"
+                ));
+            }
+            let filename = node
+                .attribute("filename")
+                .ok_or_else(|| "Xacro include is missing its filename".to_string())?;
+            let filename = substitute(filename, &context.properties)?;
+            let (included_path, included_content) = include_loader(source_path, &filename)?;
+            if !context.include_stack.insert(included_path.clone()) {
+                return Err("Xacro include cycle detected".to_string());
+            }
+            let included_root = parse_document(&included_content, context.deadline)?;
+            if local_name(&included_root.name) != "robot" {
+                return Err("Included Xacro file must have a <robot> root".to_string());
+            }
+            let included_children = expand_nodes(
+                &included_root.children,
+                &included_path,
+                context,
+                macro_depth,
+                include_loader,
+            )?;
+            context.include_stack.remove(&included_path);
+            output.extend(included_children);
+            continue;
+        }
+        if is_xacro(node, "if") || is_xacro(node, "unless") {
+            let value = node
+                .attribute("value")
+                .ok_or_else(|| "Xacro conditional is missing its value".to_string())?;
+            let value = substitute(value, &context.properties)?;
+            let mut include_children = condition(&value)?;
+            if is_xacro(node, "unless") {
+                include_children = !include_children;
+            }
+            if include_children {
+                output.extend(expand_nodes(
+                    &node.children,
+                    source_path,
+                    context,
+                    macro_depth,
+                    include_loader,
+                )?);
+            }
+            continue;
+        }
+        if node.name.starts_with("xacro:") {
+            let macro_name = local_name(&node.name);
+            let definition =
+                context.macros.get(macro_name).cloned().ok_or_else(|| {
+                    format!("Unsupported or undefined Xacro element '{}'", node.name)
+                })?;
+            let params = parse_macro_params(
+                definition
+                    .attribute("params")
+                    .ok_or_else(|| format!("Xacro macro '{macro_name}' has no params"))?,
+            )?;
+            let mut local_properties = context.properties.clone();
+            for (name, default) in params {
+                let value = if let Some(value) = node.attribute(&name) {
+                    substitute(value, &context.properties)?
+                } else if let Some(default) = default {
+                    substitute(&default, &context.properties)?
+                } else {
+                    return Err(format!(
+                        "Xacro macro '{macro_name}' is missing parameter '{name}'"
+                    ));
+                };
+                local_properties.insert(name, value);
+            }
+            let previous = std::mem::replace(&mut context.properties, local_properties);
+            let expanded = expand_nodes(
+                &definition.children,
+                source_path,
+                context,
+                macro_depth + 1,
+                include_loader,
+            );
+            context.properties = previous;
+            output.extend(expanded?);
+            continue;
+        }
+
+        let mut expanded = node.clone();
+        for (name, value) in &mut expanded.attributes {
+            if !name.starts_with("xmlns") {
+                *value = substitute(value, &context.properties)?;
+            }
+        }
+        expanded.text = substitute(&expanded.text, &context.properties)?;
+        expanded.children = expand_nodes(
+            &node.children,
+            source_path,
+            context,
+            macro_depth,
+            include_loader,
+        )?;
+        context.expanded_bytes = context
+            .expanded_bytes
+            .saturating_add(expanded.name.len())
+            .saturating_add(expanded.text.len())
+            .saturating_add(
+                expanded
+                    .attributes
+                    .iter()
+                    .map(|(name, value)| name.len() + value.len())
+                    .sum::<usize>(),
+            );
+        if context.expanded_bytes > MAX_OUTPUT_BYTES {
+            return Err(format!(
+                "Xacro expansion exceeds the {MAX_OUTPUT_BYTES}-byte output limit"
+            ));
+        }
+        output.push(expanded);
+    }
+    Ok(output)
+}
+
+fn write_node(writer: &mut Writer<Cursor<Vec<u8>>>, node: &XmlNode) -> Result<(), String> {
+    let mut start = BytesStart::new(node.name.as_str());
+    for (name, value) in &node.attributes {
+        start.push_attribute((name.as_str(), value.as_str()));
+    }
+    if node.children.is_empty() && node.text.is_empty() {
+        writer
+            .write_event(Event::Empty(start))
+            .map_err(|error| format!("Could not serialize expanded Xacro: {error}"))?;
+        return Ok(());
+    }
+    writer
+        .write_event(Event::Start(start))
+        .map_err(|error| format!("Could not serialize expanded Xacro: {error}"))?;
+    if !node.text.is_empty() {
+        writer
+            .write_event(Event::Text(BytesText::new(&node.text)))
+            .map_err(|error| format!("Could not serialize expanded Xacro: {error}"))?;
+    }
+    for child in &node.children {
+        write_node(writer, child)?;
+    }
+    writer
+        .write_event(Event::End(BytesEnd::new(node.name.as_str())))
+        .map_err(|error| format!("Could not serialize expanded Xacro: {error}"))?;
+    Ok(())
+}
+
+pub fn expand<F>(
+    content: &str,
+    source_path: &Path,
+    include_loader: &mut F,
+) -> Result<String, String>
+where
+    F: FnMut(&Path, &str) -> Result<(PathBuf, String), String>,
+{
+    let deadline = Instant::now() + MAX_EXPANSION_TIME;
+    let root = parse_document(content, deadline)?;
+    if local_name(&root.name) != "robot" {
+        return Err("Xacro document must have a <robot> root".to_string());
+    }
+    let mut context = Context {
+        properties: HashMap::new(),
+        macros: HashMap::new(),
+        include_stack: HashSet::new(),
+        expanded_nodes: 0,
+        expanded_bytes: 0,
+        include_count: 0,
+        deadline,
+    };
+    if source_path.is_absolute() {
+        context.include_stack.insert(source_path.to_path_buf());
+    }
+    let expanded_children =
+        expand_nodes(&root.children, source_path, &mut context, 0, include_loader)?;
+    let mut expanded_root = root;
+    for (name, value) in &mut expanded_root.attributes {
+        if !name.starts_with("xmlns") {
+            *value = substitute(value, &context.properties)?;
+        }
+    }
+    expanded_root.text = substitute(&expanded_root.text, &context.properties)?;
+    expanded_root.children = expanded_children;
+    let mut writer = Writer::new(Cursor::new(Vec::new()));
+    write_node(&mut writer, &expanded_root)?;
+    let bytes = writer.into_inner().into_inner();
+    if bytes.len() > MAX_OUTPUT_BYTES {
+        return Err(format!(
+            "Expanded Xacro exceeds the {MAX_OUTPUT_BYTES}-byte limit"
+        ));
+    }
+    String::from_utf8(bytes).map_err(|error| format!("Expanded Xacro is not UTF-8: {error}"))
+}
+
+#[cfg(test)]
+mod tests {
+    use super::expand;
+    use std::path::Path;
+
+    #[test]
+    fn expands_properties_arguments_macros_and_conditionals_without_evaluating_code() {
+        let input = r#"<robot name="test" xmlns:xacro="http://www.ros.org/wiki/xacro">
+            <xacro:arg name="prefix" default="arm"/>
+            <xacro:property name="root" value="${prefix}_base"/>
+            <xacro:property name="mesh" value="$(find demo_robot)/meshes/base.stl"/>
+            <xacro:macro name="link" params="name color">
+                <link name="${name}">
+                    <origin rpy="${pi/2} 0 0"/>
+                    <visual><geometry><box size="1 1 1"/></geometry>
+                    <material name="body"><color rgba="${color}"/></material></visual>
+                </link>
+            </xacro:macro>
+            <xacro:link name="${root}" color="0.2 0.3 0.4 1"/>
+            <link name="resource" filename="${mesh}"/>
+            <xacro:if value="true"><link name="tool"/></xacro:if>
+            <xacro:unless value="false"><link name="sensor"/></xacro:unless>
+        </robot>"#;
+        let mut include_loader = |_: &Path, _: &str| Err("no includes expected".to_string());
+        let expanded = expand(input, Path::new("robot.xacro"), &mut include_loader).unwrap();
+        assert!(expanded.contains("name=\"arm_base\""));
+        assert!(expanded.contains("name=\"tool\""));
+        assert!(expanded.contains("name=\"sensor\""));
+        assert!(expanded.contains("filename=\"package://demo_robot/meshes/base.stl\""));
+        assert!(!expanded.contains("xacro:"));
+        assert!(expanded.contains("rpy=\"1.5707963267948966 0 0\""));
+    }
+
+    #[test]
+    fn expands_relative_includes_and_rejects_cycles() {
+        let input = r#"<robot name="test" xmlns:xacro="http://www.ros.org/wiki/xacro"><xacro:include filename="parts.xacro"/></robot>"#;
+        let mut include_loader = |_: &Path, filename: &str| {
+            assert_eq!(filename, "parts.xacro");
+            Ok((
+                Path::new("/workspace/parts.xacro").to_path_buf(),
+                r#"<robot name="included"><link name="base"/></robot>"#.to_string(),
+            ))
+        };
+        let expanded = expand(
+            input,
+            Path::new("/workspace/main.xacro"),
+            &mut include_loader,
+        )
+        .unwrap();
+        assert!(expanded.contains("<link name=\"base\"/>"));
+
+        let mut cycle_loader = |_: &Path, _: &str| {
+            Ok((
+                Path::new("/workspace/main.xacro").to_path_buf(),
+                r#"<robot name="test"/>"#.to_string(),
+            ))
+        };
+        assert!(
+            expand(input, Path::new("/workspace/main.xacro"), &mut cycle_loader)
+                .unwrap_err()
+                .contains("cycle")
+        );
+    }
+
+    #[test]
+    fn rejects_expression_execution_and_unbounded_expansion() {
+        let mut include_loader = |_: &Path, _: &str| Err("no includes expected".to_string());
+        let code = r#"<robot name="test" xmlns:xacro="http://www.ros.org/wiki/xacro"><link name="${__import__('os').system('x')}"/></robot>"#;
+        assert!(expand(code, Path::new("robot.xacro"), &mut include_loader)
+            .unwrap_err()
+            .contains("not defined"));
+        let looped = r#"<robot name="test" xmlns:xacro="http://www.ros.org/wiki/xacro"><xacro:macro name="again" params=""><xacro:again/></xacro:macro><xacro:again/></robot>"#;
+        assert!(
+            expand(looped, Path::new("robot.xacro"), &mut include_loader)
+                .unwrap_err()
+                .contains("recursion")
+        );
+    }
+}

diff --git a/tests/fixtures/urdf_workspace/src/demo_robot/meshes/triangle.dae b/tests/fixtures/urdf_workspace/src/demo_robot/meshes/triangle.dae
new file mode 100644
index 0000000..8a17486
--- /dev/null
+++ b/tests/fixtures/urdf_workspace/src/demo_robot/meshes/triangle.dae
@@ -0,0 +1,31 @@
+<?xml version="1.0" encoding="UTF-8"?>
+<COLLADA xmlns="http://www.collada.org/2005/11/COLLADASchema" version="1.4.1">
+  <asset><up_axis>Z_UP</up_axis></asset>
+  <library_geometries>
+    <geometry id="triangle" name="triangle">
+      <mesh>
+        <source id="triangle-positions">
+          <float_array id="triangle-positions-array" count="9">0 0 0 1 0 0 0 1 0</float_array>
+          <technique_common>
+            <accessor source="#triangle-positions-array" count="3" stride="3">
+              <param name="X" type="float"/><param name="Y" type="float"/><param name="Z" type="float"/>
+            </accessor>
+          </technique_common>
+        </source>
+        <vertices id="triangle-vertices">
+          <input semantic="POSITION" source="#triangle-positions"/>
+        </vertices>
+        <triangles count="1">
+          <input semantic="VERTEX" source="#triangle-vertices" offset="0"/>
+          <p>0 1 2</p>
+        </triangles>
+      </mesh>
+    </geometry>
+  </library_geometries>
+  <library_visual_scenes>
+    <visual_scene id="Scene">
+      <node><instance_geometry url="#triangle"/></node>
+    </visual_scene>
+  </library_visual_scenes>
+  <scene><instance_visual_scene url="#Scene"/></scene>
+</COLLADA>

diff --git a/tests/fixtures/urdf_workspace/src/demo_robot/meshes/triangle.stl b/tests/fixtures/urdf_workspace/src/demo_robot/meshes/triangle.stl
new file mode 100644
index 0000000..243b042
--- /dev/null
+++ b/tests/fixtures/urdf_workspace/src/demo_robot/meshes/triangle.stl
@@ -0,0 +1,9 @@
+solid triangle
+  facet normal 0 0 1
+    outer loop
+      vertex 0 0 0
+      vertex 1 0 0
+      vertex 0 1 0
+    endloop
+  endfacet
+endsolid triangle

diff --git a/tests/fixtures/urdf_workspace/src/demo_robot/package.xml b/tests/fixtures/urdf_workspace/src/demo_robot/package.xml
new file mode 100644
index 0000000..22c6616
--- /dev/null
+++ b/tests/fixtures/urdf_workspace/src/demo_robot/package.xml
@@ -0,0 +1,8 @@
+<?xml version="1.0"?>
+<package format="3">
+  <name>demo_robot</name>
+  <version>0.1.0</version>
+  <description>Mesh preview test fixture</description>
+  <maintainer email="tests@example.invalid">Tests</maintainer>
+  <license>Apache-2.0</license>
+</package>

diff --git a/tests/fixtures/urdf_workspace/src/demo_robot/urdf/materials.xacro b/tests/fixtures/urdf_workspace/src/demo_robot/urdf/materials.xacro
new file mode 100644
index 0000000..7532481
--- /dev/null
+++ b/tests/fixtures/urdf_workspace/src/demo_robot/urdf/materials.xacro
@@ -0,0 +1,6 @@
+<?xml version="1.0"?>
+<robot name="materials" xmlns:xacro="http://www.ros.org/wiki/xacro">
+  <material name="body_blue">
+    <color rgba="0.1 0.3 0.8 1"/>
+  </material>
+</robot>

diff --git a/tests/fixtures/urdf_workspace/src/demo_robot/urdf/robot.xacro b/tests/fixtures/urdf_workspace/src/demo_robot/urdf/robot.xacro
new file mode 100644
index 0000000..11f3f0b
--- /dev/null
+++ b/tests/fixtures/urdf_workspace/src/demo_robot/urdf/robot.xacro
@@ -0,0 +1,25 @@
+<?xml version="1.0"?>
+<robot name="${prefix}_robot" xmlns:xacro="http://www.ros.org/wiki/xacro">
+  <xacro:arg name="prefix" default="demo"/>
+  <xacro:include filename="$(find demo_robot)/urdf/materials.xacro"/>
+  <xacro:macro name="mesh_link" params="name mesh">
+    <link name="${name}">
+      <inertial>
+        <origin xyz="0 0 0.1" rpy="0 0 0"/>
+        <mass value="1.5"/>
+        <inertia ixx="0.2" ixy="0" ixz="0" iyy="0.3" iyz="0" izz="0.4"/>
+      </inertial>
+      <visual>
+        <origin xyz="0 0 0.2" rpy="0 0 0"/>
+        <geometry>
+          <mesh filename="${mesh}" scale="1 2 1"/>
+        </geometry>
+        <material name="body_blue"/>
+      </visual>
+      <collision>
+        <geometry><box size="0.5 0.5 0.5"/></geometry>
+      </collision>
+    </link>
+  </xacro:macro>
+  <xacro:mesh_link name="base" mesh="package://demo_robot/meshes/triangle.stl"/>
+</robot>
````
