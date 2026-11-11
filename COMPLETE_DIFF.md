# Complete Git Diff

Base: `b907550` on `main`; generated from the current worktree.
Includes tracked changes and untracked files. Excludes this file and the read-only pasted attachment.

Validation summary: npm build and 7 frontend tests passed; Rust fmt, 40-test suite (39 passed, 1 ignored), and strict Clippy passed; Python service-core suite passed (42 tests); shell syntax and protocol-copy checks passed. The full Docker ROS 2 Jazzy/Gazebo Harmonic integration passed, including live graph, TF, joint-state, SDFormat, TLS/auth, stop/reset, event replay/restart, and Gazebo checks. Cleanup exited 0 and left no integration container or temporary directory. Vite reports the existing ~828 kB chunk warning. No commit, push, tag, or release was created.

## Full diff

````diff
diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
index d6fe722..6afb8b5 100644
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -18,6 +18,7 @@ jobs:
       - run: npm ci
       - run: npm audit --audit-level=moderate
       - run: npm run build
+      - run: npm run test:frontend

   rust:
     name: Rust tests and lint
diff --git a/README.md b/README.md
index 029ab36..bbd01a4 100644
--- a/README.md
+++ b/README.md
@@ -16,11 +16,17 @@ upfront about that than pretend otherwise.
   connects to a Docker-based ROS 2 environment automatically on launch. No
   manual Docker/Dev Container setup required.
 - **Live ROS status** — node/topic snapshots and TF updates are streamed over
-  authenticated gRPC while the system runs. The current UI does not yet
-  provide full package, service, or action exploration.
-- **3D visualization** — robot state rendered live in a three.js viewport,
-  with in-app controls to publish `cmd_vel` commands directly (no manual
-  container terminal needed for basic teleop).
+  authenticated gRPC while the system runs. The ROS sidebar also discovers
+  source packages in the selected workspace, shows package metadata and
+  resources, and lets you inspect package files. The live graph shows topic
+  publisher/subscriber QoS, service endpoints, and discovered action servers
+  and clients. Service invocation and action goal monitoring are not yet
+  available.
+- **3D visualization** — preview primitive URDF visuals and articulated
+  joints in the three.js viewport when a `.urdf` file is active; streamed
+  `/joint_states` update supported joints. Without an active URDF, the view
+  keeps its placeholder robot. In-app controls publish bounded `cmd_vel`
+  commands directly (no manual container terminal needed for basic teleop).
 - **Custom Gazebo world support** — load your own world file instead of
   being limited to the bundled demo.
 - **AI assistant with real context** — ask questions about your open file,
@@ -132,6 +138,11 @@ any of this — the app manages the environment for you.
   bounded limits, latch the software emergency stop, or explicitly reset it.
   The backend remains authoritative and rejects commands while stopped.
 - **ROS Log tab** — live feed of ROS 2 node/topic activity.
+- **ROS Explorer sidebar** — filter packages from the selected workspace,
+  inspect build type, version, description, dependencies, installed
+  executables, launch/configuration files, and browse package files. Refresh
+  after builds to update the workspace package inventory. The live graph also
+  shows nodes, topic endpoints/QoS, services, and action endpoint metadata.
 - **TF tab** — live transform updates, with an **Explain** action per
   frame for an AI-generated plain-language breakdown.
 - **AI Assistant (right panel)** — ask questions about your open file,
@@ -157,22 +168,25 @@ any of this — the app manages the environment for you.

 ## Known limitations (please break things and tell me what you find)

-- 3D view uses accurately-dimensioned placeholder geometry, not full URDF
-  mesh loading yet — visually approximate, not a rendered CAD model.
-- The TF tab currently lists/group-displays received transform edges; it does
-  not yet provide a fully validated hierarchy, stale-frame expiry, or cycle
-  detection.
-- ROS package discovery, detailed node endpoint inspection, and service/action
-  exploration are not implemented. The live ROS view is not a complete graph
-  explorer.
-- URDF editing and AI generation are available, but URDF parsing, Xacro
-  processing, package-resource resolution, and description-based model
-  rendering are not implemented. The 3D view uses placeholder geometry.
-- SDF imports are checked for XML structure, a supported declared version,
-  and a world element; this is not SDFormat schema or resource-reference
+- The URDF preview supports box, cylinder, and sphere visuals plus
+  fixed/revolute/continuous/prismatic joints. It does not expand Xacro or
+  resolve mesh/package resources; mesh visuals are reported and skipped.
+  Floating/planar joints and full URDF semantics are not supported. The ROS
+  to three.js coordinate mapping is a first pass and still needs convention
   validation.
-- ROS↔three.js coordinate mapping is a first-pass approximation — rotation
-  direction hasn't been rigorously verified against every convention.
+- The TF tab builds a live transform forest and warns about multiple roots,
+  cycles, multiple parents, stale dynamic transforms, non-finite values, and
+  invalid quaternions. Static `/tf_static` edges use transient-local QoS and
+  are not marked stale. This is diagnostic validation, not a substitute for
+  `tf2` authority or full ROS graph validation.
+- ROS graph snapshots are bounded and read-only. Service invocation and
+  action goal submission/monitoring are not implemented because arbitrary
+  operations could bypass the motion safety path. Package discovery reads
+  source package manifests and workspace install outputs, but does not show
+  every underlay package.
+- SDF imports are structurally checked, and the world is validated against
+  the installed SDFormat schema before Gazebo starts. This does not resolve or
+  validate every external model/mesh resource.
 - Only one bundled project template (`basic_publisher`) so far.
 - Local/offline model *running* isn't implemented — Settings will suggest
   appropriately-sized models for your hardware, but you'll still need to
@@ -304,6 +318,7 @@ cargo check --manifest-path src-tauri/Cargo.toml --locked
 cargo test --manifest-path src-tauri/Cargo.toml --locked
 cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features --locked -- -D warnings
 python3 -m unittest discover -s src-tauri/resources/devcontainer -p 'test_*.py' -v
+npm run test:frontend
 ```

 For the live container integration (Docker Desktop or Docker Engine must be
@@ -314,18 +329,19 @@ running), execute:
 ```

 The script builds the supported ROS 2 Jazzy/Gazebo Harmonic image, mounts a
-temporary workspace, verifies the pinned-TLS Rust client, gRPC authentication,
-emergency-stop gating, event replay after service restart, Gazebo world/model
+temporary workspace, verifies selected-workspace package discovery, the
+pinned-TLS Rust client, gRPC authentication, emergency-stop gating, event
+replay after service restart, live `/tf_static` delivery, Gazebo world/model
 readiness, reset, and ROS-Gazebo topic bridging. It performs no robot movement
 and removes its temporary container and workspace on exit. CI runs the same
 script on its Linux runner.

-These are unit/build checks, not Docker/ROS 2/Gazebo integration tests. They
-require Docker Desktop running for a live environment; if it is unavailable,
-simulation and gRPC integration behavior remains unverified. The current
-supported development baseline is ROS 2 Jazzy with Gazebo Harmonic in the
-bundled container. macOS is tested most often; Linux/Windows builds and
-runtime behavior need additional platform CI coverage.
+Unit/build checks are not a substitute for live Docker/ROS 2/Gazebo checks.
+They require Docker Desktop running for an integration environment; if it is
+unavailable, simulation and gRPC integration behavior remains unverified. The
+current supported development baseline is ROS 2 Jazzy with Gazebo Harmonic in
+the bundled container. macOS is tested most often; Linux/Windows runtime
+behavior and packaged desktop lifecycle still need platform-specific testing.

 ---

diff --git a/RELEASE_READINESS.md b/RELEASE_READINESS.md
index feee44c..7788803 100644
--- a/RELEASE_READINESS.md
+++ b/RELEASE_READINESS.md
@@ -2,110 +2,97 @@

 ## Decision

-**Not ready for release.** The current worktree has meaningful security,
-recovery, and simulator-integration improvements, and the local validation
-listed below passes. The product still lacks several core workflows named in
-the v0.3.0 scope, and cross-platform desktop/CI validation is incomplete.
+**Not ready for release.** The checkout starts at `b907550` on `main`; the
+worktree now contains uncommitted ROS package, TF, graph, SDF-validation, and
+initial URDF-preview improvements.
+The hosted CI run for `b907550` passed all three jobs, but it does not include
+the current uncommitted changes. Several major v0.3.0 acceptance criteria
+remain incomplete. No commit, push, tag, or release was created.

-The checkout is on `main` at `83e88f6` with uncommitted changes. No commit,
-push, tag, or release was created.
-
-## Scope checklist
+## Acceptance checklist

 | # | Area | Status | Evidence and remaining work |
 |---|---|---|---|
-| 1 | gRPC authentication and transport | Pass (local) | Bearer-token requests use TLS with a client-pinned CA; the Docker integration rejects unauthenticated calls and accepts authenticated calls. |
-| 2 | Trusted ROS service installation | Pass (local) | The runtime service and generated protobufs are installed outside the writable workspace mount. Workspace migration compares complete file bytes and uses approved-root operations. |
-| 3 | Workspace filesystem boundaries | Partial | Unix descriptor-rooted reads, writes, imports, and template copies reject symlink escapes. Windows does not yet have equivalent resistance to parent-directory races. |
-| 4 | Dependency security | Partial | `cargo audit` and `npm audit` report zero vulnerabilities. RustSec still reports 13 allowed unmaintained/unsound dependency warnings; see below. |
-| 5 | Critical ROS event recovery | Partial | Critical lifecycle and safety records survive service restart and replay with sequence/gap metadata. Telemetry is best-effort, and the UI does not acknowledge delivery; end-to-end exactly-once delivery is not provided. |
-| 6 | Motion bounds and emergency stop | Partial | The backend enforces conservative per-topic motion limits and a software stop latch. This is not a physical or safety-rated emergency-stop system. |
-| 7 | Workspace selection and isolation | Partial | The backend persists and revalidates the selected workspace. Workspace propagation and cross-platform isolation have not been verified across every operation. |
-| 8 | Build process control | Partial | `colcon build` has a dedicated cancellation path. There is no centralized process registry, general timeout policy, or unified status model. |
-| 9 | Container and desktop lifecycle | Partial | A manual ROS 2 Jazzy/Gazebo Harmonic container integration passes. The packaged desktop app's full startup, shutdown, and supported-platform lifecycle has not been verified. |
-| 10 | ROS node discovery | Partial | Node snapshots are available; a complete graph explorer and detailed endpoint state are not implemented. |
-| 11 | ROS topic discovery | Partial | Topic snapshots and tested ROS-Gazebo bridges are available; detailed publisher/subscriber endpoint and QoS exploration is incomplete. |
-| 12 | ROS service exploration | Not implemented | The UI does not enumerate or inspect ROS services. |
-| 13 | ROS action exploration | Not implemented | The UI does not enumerate or inspect ROS actions. |
-| 14 | TF hierarchy validation | Partial | TF edges are displayed, but a complete validated hierarchy is not maintained. |
-| 15 | TF stale-frame and cycle handling | Not implemented | Stale-frame expiry and cycle detection are absent. |
-| 16 | Robot visualization fidelity | Partial | The 3D view uses placeholder geometry rather than a model derived from the robot description. |
-| 17 | URDF parsing | Not implemented | Robot descriptions are not parsed for visualization. |
-| 18 | Xacro processing | Not implemented | Xacro expansion is not implemented. |
-| 19 | SDF document validation | Partial | XML structure, SDF root/world presence, supported declared versions, and DTD rejection are checked; full SDFormat schema validation is absent. |
-| 20 | Model and mesh resource resolution | Not implemented | External model/mesh references are not comprehensively resolved or validated. |
-| 21 | Frontend and desktop UX test coverage | Not ready | The frontend type-check/build passes, but automated frontend/UI behavior tests and accessibility review were not run. |
-| 22 | Cross-platform release and CI verification | Not ready | The local Docker integration passes, but the hosted workflow, Windows/Linux desktop runtime, and packaged release artifacts have not been validated. |
-
-## Security review and remediation
-
-Three security issues were addressed during this work:
-
-1. **High — workspace marker spoofing.** A marker substring in a workspace
-   service file was previously accepted as evidence that the file was trusted.
-   Migration now compares the complete contents against bundled files; the
-   service actually runs from the application-installed container directory.
-2. **High — bearer token over plaintext gRPC.** A local process could
-   impersonate the service and capture the token. The client now establishes
-   TLS and validates the server against the CA certificate obtained through
-   Docker's control channel before sending authenticated requests.
-3. **Medium (local filesystem boundary) — symlinked destination during resource
-   copy.** Recursive copies could follow a workspace directory symlink and
-   write outside the approved root. Copies and service migration now use
-   approved-root operations; regression tests cover symlinked directories.
-
-The review did not establish additional exploitable issues in the event
-journal, motion validation, or the descriptor-rooted Unix file operations.
-That is not a substitute for platform-specific testing. Docker control and
-code running with container privileges remain inside the trusted boundary.
-
-## Dependency audit residuals
-
-`cargo audit` found **zero vulnerabilities** and 13 allowed warnings:
-
-| Warning group | Advisory IDs | Notes |
-|---|---|---|
-| `proc-macro-error 1.0.4` | `RUSTSEC-2024-0370` | Unmaintained; no fixed upgrade available. |
-| `unic-* 0.9.0` family | `RUSTSEC-2025-0075`, `-0080`, `-0081`, `-0098`, `-0100` | Unmaintained transitive URL-pattern dependencies; no fixed upgrade available. |
-| `zbus_names`, `zvariant`, `zvariant_derive` | `RUSTSEC-2026-0370`, `-0371`, `-0372` | Unmaintained transitive crates. Some older lockfile entries are not on an active dependency path; newer entries remain in the Tauri dependency tree. |
-| `glib 0.18.5` | `RUSTSEC-2024-0429` | Unsoundness advisory; the suggested GLib 0.20+ upgrade is blocked by the current GTK/Tauri dependency chain. |
-
-The advisories above did not provide CVSS scores in the audit output. These
-warnings are disclosed rather than suppressed.
-
-## Validation performed
-
-All commands below were run against the current worktree:
+| 1 | gRPC authentication and TLS | PARTIAL | Local Docker integration confirms the pinned-TLS authenticated client and rejects unauthenticated calls. Cross-platform and packaged-app transport testing remains. |
+| 2 | Trusted ROS service installation | PARTIAL | Service code and protocol are installed outside the writable workspace mount and exercised in Docker. Upgrade/migration paths need broader platform testing. |
+| 3 | Workspace filesystem boundaries | PARTIAL | Unix descriptor-rooted file operations and package-path containment have regression tests. Windows race resistance and several other operations remain incomplete. |
+| 4 | Dependency security | PARTIAL | Current local `npm audit` reports no vulnerabilities and `cargo audit` reports no vulnerability findings, with 13 allowed warnings for unmaintained/unsound transitive dependencies. Hosted CI has not run on this worktree. |
+| 5 | Critical ROS event recovery | PARTIAL | Journal/replay, gap handling, and service-restart recovery are tested. The UI does not acknowledge event application and telemetry is best-effort. |
+| 6 | Motion bounds and emergency stop | PARTIAL | Rust/Python limits, stop latch, persistence, and restart behavior have tests and integration checks. This is software-only and not safety-rated. |
+| 7 | Workspace selection and isolation | PARTIAL | Selected workspace is persisted and passed to container lookup/build/simulation. Package discovery now filters to source/install trees inside that mounted workspace; full operation/platform matrix remains unverified. |
+| 8 | ROS package explorer | PARTIAL | Selected-workspace package metadata, build type, dependencies, executable outputs, launch/config files, filtering, refresh, and file browsing are implemented and tested. Underlay packages, dependency resolution, and full package content workflows remain out of scope. |
+| 9 | ROS node explorer | PARTIAL | Bounded node names/namespaces and per-topic endpoint node identities are streamed and shown. Full node-specific endpoint grouping and lifecycle details remain. |
+| 10 | ROS topic explorer | PARTIAL | Bounded publisher/subscriber identities and QoS policies are streamed and shown. Safe live-message inspection and subscription controls are not implemented. |
+| 11 | ROS service explorer | PARTIAL | Bounded service names/types and discovered server/client nodes are shown read-only. Structured invocation, validation, timeout, and cancellation remain absent to avoid bypassing safety controls. |
+| 12 | ROS action explorer | PARTIAL | Action names/types and discovered server/client nodes are shown read-only. Goal submission, feedback/results, and cancellation are not implemented. |
+| 13 | Consistent ROS graph refresh | PARTIAL | Nodes, topics, services, and actions are snapshotted from one service timer and bounded. Automatic client subscriptions and full graph transactionality remain absent. |
+| 14 | TF hierarchy and transform inspection | PARTIAL | A UI forest, translation/quaternion/RPY inspector, multiple-root/multiple-parent/cycle checks, and pure unit tests are implemented. It is not authoritative `tf2` validation. |
+| 15 | TF stale and static-frame behavior | PARTIAL | Dynamic edges become stale after five seconds; `/tf_static` uses reliable transient-local QoS and is exempt. A live Docker integration publishes and verifies a static TF edge. Missing authorities cannot be inferred from edge data. |
+| 16 | URDF parsing and validation | PARTIAL | A bounded parser rejects malformed/unsafe XML, validates link/joint-tree structure and primitive dimensions, and reports unsupported mesh visuals. Full URDF semantics, Xacro, and resource validation remain. |
+| 17 | Xacro expansion | NOT STARTED | No controlled ROS Xacro expansion workflow exists. |
+| 18 | Mesh/resource resolution | NOT STARTED | `package://`, relative model paths, and mesh formats are not resolved for rendering. |
+| 19 | Robot model and joint-state rendering | PARTIAL | Active `.urdf` files render box/cylinder/sphere visuals and supported joint transforms; bounded `/joint_states` updates are covered by live Docker integration. No mesh/Xacro, coordinate-convention verification, or visual desktop test yet. |
+| 20 | Process lifecycle manager | PARTIAL | Builds and process-registry data have dedicated support, but build, Docker, Gazebo, bridge, and ROS service processes do not share reliable lifecycle, timeout, cancellation, and shutdown handling. |
+| 21 | Gazebo and SDF | PARTIAL | Jazzy/Harmonic startup, reset, bridge, and pre-launch `gz sdf --check` schema validation are integration-tested. Comprehensive external resource resolution and validation remain absent. |
+| 22 | Frontend workflow tests/accessibility | PARTIAL | Dependency-free TF graph and robot-rendering tests plus production type-check/build pass. React workflow tests, accessibility review, and coverage of workspace/ROS/safety UI remain. |
+| 23 | Cross-platform runtime and release artifacts | BLOCKED | CI has Linux checks and a release workflow matrix for macOS/Linux/Windows, but no current packaged artifacts or runtime verification on every platform were produced. Signing/notarization and upgrade verification remain. |
+
+## Security and safety notes
+
+- Package discovery now runs from the application-installed `server_core.py`
+  in the selected container. It scans only that container's `/workspace/src`
+  and `/workspace/install`, bounds package/file counts and manifest sizes,
+  rejects DTD/entity declarations, reports malformed manifests, and skips
+  symlinked package directories. Rust canonicalizes returned relative paths
+  and checks that they remain under the selected host workspace before
+  exposing them to the approved-root file browser.
+- The package command no longer interpolates a host-controlled path into a
+  shell script. The active container is selected from the validated workspace
+  label and uses its fixed `/workspace` mount.
+- TF validation is diagnostic. It does not replace ROS `tf2`, identify
+  transform authorities, or make motion-safety guarantees.
+- Existing TLS/authentication, emergency-stop, and filesystem controls were
+  not intentionally weakened. No new independent security audit was performed
+  for this implementation pass.
+- The software emergency stop cannot guarantee physical braking or replace a
+  safety-rated controller or physical emergency-stop circuit.
+
+## Validation performed on the current worktree

 | Check | Result |
 |---|---|
-| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Pass |
-| `cargo check --manifest-path src-tauri/Cargo.toml --locked` | Pass |
-| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | Pass: 32 passed, 1 ignored (the ignored test requires the live service). |
-| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets --all-features -- -D warnings` | Pass |
-| `python3 -B -m unittest test_server_core` from `src-tauri/resources/devcontainer` | Pass: 36 tests. |
-| `npm run build` | Pass; Vite reports the existing ~811 kB JavaScript chunk-size warning. |
+| `npm run build` | Pass; Vite reports an existing large JavaScript chunk warning (~820 kB). |
+| `npm run test:frontend` | Pass: 7 TF graph and robot-description tests (Node 26.5.0). |
+| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Pass. |
+| `cargo check --manifest-path src-tauri/Cargo.toml --locked` | Pass. |
+| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | Pass: 39 passed, 1 ignored (live-service test). |
+| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features --locked -- -D warnings` | Pass. |
+| `python3 -B -m unittest -v test_server_core` from `src-tauri/resources/devcontainer` | Pass: 42 tests, including package discovery/isolation and action endpoint parsing. |
+| Python `py_compile` for service and tests | Pass. |
+| `bash -n scripts/docker-integration.sh` | Pass. |
+| `scripts/docker-integration.sh` | Pass: selected-workspace package discovery, live graph node/topic/service/action endpoints, `/tf_static` and `/joint_states` delivery, valid/invalid SDFormat checks, pinned TLS/auth, stop/reset enforcement, replay and safety history after restart, Gazebo readiness/reset and topic bridge. Cleanup exited 0 and left no integration container or temporary workspace. |
+| Protocol-copy comparison | Pass: Rust and container protobuf definitions match. |
+| `cargo audit --file src-tauri/Cargo.lock` | Pass with 13 allowed warnings: no vulnerability findings; remaining advisories are unmaintained/unsound transitive dependencies disclosed in the previous readiness review. Patched Rustix advisory was fixed by updating to 1.1.6. |
 | `npm audit --audit-level=moderate` | Pass: 0 vulnerabilities. |
-| `cargo audit` from `src-tauri` | Pass: 0 vulnerabilities, 13 allowed warnings. |
-| `scripts/docker-integration.sh` | Pass: pinned TLS/auth, safety latch/reset, event replay after restart, Gazebo world readiness/reset, and ROS-Gazebo topic bridge. No robot motion was commanded. |
-| `git diff --check`, protocol-copy comparison, script syntax check | Pass. |
-
-The Docker integration is a manually run container harness, not a desktop UI
-test or hosted CI run. Its successful result does not establish support for all
-target operating systems or physical robot safety.
+| Hosted GitHub Actions | Pass for baseline commit `b907550` in run `43087023826` (frontend, Rust/integration, and Python jobs); not run against this worktree's changes. |

 ## Release blockers

-- Implement or explicitly defer full ROS package, service, and action
-  exploration.
-- Add a centralized process lifecycle/status/timeout model and verify shutdown
-  and cancellation in the managed desktop runtime.
-- Complete TF hierarchy validation and stale/cycle handling.
-- Implement description-based robot rendering, including a declared URDF,
-  Xacro, and mesh-resource support boundary.
-- Decide whether SDF import requires full schema and external-resource
-  validation; implement it or clearly constrain supported input.
-- Complete Windows/Linux runtime and filesystem testing, frontend behavior
-  tests, and hosted CI validation.
-- Reassess the disclosed RustSec warnings when compatible upstream dependency
-  upgrades are available.
+- Complete live ROS node/topic endpoint details and implement service/action
+  safe structured invocation/goal monitoring without bypassing the motion
+  safety controls. Bounded read-only graph discovery is implemented.
+- Validate TF reconnect/disappearance behavior against a live ROS publisher;
+  static `/tf_static` delivery is covered by the Docker integration test.
+- Complete URDF semantics and coordinate validation; implement Xacro expansion
+  and safe package/model resource resolution. The current bounded parser,
+  primitive preview, and live joint-state updates are partial support only.
+- Unify application-managed process state, failure reporting, timeouts,
+  cancellation confirmation, and shutdown cleanup across supported platforms.
+- Complete filesystem race hardening for remaining operations and Windows.
+- Add frontend workflow tests and conduct accessibility review.
+- Run hosted CI on the modified worktree; verify Linux, Windows, macOS Intel,
+  and Apple Silicon packaging/runtime paths and test upgrades from v0.2.0.
+- Complete external SDF model/mesh resource resolution and validation.
+- Re-run current dependency audits and reassess disclosed RustSec advisories.
+- Prepare and verify unsigned/signed release artifacts as applicable; do not
+  publish a tag or release until explicitly approved.
diff --git a/docs/architecture.md b/docs/architecture.md
index b1dbb22..d7a8dc2 100644
--- a/docs/architecture.md
+++ b/docs/architecture.md
@@ -85,23 +85,45 @@ coverage remain incomplete.

 ## Supported ROS surface

-The current service streams node and topic snapshots, transform and odometry
-updates, safety state, and event-recovery metadata. It is not a full ROS graph
-explorer: detailed publisher/subscriber endpoints, services, and actions are
-not exposed. TF updates are visualized as received edges but are not yet
-validated as a complete hierarchy with stale-frame expiry and cycle detection.
-The 3D view uses placeholder geometry; URDF parsing, Xacro processing, mesh
-resource resolution, and description-based robot rendering are not implemented.
+The current service streams bounded node, topic endpoint/QoS, service, and
+action snapshots alongside transform and odometry updates, safety state, and
+event-recovery metadata. The package explorer scans package manifests in the
+selected workspace's source and install trees rather than treating ROS
+underlay packages as workspace packages. It reports bounded metadata,
+installed executables, launch/configuration files, and allows the frontend to
+browse package files through approved-root file commands. The graph explorer
+is read-only: it does not invoke arbitrary ROS services or submit action goals,
+which could otherwise bypass the application's motion safety path.
+
+TF updates include their source topic. `/tf_static` uses transient-local
+reliable QoS so late subscribers receive latched static transforms. The
+frontend constructs a transform forest, warns on multiple roots, cycles,
+multiple parents, stale dynamic transforms, non-finite values, and invalid
+quaternions, and provides translation/quaternion/RPY inspection. It cannot
+infer missing transform authorities from edge data alone, and is diagnostic
+validation rather than a replacement for `tf2`.
+
+When an active editor file is a `.urdf`, the native backend parses a bounded
+URDF subset and the frontend renders box, cylinder, and sphere visuals with a
+connected link tree. Fixed, revolute, continuous, and prismatic joint states
+are updated from bounded `/joint_states` telemetry. Mesh visuals are surfaced
+as warnings and skipped. Xacro expansion, mesh/package resource resolution,
+floating/planar joints, and full URDF semantics are not implemented. The
+ROS-to-three.js coordinate mapping still needs broader convention validation.

 SDF import checks XML well-formedness, an SDF root, a world element, supported
-declared versions, and rejects DTDs. It does not perform full SDFormat schema
-validation or resolve and validate all external model and mesh resources.
+declared versions, and rejects DTDs. Before launching Gazebo, the container's
+`gz sdf --check` validates the selected world against the installed SDFormat
+schema. External model and mesh resources are not comprehensively resolved or
+validated.

 ## Validation boundary

 Run unit/build checks with the commands in the README. The live integration
 script `scripts/docker-integration.sh` builds the container image and exercises
-the real ROS service, pinned TLS, event replay after restart, Gazebo world
-readiness/reset, and ROS-Gazebo topic bridging. This does not substitute for
-testing the desktop UI on every target operating system or testing a physical
-robot.
+the real ROS service, selected-workspace package discovery, pinned TLS, event
+replay after restart, live `/tf_static` and `/joint_states` delivery, Gazebo
+world readiness/reset, and ROS-Gazebo topic bridging. Unit tests cover TF graph
+validation, URDF parsing, primitive/joint rendering, and package metadata
+discovery. This does not substitute for visual desktop validation on every
+target operating system or testing a physical robot.
diff --git a/package.json b/package.json
index 4c6b264..8167567 100644
--- a/package.json
+++ b/package.json
@@ -7,7 +7,8 @@
     "dev": "vite",
     "build": "tsc && vite build",
     "preview": "vite preview",
-    "tauri": "tauri"
+    "tauri": "tauri",
+    "test:frontend": "node --experimental-strip-types --test tests/*.test.mjs"
   },
   "dependencies": {
     "@monaco-editor/react": "^4.7.0",
diff --git a/scripts/docker-integration.sh b/scripts/docker-integration.sh
index 43c78e0..05852ff 100755
--- a/scripts/docker-integration.sh
+++ b/scripts/docker-integration.sh
@@ -67,6 +67,30 @@ docker build -t "$image" "$repo_root/src-tauri/resources/devcontainer"

 cp "$repo_root/src-tauri/resources/devcontainer/diff_drive.sdf" "$workspace/diff_drive.sdf"
 printf 'workspace-mount-ok\n' > "$workspace/mount-check"
+mkdir -p "$workspace/worlds"
+printf '<sdf version="1.9"><world name="invalid"><model name="bad"><link name="bad"><visual name="bad"><geometry><box><size>1</size></box></geometry></visual></link></model></world></sdf>\n' \
+  > "$workspace/worlds/invalid-schema.sdf"
+package_dir="$workspace/src/robotics_studio_integration_fixture"
+mkdir -p "$package_dir/launch" "$package_dir/config"
+cat > "$package_dir/package.xml" <<'XML'
+<package format="3">
+  <name>robotics_studio_integration_fixture</name>
+  <version>1.0.0</version>
+  <description>Temporary package discovery integration fixture</description>
+  <buildtool_depend>ament_cmake</buildtool_depend>
+  <depend>rclpy</depend>
+</package>
+XML
+printf '<launch/>\n' > "$package_dir/launch/start.launch.xml"
+printf 'enabled: true\n' > "$package_dir/config/settings.yaml"
+mkdir -p "$workspace/install/lib/robotics_studio_integration_fixture"
+printf '#!/bin/sh\n' > "$workspace/install/lib/robotics_studio_integration_fixture/fixture_node"
+chmod +x "$workspace/install/lib/robotics_studio_integration_fixture/fixture_node"
+mkdir -p "$workspace/other_workspace/src/not_selected"
+printf '<package><name>not_selected</name></package>\n' \
+  > "$workspace/other_workspace/src/not_selected/package.xml"
+mkdir -p "$workspace/src/malformed_fixture"
+printf '<package>\n' > "$workspace/src/malformed_fixture/package.xml"
 container_may_exist=1
 docker run -d \
   --name "$container" \
@@ -85,6 +109,32 @@ docker cp "$repo_root/src-tauri/resources/devcontainer/server_core.py" \
 docker cp "$repo_root/src-tauri/resources/devcontainer/proto/ros_engine.proto" \
   "$container:/opt/robotics-studio/proto/ros_engine.proto" >/dev/null

+docker exec -i "$container" bash -c 'cd /opt/robotics-studio && python3 -' <<'PY'
+import sys
+
+sys.path.insert(0, "/opt/robotics-studio")
+from server_core import discover_workspace_packages
+
+packages = {
+    package["name"]: package
+    for package in discover_workspace_packages("/workspace")
+}
+assert set(packages) == {
+    "malformed_fixture",
+    "robotics_studio_integration_fixture",
+}, sorted(packages)
+fixture = packages["robotics_studio_integration_fixture"]
+assert fixture["package_type"] == "ament_cmake"
+assert fixture["version"] == "1.0.0"
+assert fixture["description"] == "Temporary package discovery integration fixture"
+assert fixture["dependencies"] == ["ament_cmake", "rclpy"]
+assert fixture["executables"] == ["fixture_node"]
+assert fixture["launch_files"] == ["launch/start.launch.xml"]
+assert fixture["config_files"] == ["config/settings.yaml"]
+assert packages["malformed_fixture"]["metadata_error"]
+print("Selected-workspace ROS package discovery passed")
+PY
+
 docker exec "$container" bash -lc \
   'source /opt/ros/jazzy/setup.bash && python3 -m grpc_tools.protoc -I /opt/robotics-studio/proto --python_out=/opt/robotics-studio/generated --grpc_python_out=/opt/robotics-studio/generated /opt/robotics-studio/proto/ros_engine.proto'

@@ -138,8 +188,162 @@ start_ros_server() {
 }

 test "$(docker exec "$container" cat /workspace/mount-check)" = workspace-mount-ok
+docker exec "$container" bash -lc \
+  'source /opt/ros/jazzy/setup.bash && gz sdf --check /workspace/diff_drive.sdf'
+if docker exec "$container" bash -lc \
+  'source /opt/ros/jazzy/setup.bash && gz sdf --check /workspace/worlds/invalid-schema.sdf' \
+  >/dev/null 2>&1; then
+  echo "Gazebo accepted an invalid SDF world" >&2
+  exit 1
+fi
+echo "SDFormat schema validation accepted the valid world and rejected invalid geometry"
 start_ros_server

+docker exec -i "$container" bash -lc \
+  'source /opt/ros/jazzy/setup.bash && cd /opt/robotics-studio && python3 -' <<'PY'
+import sys
+import time
+
+sys.path.insert(0, "/opt/robotics-studio/generated")
+import grpc
+import rclpy
+import ros_engine_pb2 as pb
+import ros_engine_pb2_grpc as rpc
+from action_tutorials_interfaces.action import Fibonacci
+from geometry_msgs.msg import TransformStamped
+from rclpy.action import ActionClient, ActionServer
+from rclpy.qos import DurabilityPolicy, QoSProfile, ReliabilityPolicy
+from sensor_msgs.msg import JointState
+from std_srvs.srv import Trigger
+from tf2_msgs.msg import TFMessage
+
+certificate = open("/run/robotics-studio/grpc-ca.crt", "rb").read()
+token = open("/run/robotics-studio/grpc-auth.json", encoding="utf-8").read()
+import json
+
+metadata = (("x-robotics-studio-token", json.loads(token)["token"]),)
+stub = rpc.RosEngineStub(
+    grpc.secure_channel(
+        "127.0.0.1:50051",
+        grpc.ssl_channel_credentials(root_certificates=certificate),
+    )
+)
+events = stub.ReplayEvents(
+    pb.ReplayEventsRequest(has_after_sequence=True, after_sequence=0),
+    metadata=metadata,
+    timeout=10,
+)
+
+rclpy.init()
+node = rclpy.create_node("robotics_studio_tf_integration")
+service = node.create_service(
+    Trigger,
+    "/integration/echo",
+    lambda request, response: response,
+)
+service_client = node.create_client(Trigger, "/integration/echo")
+action_server = ActionServer(
+    node,
+    Fibonacci,
+    "/integration/fibonacci",
+    lambda goal_handle: Fibonacci.Result(),
+)
+action_client = ActionClient(node, Fibonacci, "/integration/fibonacci")
+publisher = node.create_publisher(
+    TFMessage,
+    "/tf_static",
+    QoSProfile(
+        depth=10,
+        reliability=ReliabilityPolicy.RELIABLE,
+        durability=DurabilityPolicy.TRANSIENT_LOCAL,
+    ),
+)
+joint_state_publisher = node.create_publisher(JointState, "/joint_states", 10)
+deadline = time.monotonic() + 8
+while (
+    publisher.get_subscription_count() == 0
+    or joint_state_publisher.get_subscription_count() == 0
+) and time.monotonic() < deadline:
+    rclpy.spin_once(node, timeout_sec=0.1)
+assert publisher.get_subscription_count() > 0, "ROS service did not subscribe to /tf_static"
+assert (
+    joint_state_publisher.get_subscription_count() > 0
+), "ROS service did not subscribe to /joint_states"
+
+transform = TransformStamped()
+transform.header.frame_id = "map"
+transform.child_frame_id = "integration_base"
+transform.transform.translation.x = 0.25
+transform.transform.rotation.w = 1.0
+publisher.publish(TFMessage(transforms=[transform]))
+joint_state = JointState()
+joint_state.name = ["integration_joint"]
+joint_state.position = [0.5]
+joint_state_publisher.publish(joint_state)
+
+tf_verified = False
+graph_verified = False
+joint_state_verified = False
+for event in events:
+    event_type = event.WhichOneof("event")
+    if event_type == "tf_update":
+        assert event.tf_update.parent_frame == "map"
+        assert event.tf_update.child_frame == "integration_base"
+        assert event.tf_update.topic_name == "/tf_static"
+        assert abs(event.tf_update.x - 0.25) < 1e-9
+        tf_verified = True
+    elif event_type == "joint_state_update":
+        joint_state_verified = (
+            event.joint_state_update.topic_name == "/joint_states"
+            and event.joint_state_update.names == ["integration_joint"]
+            and list(event.joint_state_update.positions) == [0.5]
+        )
+    elif event_type == "ros_graph_snapshot":
+        graph = event.ros_graph_snapshot
+        tf_topic = next(
+            (topic for topic in graph.topics if topic.name == "/tf_static"),
+            None,
+        )
+        service_info = next(
+            (service for service in graph.services if service.name == "/integration/echo"),
+            None,
+        )
+        action_info = next(
+            (action for action in graph.actions if action.name == "/integration/fibonacci"),
+            None,
+        )
+        if tf_topic and service_info and action_info:
+            graph_verified = (
+                any(
+                endpoint.node_name == "robotics_studio_tf_integration"
+                and endpoint.reliability == "RELIABLE"
+                and endpoint.durability == "TRANSIENT_LOCAL"
+                for endpoint in tf_topic.publishers
+                )
+                and "/robotics_studio_tf_integration" in service_info.server_nodes
+                and "/robotics_studio_tf_integration" in service_info.client_nodes
+                and action_info.types
+                == ["action_tutorials_interfaces/action/Fibonacci"]
+                and "/robotics_studio_tf_integration" in action_info.server_nodes
+                and "/robotics_studio_tf_integration" in action_info.client_nodes
+            )
+    if tf_verified and graph_verified and joint_state_verified:
+        break
+else:
+    raise AssertionError(
+        "ROS service did not stream TF, joint-state, and graph details"
+    )
+if not graph_verified:
+    raise AssertionError("ROS graph snapshot omitted the /tf_static publisher endpoint")
+if not joint_state_verified:
+    raise AssertionError("ROS service did not stream the joint-state update")
+
+events.cancel()
+node.destroy_node()
+rclpy.shutdown()
+print("Live ROS TF, joint-state, and graph endpoint discovery passed")
+PY
+
 ROBOTICS_STUDIO_TEST_TLS_CERTIFICATE="$workspace/grpc-ca.crt" \
 ROBOTICS_STUDIO_TEST_AUTH_TOKEN="$token" \
   cargo test --manifest-path "$repo_root/src-tauri/Cargo.toml" --locked -- \
diff --git a/src-tauri/Cargo.lock b/src-tauri/Cargo.lock
index 0dbebe6..6ceb3ad 100644
--- a/src-tauri/Cargo.lock
+++ b/src-tauri/Cargo.lock
@@ -1091,7 +1091,7 @@ source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "39cab71617ae0d63f51a36d69f866391735b51691dbda63cf6f96d042b63efeb"
 dependencies = [
  "libc",
- "windows-sys 0.52.0",
+ "windows-sys 0.61.2",
 ]

 [[package]]
@@ -3229,7 +3229,7 @@ dependencies = [
  "once_cell",
  "socket2",
  "tracing",
- "windows-sys 0.52.0",
+ "windows-sys 0.61.2",
 ]

 [[package]]
@@ -3505,15 +3505,15 @@ dependencies = [

 [[package]]
 name = "rustix"
-version = "1.1.4"
+version = "1.1.6"
 source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "b6fe4565b9518b83ef4f91bb47ce29620ca828bd32cb7e408f0062e9930ba190"
+checksum = "e5e76742e5294153d4dde1207dbec5bdd81eef6fabdc0d2118babadabfd858aa"
 dependencies = [
  "bitflags 2.13.1",
  "errno",
  "libc",
  "linux-raw-sys",
- "windows-sys 0.52.0",
+ "windows-sys 0.61.2",
 ]

 [[package]]
@@ -3572,7 +3572,7 @@ dependencies = [
  "security-framework 3.7.0",
  "security-framework-sys",
  "webpki-root-certs",
- "windows-sys 0.52.0",
+ "windows-sys 0.61.2",
 ]

 [[package]]
@@ -4554,10 +4554,10 @@ source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "32497e9a4c7b38532efcdebeef879707aa9f794296a4f0244f6f69e9bc8574bd"
 dependencies = [
  "fastrand",
- "getrandom 0.3.4",
+ "getrandom 0.4.3",
  "once_cell",
  "rustix",
- "windows-sys 0.52.0",
+ "windows-sys 0.61.2",
 ]

 [[package]]
@@ -5432,7 +5432,7 @@ version = "0.1.11"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "c2a7b1c03c876122aa43f3020e6c3c3ee5c05081c9a00739faf7503aeba10d22"
 dependencies = [
- "windows-sys 0.52.0",
+ "windows-sys 0.61.2",
 ]

 [[package]]
diff --git a/src-tauri/Cargo.toml b/src-tauri/Cargo.toml
index e7ec03d..1e9b7db 100644
--- a/src-tauri/Cargo.toml
+++ b/src-tauri/Cargo.toml
@@ -35,4 +35,4 @@ getrandom = "0.3.4"
 keyring = { version = "3.6.3", default-features = false, features = ["apple-native", "windows-native", "linux-native-sync-persistent", "crypto-rust"] }

 [target.'cfg(unix)'.dependencies]
-rustix = { version = "1.1.4", features = ["fs"] }
+rustix = { version = "1.1.6", features = ["fs"] }
diff --git a/src-tauri/proto/ros_engine.proto b/src-tauri/proto/ros_engine.proto
index 02ad148..b92cfca 100644
--- a/src-tauri/proto/ros_engine.proto
+++ b/src-tauri/proto/ros_engine.proto
@@ -94,11 +94,64 @@ message WorkspaceEvent {
     SafetyState safety_state = 7;
     EventHealth event_health = 8;
     EventRecovery event_recovery = 9;
+    RosGraphSnapshot ros_graph_snapshot = 12;
+    JointStateUpdate joint_state_update = 13;
   }
   int64 timestamp_ns = 10;
   uint64 sequence = 11;
 }

+message JointStateUpdate {
+  string topic_name = 1;
+  repeated string names = 2;
+  repeated double positions = 3;
+  bool truncated = 4;
+}
+
+message RosGraphSnapshot {
+  repeated GraphNode nodes = 1;
+  repeated GraphTopic topics = 2;
+  repeated GraphService services = 3;
+  repeated GraphAction actions = 4;
+  bool truncated = 5;
+}
+
+message GraphNode {
+  string name = 1;
+  string namespace = 2;
+}
+
+message GraphEndpoint {
+  string node_name = 1;
+  string node_namespace = 2;
+  string topic_type = 3;
+  string reliability = 4;
+  string durability = 5;
+  string history = 6;
+  uint32 depth = 7;
+}
+
+message GraphTopic {
+  string name = 1;
+  repeated string types = 2;
+  repeated GraphEndpoint publishers = 3;
+  repeated GraphEndpoint subscribers = 4;
+}
+
+message GraphService {
+  string name = 1;
+  repeated string types = 2;
+  repeated string server_nodes = 3;
+  repeated string client_nodes = 4;
+}
+
+message GraphAction {
+  string name = 1;
+  repeated string types = 2;
+  repeated string server_nodes = 3;
+  repeated string client_nodes = 4;
+}
+
 message EventRecovery {
   bool gap_detected = 1;
   uint64 oldest_available_sequence = 2;
@@ -121,6 +174,7 @@ message TFUpdate {
   double qy = 7;
   double qz = 8;
   double qw = 9;
+  string topic_name = 10;
 }

 message NodeStarted {
diff --git a/src-tauri/resources/devcontainer/proto/ros_engine.proto b/src-tauri/resources/devcontainer/proto/ros_engine.proto
index 02ad148..b92cfca 100644
--- a/src-tauri/resources/devcontainer/proto/ros_engine.proto
+++ b/src-tauri/resources/devcontainer/proto/ros_engine.proto
@@ -94,11 +94,64 @@ message WorkspaceEvent {
     SafetyState safety_state = 7;
     EventHealth event_health = 8;
     EventRecovery event_recovery = 9;
+    RosGraphSnapshot ros_graph_snapshot = 12;
+    JointStateUpdate joint_state_update = 13;
   }
   int64 timestamp_ns = 10;
   uint64 sequence = 11;
 }

+message JointStateUpdate {
+  string topic_name = 1;
+  repeated string names = 2;
+  repeated double positions = 3;
+  bool truncated = 4;
+}
+
+message RosGraphSnapshot {
+  repeated GraphNode nodes = 1;
+  repeated GraphTopic topics = 2;
+  repeated GraphService services = 3;
+  repeated GraphAction actions = 4;
+  bool truncated = 5;
+}
+
+message GraphNode {
+  string name = 1;
+  string namespace = 2;
+}
+
+message GraphEndpoint {
+  string node_name = 1;
+  string node_namespace = 2;
+  string topic_type = 3;
+  string reliability = 4;
+  string durability = 5;
+  string history = 6;
+  uint32 depth = 7;
+}
+
+message GraphTopic {
+  string name = 1;
+  repeated string types = 2;
+  repeated GraphEndpoint publishers = 3;
+  repeated GraphEndpoint subscribers = 4;
+}
+
+message GraphService {
+  string name = 1;
+  repeated string types = 2;
+  repeated string server_nodes = 3;
+  repeated string client_nodes = 4;
+}
+
+message GraphAction {
+  string name = 1;
+  repeated string types = 2;
+  repeated string server_nodes = 3;
+  repeated string client_nodes = 4;
+}
+
 message EventRecovery {
   bool gap_detected = 1;
   uint64 oldest_available_sequence = 2;
@@ -121,6 +174,7 @@ message TFUpdate {
   double qy = 7;
   double qz = 8;
   double qw = 9;
+  string topic_name = 10;
 }

 message NodeStarted {
diff --git a/src-tauri/resources/devcontainer/server.py b/src-tauri/resources/devcontainer/server.py
index 68cbdbb..44521b7 100644
--- a/src-tauri/resources/devcontainer/server.py
+++ b/src-tauri/resources/devcontainer/server.py
@@ -9,6 +9,7 @@ from concurrent import futures

 import rclpy
 from rclpy.node import Node
+from rclpy.qos import DurabilityPolicy, QoSProfile, ReliabilityPolicy

 AUTH_PROTOCOL_VERSION = 1

@@ -26,6 +27,7 @@ from tf2_msgs.msg import TFMessage

 from nav_msgs.msg import Odometry
 from geometry_msgs.msg import Twist
+from sensor_msgs.msg import JointState
 from server_core import (
     CommandLeaseManager,
     EventPersistenceError,
@@ -36,10 +38,22 @@ from server_core import (
     SafetyController,
     is_valid_auth_token,
     load_auth_credentials,
+    action_from_send_goal_service,
     validate_twist_request,
 )

 LOGGER = logging.getLogger("ros_engine")
+MAX_GRAPH_NODES = 256
+MAX_GRAPH_TOPICS = 512
+MAX_GRAPH_SERVICES = 512
+MAX_GRAPH_ACTIONS = 128
+MAX_GRAPH_ENDPOINTS = 2048
+MAX_JOINT_STATE_ENTRIES = 512
+
+
+def _qos_policy_name(value):
+    return getattr(value, "name", str(value).rsplit(".", 1)[-1])
+

 class RosIntrospector(Node):
     def __init__(self, event_hub: EventHub):
@@ -69,6 +83,7 @@ class RosIntrospector(Node):
             0.1, self.stop_expired_commands
         )
         self.tf_subscribers = {}
+        self.joint_state_subscribers = {}

     def _publish_stop_to_all(self):
         with self._command_lock:
@@ -135,6 +150,7 @@ class RosIntrospector(Node):
                     qy=transform.transform.rotation.y,
                     qz=transform.transform.rotation.z,
                     qw=transform.transform.rotation.w,
+                    topic_name=topic_name,
                 ),
                 timestamp_ns=time.time_ns()
             )
@@ -144,6 +160,27 @@ class RosIntrospector(Node):
                 key=("tf", transform.child_frame_id),
             )

+    def joint_state_callback(self, msg, topic_name):
+        names = list(msg.name[:MAX_JOINT_STATE_ENTRIES])
+        positions = list(msg.position[:MAX_JOINT_STATE_ENTRIES])
+        event = ros_engine_pb2.WorkspaceEvent(
+            joint_state_update=ros_engine_pb2.JointStateUpdate(
+                topic_name=topic_name,
+                names=names,
+                positions=positions,
+                truncated=(
+                    len(msg.name) > MAX_JOINT_STATE_ENTRIES
+                    or len(msg.position) > MAX_JOINT_STATE_ENTRIES
+                ),
+            ),
+            timestamp_ns=time.time_ns(),
+        )
+        self.event_hub.publish(
+            event,
+            priority="telemetry",
+            key=("joint_state", topic_name),
+        )
+
     def tick(self):
         try:
             self._tick_impl()
@@ -204,6 +241,12 @@ class RosIntrospector(Node):
         for name in stale_tf:
             del self.tf_subscribers[name]

+        stale_joint_state = (
+            set(self.joint_state_subscribers.keys()) - current_topic_names
+        )
+        for name in stale_joint_state:
+            del self.joint_state_subscribers[name]
+
         for name, types in topics:
             if 'nav_msgs/msg/Odometry' in types and name not in self.odom_subscribers:
                 sub = self.create_subscription(
@@ -216,14 +259,37 @@ class RosIntrospector(Node):

         for name, types in topics:
             if 'tf2_msgs/msg/TFMessage' in types and name not in self.tf_subscribers:
+                qos = (
+                    QoSProfile(
+                        depth=100,
+                        reliability=ReliabilityPolicy.RELIABLE,
+                        durability=DurabilityPolicy.TRANSIENT_LOCAL,
+                    )
+                    if name.endswith("/tf_static")
+                    else 10
+                )
                 sub = self.create_subscription(
                     TFMessage, name,
                     lambda msg, n=name: self.tf_callback(msg, n),
-                    10
+                    qos
                 )
                 self.tf_subscribers[name] = sub
                 self.get_logger().info(f'Subscribed to TF topic: {name}')

+        for name, types in topics:
+            if (
+                "sensor_msgs/msg/JointState" in types
+                and name not in self.joint_state_subscribers
+            ):
+                subscription = self.create_subscription(
+                    JointState,
+                    name,
+                    lambda msg, topic=name: self.joint_state_callback(msg, topic),
+                    10,
+                )
+                self.joint_state_subscribers[name] = subscription
+                self.get_logger().info(f"Subscribed to joint state topic: {name}")
+
         topic_msgs = [
             ros_engine_pb2.Topic(name=n, types=list(t)) for n, t in topics
         ]
@@ -257,6 +323,139 @@ class RosIntrospector(Node):
             key="event_health",
             snapshot=True,
         )
+        self.publish_graph_snapshot(topics)
+
+    def publish_graph_snapshot(self, topics):
+        graph = ros_engine_pb2.RosGraphSnapshot()
+        truncated = False
+
+        node_entries = sorted(self.get_node_names_and_namespaces())
+        if len(node_entries) > MAX_GRAPH_NODES:
+            truncated = True
+        node_entries = node_entries[:MAX_GRAPH_NODES]
+        graph.nodes.extend(
+            ros_engine_pb2.GraphNode(name=name, namespace=namespace)
+            for name, namespace in node_entries
+        )
+
+        service_roles = {}
+        for node_name, node_namespace in node_entries:
+            node_id = f"{node_namespace.rstrip('/')}/{node_name}"
+            for service_name, _ in self.get_service_names_and_types_by_node(
+                node_name, node_namespace
+            ):
+                service_roles.setdefault(service_name, {"servers": [], "clients": []})[
+                    "servers"
+                ].append(node_id)
+            for service_name, _ in self.get_client_names_and_types_by_node(
+                node_name, node_namespace
+            ):
+                service_roles.setdefault(service_name, {"servers": [], "clients": []})[
+                    "clients"
+                ].append(node_id)
+
+        endpoint_count = 0
+        topic_entries = sorted(topics)
+        if len(topic_entries) > MAX_GRAPH_TOPICS:
+            truncated = True
+        for topic_name, types in topic_entries[:MAX_GRAPH_TOPICS]:
+            topic = ros_engine_pb2.GraphTopic(name=topic_name, types=sorted(types))
+            for field, query in (
+                ("publishers", self.get_publishers_info_by_topic),
+                ("subscribers", self.get_subscriptions_info_by_topic),
+            ):
+                remaining = MAX_GRAPH_ENDPOINTS - endpoint_count
+                if remaining <= 0:
+                    truncated = True
+                    break
+                endpoints = sorted(
+                    query(topic_name),
+                    key=lambda endpoint: (
+                        endpoint.node_namespace,
+                        endpoint.node_name,
+                        endpoint.topic_type,
+                    ),
+                )
+                if len(endpoints) > remaining:
+                    truncated = True
+                selected = endpoints[:remaining]
+                endpoint_count += len(selected)
+                getattr(topic, field).extend(
+                    ros_engine_pb2.GraphEndpoint(
+                        node_name=endpoint.node_name,
+                        node_namespace=endpoint.node_namespace,
+                        topic_type=endpoint.topic_type,
+                        reliability=_qos_policy_name(
+                            endpoint.qos_profile.reliability
+                        ),
+                        durability=_qos_policy_name(
+                            endpoint.qos_profile.durability
+                        ),
+                        history=_qos_policy_name(endpoint.qos_profile.history),
+                        depth=endpoint.qos_profile.depth,
+                    )
+                    for endpoint in selected
+                )
+            graph.topics.append(topic)
+
+        service_entries = sorted(self.get_service_names_and_types())
+        if len(service_entries) > MAX_GRAPH_SERVICES:
+            truncated = True
+        action_services = {}
+        for service_name, types in service_entries:
+            role = service_roles.get(service_name, {"servers": [], "clients": []})
+            action_info = None
+            for service_type in types:
+                action_info = action_from_send_goal_service(
+                    service_name, service_type
+                )
+                if action_info:
+                    break
+            if action_info:
+                action_name, action_type = action_info
+                action = action_services.setdefault(
+                    action_name,
+                    {"types": set(), "servers": set(), "clients": set()},
+                )
+                action["types"].add(action_type)
+                action["servers"].update(role["servers"])
+                action["clients"].update(role["clients"])
+                continue
+            if "/_action/" in service_name:
+                continue
+            graph.services.append(
+                ros_engine_pb2.GraphService(
+                    name=service_name,
+                    types=sorted(types),
+                    server_nodes=sorted(set(role["servers"])),
+                    client_nodes=sorted(set(role["clients"])),
+                )
+            )
+
+        if len(action_services) > MAX_GRAPH_ACTIONS:
+            truncated = True
+        for action_name, action in sorted(action_services.items())[
+            :MAX_GRAPH_ACTIONS
+        ]:
+            graph.actions.append(
+                ros_engine_pb2.GraphAction(
+                    name=action_name,
+                    types=sorted(action["types"]),
+                    server_nodes=sorted(action["servers"]),
+                    client_nodes=sorted(action["clients"]),
+                )
+            )
+
+        graph.truncated = truncated
+        self.event_hub.publish(
+            ros_engine_pb2.WorkspaceEvent(
+                ros_graph_snapshot=graph,
+                timestamp_ns=time.time_ns(),
+            ),
+            priority="telemetry",
+            key="ros_graph_snapshot",
+            snapshot=True,
+        )

     def publish_twist(self, request):
         def publish():
diff --git a/src-tauri/resources/devcontainer/server_core.py b/src-tauri/resources/devcontainer/server_core.py
index 745edbb..a291f2b 100644
--- a/src-tauri/resources/devcontainer/server_core.py
+++ b/src-tauri/resources/devcontainer/server_core.py
@@ -12,6 +12,7 @@ import tempfile
 import threading
 import time
 from pathlib import Path
+import xml.etree.ElementTree as ET

 AUTH_PROTOCOL_VERSION = 1
 MAX_EVENT_QUEUE_SIZE = 256
@@ -27,6 +28,284 @@ MAX_CONFIG_ANGULAR_VELOCITY = 1.0
 MAX_CONFIG_LINEAR_ACCELERATION = 2.0
 MAX_CONFIG_ANGULAR_ACCELERATION = 4.0
 ALLOWED_MOTION_MODES = frozenset(("planar", "full_twist"))
+MAX_WORKSPACE_PACKAGES = 1000
+MAX_PACKAGE_SCAN_FILES = 5000
+MAX_PACKAGE_DISCOVERY_FILES = 250
+ACTION_SEND_GOAL_SUFFIX = "/_action/send_goal"
+PACKAGE_METADATA_LIMIT = 1024 * 1024
+_PACKAGE_PRUNED_DIRECTORIES = frozenset(
+    (".git", ".robotics-studio", "__pycache__", "build", "install", "log", "node_modules")
+)
+
+
+def _package_directory_is_within(root, path):
+    try:
+        path.resolve(strict=True).relative_to(root)
+        return True
+    except (OSError, ValueError):
+        return False
+
+
+def _workspace_package_directories(workspace):
+    root = Path(workspace)
+    if root.is_symlink() or not root.is_dir():
+        raise ValueError("Selected ROS workspace is missing or is not a directory")
+    root = root.resolve(strict=True)
+    found = []
+
+    root_manifest = root / "package.xml"
+    if root_manifest.is_file() and not root_manifest.is_symlink():
+        found.append(root)
+
+    source = root / "src"
+    if source.is_dir() and not source.is_symlink():
+        def on_error(error):
+            raise error
+
+        for current, directories, files in os.walk(
+            source, topdown=True, followlinks=False, onerror=on_error
+        ):
+            current_path = Path(current)
+            directories[:] = sorted(
+                name
+                for name in directories
+                if name not in _PACKAGE_PRUNED_DIRECTORIES
+                and not (current_path / name).is_symlink()
+            )
+            if "package.xml" in files:
+                manifest = current_path / "package.xml"
+                if not manifest.is_symlink() and _package_directory_is_within(root, current_path):
+                    found.append(current_path)
+                    directories[:] = []
+                    if len(found) > MAX_WORKSPACE_PACKAGES:
+                        raise ValueError("ROS workspace contains too many packages")
+
+    install = root / "install"
+    if install.is_dir() and not install.is_symlink():
+        share_roots = [install / "share"]
+        try:
+            prefixes = sorted(
+                child
+                for child in install.iterdir()
+                if child.is_dir()
+                and not child.is_symlink()
+                and child.name not in _PACKAGE_PRUNED_DIRECTORIES
+            )
+        except OSError as error:
+            raise ValueError(f"Could not inspect ROS workspace install directory: {error}") from error
+        share_roots.extend(prefix / "share" for prefix in prefixes)
+        for share_root in share_roots:
+            if not share_root.is_dir() or share_root.is_symlink():
+                continue
+            try:
+                packages = sorted(share_root.iterdir())
+            except OSError as error:
+                raise ValueError(f"Could not inspect ROS package index directory: {error}") from error
+            for package_share in packages:
+                if (
+                    package_share.is_dir()
+                    and not package_share.is_symlink()
+                    and (package_share / "package.xml").is_file()
+                    and not (package_share / "package.xml").is_symlink()
+                    and _package_directory_is_within(root, package_share)
+                ):
+                    found.append(package_share)
+                    if len(found) > MAX_WORKSPACE_PACKAGES:
+                        raise ValueError("ROS workspace contains too many packages")
+
+    return root, found
+
+
+def _parse_package_manifest(package_directory):
+    manifest_path = package_directory / "package.xml"
+    package = {
+        "name": package_directory.name,
+        "version": None,
+        "description": None,
+        "package_type": "unknown",
+        "dependencies": [],
+        "metadata_error": None,
+    }
+    try:
+        if manifest_path.stat().st_size > PACKAGE_METADATA_LIMIT:
+            raise ValueError("package.xml exceeds the metadata size limit")
+        contents = manifest_path.read_bytes()
+        if b"<!DOCTYPE" in contents.upper() or b"<!ENTITY" in contents.upper():
+            raise ValueError("package.xml must not declare a DTD or entities")
+        root = ET.fromstring(contents)
+        if root.tag.rsplit("}", 1)[-1] != "package":
+            raise ValueError("package.xml root element must be <package>")
+
+        name = (root.findtext("name") or "").strip()
+        if name:
+            package["name"] = name
+        package["version"] = (root.findtext("version") or "").strip() or None
+        package["description"] = (root.findtext("description") or "").strip() or None
+        missing_fields = [
+            field
+            for field, value in (
+                ("name", name),
+                ("version", package["version"]),
+                ("description", package["description"]),
+            )
+            if not value
+        ]
+        if missing_fields:
+            package["metadata_error"] = (
+                "package.xml is missing required fields: " + ", ".join(missing_fields)
+            )[:256]
+
+        build_type = (root.findtext("export/build_type") or "").strip()
+        build_tools = {
+            (element.text or "").strip()
+            for tag in ("buildtool_depend", "buildtool_export_depend")
+            for element in root.findall(tag)
+            if (element.text or "").strip()
+        }
+        if build_type in ("ament_cmake", "ament_python", "cmake"):
+            package["package_type"] = build_type
+        elif "ament_cmake" in build_tools:
+            package["package_type"] = "ament_cmake"
+        elif "ament_python" in build_tools:
+            package["package_type"] = "ament_python"
+
+        dependencies = {
+            (element.text or "").strip()
+            for tag in (
+                "depend",
+                "build_depend",
+                "buildtool_depend",
+                "buildtool_export_depend",
+                "exec_depend",
+                "test_depend",
+                "build_export_depend",
+                "doc_depend",
+                "group_depend",
+            )
+            for element in root.findall(tag)
+            if (element.text or "").strip()
+        }
+        package["dependencies"] = sorted(dependencies)
+    except (OSError, ET.ParseError, ValueError) as error:
+        package["metadata_error"] = str(error)[:256]
+    return package
+
+
+def _package_files(package_directory):
+    launch_files = []
+    config_files = []
+    files_seen = 0
+
+    def on_error(error):
+        raise error
+
+    for current, directories, files in os.walk(
+        package_directory, topdown=True, followlinks=False, onerror=on_error
+    ):
+        current_path = Path(current)
+        directories[:] = sorted(
+            name
+            for name in directories
+            if name not in _PACKAGE_PRUNED_DIRECTORIES
+            and not (current_path / name).is_symlink()
+        )
+        for name in sorted(files):
+            candidate = current_path / name
+            if candidate.is_symlink() or not candidate.is_file():
+                continue
+            files_seen += 1
+            if files_seen > MAX_PACKAGE_SCAN_FILES:
+                raise ValueError("ROS package contains too many files to inspect")
+            relative = candidate.relative_to(package_directory).as_posix()
+            if name.endswith((".launch.py", ".launch.xml")) or name.endswith(".launch"):
+                if len(launch_files) < MAX_PACKAGE_DISCOVERY_FILES:
+                    launch_files.append(relative)
+            if candidate.suffix.lower() in (".yaml", ".yml", ".json", ".config", ".ini"):
+                if len(config_files) < MAX_PACKAGE_DISCOVERY_FILES:
+                    config_files.append(relative)
+
+    return sorted(launch_files), sorted(config_files)
+
+
+def _package_executables(workspace, package_name):
+    install = workspace / "install"
+    if install.is_symlink() or not install.is_dir():
+        return []
+    candidates = (install / "lib" / package_name,)
+    try:
+        prefixes = sorted(
+            child
+            for child in install.iterdir()
+            if child.is_dir()
+            and not child.is_symlink()
+            and child.name not in _PACKAGE_PRUNED_DIRECTORIES
+        )
+    except (FileNotFoundError, NotADirectoryError):
+        prefixes = ()
+    candidates += tuple(prefix / "lib" / package_name for prefix in prefixes)
+
+    executables = set()
+    for directory in candidates:
+        if (
+            not directory.is_dir()
+            or directory.is_symlink()
+            or directory.parent.is_symlink()
+            or not _package_directory_is_within(workspace, directory)
+        ):
+            continue
+        try:
+            entries = sorted(directory.iterdir())
+        except OSError as error:
+            raise ValueError(f"Could not inspect executables for package {package_name}: {error}") from error
+        for entry in entries:
+            if (
+                not entry.is_symlink()
+                and entry.is_file()
+                and os.access(entry, os.X_OK)
+            ):
+                executables.add(entry.name)
+    return sorted(executables)[:MAX_PACKAGE_DISCOVERY_FILES]
+
+
+def discover_workspace_packages(workspace="/workspace"):
+    """Return bounded package metadata found only inside the selected workspace."""
+    root, package_directories = _workspace_package_directories(workspace)
+    result_by_name = {}
+    for package_directory in package_directories:
+        package = _parse_package_manifest(package_directory)
+        name = package["name"]
+        existing = result_by_name.get(name)
+        relative_directory = package_directory.relative_to(root)
+        is_source_package = not relative_directory.parts or relative_directory.parts[0] == "src"
+        if existing and (not is_source_package or existing["is_source"]):
+            continue
+
+        launch_files, config_files = _package_files(package_directory)
+        package.update(
+            {
+                "path": relative_directory.as_posix(),
+                "executables": _package_executables(root, name),
+                "launch_files": launch_files,
+                "config_files": config_files,
+                "is_source": is_source_package,
+            }
+        )
+        result_by_name[name] = package
+
+    return [
+        {key: value for key, value in package.items() if key != "is_source"}
+        for _, package in sorted(result_by_name.items())
+    ]
+
+
+def action_from_send_goal_service(service_name, service_type):
+    """Return an action name/type only for canonical ROS 2 send-goal endpoints."""
+    if not service_name.endswith(ACTION_SEND_GOAL_SUFFIX):
+        return None
+    action_type, separator, suffix = service_type.rpartition("_SendGoal")
+    if not separator or suffix or not action_type:
+        return None
+    return service_name[: -len(ACTION_SEND_GOAL_SUFFIX)], action_type


 class EventOverflow(Exception):
diff --git a/src-tauri/resources/devcontainer/test_server_core.py b/src-tauri/resources/devcontainer/test_server_core.py
index 92a2a81..b969520 100644
--- a/src-tauri/resources/devcontainer/test_server_core.py
+++ b/src-tauri/resources/devcontainer/test_server_core.py
@@ -15,6 +15,8 @@ from server_core import (
     RobotConfigRegistry,
     RobotMotionConfig,
     SafetyController,
+    action_from_send_goal_service,
+    discover_workspace_packages,
     is_valid_auth_token,
     load_auth_credentials,
     validate_twist_request,
@@ -35,6 +37,139 @@ def request(topic="/robot/cmd_vel", value=0.0, timestamp_ns=None, linear_y=0.0,
     )


+class RosGraphDiscoveryTests(unittest.TestCase):
+    def test_extracts_canonical_ros_action_from_send_goal_service(self):
+        self.assertEqual(
+            action_from_send_goal_service(
+                "/fibonacci/_action/send_goal",
+                "example_interfaces/action/Fibonacci_SendGoal",
+            ),
+            ("/fibonacci", "example_interfaces/action/Fibonacci"),
+        )
+
+    def test_ignores_non_action_or_malformed_send_goal_services(self):
+        self.assertIsNone(
+            action_from_send_goal_service(
+                "/fibonacci/send_goal",
+                "example_interfaces/action/Fibonacci_SendGoal",
+            )
+        )
+        self.assertIsNone(
+            action_from_send_goal_service(
+                "/fibonacci/_action/send_goal",
+                "example_interfaces/srv/SendGoal",
+            )
+        )
+
+
+class WorkspacePackageDiscoveryTests(unittest.TestCase):
+    @staticmethod
+    def create_package(workspace, package_path, manifest):
+        package = os.path.join(workspace, package_path)
+        os.makedirs(package, exist_ok=True)
+        with open(os.path.join(package, "package.xml"), "w", encoding="utf-8") as file:
+            file.write(manifest)
+        return package
+
+    def test_discovers_source_package_metadata_and_workspace_executables(self):
+        with tempfile.TemporaryDirectory() as workspace:
+            package = self.create_package(
+                workspace,
+                "src/robot_app",
+                """<package format="3">
+                  <name>robot_app</name><version>1.2.3</version>
+                  <description>Robot runtime package</description>
+                  <buildtool_depend>ament_cmake</buildtool_depend>
+                  <depend>rclpy</depend><exec_depend>geometry_msgs</exec_depend>
+                </package>""",
+            )
+            os.makedirs(os.path.join(package, "launch"))
+            os.makedirs(os.path.join(package, "config"))
+            with open(os.path.join(package, "launch", "start.launch.py"), "w") as file:
+                file.write("pass\n")
+            with open(os.path.join(package, "config", "robot.yaml"), "w") as file:
+                file.write("name: robot\n")
+            executable = os.path.join(workspace, "install", "lib", "robot_app", "driver")
+            os.makedirs(os.path.dirname(executable))
+            with open(executable, "w") as file:
+                file.write("#!/bin/sh\n")
+            os.chmod(executable, 0o755)
+
+            packages = discover_workspace_packages(workspace)
+
+            self.assertEqual(len(packages), 1)
+            self.assertEqual(
+                packages[0],
+                {
+                    "name": "robot_app",
+                    "version": "1.2.3",
+                    "description": "Robot runtime package",
+                    "package_type": "ament_cmake",
+                    "dependencies": ["ament_cmake", "geometry_msgs", "rclpy"],
+                    "metadata_error": None,
+                    "path": "src/robot_app",
+                    "executables": ["driver"],
+                    "launch_files": ["launch/start.launch.py"],
+                    "config_files": ["config/robot.yaml"],
+                },
+            )
+
+    def test_detects_ament_python_and_malformed_package_metadata(self):
+        with tempfile.TemporaryDirectory() as workspace:
+            self.create_package(
+                workspace,
+                "src/python_node",
+                """<package format="3"><name>python_node</name>
+                  <version>0.1.0</version><description>Python node package</description>
+                  <buildtool_depend>ament_python</buildtool_depend></package>""",
+            )
+            self.create_package(workspace, "src/broken_node", "<package>")
+
+            packages = {
+                package["name"]: package
+                for package in discover_workspace_packages(workspace)
+            }
+
+            self.assertEqual(packages["python_node"]["package_type"], "ament_python")
+            self.assertEqual(packages["python_node"]["version"], "0.1.0")
+            self.assertIsNone(packages["python_node"]["metadata_error"])
+            self.assertEqual(packages["broken_node"]["package_type"], "unknown")
+            self.assertTrue(packages["broken_node"]["metadata_error"])
+
+    def test_rejects_missing_workspace_and_isolates_selected_workspace(self):
+        with tempfile.TemporaryDirectory() as parent:
+            first = os.path.join(parent, "first")
+            second = os.path.join(parent, "second")
+            os.makedirs(first)
+            os.makedirs(second)
+            self.create_package(
+                first, "src/first_pkg", "<package><name>first_pkg</name></package>"
+            )
+            self.create_package(
+                second, "src/second_pkg", "<package><name>second_pkg</name></package>"
+            )
+
+            self.assertEqual(
+                [package["name"] for package in discover_workspace_packages(first)],
+                ["first_pkg"],
+            )
+            with self.assertRaisesRegex(ValueError, "missing or is not a directory"):
+                discover_workspace_packages(os.path.join(parent, "missing"))
+
+    @unittest.skipUnless(hasattr(os, "symlink"), "symlinks are unavailable")
+    def test_does_not_follow_symlinked_package_directories(self):
+        with tempfile.TemporaryDirectory() as parent:
+            workspace = os.path.join(parent, "workspace")
+            outside = os.path.join(parent, "outside")
+            os.makedirs(os.path.join(workspace, "src"))
+            self.create_package(
+                parent, "outside/escaped", "<package><name>escaped</name></package>"
+            )
+            os.symlink(outside, os.path.join(workspace, "src", "linked"))
+
+            self.assertEqual(discover_workspace_packages(workspace), [])
+
+
 class EventHubTests(unittest.TestCase):
     def test_subscribers_receive_independent_events(self):
         hub = EventHub()
diff --git a/src-tauri/src/lib.rs b/src-tauri/src/lib.rs
index 586a93c..9f082ee 100644
--- a/src-tauri/src/lib.rs
+++ b/src-tauri/src/lib.rs
@@ -3,6 +3,8 @@ use std::fs;
 use std::path::{Component, Path, PathBuf};
 use std::sync::{Arc, Mutex, OnceLock};

+mod urdf;
+
 const MAX_FILE_SIZE: u64 = 8 * 1024 * 1024;
 const MAX_WORLD_SIZE: u64 = 10 * 1024 * 1024;
 const MAX_DIRECTORY_ENTRIES: usize = 10_000;
@@ -980,6 +982,11 @@ fn validate_sdf_file(path: &Path, content: &str) -> Result<(), String> {
     Ok(())
 }

+#[tauri::command]
+fn parse_robot_description(content: String) -> Result<urdf::RobotDescription, String> {
+    urdf::parse_robot_description(&content)
+}
+
 fn import_world_contents(
     access: &FsAccess,
     workspace: &Path,
@@ -1104,6 +1111,7 @@ pub fn run() {
             list_dir,
             pick_directory,
             set_ros_workspace,
+            parse_robot_description,
             pick_save_path,
             pick_keybindings_file,
             import_world,
@@ -1367,6 +1375,7 @@ async fn start_ros_stream(app: AppHandle) -> Result<(), String> {
                                     qy: tf.qy,
                                     qz: tf.qz,
                                     qw: tf.qw,
+                                    topic_name: tf.topic_name.clone(),
                                 },
                             );
                         }
@@ -1375,6 +1384,25 @@ async fn start_ros_stream(app: AppHandle) -> Result<(), String> {
                         {
                             let _ = app.emit("node-snapshot", snap.node_names.clone());
                         }
+                        if let Some(rosengine::workspace_event::Event::RosGraphSnapshot(snapshot)) =
+                            &evt.event
+                        {
+                            let _ = app
+                                .emit("ros-graph-snapshot", ros_graph_snapshot_response(snapshot));
+                        }
+                        if let Some(rosengine::workspace_event::Event::JointStateUpdate(update)) =
+                            &evt.event
+                        {
+                            let _ = app.emit(
+                                "joint-state-update",
+                                JointStateUpdateResponse {
+                                    topic_name: update.topic_name.clone(),
+                                    names: update.names.clone(),
+                                    positions: update.positions.clone(),
+                                    truncated: update.truncated,
+                                },
+                            );
+                        }
                         if let Some(rosengine::workspace_event::Event::SafetyState(state)) =
                             &evt.event
                         {
@@ -1623,6 +1651,139 @@ struct EventRecoveryResponse {
     retained_critical_events: u32,
 }

+#[derive(Serialize, Clone)]
+#[serde(rename_all = "camelCase")]
+struct RosGraphNodeResponse {
+    name: String,
+    namespace: String,
+}
+
+#[derive(Serialize, Clone)]
+#[serde(rename_all = "camelCase")]
+struct RosGraphEndpointResponse {
+    node_name: String,
+    node_namespace: String,
+    topic_type: String,
+    reliability: String,
+    durability: String,
+    history: String,
+    depth: u32,
+}
+
+#[derive(Serialize, Clone)]
+#[serde(rename_all = "camelCase")]
+struct RosGraphTopicResponse {
+    name: String,
+    types: Vec<String>,
+    publishers: Vec<RosGraphEndpointResponse>,
+    subscribers: Vec<RosGraphEndpointResponse>,
+}
+
+#[derive(Serialize, Clone)]
+#[serde(rename_all = "camelCase")]
+struct RosGraphServiceResponse {
+    name: String,
+    types: Vec<String>,
+    server_nodes: Vec<String>,
+    client_nodes: Vec<String>,
+}
+
+#[derive(Serialize, Clone)]
+#[serde(rename_all = "camelCase")]
+struct RosGraphActionResponse {
+    name: String,
+    types: Vec<String>,
+    server_nodes: Vec<String>,
+    client_nodes: Vec<String>,
+}
+
+#[derive(Serialize, Clone)]
+#[serde(rename_all = "camelCase")]
+struct RosGraphSnapshotResponse {
+    nodes: Vec<RosGraphNodeResponse>,
+    topics: Vec<RosGraphTopicResponse>,
+    services: Vec<RosGraphServiceResponse>,
+    actions: Vec<RosGraphActionResponse>,
+    truncated: bool,
+}
+
+#[derive(Serialize, Clone)]
+#[serde(rename_all = "camelCase")]
+struct JointStateUpdateResponse {
+    topic_name: String,
+    names: Vec<String>,
+    positions: Vec<f64>,
+    truncated: bool,
+}
+
+fn ros_graph_snapshot_response(snapshot: &rosengine::RosGraphSnapshot) -> RosGraphSnapshotResponse {
+    RosGraphSnapshotResponse {
+        nodes: snapshot
+            .nodes
+            .iter()
+            .map(|node| RosGraphNodeResponse {
+                name: node.name.clone(),
+                namespace: node.namespace.clone(),
+            })
+            .collect(),
+        topics: snapshot
+            .topics
+            .iter()
+            .map(|topic| RosGraphTopicResponse {
+                name: topic.name.clone(),
+                types: topic.types.clone(),
+                publishers: topic
+                    .publishers
+                    .iter()
+                    .map(|endpoint| RosGraphEndpointResponse {
+                        node_name: endpoint.node_name.clone(),
+                        node_namespace: endpoint.node_namespace.clone(),
+                        topic_type: endpoint.topic_type.clone(),
+                        reliability: endpoint.reliability.clone(),
+                        durability: endpoint.durability.clone(),
+                        history: endpoint.history.clone(),
+                        depth: endpoint.depth,
+                    })
+                    .collect(),
+                subscribers: topic
+                    .subscribers
+                    .iter()
+                    .map(|endpoint| RosGraphEndpointResponse {
+                        node_name: endpoint.node_name.clone(),
+                        node_namespace: endpoint.node_namespace.clone(),
+                        topic_type: endpoint.topic_type.clone(),
+                        reliability: endpoint.reliability.clone(),
+                        durability: endpoint.durability.clone(),
+                        history: endpoint.history.clone(),
+                        depth: endpoint.depth,
+                    })
+                    .collect(),
+            })
+            .collect(),
+        services: snapshot
+            .services
+            .iter()
+            .map(|service| RosGraphServiceResponse {
+                name: service.name.clone(),
+                types: service.types.clone(),
+                server_nodes: service.server_nodes.clone(),
+                client_nodes: service.client_nodes.clone(),
+            })
+            .collect(),
+        actions: snapshot
+            .actions
+            .iter()
+            .map(|action| RosGraphActionResponse {
+                name: action.name.clone(),
+                types: action.types.clone(),
+                server_nodes: action.server_nodes.clone(),
+                client_nodes: action.client_nodes.clone(),
+            })
+            .collect(),
+        truncated: snapshot.truncated,
+    }
+}
+
 fn safety_state_response(state: &rosengine::SafetyState) -> SafetyStateResponse {
     SafetyStateResponse {
         state: match state.state {
@@ -2089,93 +2250,65 @@ struct RosPackageInfo {
     path: String,
     package_type: String,
     version: Option<String>,
+    description: Option<String>,
     dependencies: Vec<String>,
     executables: Vec<String>,
     launch_files: Vec<String>,
     config_files: Vec<String>,
+    metadata_error: Option<String>,
+}
+
+fn resolve_workspace_package_path(
+    workspace: &Path,
+    package_name: &str,
+    relative_path: &str,
+) -> Result<PathBuf, String> {
+    let relative_path = Path::new(relative_path);
+    if relative_path.is_absolute()
+        || relative_path.components().any(|component| {
+            matches!(
+                component,
+                std::path::Component::ParentDir
+                    | std::path::Component::RootDir
+                    | std::path::Component::Prefix(_)
+            )
+        })
+    {
+        return Err(format!(
+            "ROS package {package_name} returned a path outside the selected workspace"
+        ));
+    }
+
+    let canonical_workspace = workspace
+        .canonicalize()
+        .map_err(|error| format!("Could not validate selected ROS workspace: {error}"))?;
+    let canonical_package_path = canonical_workspace
+        .join(relative_path)
+        .canonicalize()
+        .map_err(|error| {
+            format!("Could not validate path for ROS package {package_name}: {error}")
+        })?;
+    if !canonical_package_path.starts_with(&canonical_workspace) || !canonical_package_path.is_dir()
+    {
+        return Err(format!(
+            "ROS package {package_name} resolved outside the selected workspace"
+        ));
+    }
+    Ok(canonical_package_path)
 }

 #[tauri::command]
 fn list_ros_packages() -> Result<Vec<RosPackageInfo>, String> {
     let container_name = find_dev_container()?;
-    let workspace = get_default_workspace_dir()?;
-    let script = format!(
-        r#"source /opt/ros/jazzy/setup.bash >/dev/null 2>&1 || exit 1
-export ROBOTICS_STUDIO_WORKSPACE="{}"
-if [ -f /workspace/install/setup.bash ]; then source /workspace/install/setup.bash >/dev/null 2>&1 || true; fi
-python3 - <<'PY'
-import json
-import os
-import xml.etree.ElementTree as ET
-from pathlib import Path
-
-try:
-    from ament_index_python.packages import get_packages
-except Exception as exc:  # pragma: no cover - depends on ROS installation
-    print(json.dumps([]))
-    raise SystemExit(0)
-
-packages = get_packages()
-results = []
-for name in sorted(packages):
-    pkg_path = Path(packages[name])
-    package_xml = pkg_path / 'package.xml'
-    version = None
-    package_type = 'ros_package'
-    dependencies = []
-    executables = []
-    launch_files = []
-    config_files = []
-
-    if package_xml.exists():
-        try:
-            root = ET.parse(package_xml).getroot()
-            version = root.attrib.get('version')
-            package_type = root.findtext('export/build_type') or root.tag.rsplit('}}', 1)[-1] or 'ros_package'
-            for tag_name in ['depend', 'build_depend', 'buildtool_depend', 'exec_depend', 'test_depend', 'build_export_depend']:
-                for element in root.findall(tag_name):
-                    value = (element.text or '').strip()
-                    if value:
-                        dependencies.append(value)
-            dependencies = sorted(dict.fromkeys(dependencies))
-        except Exception:
-            pass
-
-    for candidate in sorted(pkg_path.iterdir()):
-        if candidate.name.endswith('.py') and os.access(candidate, os.X_OK):
-            executables.append(candidate.name)
-        if candidate.suffix in {{'.launch', '.xml'}} and candidate.name.endswith('.launch'):
-            launch_files.append(candidate.name)
-        if candidate.suffix in {{'.yaml', '.yml', '.json', '.config', '.ini'}}:
-            config_files.append(candidate.name)
-
-    if not version and package_xml.exists():
-        try:
-            for elem in ET.parse(package_xml).getroot():
-                if elem.tag.endswith('version'):
-                    version = (elem.text or '').strip()
-                    break
-        except Exception:
-            pass
-
-    results.append({{
-        'name': name,
-        'path': str(pkg_path),
-        'package_type': package_type,
-        'version': version,
-        'dependencies': dependencies,
-        'executables': executables,
-        'launch_files': launch_files,
-        'config_files': config_files,
-    }})
-
-print(json.dumps(results))
-PY
-"#,
-        workspace
-    );
+    let workspace = PathBuf::from(get_default_workspace_dir()?);
     let output = docker_command()
-        .args(["exec", &container_name, "bash", "-lc", &script])
+        .args([
+            "exec",
+            &container_name,
+            "bash",
+            "-lc",
+            "cd /opt/robotics-studio && python3 -c \"import json; from server_core import discover_workspace_packages; print(json.dumps(discover_workspace_packages('/workspace')))\"",
+        ])
         .output()
         .map_err(|error| format!("Could not inspect ROS packages: {error}"))?;
     if !output.status.success() {
@@ -2185,8 +2318,13 @@ PY
         ));
     }
     let stdout = String::from_utf8_lossy(&output.stdout);
-    let parsed: Vec<RosPackageInfo> = serde_json::from_str(stdout.trim())
+    let mut parsed: Vec<RosPackageInfo> = serde_json::from_str(stdout.trim())
         .map_err(|error| format!("Could not parse ROS package metadata: {error}"))?;
+    for package in &mut parsed {
+        package.path = resolve_workspace_package_path(&workspace, &package.name, &package.path)?
+            .to_string_lossy()
+            .into_owned();
+    }
     Ok(parsed)
 }

@@ -2838,6 +2976,30 @@ fn gazebo_world_control_service(container_name: &str) -> Result<Option<String>,
     parse_gazebo_world_control_service(&String::from_utf8_lossy(&output.stdout))
 }

+fn validate_sdf_with_gazebo(container_name: &str, world_path: &str) -> Result<(), String> {
+    let output = docker_command()
+        .args([
+            "exec",
+            container_name,
+            "bash",
+            "-lc",
+            "source /opt/ros/jazzy/setup.bash && gz sdf --check \"$1\"",
+            "robotics-studio-sdf-check",
+            world_path,
+        ])
+        .output()
+        .map_err(|error| format!("Could not run SDFormat validation: {error}"))?;
+    if !output.status.success() {
+        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
+        return Err(if detail.is_empty() {
+            "Gazebo rejected the SDF world".to_string()
+        } else {
+            format!("Gazebo rejected the SDF world: {detail}")
+        });
+    }
+    Ok(())
+}
+
 fn stop_gazebo_processes(container_name: &str) -> Result<(), String> {
     let output = docker_command()
         .args([
@@ -2946,6 +3108,8 @@ fn start_gazebo_sim(
         }
     };

+    validate_sdf_with_gazebo(&container_name, &container_world_path)?;
+
     let models = detect_models_in_sdf(&sdf_content);
     let robot_models: Vec<&String> = models
         .iter()
@@ -4182,6 +4346,7 @@ struct TfUpdate {
     qy: f64,
     qz: f64,
     qw: f64,
+    topic_name: String,
 }

 #[derive(Deserialize)]
@@ -4817,6 +4982,37 @@ mod filesystem_security_tests {
         }
     }

+    #[test]
+    fn ros_package_paths_are_confined_to_selected_workspace() {
+        let workspace = TempDirectory::new();
+        let outside = TempDirectory::new();
+        let package = workspace.0.join("src/robot");
+        fs::create_dir_all(&package).unwrap();
+        fs::create_dir_all(outside.0.join("robot")).unwrap();
+
+        assert_eq!(
+            resolve_workspace_package_path(&workspace.0, "robot", "src/robot").unwrap(),
+            package.canonicalize().unwrap()
+        );
+        assert!(
+            resolve_workspace_package_path(&workspace.0, "robot", "../robot")
+                .unwrap_err()
+                .contains("outside the selected workspace")
+        );
+        assert!(resolve_workspace_package_path(
+            &workspace.0,
+            "robot",
+            outside.0.to_string_lossy().as_ref()
+        )
+        .is_err());
+
+        #[cfg(unix)]
+        {
+            std::os::unix::fs::symlink(&outside.0, workspace.0.join("linked")).unwrap();
+            assert!(resolve_workspace_package_path(&workspace.0, "robot", "linked/robot").is_err());
+        }
+    }
+
     #[test]
     fn workspace_service_marker_does_not_authenticate_modified_server_files() {
         let directory = TempDirectory::new();
diff --git a/src/App.tsx b/src/App.tsx
index 9483e16..32f14b1 100644
--- a/src/App.tsx
+++ b/src/App.tsx
@@ -4,6 +4,8 @@ import { invoke } from "@tauri-apps/api/core";
 import RobotView, { RobotControls } from "./components/RobotView";
 import { listen, emit } from "@tauri-apps/api/event";
 import TFTree from "./components/TFTree";
+import RosGraphExplorer from "./components/RosGraphExplorer";
+import type { RosGraphSnapshot } from "./components/RosGraphExplorer";
 import Settings from "./components/Settings";
 import NewProjectModal from "./components/NewProjectModal";
 import {
@@ -55,10 +57,12 @@ interface RosPackageInfo {
   path: string;
   package_type: string;
   version: string | null;
+  description: string | null;
   dependencies: string[];
   executables: string[];
   launch_files: string[];
   config_files: string[];
+  metadata_error: string | null;
 }

 interface LogEvent {
@@ -280,8 +284,17 @@ function App() {
   const [rosEvents, setRosEvents] = useState<string[]>([]);
   const [rosConnected, setRosConnected] = useState(false);
   const [rosPackages, setRosPackages] = useState<RosPackageInfo[]>([]);
+  const [rosGraph, setRosGraph] = useState<RosGraphSnapshot | null>(null);
   const [rosPackagesLoading, setRosPackagesLoading] = useState(false);
   const [rosPackagesError, setRosPackagesError] = useState<string | null>(null);
+  const [rosPackageFilter, setRosPackageFilter] = useState("");
+  const [expandedRosPackage, setExpandedRosPackage] = useState<string | null>(
+    null,
+  );
+  const [rosPackageContentsError, setRosPackageContentsError] = useState<
+    string | null
+  >(null);
+  const rosPackagesRequest = useRef(0);
   const [initializing, setInitializing] = useState(false);
   const [safetyState, setSafetyState] = useState<
     "READY" | "ACTIVE" | "RESET_PENDING" | "FAULT" | "DISCONNECTED"
@@ -330,6 +343,21 @@ function App() {
         ]
       : []),
   ];
+  const filteredRosPackages = rosPackages.filter((pkg) => {
+    const filter = rosPackageFilter.trim().toLocaleLowerCase();
+    return (
+      !filter ||
+      [
+        pkg.name,
+        pkg.description ?? "",
+        pkg.package_type,
+        ...pkg.dependencies,
+      ]
+        .join(" ")
+        .toLocaleLowerCase()
+        .includes(filter)
+    );
+  });

   // --- AI chat ---
   const [chatInput, setChatInput] = useState("");
@@ -561,28 +589,55 @@ function App() {
   };

   // ---------- ROS workspace selection ----------
-  const refreshRosPackages = useCallback(async () => {
-    if (!rosWorkspacePath) {
-      setRosPackages([]);
+  const refreshRosPackages = useCallback(
+    async (allowUninitializedWorkspace = false) => {
+      const requestId = ++rosPackagesRequest.current;
+      if (!rosWorkspacePath && !allowUninitializedWorkspace) {
+        setRosPackagesLoading(false);
+        setRosPackages([]);
+        setRosPackagesError(null);
+        return;
+      }
+      setRosPackagesLoading(true);
       setRosPackagesError(null);
+      try {
+        const packages = await invoke<RosPackageInfo[]>("list_ros_packages");
+        if (requestId === rosPackagesRequest.current) {
+          setRosPackages(packages);
+        }
+      } catch (error) {
+        if (requestId === rosPackagesRequest.current) {
+          setRosPackages([]);
+          setRosPackagesError(String(error));
+        }
+      } finally {
+        if (requestId === rosPackagesRequest.current) {
+          setRosPackagesLoading(false);
+        }
+      }
+    },
+    [rosWorkspacePath],
+  );
+
+  useEffect(() => {
+    void refreshRosPackages();
+  }, [refreshRosPackages]);
+
+  const inspectRosPackage = async (path: string) => {
+    if (expandedRosPackage === path) {
+      setExpandedRosPackage(null);
+      setRosPackageContentsError(null);
       return;
     }
-    setRosPackagesLoading(true);
-    setRosPackagesError(null);
+    setExpandedRosPackage(path);
+    setRosPackageContentsError(null);
     try {
-      const packages = await invoke<RosPackageInfo[]>("list_ros_packages");
-      setRosPackages(packages);
+      const contents = await invoke<FileEntry[]>("list_dir", { path });
+      setChildrenByPath((previous) => ({ ...previous, [path]: contents }));
     } catch (error) {
-      setRosPackages([]);
-      setRosPackagesError(String(error));
-    } finally {
-      setRosPackagesLoading(false);
+      setRosPackageContentsError(String(error));
     }
-  }, [rosWorkspacePath]);
-
-  useEffect(() => {
-    void refreshRosPackages();
-  }, [refreshRosPackages]);
+  };

   const pickRosWorkspace = async () => {
     try {
@@ -618,6 +673,7 @@ function App() {
     setActiveTab("build");
     try {
       await invoke("run_colcon_build_streaming");
+      await refreshRosPackages();
     } catch (error) {
       setBuilding(false);
       setBuildOutput((prev) => appendLog(prev, `ERROR: ${String(error)}`));
@@ -639,6 +695,7 @@ function App() {
     setCurrentSetupLine("Starting...");
     try {
       await invoke("initialize_ros_environment");
+      await refreshRosPackages(true);
     } catch (err) {
       setSetupOutput((prev) => appendLog(prev, `❌ ${err}`));
       setCurrentSetupLine(`❌ ${err}`);
@@ -895,6 +952,7 @@ function App() {
     const unlistenError = listen<string>("ros-error", (event) => {
       console.error("ROS error:", event.payload);
       setRosConnected(false);
+      setRosGraph(null);
       setSafetyState((state) =>
         state === "ACTIVE" || state === "FAULT" ? state : "DISCONNECTED",
       );
@@ -902,6 +960,7 @@ function App() {

     const unlistenAuthError = listen<string>("ros-auth-error", (event) => {
       setRosConnected(false);
+      setRosGraph(null);
       setSafetyState((state) =>
         state === "ACTIVE" || state === "FAULT" ? state : "DISCONNECTED",
       );
@@ -917,6 +976,7 @@ function App() {

     const unlistenConnecting = listen<void>("ros-connecting", () => {
       setRosConnected(false);
+      setRosGraph(null);
       setSafetyState((state) =>
         state === "ACTIVE" || state === "FAULT" ? state : "DISCONNECTED",
       );
@@ -953,6 +1013,10 @@ function App() {
     }>("event-recovery", (event) => {
       setEventRecovery(event.payload);
     });
+    const unlistenRosGraph = listen<RosGraphSnapshot>(
+      "ros-graph-snapshot",
+      (event) => setRosGraph(event.payload),
+    );

     const unlistenBuildOutput = listen<LogEvent>("build-output", (event) => {
       const { kind, text } = event.payload;
@@ -994,6 +1058,7 @@ function App() {
       unlistenSafety.then((f) => f());
       unlistenEventHealth.then((f) => f());
       unlistenEventRecovery.then((f) => f());
+      unlistenRosGraph.then((f) => f());
       unlistenBuildOutput.then((f) => f());
       unlistenBuildError.then((f) => f());
       unlistenBuildFinished.then((f) => f());
@@ -1436,15 +1501,43 @@ function App() {

               {activityView === "ros" && (
                 <div style={{ padding: "4px 14px", fontSize: 12 }}>
-                  <div style={{ opacity: 0.7, marginBottom: 6 }}>
-                    Workspace:{" "}
-                    <span
-                      style={{
-                        color: rosWorkspacePath ? "#7cd992" : "#e08080",
-                      }}
+                  <div
+                    style={{
+                      display: "flex",
+                      alignItems: "center",
+                      gap: 8,
+                      marginBottom: 8,
+                    }}
+                  >
+                    <div style={{ opacity: 0.7, flex: 1, minWidth: 0 }}>
+                      Workspace:{" "}
+                      <span
+                        title={rosWorkspacePath ?? undefined}
+                        style={{
+                          color: rosWorkspacePath ? "#7cd992" : "#e08080",
+                          overflowWrap: "anywhere",
+                        }}
+                      >
+                        {rosWorkspacePath ? "selected" : "not set"}
+                      </span>
+                    </div>
+                    <button
+                      className="ide-btn"
+                      type="button"
+                      onClick={() => void refreshRosPackages()}
+                      disabled={rosPackagesLoading || !rosWorkspacePath}
+                      aria-label="Refresh ROS packages"
+                      title="Refresh ROS packages"
                     >
-                      {rosWorkspacePath ? "set" : "not set"}
-                    </span>
+                      <RotateCw
+                        size={13}
+                        style={{
+                          animation: rosPackagesLoading
+                            ? "spin 1s linear infinite"
+                            : undefined,
+                        }}
+                      />
+                    </button>
                   </div>

                   {rosPackagesLoading && (
@@ -1471,21 +1564,47 @@ function App() {
                     !rosPackagesError &&
                     rosPackages.length === 0 && (
                       <div style={{ opacity: 0.6 }}>
-                        No ROS packages discovered in the active environment.
+                        No packages found in the selected workspace. Build the
+                        workspace after adding packages, then refresh.
                       </div>
                     )}

                   {!rosPackagesLoading &&
                     !rosPackagesError &&
                     rosPackages.length > 0 && (
-                      <div
-                        style={{
-                          display: "flex",
-                          flexDirection: "column",
-                          gap: 8,
-                        }}
-                      >
-                        {rosPackages.map((pkg) => (
+                      <>
+                        <input
+                          type="search"
+                          aria-label="Filter ROS packages"
+                          value={rosPackageFilter}
+                          onChange={(event) =>
+                            setRosPackageFilter(event.target.value)
+                          }
+                          placeholder="Filter packages..."
+                          style={{
+                            boxSizing: "border-box",
+                            width: "100%",
+                            marginBottom: 8,
+                            padding: "6px 8px",
+                            color: "#ddd",
+                            background: "#1e1e1e",
+                            border: "1px solid #444",
+                            borderRadius: 4,
+                          }}
+                        />
+                        {filteredRosPackages.length === 0 ? (
+                          <div style={{ opacity: 0.6 }}>
+                            No packages match this filter.
+                          </div>
+                        ) : (
+                          <div
+                            style={{
+                              display: "flex",
+                              flexDirection: "column",
+                              gap: 8,
+                            }}
+                          >
+                            {filteredRosPackages.map((pkg) => (
                           <div
                             key={pkg.name}
                             style={{
@@ -1504,6 +1623,17 @@ function App() {
                             >
                               {pkg.name}
                             </div>
+                            {pkg.description && (
+                              <div
+                                style={{
+                                  opacity: 0.75,
+                                  marginBottom: 4,
+                                  overflowWrap: "anywhere",
+                                }}
+                              >
+                                {pkg.description}
+                              </div>
+                            )}
                             <div
                               style={{
                                 opacity: 0.7,
@@ -1543,6 +1673,19 @@ function App() {
                                 </span>
                               )}
                             </div>
+                            {pkg.metadata_error && (
+                              <div
+                                role="status"
+                                style={{
+                                  color: "#e0b060",
+                                  marginTop: 6,
+                                  overflowWrap: "anywhere",
+                                }}
+                              >
+                                Package metadata is incomplete:{" "}
+                                {pkg.metadata_error}
+                              </div>
+                            )}
                             {pkg.dependencies.length > 0 && (
                               <div style={{ marginTop: 6 }}>
                                 <div style={{ opacity: 0.6, marginBottom: 2 }}>
@@ -1598,10 +1741,59 @@ function App() {
                                 )}
                               </div>
                             )}
+                            <button
+                              className="ide-btn"
+                              type="button"
+                              aria-expanded={expandedRosPackage === pkg.path}
+                              onClick={() => void inspectRosPackage(pkg.path)}
+                              style={{ marginTop: 8 }}
+                            >
+                              {expandedRosPackage === pkg.path
+                                ? "Hide contents"
+                                : "Inspect contents"}
+                            </button>
+                            {expandedRosPackage === pkg.path &&
+                              rosPackageContentsError && (
+                                <div
+                                  role="alert"
+                                  style={{
+                                    color: "#f0a3a3",
+                                    marginTop: 6,
+                                    overflowWrap: "anywhere",
+                                  }}
+                                >
+                                  Could not inspect package:{" "}
+                                  {rosPackageContentsError}
+                                </div>
+                              )}
+                            {expandedRosPackage === pkg.path &&
+                              childrenByPath[pkg.path] && (
+                                <div style={{ marginTop: 6 }}>
+                                  <FileTree
+                                    entries={childrenByPath[pkg.path]}
+                                    depth={0}
+                                    expandedPaths={expandedPaths}
+                                    childrenByPath={childrenByPath}
+                                    activeFilePath={activeFilePath}
+                                    onToggleFolder={toggleFolder}
+                                    onOpenFile={(path, name) => {
+                                      void openFile(path, name).catch(
+                                        (error) =>
+                                          setRosPackageContentsError(
+                                            String(error),
+                                          ),
+                                      );
+                                    }}
+                                  />
+                                </div>
+                              )}
                           </div>
-                        ))}
-                      </div>
+                            ))}
+                          </div>
+                        )}
+                      </>
                     )}
+                  <RosGraphExplorer graph={rosGraph} connected={rosConnected} />
                 </div>
               )}
             </div>
@@ -2240,7 +2432,13 @@ function App() {
               {activeTab === "sim" && (
                 <div style={{ height: "100%", display: "flex" }}>
                   <div style={{ flex: 1 }}>
-                    <RobotView />
+                    <RobotView
+                      descriptionContent={
+                        activeFile?.name.toLowerCase().endsWith(".urdf")
+                          ? activeFile.content
+                          : null
+                      }
+                    />
                   </div>
                   <div style={{ width: 200, borderLeft: "1px solid #333" }}>
                     <RobotControls
diff --git a/src/components/RobotView.tsx b/src/components/RobotView.tsx
index ccf33c3..624f0eb 100644
--- a/src/components/RobotView.tsx
+++ b/src/components/RobotView.tsx
@@ -4,6 +4,8 @@ import { listen } from "@tauri-apps/api/event";
 import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
 import { invoke } from "@tauri-apps/api/core";
 import { ArrowUp, ArrowDown, RotateCcw, RotateCw, Square } from "lucide-react";
+import { buildRobotInstance } from "../robotDescription";
+import type { RobotDescription, RobotInstance } from "../robotDescription";

 interface OdometryUpdate {
   node: string;
@@ -16,9 +18,89 @@ interface OdometryUpdate {
   qw: number;
 }

-export default function RobotView() {
+interface JointStateUpdate {
+  names: string[];
+  positions: number[];
+}
+
+function buildPlaceholderRobot(node: string): RobotInstance {
+  const isBlue = node.includes("blue");
+  const bodyColor = isBlue ? 0x8080ff : 0x80ff80;
+  const wheelColor = 0x333333;
+  const group = new THREE.Group();
+
+  const chassis = new THREE.Mesh(
+    new THREE.BoxGeometry(2.01142, 0.568726, 1),
+    new THREE.MeshStandardMaterial({ color: bodyColor }),
+  );
+  chassis.position.set(-0.151427, 0.175, 0);
+  chassis.castShadow = true;
+  chassis.receiveShadow = true;
+  group.add(chassis);
+
+  const wheelMaterial = new THREE.MeshStandardMaterial({ color: wheelColor });
+  const leftWheel = new THREE.Mesh(
+    new THREE.SphereGeometry(0.3, 16, 16),
+    wheelMaterial,
+  );
+  leftWheel.position.set(0.554283, -0.025, -0.625029);
+  leftWheel.castShadow = true;
+  group.add(leftWheel);
+
+  const rightWheel = new THREE.Mesh(
+    new THREE.SphereGeometry(0.3, 16, 16),
+    wheelMaterial,
+  );
+  rightWheel.position.set(0.554282, -0.025, 0.625029);
+  rightWheel.castShadow = true;
+  group.add(rightWheel);
+
+  const caster = new THREE.Mesh(
+    new THREE.SphereGeometry(0.2, 16, 16),
+    new THREE.MeshStandardMaterial({ color: wheelColor }),
+  );
+  caster.position.set(-0.957138, -0.125, 0);
+  caster.castShadow = true;
+  group.add(caster);
+
+  return { group, updateJoints: () => undefined };
+}
+
+export default function RobotView({
+  descriptionContent,
+}: {
+  descriptionContent: string | null;
+}) {
   const containerRef = useRef<HTMLDivElement>(null);
-  const robotsRef = useRef<Record<string, THREE.Group>>({});
+  const robotsRef = useRef<Record<string, RobotInstance>>({});
+  const [robotDescription, setRobotDescription] =
+    useState<RobotDescription | null>(null);
+  const [descriptionError, setDescriptionError] = useState<string | null>(null);
+
+  useEffect(() => {
+    if (!descriptionContent) {
+      setRobotDescription(null);
+      setDescriptionError(null);
+      return;
+    }
+
+    let active = true;
+    setRobotDescription(null);
+    setDescriptionError(null);
+    invoke<RobotDescription>("parse_robot_description", {
+      content: descriptionContent,
+    })
+      .then((description) => {
+        if (active) setRobotDescription(description);
+      })
+      .catch((error) => {
+        if (active) setDescriptionError(String(error));
+      });
+
+    return () => {
+      active = false;
+    };
+  }, [descriptionContent]);

   useEffect(() => {
     if (!containerRef.current) return;
@@ -93,6 +175,12 @@ export default function RobotView() {
     scene.add(light);
     scene.add(new THREE.AmbientLight(0x404040));

+    if (robotDescription) {
+      const preview = buildRobotInstance(robotDescription);
+      scene.add(preview.group);
+      robotsRef.current["__description_preview__"] = preview;
+    }
+
     const animate = () => {
       if (!isActive) return;

@@ -106,8 +194,15 @@ export default function RobotView() {
     const unlistenClear = listen("clear-robots", () => {
       if (!isActive) return;

-      Object.values(robotsRef.current).forEach((mesh) => scene.remove(mesh));
+      Object.values(robotsRef.current).forEach((robot) =>
+        scene.remove(robot.group),
+      );
       robotsRef.current = {};
+      if (robotDescription) {
+        const preview = buildRobotInstance(robotDescription);
+        scene.add(preview.group);
+        robotsRef.current["__description_preview__"] = preview;
+      }
     });

     const unlistenPromise = listen<OdometryUpdate>(
@@ -117,59 +212,38 @@ export default function RobotView() {

         const { node, x, y, z, qx, qy, qz, qw } = event.payload;

-        let mesh = robotsRef.current[node];
-
-        if (!mesh) {
-          const isBlue = node.includes("blue");
-          const bodyColor = isBlue ? 0x8080ff : 0x80ff80;
-          const wheelColor = 0x333333;
-
-          const group = new THREE.Group();
-
-          // Chassis — matches SDF: box 2.01142 x 1 x 0.568726,
-          // offset -0.151427 0 0.175 relative to vehicle origin
-          const chassisGeo = new THREE.BoxGeometry(2.01142, 0.568726, 1);
-          const chassisMat = new THREE.MeshStandardMaterial({
-            color: bodyColor,
-          });
-          const chassis = new THREE.Mesh(chassisGeo, chassisMat);
-          chassis.position.set(-0.151427, 0.175, 0);
-          chassis.castShadow = true;
-          chassis.receiveShadow = true;
-          group.add(chassis);
-
-          // Left wheel — sphere radius 0.3, offset 0.554283 0.625029 -0.025
-          const wheelGeo = new THREE.SphereGeometry(0.3, 16, 16);
-          const wheelMat = new THREE.MeshStandardMaterial({
-            color: wheelColor,
-          });
-          const leftWheel = new THREE.Mesh(wheelGeo, wheelMat);
-          leftWheel.position.set(0.554283, -0.025, -0.625029);
-          leftWheel.castShadow = true;
-          group.add(leftWheel);
-
-          // Right wheel — mirrored on y
-          const rightWheel = new THREE.Mesh(wheelGeo.clone(), wheelMat);
-          rightWheel.position.set(0.554282, -0.025, 0.625029);
-          rightWheel.castShadow = true;
-          group.add(rightWheel);
-
-          // Caster — sphere radius 0.2, offset -0.957138 0 -0.125
-          const casterGeo = new THREE.SphereGeometry(0.2, 16, 16);
-          const casterMat = new THREE.MeshStandardMaterial({
-            color: wheelColor,
-          });
-          const caster = new THREE.Mesh(casterGeo, casterMat);
-          caster.position.set(-0.957138, -0.125, 0);
-          caster.castShadow = true;
-          group.add(caster);
-
-          scene.add(group);
-          robotsRef.current[node] = group;
+        let robot = robotsRef.current[node];
+
+        if (!robot) {
+          robot =
+            robotsRef.current["__description_preview__"] ??
+            (robotDescription
+              ? buildRobotInstance(robotDescription)
+              : buildPlaceholderRobot(node));
+          if (robotsRef.current["__description_preview__"]) {
+            delete robotsRef.current["__description_preview__"];
+          } else {
+            scene.add(robot.group);
+          }
+          robotsRef.current[node] = robot;
         }

-        mesh.position.set(x, z, -y);
-        mesh.quaternion.set(qx, qz, -qy, qw);
+        robot.group.position.set(x, z, -y);
+        robot.group.quaternion.set(qx, qz, -qy, qw);
+      },
+    );
+    const unlistenJointState = listen<JointStateUpdate>(
+      "joint-state-update",
+      (event) => {
+        if (!isActive) return;
+        const { names, positions } = event.payload;
+        const jointPositions = new Map<string, number>();
+        for (let index = 0; index < Math.min(names.length, positions.length); index += 1) {
+          jointPositions.set(names[index], positions[index]);
+        }
+        Object.values(robotsRef.current).forEach((robot) =>
+          robot.updateJoints(jointPositions),
+        );
       },
     );

@@ -182,10 +256,11 @@ export default function RobotView() {

       unlistenPromise.then((f) => f()).catch(() => {});
       unlistenClear.then((f) => f()).catch(() => {});
+      unlistenJointState.then((f) => f()).catch(() => {});

-      Object.values(robotsRef.current).forEach((group) => {
-        scene.remove(group);
-        group.traverse((child) => {
+      Object.values(robotsRef.current).forEach((robot) => {
+        scene.remove(robot.group);
+        robot.group.traverse((child) => {
           if (child instanceof THREE.Mesh) {
             child.geometry.dispose();
             if (Array.isArray(child.material)) {
@@ -207,7 +282,7 @@ export default function RobotView() {
         containerRef.current.innerHTML = "";
       }
     };
-  }, []);
+  }, [robotDescription]);

   return (
     <div
@@ -216,8 +291,61 @@ export default function RobotView() {
         width: "100%",
         height: "100%",
         overflow: "hidden",
+        position: "relative",
       }}
-    />
+    >
+      {(descriptionError || robotDescription?.warnings.length) && (
+        <div
+          role={descriptionError ? "alert" : "status"}
+          style={{
+            position: "absolute",
+            zIndex: 1,
+            top: 8,
+            left: 8,
+            maxWidth: 420,
+            padding: 8,
+            background: "rgba(25,25,25,0.92)",
+            color: descriptionError ? "#ff8b82" : "#e0b060",
+            fontSize: 12,
+            whiteSpace: "pre-wrap",
+          }}
+        >
+          {descriptionError ?? robotDescription?.warnings.join("\n")}
+        </div>
+      )}
+      {descriptionContent && !robotDescription && !descriptionError && (
+        <div
+          role="status"
+          style={{
+            position: "absolute",
+            zIndex: 1,
+            top: 8,
+            left: 8,
+            padding: 8,
+            background: "rgba(25,25,25,0.92)",
+            fontSize: 12,
+          }}
+        >
+          Validating robot description...
+        </div>
+      )}
+      {!descriptionContent && !descriptionError && (
+        <div
+          role="status"
+          style={{
+            position: "absolute",
+            zIndex: 1,
+            top: 8,
+            left: 8,
+            padding: 8,
+            background: "rgba(25,25,25,0.92)",
+            fontSize: 12,
+          }}
+        >
+          Open a URDF file to preview its geometry; a placeholder robot is shown.
+        </div>
+      )}
+    </div>
   );
 }

diff --git a/src/components/TFTree.tsx b/src/components/TFTree.tsx
index 959fb26..84e7fb7 100644
--- a/src/components/TFTree.tsx
+++ b/src/components/TFTree.tsx
@@ -1,89 +1,164 @@
-import { useEffect, useState, useRef } from "react";
+import { useEffect, useMemo, useRef, useState } from "react";
 import { listen } from "@tauri-apps/api/event";
 import { AlertTriangle, Sparkles } from "lucide-react";
+import {
+  buildTfGraph,
+  tfEdgeKey,
+  type TfTransform,
+  type TfTreeNode,
+} from "../tfGraph";

-interface TfUpdate {
-  parent_frame: string;
-  child_frame: string;
-  x: number;
-  y: number;
-  z: number;
-  qx: number;
-  qy: number;
-  qz: number;
-  qw: number;
-}
-
-type TfUpdateWithValidation = TfUpdate & {
-  validationIssues?: string[];
+type TfUpdateWithValidation = TfTransform & {
+  validationIssues: string[];
 };

+interface TfEvent extends Omit<TfTransform, "receivedAtMs"> {}
+
 interface Props {
   onExplain?: (tf: TfUpdateWithValidation) => void;
 }

-const QUATERNION_TOLERANCE = 0.01;
-const TRANSLATION_WARNING_THRESHOLD = 1000;
-
-function validateTfUpdate(tf: TfUpdate): string[] {
-  const issues: string[] = [];
-  const values = [tf.x, tf.y, tf.z, tf.qx, tf.qy, tf.qz, tf.qw];
-
-  if (values.some((value) => !Number.isFinite(value))) {
-    issues.push("Contains NaN/Infinity");
-  }
-
-  const quaternionNorm =
-    tf.qx * tf.qx + tf.qy * tf.qy + tf.qz * tf.qz + tf.qw * tf.qw;
-
-  if (Math.abs(quaternionNorm) < 1e-12) {
-    issues.push("Zero quaternion — likely uninitialized");
-  } else if (Math.abs(quaternionNorm - 1) > QUATERNION_TOLERANCE) {
-    issues.push("Non-normalized quaternion");
-  }
-
-  if (
-    Math.abs(tf.x) > TRANSLATION_WARNING_THRESHOLD ||
-    Math.abs(tf.y) > TRANSLATION_WARNING_THRESHOLD ||
-    Math.abs(tf.z) > TRANSLATION_WARNING_THRESHOLD
-  ) {
-    issues.push("Large translation magnitude");
-  }
+function TreeBranch({
+  node,
+  depth,
+  selectedEdgeKey,
+  onSelect,
+}: {
+  node: TfTreeNode;
+  depth: number;
+  selectedEdgeKey: string | null;
+  onSelect: (key: string) => void;
+}) {
+  const issues = node.edge?.issues ?? [];
+  return (
+    <div key={node.edge?.key ?? node.frame}>
+      <div
+        style={{
+          display: "flex",
+          alignItems: "center",
+          gap: 5,
+          paddingLeft: depth * 14,
+          minHeight: 24,
+        }}
+      >
+        {depth > 0 && <span style={{ opacity: 0.45 }}>└─</span>}
+        {node.edge ? (
+          <button
+            type="button"
+            onClick={() => onSelect(node.edge!.key)}
+            aria-pressed={selectedEdgeKey === node.edge.key}
+            style={{
+              textAlign: "left",
+              padding: "2px 4px",
+              border: 0,
+              borderRadius: 3,
+              color: issues.length ? "#e0b060" : "#7cd992",
+              background:
+                selectedEdgeKey === node.edge.key
+                  ? "rgba(79,195,247,0.12)"
+                  : "transparent",
+              fontFamily: "inherit",
+              fontSize: "inherit",
+              cursor: "pointer",
+            }}
+          >
+            {node.frame}
+          </button>
+        ) : (
+          <span style={{ color: "#4fc3f7", fontWeight: 600 }}>
+            {node.frame}
+          </span>
+        )}
+        {issues.length > 0 && (
+          <span title={issues.join("; ")}>
+            <AlertTriangle
+              aria-label={issues.join("; ")}
+              size={13}
+              color="#e0b060"
+            />
+          </span>
+        )}
+      </div>
+      {node.children.map((child) => (
+        <TreeBranch
+          key={child.edge?.key ?? child.frame}
+          node={child}
+          depth={depth + 1}
+          selectedEdgeKey={selectedEdgeKey}
+          onSelect={onSelect}
+        />
+      ))}
+    </div>
+  );
+}

-  return issues;
+function quaternionEulerDegrees(transform: TfTransform) {
+  const { qx, qy, qz, qw } = transform;
+  if (![qx, qy, qz, qw].every(Number.isFinite)) return null;
+  const norm = Math.hypot(qx, qy, qz, qw);
+  if (norm < 1e-12) return null;
+  const x = qx / norm;
+  const y = qy / norm;
+  const z = qz / norm;
+  const w = qw / norm;
+  const sinRoll = 2 * (w * x + y * z);
+  const cosRoll = 1 - 2 * (x * x + y * y);
+  const sinPitch = 2 * (w * y - z * x);
+  const sinYaw = 2 * (w * z + x * y);
+  const cosYaw = 1 - 2 * (y * y + z * z);
+  const degrees = 180 / Math.PI;
+  return {
+    roll: Math.atan2(sinRoll, cosRoll) * degrees,
+    pitch: Math.asin(Math.max(-1, Math.min(1, sinPitch))) * degrees,
+    yaw: Math.atan2(sinYaw, cosYaw) * degrees,
+  };
 }

 export default function TFTree({ onExplain }: Props) {
-  const [transforms, setTransforms] = useState<Record<string, TfUpdate>>({});
-  const isActiveRef = useRef(true);
+  const [transforms, setTransforms] = useState<Record<string, TfTransform>>(
+    {},
+  );
+  const transformsRef = useRef<Record<string, TfTransform>>({});
+  const [selectedEdgeKey, setSelectedEdgeKey] = useState<string | null>(null);
+  const [nowMs, setNowMs] = useState(() => Date.now());

   useEffect(() => {
-    isActiveRef.current = true;
-    const unlisten = listen<TfUpdate>("tf-update", (event) => {
-      if (!isActiveRef.current) return;
-      const key = `${event.payload.parent_frame}->${event.payload.child_frame}`;
-      setTransforms((prev) => ({ ...prev, [key]: event.payload }));
+    let active = true;
+    let updateFrame: number | null = null;
+    const listener = listen<TfEvent>("tf-update", (event) => {
+      if (!active) return;
+      const transform: TfTransform = {
+        ...event.payload,
+        receivedAtMs: Date.now(),
+      };
+      const key = tfEdgeKey(transform.parent_frame, transform.child_frame);
+      transformsRef.current[key] = transform;
+      if (updateFrame === null) {
+        updateFrame = window.requestAnimationFrame(() => {
+          updateFrame = null;
+          if (active) setTransforms({ ...transformsRef.current });
+        });
+      }
     });
+    const timer = window.setInterval(() => setNowMs(Date.now()), 1000);

     return () => {
-      isActiveRef.current = false;
-      unlisten.then((f) => f());
+      active = false;
+      window.clearInterval(timer);
+      if (updateFrame !== null) {
+        window.cancelAnimationFrame(updateFrame);
+      }
+      listener.then((unlisten) => unlisten()).catch(() => undefined);
     };
   }, []);

-  // Group by parent frame, so we can render a simple indented tree
-  const byParent: Record<string, TfUpdateWithValidation[]> = {};
-  Object.values(transforms).forEach((t) => {
-    const withValidation: TfUpdateWithValidation = {
-      ...t,
-      validationIssues: validateTfUpdate(t),
-    };
-
-    if (!byParent[t.parent_frame]) byParent[t.parent_frame] = [];
-    byParent[t.parent_frame].push(withValidation);
-  });
-
-  const roots = Object.keys(byParent);
+  const graph = useMemo(
+    () => buildTfGraph(Object.values(transforms), nowMs),
+    [transforms, nowMs],
+  );
+  const selected =
+    graph.edges.find((edge) => edge.key === selectedEdgeKey) ?? graph.edges[0];
+  const euler = selected ? quaternionEulerDegrees(selected) : null;

   return (
     <div
@@ -96,92 +171,117 @@ export default function TFTree({ onExplain }: Props) {
         height: "100%",
       }}
     >
-      {roots.length === 0 && (
+      <div style={{ opacity: 0.65, marginBottom: 8 }}>
+        {graph.edges.length} transform(s) · {graph.roots.length} root frame(s)
+      </div>
+      {graph.warnings.map((warning) => (
+        <div
+          key={warning}
+          role="status"
+          style={{
+            display: "flex",
+            alignItems: "center",
+            gap: 6,
+            color: "#e0b060",
+            marginBottom: 6,
+          }}
+        >
+          <AlertTriangle size={13} />
+          {warning}
+        </div>
+      ))}
+      {graph.edges.length === 0 && (
         <div style={{ opacity: 0.5 }}>
-          No transforms detected yet — start a ROS node that publishes to /tf.
+          No transforms detected yet — start a ROS node that publishes to /tf
+          or /tf_static.
         </div>
       )}
-      {roots.map((parent) => (
-        <div key={parent} style={{ marginBottom: 10 }}>
-          <div style={{ color: "#4fc3f7", fontWeight: 600 }}>{parent}</div>
-          {byParent[parent].map((t) => {
-            const issues = t.validationIssues ?? [];
-            const issueSummary = issues.length > 0 ? issues.join(" • ") : "";
-
-            return (
-              <div
-                key={t.child_frame}
-                style={{ paddingLeft: 20, opacity: 0.9 }}
-              >
-                └─ <span style={{ color: "#7cd992" }}>{t.child_frame}</span>
-                {issues.length > 0 && (
-                  <span
-                    title={issueSummary}
-                    style={{
-                      marginLeft: 8,
-                      color: "#e08080",
-                      display: "inline-flex",
-                      alignItems: "center",
-                      gap: 4,
-                      cursor: "help",
-                    }}
-                  >
-                    <AlertTriangle
-                      style={{
-                        width: "var(--icon-sm)",
-                        height: "var(--icon-sm)",
-                        verticalAlign: "middle",
-                      }}
-                    />
-                    <span style={{ opacity: 0.9 }}>{issues[0]}</span>
-                  </span>
-                )}
-                {onExplain && (
-                  <span
-                    onClick={() =>
-                      onExplain({ ...t, validationIssues: issues })
-                    }
-                    title="Explain this frame with AI"
-                    style={{
-                      marginLeft: "var(--spacing-sm)",
-                      fontSize: "var(--font-xs)",
-                      color: "#4fc3f7",
-                      opacity: 0.8,
-                      cursor: "pointer",
-                      transition: "opacity 0.12s ease",
-                    }}
-                    onMouseEnter={(e) => {
-                      e.currentTarget.style.opacity = "1";
-                    }}
-                    onMouseLeave={(e) => {
-                      e.currentTarget.style.opacity = "0.8";
-                    }}
-                  >
-                    <Sparkles
-                      style={{
-                        width: "var(--icon-sm)",
-                        height: "var(--icon-sm)",
-                        marginRight: 4,
-                        display: "inline",
-                      }}
-                    />{" "}
-                    Explain
-                  </span>
-                )}
-                <div
-                  style={{
-                    paddingLeft: 20,
-                    opacity: 0.6,
-                    fontSize: "var(--font-xs)",
-                  }}
-                >
-                  pos: ({t.x.toFixed(3)}, {t.y.toFixed(3)}, {t.z.toFixed(3)})
-                </div>
-              </div>
-            );
-          })}
-        </div>
+      {graph.roots.map((root) => (
+        <TreeBranch
+          key={root.frame}
+          node={root}
+          depth={0}
+          selectedEdgeKey={selected?.key ?? null}
+          onSelect={setSelectedEdgeKey}
+        />
       ))}
+      {graph.unattachedFrames.length > 0 && (
+        <div style={{ marginTop: 10 }}>
+          <div style={{ color: "#e0b060", marginBottom: 4 }}>
+            Cyclic or unreachable frames
+          </div>
+          {graph.unattachedFrames.map((frame) => (
+            <div key={frame} style={{ color: "#e0b060", paddingLeft: 8 }}>
+              {frame}
+            </div>
+          ))}
+        </div>
+      )}
+      {selected && (
+        <section
+          aria-label="Transform inspector"
+          style={{
+            marginTop: 14,
+            padding: 10,
+            border: "1px solid rgba(255,255,255,0.12)",
+            borderRadius: 5,
+            background: "rgba(255,255,255,0.025)",
+          }}
+        >
+          <div style={{ fontWeight: 600, color: "#4fc3f7", marginBottom: 6 }}>
+            {selected.parent_frame} → {selected.child_frame}
+          </div>
+          <div style={{ opacity: 0.65, marginBottom: 6 }}>
+            Source: {selected.topic_name || "unknown topic"}
+            {selected.topic_name.endsWith("/tf_static")
+              ? " · static"
+              : ` · updated ${Math.max(
+                  0,
+                  Math.floor((nowMs - selected.receivedAtMs) / 1000),
+                )}s ago`}
+          </div>
+          <div>
+            Translation (m): ({selected.x.toFixed(4)}, {selected.y.toFixed(4)},{" "}
+            {selected.z.toFixed(4)})
+          </div>
+          <div>
+            Quaternion (x, y, z, w): ({selected.qx.toFixed(4)},{" "}
+            {selected.qy.toFixed(4)}, {selected.qz.toFixed(4)},{" "}
+            {selected.qw.toFixed(4)})
+          </div>
+          {euler && (
+            <div style={{ marginTop: 4 }}>
+              RPY (deg): ({euler.roll.toFixed(2)}, {euler.pitch.toFixed(2)},{" "}
+              {euler.yaw.toFixed(2)})
+            </div>
+          )}
+          {selected.issues.map((issue) => (
+            <div
+              key={issue}
+              role="alert"
+              style={{ color: "#e0b060", marginTop: 5 }}
+            >
+              {issue}
+            </div>
+          ))}
+          {onExplain && (
+            <button
+              type="button"
+              className="ide-btn"
+              onClick={() =>
+                onExplain({
+                  ...selected,
+                  validationIssues: selected.issues,
+                })
+              }
+              style={{ marginTop: 8 }}
+            >
+              <Sparkles size={13} style={{ marginRight: 4 }} />
+              Explain transform
+            </button>
+          )}
+        </section>
+      )}
     </div>
   );
 }
diff --git a/CHANGELOG.md b/CHANGELOG.md
new file mode 100644
index 0000000..ab7e200
--- /dev/null
+++ b/CHANGELOG.md
@@ -0,0 +1,86 @@
+# Changelog
+
+## [0.3.0] - Unreleased
+
+This version is still in development and is not release-ready. See
+[RELEASE_READINESS.md](./RELEASE_READINESS.md) for verified scope and blockers.
+
+### Added
+
+- Selected-workspace ROS package discovery with package build type, version,
+  description, dependencies, installed executables, launch/configuration file
+  paths, filtering, refresh, and approved-root package file browsing.
+- TF parent-child forest and transform inspector with multiple-root,
+  multiple-parent, cycle, stale dynamic-transform, quaternion, and finite-value
+  diagnostics.
+- Bounded read-only ROS graph snapshots with node namespaces, topic
+  publisher/subscriber identities and QoS, service types/endpoints, and action
+  server/client discovery.
+- Reliable transient-local subscription to `/tf_static`.
+- Bounded URDF parsing for primitive visuals and connected link trees, with
+  safe rejection of malformed or unsupported descriptions.
+- Three.js rendering for box, cylinder, and sphere visuals and supported
+  fixed/revolute/continuous/prismatic joints; live `/joint_states` updates are
+  streamed to the preview.
+- Dependency-free frontend unit tests for TF graph construction and robot
+  geometry/joint transforms.
+- Docker integration coverage for selected-workspace package discovery,
+  malformed package manifests, installed executables, workspace isolation,
+  live ROS graph endpoints, `/tf_static` and `/joint_states` delivery, and
+  valid/invalid SDFormat schema checks.
+
+### Security and reliability
+
+- Package discovery is restricted to package manifests and install outputs
+  within the selected workspace. Discovery bounds scan sizes, avoids symlinked
+  package directories, rejects DTD/entity declarations in package manifests,
+  and surfaces malformed metadata.
+- ROS package paths are returned as relative paths from the container and are
+  canonicalized and confined to the selected host workspace before the
+  frontend can inspect them.
+- ROS package discovery no longer interpolates the host workspace path into a
+  shell command.
+- Service calls and action goals are intentionally not exposed by the graph
+  explorer, avoiding a bypass of the backend's bounded motion/safety path.
+- Gazebo validates the selected SDF against the installed SDFormat schema
+  before startup; external model/mesh resource validation remains incomplete.
+- Existing v0.3.0 protections include TLS-pinned authenticated gRPC,
+  approved-root filesystem access, bounded motion commands, persistent
+  software stop state, critical event replay, and structural SDF checks.
+
+### Changed
+
+- The ROS sidebar distinguishes source packages in the selected workspace
+  from packages available in the ROS underlay.
+- The TF panel displays hierarchy and transform values instead of only a
+  flat/grouped edge list.
+- Documentation now identifies package, TF, cross-platform, and release
+  limitations based on the current verified implementation.
+
+### Fixed
+
+- Workspace package discovery no longer silently converts missing package
+  tooling or malformed overall command output into an empty package list.
+- Package file inspection uses the backend's approved-root file commands.
+
+### Breaking changes
+
+- No application-facing breaking change has been identified in this
+  development pass. New protobuf graph and joint-state event fields are
+  additive for protobuf readers; matching bundled protocol definitions must
+  still be deployed together.
+
+### Known limitations
+
+- Structured service/action invocation and monitoring are not implemented;
+  graph discovery remains read-only.
+- TF diagnostics are not a substitute for `tf2` authority validation; missing
+  frame authorities cannot be determined from received edges alone.
+- URDF support is a bounded subset: Xacro expansion, safe mesh/model
+  resolution, floating/planar joints, and full URDF semantics remain
+  outstanding. Coordinate-convention validation remains incomplete.
+- Build, simulation, and service processes do not yet share a complete
+  lifecycle manager.
+- Windows filesystem race resistance, full SDF schema/resource validation,
+  frontend workflow tests, cross-platform runtime testing, and verified
+  release artifacts remain outstanding.
diff --git a/src-tauri/src/urdf.rs b/src-tauri/src/urdf.rs
new file mode 100644
index 0000000..ca4a186
--- /dev/null
+++ b/src-tauri/src/urdf.rs
@@ -0,0 +1,605 @@
+use serde::Serialize;
+use std::collections::{HashMap, HashSet};
+
+const MAX_DESCRIPTION_BYTES: usize = 2 * 1024 * 1024;
+const MAX_LINKS: usize = 512;
+const MAX_JOINTS: usize = 512;
+const MAX_VISUALS_PER_LINK: usize = 64;
+
+#[derive(Clone, Debug, Default, PartialEq, Serialize)]
+#[serde(rename_all = "camelCase")]
+pub struct RobotOrigin {
+    pub xyz: [f64; 3],
+    pub rpy: [f64; 3],
+}
+
+#[derive(Clone, Debug, PartialEq, Serialize)]
+#[serde(tag = "kind", rename_all = "camelCase")]
+pub enum RobotGeometry {
+    Box { size: [f64; 3] },
+    Cylinder { radius: f64, length: f64 },
+    Sphere { radius: f64 },
+    Unsupported { description: String },
+}
+
+#[derive(Clone, Debug, PartialEq, Serialize)]
+#[serde(rename_all = "camelCase")]
+pub struct RobotVisual {
+    pub origin: RobotOrigin,
+    pub geometry: RobotGeometry,
+    pub color: Option<[f64; 4]>,
+}
+
+#[derive(Clone, Debug, PartialEq, Serialize)]
+#[serde(rename_all = "camelCase")]
+pub struct RobotLink {
+    pub name: String,
+    pub visuals: Vec<RobotVisual>,
+}
+
+#[derive(Clone, Debug, PartialEq, Serialize)]
+#[serde(rename_all = "camelCase")]
+pub struct RobotJoint {
+    pub name: String,
+    pub joint_type: String,
+    pub parent: String,
+    pub child: String,
+    pub origin: RobotOrigin,
+    pub axis: [f64; 3],
+}
+
+#[derive(Clone, Debug, PartialEq, Serialize)]
+#[serde(rename_all = "camelCase")]
+pub struct RobotDescription {
+    pub name: String,
+    pub root_link: String,
+    pub links: Vec<RobotLink>,
+    pub joints: Vec<RobotJoint>,
+    pub warnings: Vec<String>,
+}
+
+fn attribute_map(
+    element: &quick_xml::events::BytesStart<'_>,
+    decoder: quick_xml::encoding::Decoder,
+) -> Result<HashMap<String, String>, String> {
+    let mut result = HashMap::new();
+    for attribute in element.attributes().with_checks(true) {
+        let attribute = attribute.map_err(|error| format!("Malformed URDF attribute: {error}"))?;
+        let name = String::from_utf8_lossy(attribute.key.as_ref()).into_owned();
+        let value = attribute
+            .decoded_and_normalized_value(quick_xml::XmlVersion::Implicit1_0, decoder)
+            .map_err(|error| format!("Malformed URDF attribute value: {error}"))?
+            .into_owned();
+        result.insert(name, value);
+    }
+    Ok(result)
+}
+
+fn required_attribute<'a>(
+    attributes: &'a HashMap<String, String>,
+    name: &str,
+) -> Result<&'a str, String> {
+    attributes
+        .get(name)
+        .map(String::as_str)
+        .filter(|value| !value.trim().is_empty())
+        .ok_or_else(|| format!("URDF element is missing its '{name}' attribute"))
+}
+
+fn parse_number(value: &str, description: &str) -> Result<f64, String> {
+    let number = value
+        .parse::<f64>()
+        .map_err(|_| format!("Invalid {description}: expected a number"))?;
+    if !number.is_finite() {
+        return Err(format!("Invalid {description}: value must be finite"));
+    }
+    Ok(number)
+}
+
+fn parse_vector<const N: usize>(value: &str, description: &str) -> Result<[f64; N], String> {
+    let values = value
+        .split_whitespace()
+        .map(|part| parse_number(part, description))
+        .collect::<Result<Vec<_>, _>>()?;
+    values
+        .try_into()
+        .map_err(|_| format!("Invalid {description}: expected {N} values"))
+}
+
+fn parse_origin(attributes: &HashMap<String, String>) -> Result<RobotOrigin, String> {
+    Ok(RobotOrigin {
+        xyz: attributes
+            .get("xyz")
+            .map(|value| parse_vector(value, "origin xyz"))
+            .transpose()?
+            .unwrap_or([0.0; 3]),
+        rpy: attributes
+            .get("rpy")
+            .map(|value| parse_vector(value, "origin rpy"))
+            .transpose()?
+            .unwrap_or([0.0; 3]),
+    })
+}
+
+fn parse_color(value: &str) -> Result<[f64; 4], String> {
+    let color = parse_vector(value, "material rgba")?;
+    if color
+        .iter()
+        .any(|component| !(0.0..=1.0).contains(component))
+    {
+        return Err("Invalid material rgba: components must be between 0 and 1".to_string());
+    }
+    Ok(color)
+}
+
+fn element_name(element: &quick_xml::events::BytesStart<'_>) -> String {
+    String::from_utf8_lossy(element.local_name().as_ref()).into_owned()
+}
+
+fn parse_geometry(
+    element_name: &str,
+    attributes: &HashMap<String, String>,
+) -> Result<RobotGeometry, String> {
+    match element_name {
+        "box" => {
+            let size = parse_vector(required_attribute(attributes, "size")?, "box size")?;
+            if size.iter().any(|dimension| *dimension <= 0.0) {
+                return Err("URDF box dimensions must be positive".to_string());
+            }
+            Ok(RobotGeometry::Box { size })
+        }
+        "cylinder" => {
+            let radius =
+                parse_number(required_attribute(attributes, "radius")?, "cylinder radius")?;
+            let length =
+                parse_number(required_attribute(attributes, "length")?, "cylinder length")?;
+            if radius <= 0.0 || length <= 0.0 {
+                return Err("URDF cylinder dimensions must be positive".to_string());
+            }
+            Ok(RobotGeometry::Cylinder { radius, length })
+        }
+        "sphere" => {
+            let radius = parse_number(required_attribute(attributes, "radius")?, "sphere radius")?;
+            if radius <= 0.0 {
+                return Err("URDF sphere radius must be positive".to_string());
+            }
+            Ok(RobotGeometry::Sphere { radius })
+        }
+        "mesh" => Ok(RobotGeometry::Unsupported {
+            description: attributes
+                .get("filename")
+                .cloned()
+                .unwrap_or_else(|| "mesh with no filename".to_string()),
+        }),
+        _ => Err(format!("Unsupported URDF geometry element: {element_name}")),
+    }
+}
+
+fn validate_robot_tree(links: &[RobotLink], joints: &[RobotJoint]) -> Result<String, String> {
+    if links.is_empty() {
+        return Err("URDF must define at least one link".to_string());
+    }
+    let link_names = links
+        .iter()
+        .map(|link| link.name.as_str())
+        .collect::<HashSet<_>>();
+    if link_names.len() != links.len() {
+        return Err("URDF contains duplicate link names".to_string());
+    }
+    let joint_names = joints
+        .iter()
+        .map(|joint| joint.name.as_str())
+        .collect::<HashSet<_>>();
+    if joint_names.len() != joints.len() {
+        return Err("URDF contains duplicate joint names".to_string());
+    }
+
+    let mut child_links = HashSet::new();
+    let mut children_by_parent: HashMap<&str, Vec<&str>> = HashMap::new();
+    for joint in joints {
+        if !link_names.contains(joint.parent.as_str()) || !link_names.contains(joint.child.as_str())
+        {
+            return Err(format!(
+                "Joint '{}' refers to an undefined parent or child link",
+                joint.name
+            ));
+        }
+        if joint.parent == joint.child {
+            return Err(format!("Joint '{}' connects a link to itself", joint.name));
+        }
+        if !child_links.insert(joint.child.as_str()) {
+            return Err(format!(
+                "Link '{}' has more than one parent joint",
+                joint.child
+            ));
+        }
+        children_by_parent
+            .entry(&joint.parent)
+            .or_default()
+            .push(&joint.child);
+    }
+
+    let roots = links
+        .iter()
+        .filter(|link| !child_links.contains(link.name.as_str()))
+        .map(|link| link.name.as_str())
+        .collect::<Vec<_>>();
+    if roots.len() != 1 {
+        return Err("URDF links must form one connected tree with one root".to_string());
+    }
+
+    let mut visited = HashSet::new();
+    let mut pending = vec![roots[0]];
+    while let Some(link) = pending.pop() {
+        if !visited.insert(link) {
+            return Err("URDF joint graph contains a cycle".to_string());
+        }
+        if let Some(children) = children_by_parent.get(link) {
+            pending.extend(children.iter().copied());
+        }
+    }
+    if visited.len() != links.len() {
+        return Err("URDF joint graph is disconnected or cyclic".to_string());
+    }
+
+    Ok(roots[0].to_string())
+}
+
+pub fn parse_robot_description(content: &str) -> Result<RobotDescription, String> {
+    if content.len() > MAX_DESCRIPTION_BYTES {
+        return Err(format!(
+            "URDF exceeds the {MAX_DESCRIPTION_BYTES}-byte limit"
+        ));
+    }
+    if content.contains('\0') {
+        return Err("URDF contains a NUL character".to_string());
+    }
+
+    let mut reader = quick_xml::Reader::from_str(content);
+    reader.config_mut().trim_text(true);
+    let mut stack = Vec::<String>::new();
+    let mut robot_name = None;
+    let mut root_seen = false;
+    let mut root_closed = false;
+    let mut current_link = None::<RobotLink>;
+    let mut current_joint = None::<RobotJoint>;
+    let mut current_visual = None::<RobotVisual>;
+    let mut links = Vec::new();
+    let mut joints = Vec::new();
+    let mut warnings = Vec::new();
+
+    loop {
+        match reader.read_event() {
+            Ok(
+                event @ (quick_xml::events::Event::Start(_) | quick_xml::events::Event::Empty(_)),
+            ) => {
+                let (element, is_empty) = match event {
+                    quick_xml::events::Event::Start(element) => (element, false),
+                    quick_xml::events::Event::Empty(element) => (element, true),
+                    _ => unreachable!(),
+                };
+                let name = element_name(&element);
+                let parent = stack.last().map(String::as_str);
+                if root_closed {
+                    return Err("URDF contains content after its root element".to_string());
+                }
+                let attributes = attribute_map(&element, reader.decoder())?;
+
+                match name.as_str() {
+                    "robot" if stack.is_empty() && !root_seen => {
+                        let name = required_attribute(&attributes, "name")?;
+                        if name.len() > 128 {
+                            return Err("URDF robot name exceeds 128 bytes".to_string());
+                        }
+                        robot_name = Some(name.to_string());
+                        root_seen = true;
+                    }
+                    "robot" => return Err("URDF must contain exactly one <robot> root".to_string()),
+                    "link" if parent == Some("robot") => {
+                        if current_link.is_some() {
+                            return Err("Nested URDF <link> elements are invalid".to_string());
+                        }
+                        current_link = Some(RobotLink {
+                            name: required_attribute(&attributes, "name")?.to_string(),
+                            visuals: Vec::new(),
+                        });
+                    }
+                    "joint" if parent == Some("robot") => {
+                        if current_joint.is_some() {
+                            return Err("Nested URDF <joint> elements are invalid".to_string());
+                        }
+                        current_joint = Some(RobotJoint {
+                            name: required_attribute(&attributes, "name")?.to_string(),
+                            joint_type: required_attribute(&attributes, "type")?.to_string(),
+                            parent: String::new(),
+                            child: String::new(),
+                            origin: RobotOrigin::default(),
+                            axis: [1.0, 0.0, 0.0],
+                        });
+                    }
+                    "visual" if parent == Some("link") => {
+                        if current_visual.is_some() {
+                            return Err("Nested URDF <visual> elements are invalid".to_string());
+                        }
+                        current_visual = Some(RobotVisual {
+                            origin: RobotOrigin::default(),
+                            geometry: RobotGeometry::Unsupported {
+                                description: String::new(),
+                            },
+                            color: None,
+                        });
+                    }
+                    "origin" if parent == Some("visual") => {
+                        if let Some(visual) = current_visual.as_mut() {
+                            visual.origin = parse_origin(&attributes)?;
+                        }
+                    }
+                    "origin" if parent == Some("joint") => {
+                        if let Some(joint) = current_joint.as_mut() {
+                            joint.origin = parse_origin(&attributes)?;
+                        }
+                    }
+                    "parent" if parent == Some("joint") => {
+                        if let Some(joint) = current_joint.as_mut() {
+                            joint.parent = required_attribute(&attributes, "link")?.to_string();
+                        }
+                    }
+                    "child" if parent == Some("joint") => {
+                        if let Some(joint) = current_joint.as_mut() {
+                            joint.child = required_attribute(&attributes, "link")?.to_string();
+                        }
+                    }
+                    "axis" if parent == Some("joint") => {
+                        if let Some(joint) = current_joint.as_mut() {
+                            if let Some(axis) = attributes.get("xyz") {
+                                joint.axis = parse_vector(axis, "joint axis")?;
+                            }
+                        }
+                    }
+                    "color" if parent == Some("material") => {
+                        if let (Some(visual), Some(color)) =
+                            (current_visual.as_mut(), attributes.get("rgba"))
+                        {
+                            visual.color = Some(parse_color(color)?);
+                        }
+                    }
+                    "box" | "cylinder" | "sphere" | "mesh" if parent == Some("geometry") => {
+                        if let Some(visual) = current_visual.as_mut() {
+                            if !matches!(
+                                visual.geometry,
+                                RobotGeometry::Unsupported { ref description }
+                                    if description.is_empty()
+                            ) {
+                                return Err(
+                                    "URDF visual must contain exactly one geometry".to_string()
+                                );
+                            }
+                            visual.geometry = parse_geometry(&name, &attributes)?;
+                        }
+                    }
+                    _ => {}
+                }
+
+                if is_empty {
+                    match name.as_str() {
+                        "robot" => root_closed = true,
+                        "link" => {
+                            if links.len() >= MAX_LINKS {
+                                return Err(format!("URDF contains more than {MAX_LINKS} links"));
+                            }
+                            links.push(
+                                current_link
+                                    .take()
+                                    .ok_or_else(|| "URDF link state is inconsistent".to_string())?,
+                            );
+                        }
+                        "visual" => {
+                            return Err("URDF visual is missing a geometry".to_string());
+                        }
+                        "joint" => {
+                            return Err("URDF joint must define parent and child links".to_string());
+                        }
+                        _ => {}
+                    }
+                } else {
+                    stack.push(name);
+                }
+            }
+            Ok(quick_xml::events::Event::End(element)) => {
+                let name = String::from_utf8_lossy(element.local_name().as_ref()).into_owned();
+                if stack.pop().as_deref() != Some(name.as_str()) {
+                    return Err("URDF XML elements are improperly nested".to_string());
+                }
+                match name.as_str() {
+                    "visual" => {
+                        let visual = current_visual
+                            .take()
+                            .ok_or_else(|| "URDF visual state is inconsistent".to_string())?;
+                        if matches!(
+                            visual.geometry,
+                            RobotGeometry::Unsupported { ref description }
+                                if description.is_empty()
+                        ) {
+                            return Err("URDF visual is missing a supported geometry".to_string());
+                        }
+                        if let RobotGeometry::Unsupported { description } = &visual.geometry {
+                            warnings.push(format!(
+                                "Mesh geometry '{description}' is not rendered in this preview"
+                            ));
+                        }
+                        let link = current_link
+                            .as_mut()
+                            .ok_or_else(|| "URDF visual is outside a link".to_string())?;
+                        if link.visuals.len() >= MAX_VISUALS_PER_LINK {
+                            return Err(format!(
+                                "Link '{}' contains too many visual elements",
+                                link.name
+                            ));
+                        }
+                        link.visuals.push(visual);
+                    }
+                    "link" => {
+                        if links.len() >= MAX_LINKS {
+                            return Err(format!("URDF contains more than {MAX_LINKS} links"));
+                        }
+                        links.push(
+                            current_link
+                                .take()
+                                .ok_or_else(|| "URDF link state is inconsistent".to_string())?,
+                        );
+                    }
+                    "joint" => {
+                        if joints.len() >= MAX_JOINTS {
+                            return Err(format!("URDF contains more than {MAX_JOINTS} joints"));
+                        }
+                        let mut joint = current_joint
+                            .take()
+                            .ok_or_else(|| "URDF joint state is inconsistent".to_string())?;
+                        if joint.parent.is_empty() || joint.child.is_empty() {
+                            return Err(format!(
+                                "Joint '{}' must define parent and child links",
+                                joint.name
+                            ));
+                        }
+                        match joint.joint_type.as_str() {
+                            "fixed" => {}
+                            "revolute" | "continuous" | "prismatic" => {
+                                let magnitude =
+                                    joint.axis[0].hypot(joint.axis[1]).hypot(joint.axis[2]);
+                                if magnitude <= f64::EPSILON {
+                                    return Err(format!(
+                                        "Joint '{}' axis must be non-zero",
+                                        joint.name
+                                    ));
+                                }
+                                joint.axis.iter_mut().for_each(|axis| *axis /= magnitude);
+                            }
+                            "floating" | "planar" => warnings.push(format!(
+                                "Joint '{}' uses unsupported '{}' motion and remains fixed",
+                                joint.name, joint.joint_type
+                            )),
+                            _ => {
+                                return Err(format!(
+                                    "Joint '{}' has unsupported type '{}'",
+                                    joint.name, joint.joint_type
+                                ));
+                            }
+                        }
+                        joints.push(joint);
+                    }
+                    "robot" => root_closed = true,
+                    _ => {}
+                }
+            }
+            Ok(quick_xml::events::Event::DocType(_)) => {
+                return Err("URDF documents must not contain a DTD".to_string());
+            }
+            Ok(quick_xml::events::Event::Text(text)) if stack.is_empty() => {
+                let text_bytes: &[u8] = text.as_ref();
+                if !text_bytes.iter().all(u8::is_ascii_whitespace) {
+                    return Err("URDF contains text outside its root element".to_string());
+                }
+            }
+            Ok(quick_xml::events::Event::Eof) => break,
+            Ok(_) => {}
+            Err(error) => return Err(format!("Malformed URDF XML: {error}")),
+        }
+    }
+
+    if !root_seen || !root_closed || !stack.is_empty() {
+        return Err("URDF document is incomplete".to_string());
+    }
+    let root_link = validate_robot_tree(&links, &joints)?;
+    let name = robot_name.ok_or_else(|| "URDF robot name is missing".to_string())?;
+    Ok(RobotDescription {
+        name,
+        root_link,
+        links,
+        joints,
+        warnings,
+    })
+}
+
+#[cfg(test)]
+mod tests {
+    use super::{parse_robot_description, RobotGeometry};
+
+    #[test]
+    fn parses_primitive_visuals_and_normalizes_joint_axes() {
+        let robot = parse_robot_description(
+            r#"<robot name="test_bot">
+                <link name="base">
+                    <visual>
+                        <origin xyz="1 2 3" rpy="0 0 1.5"/>
+                        <geometry><box size="1 2 3"/></geometry>
+                        <material name="blue"><color rgba="0.1 0.2 0.8 1"/></material>
+                    </visual>
+                </link>
+                <link name="wheel"/>
+                <joint name="base_to_wheel" type="continuous">
+                    <parent link="base"/><child link="wheel"/>
+                    <axis xyz="0 0 2"/>
+                </joint>
+            </robot>"#,
+        )
+        .unwrap();
+
+        assert_eq!(robot.name, "test_bot");
+        assert_eq!(robot.root_link, "base");
+        assert_eq!(robot.links[0].visuals[0].origin.xyz, [1.0, 2.0, 3.0]);
+        assert_eq!(
+            robot.links[0].visuals[0].geometry,
+            RobotGeometry::Box {
+                size: [1.0, 2.0, 3.0]
+            }
+        );
+        assert_eq!(robot.links[0].visuals[0].color, Some([0.1, 0.2, 0.8, 1.0]));
+        assert_eq!(robot.joints[0].axis, [0.0, 0.0, 1.0]);
+    }
+
+    #[test]
+    fn rejects_dtds_malformed_xml_and_invalid_geometry_dimensions() {
+        assert!(parse_robot_description(
+            "<!DOCTYPE robot><robot name=\"test\"><link name=\"base\"/></robot>"
+        )
+        .is_err());
+        assert!(
+            parse_robot_description("<robot name=\"test\"><link name=\"base\"></robot>").is_err()
+        );
+        assert!(parse_robot_description(
+            r#"<robot name="test"><link name="base"><visual><geometry><sphere radius="0"/></geometry></visual></link></robot>"#
+        )
+        .is_err());
+    }
+
+    #[test]
+    fn rejects_disconnected_links_cycles_and_multiple_parent_joints() {
+        assert!(parse_robot_description(
+            r#"<robot name="test"><link name="a"/><link name="b"/></robot>"#
+        )
+        .is_err());
+        assert!(parse_robot_description(
+            r#"<robot name="test"><link name="a"/><link name="b"/><joint name="ab" type="fixed"><parent link="a"/><child link="b"/></joint><joint name="ba" type="fixed"><parent link="b"/><child link="a"/></joint></robot>"#
+        )
+        .is_err());
+        assert!(parse_robot_description(
+            r#"<robot name="test"><link name="a"/><link name="b"/><link name="c"/><joint name="ab" type="fixed"><parent link="a"/><child link="b"/></joint><joint name="ac" type="fixed"><parent link="a"/><child link="b"/></joint></robot>"#
+        )
+        .is_err());
+    }
+
+    #[test]
+    fn reports_mesh_geometry_as_unsupported_instead_of_silently_rendering_it() {
+        let robot = parse_robot_description(
+            r#"<robot name="test"><link name="base"><visual><geometry><mesh filename="package://robot/meshes/base.stl"/></geometry></visual></link></robot>"#,
+        )
+        .unwrap();
+
+        assert!(matches!(
+            robot.links[0].visuals[0].geometry,
+            RobotGeometry::Unsupported { .. }
+        ));
+        assert_eq!(robot.warnings.len(), 1);
+        assert!(robot.warnings[0].contains("base.stl"));
+    }
+}
diff --git a/src/components/RosGraphExplorer.tsx b/src/components/RosGraphExplorer.tsx
new file mode 100644
index 0000000..5c7d09e
--- /dev/null
+++ b/src/components/RosGraphExplorer.tsx
@@ -0,0 +1,235 @@
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
+function nodePath(nodeName: string, nodeNamespace: string) {
+  return `${nodeNamespace === "/" ? "" : nodeNamespace}/${nodeName}`;
+}
+
+function TypeList({ types }: { types: string[] }) {
+  if (types.length === 0) return <span style={{ opacity: 0.6 }}>Unknown type</span>;
+  return (
+    <div style={{ color: "#b7d7f0", overflowWrap: "anywhere" }}>
+      {types.join(", ")}
+    </div>
+  );
+}
+
+function GraphSection({
+  title,
+  count,
+  children,
+}: {
+  title: string;
+  count: number;
+  children: React.ReactNode;
+}) {
+  return (
+    <details style={{ marginTop: 8 }} open>
+      <summary
+        style={{
+          cursor: "pointer",
+          padding: "6px 0",
+          borderTop: "1px solid rgba(255,255,255,0.08)",
+          fontWeight: 600,
+        }}
+      >
+        {title} ({count})
+      </summary>
+      <div style={{ display: "flex", flexDirection: "column", gap: 5 }}>
+        {children}
+      </div>
+    </details>
+  );
+}
+
+function GraphCard({
+  name,
+  types,
+  details,
+}: {
+  name: string;
+  types: string[];
+  details?: React.ReactNode;
+}) {
+  return (
+    <details
+      style={{
+        padding: "5px 7px",
+        background: "rgba(255,255,255,0.025)",
+        border: "1px solid rgba(255,255,255,0.07)",
+        borderRadius: 4,
+      }}
+    >
+      <summary style={{ cursor: "pointer", overflowWrap: "anywhere" }}>
+        {name}
+      </summary>
+      <div style={{ padding: "6px 0 2px", display: "grid", gap: 5 }}>
+        <TypeList types={types} />
+        {details}
+      </div>
+    </details>
+  );
+}
+
+export default function RosGraphExplorer({
+  graph,
+  connected,
+}: {
+  graph: RosGraphSnapshot | null;
+  connected: boolean;
+}) {
+  if (!graph) {
+    return (
+      <div
+        role="status"
+        style={{ marginTop: 12, paddingTop: 8, opacity: 0.65 }}
+      >
+        {connected
+          ? "Waiting for ROS graph details..."
+          : "ROS graph details are unavailable while disconnected."}
+      </div>
+    );
+  }
+
+  const endpointList = (
+    title: string,
+    endpoints: RosGraphEndpoint[],
+  ) => (
+    <div>
+      <strong>{title} ({endpoints.length})</strong>
+      {endpoints.map((endpoint, index) => (
+        <div
+          key={`${endpoint.nodeNamespace}/${endpoint.nodeName}/${index}`}
+          style={{
+            marginTop: 3,
+            paddingLeft: 8,
+            borderLeft: "1px solid #444",
+            overflowWrap: "anywhere",
+          }}
+        >
+          <div>{nodePath(endpoint.nodeName, endpoint.nodeNamespace)}</div>
+          <div style={{ opacity: 0.7 }}>{endpoint.topicType}</div>
+          <div style={{ opacity: 0.65 }}>
+            {endpoint.reliability}, {endpoint.durability}, {endpoint.history}
+            {endpoint.history === "KEEP_LAST" ? ` (${endpoint.depth})` : ""}
+          </div>
+        </div>
+      ))}
+    </div>
+  );
+
+  const endpointsFor = (servers: string[], clients: string[]) => (
+    <div style={{ display: "grid", gap: 4, overflowWrap: "anywhere" }}>
+      <div>
+        <strong>Servers:</strong> {servers.length ? servers.join(", ") : "none"}
+      </div>
+      <div>
+        <strong>Clients:</strong> {clients.length ? clients.join(", ") : "none"}
+      </div>
+    </div>
+  );
+
+  return (
+    <section
+      aria-label="Live ROS graph"
+      style={{ marginTop: 14, paddingTop: 6, borderTop: "1px solid #333" }}
+    >
+      <div style={{ fontWeight: 600, marginBottom: 4 }}>Live ROS Graph</div>
+      <div style={{ opacity: 0.65, fontSize: 11, marginBottom: 6 }}>
+        Read-only discovery. This panel does not call services or send action
+        goals.
+      </div>
+      {graph.truncated && (
+        <div role="status" style={{ color: "#e0b060", marginBottom: 6 }}>
+          Graph is large; the displayed snapshot is bounded.
+        </div>
+      )}
+      <GraphSection title="Nodes" count={graph.nodes.length}>
+        {graph.nodes.map((node) => (
+          <div
+            key={`${node.namespace}/${node.name}`}
+            style={{ overflowWrap: "anywhere", fontFamily: "monospace" }}
+          >
+            {nodePath(node.name, node.namespace)}
+          </div>
+        ))}
+      </GraphSection>
+      <GraphSection title="Topics" count={graph.topics.length}>
+        {graph.topics.map((topic) => (
+          <GraphCard
+            key={topic.name}
+            name={topic.name}
+            types={topic.types}
+            details={
+              <>
+                {endpointList("Publishers", topic.publishers)}
+                {endpointList("Subscribers", topic.subscribers)}
+              </>
+            }
+          />
+        ))}
+      </GraphSection>
+      <GraphSection title="Services" count={graph.services.length}>
+        {graph.services.map((service) => (
+          <GraphCard
+            key={service.name}
+            name={service.name}
+            types={service.types}
+            details={endpointsFor(service.serverNodes, service.clientNodes)}
+          />
+        ))}
+      </GraphSection>
+      <GraphSection title="Actions" count={graph.actions.length}>
+        {graph.actions.map((action) => (
+          <GraphCard
+            key={action.name}
+            name={action.name}
+            types={action.types}
+            details={endpointsFor(action.serverNodes, action.clientNodes)}
+          />
+        ))}
+      </GraphSection>
+    </section>
+  );
+}
diff --git a/src/robotDescription.ts b/src/robotDescription.ts
new file mode 100644
index 0000000..820bb28
--- /dev/null
+++ b/src/robotDescription.ts
@@ -0,0 +1,162 @@
+import * as THREE from "three";
+
+export interface RobotOrigin {
+  xyz: [number, number, number];
+  rpy: [number, number, number];
+}
+
+export type RobotGeometry =
+  | { kind: "box"; size: [number, number, number] }
+  | { kind: "cylinder"; radius: number; length: number }
+  | { kind: "sphere"; radius: number }
+  | { kind: "unsupported"; description: string };
+
+export interface RobotVisual {
+  origin: RobotOrigin;
+  geometry: RobotGeometry;
+  color: [number, number, number, number] | null;
+}
+
+export interface RobotLink {
+  name: string;
+  visuals: RobotVisual[];
+}
+
+export interface RobotJoint {
+  name: string;
+  jointType: string;
+  parent: string;
+  child: string;
+  origin: RobotOrigin;
+  axis: [number, number, number];
+}
+
+export interface RobotDescription {
+  name: string;
+  rootLink: string;
+  links: RobotLink[];
+  joints: RobotJoint[];
+  warnings: string[];
+}
+
+export interface RobotInstance {
+  group: THREE.Group;
+  updateJoints: (positions: ReadonlyMap<string, number>) => void;
+}
+
+function createVisual(visual: RobotVisual) {
+  const { geometry, color } = visual;
+  let shape: THREE.BufferGeometry;
+  switch (geometry.kind) {
+    case "box":
+      shape = new THREE.BoxGeometry(...geometry.size);
+      break;
+    case "cylinder":
+      shape = new THREE.CylinderGeometry(
+        geometry.radius,
+        geometry.radius,
+        geometry.length,
+        24,
+      );
+      break;
+    case "sphere":
+      shape = new THREE.SphereGeometry(geometry.radius, 24, 16);
+      break;
+    case "unsupported":
+      return null;
+  }
+
+  const material = new THREE.MeshStandardMaterial({
+    color: color
+      ? new THREE.Color(color[0], color[1], color[2])
+      : new THREE.Color(0x80aaff),
+    opacity: color?.[3] ?? 1,
+    transparent: (color?.[3] ?? 1) < 1,
+  });
+  const mesh = new THREE.Mesh(shape, material);
+  if (geometry.kind === "cylinder") mesh.rotation.x = Math.PI / 2;
+  mesh.castShadow = true;
+  mesh.receiveShadow = true;
+
+  const visualGroup = new THREE.Group();
+  visualGroup.position.set(...visual.origin.xyz);
+  visualGroup.rotation.set(...visual.origin.rpy, "XYZ");
+  visualGroup.add(mesh);
+  return visualGroup;
+}
+
+export function buildRobotInstance(
+  description: RobotDescription,
+): RobotInstance {
+  const links = new Map(description.links.map((link) => [link.name, link]));
+  const jointsByParent = new Map<string, RobotJoint[]>();
+  for (const joint of description.joints) {
+    const siblings = jointsByParent.get(joint.parent) ?? [];
+    siblings.push(joint);
+    jointsByParent.set(joint.parent, siblings);
+  }
+
+  const jointTransforms = new Map<
+    string,
+    { joint: RobotJoint; motion: THREE.Group }
+  >();
+
+  const buildLink = (name: string): THREE.Group => {
+    const definition = links.get(name);
+    if (!definition) {
+      throw new Error(`Robot description refers to missing link '${name}'`);
+    }
+    const link = new THREE.Group();
+    link.name = name;
+    for (const visual of definition.visuals) {
+      const mesh = createVisual(visual);
+      if (mesh) link.add(mesh);
+    }
+
+    for (const joint of jointsByParent.get(name) ?? []) {
+      const origin = new THREE.Group();
+      origin.name = `${joint.name}:origin`;
+      origin.position.set(...joint.origin.xyz);
+      origin.rotation.set(...joint.origin.rpy, "XYZ");
+      const motion = new THREE.Group();
+      motion.name = `${joint.name}:motion`;
+      origin.add(motion);
+      motion.add(buildLink(joint.child));
+      link.add(origin);
+      jointTransforms.set(joint.name, { joint, motion });
+    }
+    return link;
+  };
+
+  const group = new THREE.Group();
+  group.name = description.name;
+  const rosCoordinates = new THREE.Group();
+  rosCoordinates.name = "ros-coordinate-frame";
+  rosCoordinates.rotation.x = -Math.PI / 2;
+  rosCoordinates.add(buildLink(description.rootLink));
+  group.add(rosCoordinates);
+
+  return {
+    group,
+    updateJoints(positions) {
+      for (const [name, position] of positions) {
+        if (!Number.isFinite(position)) continue;
+        const transform = jointTransforms.get(name);
+        if (!transform) continue;
+        const { joint, motion } = transform;
+        if (joint.jointType === "revolute" || joint.jointType === "continuous") {
+          motion.quaternion.setFromAxisAngle(
+            new THREE.Vector3(...joint.axis),
+            position,
+          );
+        } else if (joint.jointType === "prismatic") {
+          motion.position.set(
+            joint.axis[0] * position,
+            joint.axis[1] * position,
+            joint.axis[2] * position,
+          );
+        }
+      }
+    },
+  };
+}
diff --git a/src/tfGraph.ts b/src/tfGraph.ts
new file mode 100644
index 0000000..68f5709
--- /dev/null
+++ b/src/tfGraph.ts
@@ -0,0 +1,182 @@
+export interface TfTransform {
+  parent_frame: string;
+  child_frame: string;
+  topic_name: string;
+  x: number;
+  y: number;
+  z: number;
+  qx: number;
+  qy: number;
+  qz: number;
+  qw: number;
+  receivedAtMs: number;
+}
+
+export interface TfGraphEdge extends TfTransform {
+  key: string;
+  issues: string[];
+}
+
+export interface TfTreeNode {
+  frame: string;
+  edge?: TfGraphEdge;
+  children: TfTreeNode[];
+}
+
+export interface TfGraph {
+  roots: TfTreeNode[];
+  unattachedFrames: string[];
+  edges: TfGraphEdge[];
+  warnings: string[];
+}
+
+const QUATERNION_TOLERANCE = 0.01;
+const TRANSLATION_WARNING_THRESHOLD = 1000;
+
+export function tfEdgeKey(parentFrame: string, childFrame: string) {
+  return JSON.stringify([parentFrame, childFrame]);
+}
+
+export function buildTfGraph(
+  transforms: TfTransform[],
+  nowMs = Date.now(),
+  staleAfterMs = 5000,
+): TfGraph {
+  const edges = transforms.map((transform): TfGraphEdge => {
+    const key = tfEdgeKey(transform.parent_frame, transform.child_frame);
+    const issues: string[] = [];
+    const values = [
+      transform.x,
+      transform.y,
+      transform.z,
+      transform.qx,
+      transform.qy,
+      transform.qz,
+      transform.qw,
+    ];
+
+    if (!transform.parent_frame || !transform.child_frame) {
+      issues.push("Parent or child frame is empty");
+    }
+    if (values.some((value) => !Number.isFinite(value))) {
+      issues.push("Contains NaN or Infinity");
+    }
+
+    const quaternionNorm = Math.hypot(
+      transform.qx,
+      transform.qy,
+      transform.qz,
+      transform.qw,
+    );
+    if (quaternionNorm < 1e-12) {
+      issues.push("Zero quaternion — likely uninitialized");
+    } else if (Math.abs(quaternionNorm - 1) > QUATERNION_TOLERANCE) {
+      issues.push("Non-normalized quaternion");
+    }
+
+    if (
+      Math.abs(transform.x) > TRANSLATION_WARNING_THRESHOLD ||
+      Math.abs(transform.y) > TRANSLATION_WARNING_THRESHOLD ||
+      Math.abs(transform.z) > TRANSLATION_WARNING_THRESHOLD
+    ) {
+      issues.push("Large translation magnitude");
+    }
+
+    if (
+      !transform.topic_name.endsWith("/tf_static") &&
+      nowMs - transform.receivedAtMs > staleAfterMs
+    ) {
+      issues.push("Transform is stale");
+    }
+
+    return { ...transform, key, issues };
+  });
+
+  const frameNames = new Set<string>();
+  const parentsByChild = new Map<string, Set<string>>();
+  const childrenByParent = new Map<string, TfGraphEdge[]>();
+  for (const edge of edges) {
+    if (!edge.parent_frame || !edge.child_frame) continue;
+    frameNames.add(edge.parent_frame);
+    frameNames.add(edge.child_frame);
+    const parents = parentsByChild.get(edge.child_frame) ?? new Set<string>();
+    parents.add(edge.parent_frame);
+    parentsByChild.set(edge.child_frame, parents);
+    const children = childrenByParent.get(edge.parent_frame) ?? [];
+    children.push(edge);
+    childrenByParent.set(edge.parent_frame, children);
+  }
+
+  for (const edge of edges) {
+    const parents = parentsByChild.get(edge.child_frame);
+    if (parents && parents.size > 1) {
+      edge.issues.push("Frame has multiple parents");
+    }
+  }
+
+  const visitState = new Map<string, "visiting" | "visited">();
+  const stack: string[] = [];
+  const cycleEdges = new Set<string>();
+  const cycleFrames = new Set<string>();
+  const visit = (frame: string) => {
+    visitState.set(frame, "visiting");
+    stack.push(frame);
+    for (const edge of childrenByParent.get(frame) ?? []) {
+      if (visitState.get(edge.child_frame) === "visiting") {
+        cycleEdges.add(edge.key);
+        const cycleStart = stack.lastIndexOf(edge.child_frame);
+        for (const cycleFrame of stack.slice(cycleStart)) {
+          cycleFrames.add(cycleFrame);
+        }
+      } else if (!visitState.has(edge.child_frame)) {
+        visit(edge.child_frame);
+      }
+    }
+    stack.pop();
+    visitState.set(frame, "visited");
+  };
+  for (const frame of [...frameNames].sort()) {
+    if (!visitState.has(frame)) visit(frame);
+  }
+  for (const edge of edges) {
+    if (cycleEdges.has(edge.key)) {
+      edge.issues.push("Cycle detected in transform graph");
+    }
+  }
+
+  const roots = [...frameNames]
+    .filter((frame) => !parentsByChild.has(frame))
+    .sort();
+  const visitedFrames = new Set<string>();
+  const buildTree = (frame: string, ancestors: Set<string>): TfTreeNode => {
+    visitedFrames.add(frame);
+    const nextAncestors = new Set(ancestors);
+    nextAncestors.add(frame);
+    const children = (childrenByParent.get(frame) ?? [])
+      .filter((edge) => !nextAncestors.has(edge.child_frame))
+      .map((edge) => ({
+        frame: edge.child_frame,
+        edge,
+        children: buildTree(edge.child_frame, nextAncestors).children,
+      }));
+    return { frame, children };
+  };
+  const treeRoots = roots.map((root) => buildTree(root, new Set()));
+  const unattachedFrames = [...frameNames]
+    .filter((frame) => !visitedFrames.has(frame))
+    .sort();
+  const warnings: string[] = [];
+  if (treeRoots.length > 1) {
+    warnings.push(`TF contains ${treeRoots.length} disconnected root frames`);
+  }
+  if (cycleFrames.size > 0) {
+    warnings.push(
+      `TF contains a cycle involving: ${[...cycleFrames].sort().join(", ")}`,
+    );
+  }
+  if (edges.some((edge) => edge.issues.includes("Frame has multiple parents"))) {
+    warnings.push("One or more frames have multiple parents");
+  }
+
+  return { roots: treeRoots, unattachedFrames, edges, warnings };
+}
diff --git a/tests/robotDescription.test.mjs b/tests/robotDescription.test.mjs
new file mode 100644
index 0000000..a8a4d3d
--- /dev/null
+++ b/tests/robotDescription.test.mjs
@@ -0,0 +1,90 @@
+import assert from "node:assert/strict";
+import test from "node:test";
+import { buildRobotInstance } from "../src/robotDescription.ts";
+
+const origin = { xyz: [0, 0, 0], rpy: [0, 0, 0] };
+
+test("builds primitive robot links and applies revolute and prismatic joint states", () => {
+  const instance = buildRobotInstance({
+    name: "test_robot",
+    rootLink: "base_link",
+    warnings: [],
+    links: [
+      {
+        name: "base_link",
+        visuals: [
+          {
+            origin,
+            geometry: { kind: "box", size: [1, 0.5, 0.25] },
+            color: [0.2, 0.4, 0.8, 1],
+          },
+        ],
+      },
+      { name: "arm_link", visuals: [] },
+      { name: "slider_link", visuals: [] },
+    ],
+    joints: [
+      {
+        name: "shoulder",
+        jointType: "revolute",
+        parent: "base_link",
+        child: "arm_link",
+        origin: { xyz: [0, 0, 0.5], rpy: [0, 0, 0] },
+        axis: [0, 0, 1],
+      },
+      {
+        name: "slider",
+        jointType: "prismatic",
+        parent: "arm_link",
+        child: "slider_link",
+        origin,
+        axis: [1, 0, 0],
+      },
+    ],
+  });
+
+  const arm = instance.group.getObjectByName("arm_link");
+  const shoulder = instance.group.getObjectByName("shoulder:motion");
+  const slider = instance.group.getObjectByName("slider:motion");
+  assert.ok(arm);
+  assert.ok(shoulder);
+  assert.ok(slider);
+
+  instance.updateJoints(
+    new Map([
+      ["shoulder", Math.PI / 2],
+      ["slider", 0.25],
+    ]),
+  );
+
+  assert.ok(Math.abs(shoulder.rotation.z - Math.PI / 2) < 1e-8);
+  assert.equal(slider.position.x, 0.25);
+  assert.equal(instance.group.getObjectByName("base_link").children.length, 2);
+});
+
+test("skips unsupported mesh visuals without failing the whole model", () => {
+  const instance = buildRobotInstance({
+    name: "mesh_robot",
+    rootLink: "base_link",
+    warnings: ["Mesh geometry 'package://robot/meshes/base.stl' is not rendered"],
+    links: [
+      {
+        name: "base_link",
+        visuals: [
+          {
+            origin,
+            geometry: {
+              kind: "unsupported",
+              description: "package://robot/meshes/base.stl",
+            },
+            color: null,
+          },
+        ],
+      },
+    ],
+    joints: [],
+  });
+
+  assert.ok(instance.group.getObjectByName("base_link"));
+  assert.equal(instance.group.getObjectByName("base_link").children.length, 0);
+});
diff --git a/tests/tfGraph.test.mjs b/tests/tfGraph.test.mjs
new file mode 100644
index 0000000..2d8d95c
--- /dev/null
+++ b/tests/tfGraph.test.mjs
@@ -0,0 +1,84 @@
+import assert from "node:assert/strict";
+import test from "node:test";
+import { buildTfGraph } from "../src/tfGraph.ts";
+
+function transform(parent, child, overrides = {}) {
+  return {
+    parent_frame: parent,
+    child_frame: child,
+    topic_name: "/tf",
+    x: 0,
+    y: 0,
+    z: 0,
+    qx: 0,
+    qy: 0,
+    qz: 0,
+    qw: 1,
+    receivedAtMs: 10000,
+    ...overrides,
+  };
+}
+
+test("builds a rooted parent-child hierarchy", () => {
+  const graph = buildTfGraph(
+    [transform("map", "base_link"), transform("base_link", "laser")],
+    10000,
+  );
+  assert.equal(graph.roots.length, 1);
+  assert.equal(graph.roots[0].frame, "map");
+  assert.equal(graph.roots[0].children[0].frame, "base_link");
+  assert.equal(graph.roots[0].children[0].children[0].frame, "laser");
+  assert.deepEqual(graph.warnings, []);
+});
+
+test("reports multiple disconnected roots and multiple parents", () => {
+  const graph = buildTfGraph(
+    [
+      transform("map_a", "base_link"),
+      transform("map_b", "base_link"),
+      transform("map_c", "camera"),
+    ],
+    10000,
+  );
+  assert.equal(graph.roots.length, 3);
+  assert.ok(graph.warnings.some((warning) => warning.includes("disconnected")));
+  assert.ok(graph.edges[0].issues.includes("Frame has multiple parents"));
+  assert.ok(graph.edges[1].issues.includes("Frame has multiple parents"));
+});
+
+test("detects cycles and keeps cyclic frames visible as unattached", () => {
+  const graph = buildTfGraph(
+    [transform("a", "b"), transform("b", "c"), transform("c", "a")],
+    10000,
+  );
+  assert.ok(graph.warnings.some((warning) => warning.includes("cycle")));
+  assert.ok(graph.edges.some((edge) => edge.issues.includes("Cycle detected in transform graph")));
+  assert.deepEqual(graph.unattachedFrames, ["a", "b", "c"]);
+});
+
+test("marks old dynamic transforms stale but exempts static transforms", () => {
+  const graph = buildTfGraph(
+    [
+      transform("map", "dynamic", { receivedAtMs: 0 }),
+      transform("map", "fixed", {
+        topic_name: "/tf_static",
+        receivedAtMs: 0,
+      }),
+    ],
+    10000,
+  );
+  assert.ok(graph.edges[0].issues.includes("Transform is stale"));
+  assert.ok(!graph.edges[1].issues.includes("Transform is stale"));
+});
+
+test("reports invalid quaternions and non-finite transforms", () => {
+  const graph = buildTfGraph(
+    [
+      transform("map", "zero", { qx: 0, qy: 0, qz: 0, qw: 0 }),
+      transform("map", "invalid", { x: Number.NaN }),
+    ],
+    10000,
+  );
+  assert.ok(graph.edges[0].issues.includes("Zero quaternion — likely uninitialized"));
+  assert.ok(graph.edges[1].issues.includes("Contains NaN or Infinity"));
+});
````
