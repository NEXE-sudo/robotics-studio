# Complete Git Diff

Base: `83e88f6` on `main`; generated from the current worktree.
Includes tracked changes and the architecture, integration-script, and release-readiness files. Excludes this file and the read-only `COMPLETE_GIT_DIFF.md` snapshot.

Validation summary: Rust fmt/check/tests/strict Clippy passed (32 tests, 1 ignored); Python 36 tests passed; frontend build and npm audit passed; cargo audit found 0 vulnerabilities and 13 allowed warnings; Docker ROS/Gazebo integration and temporary-directory cleanup passed (shell syntax valid, integration exit code 0, no temporary directory or container left behind).

## Full diff

````diff
diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
index 3363c0f..d6fe722 100644
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -31,6 +31,7 @@ jobs:
             build-essential \
             curl \
             file \
+            protobuf-compiler \
             libayatana-appindicator3-dev \
             libxdo-dev \
             libssl-dev \
@@ -40,10 +41,14 @@ jobs:
       - uses: dtolnay/rust-toolchain@stable
         with:
           components: clippy
+      - run: cargo install cargo-audit --version 0.23.0 --locked
+      - run: cargo audit --file src-tauri/Cargo.lock
       - run: cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
       - run: cargo check --manifest-path src-tauri/Cargo.toml --locked
       - run: cargo test --manifest-path src-tauri/Cargo.toml --locked
       - run: cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features --locked -- -D warnings
+      - name: Docker ROS 2 and Gazebo integration
+        run: ./scripts/docker-integration.sh
 
   python:
     name: Python service-core tests
@@ -54,4 +59,4 @@ jobs:
         with:
           python-version: "3.12"
       - run: python -m unittest discover -s src-tauri/resources/devcontainer -p 'test_*.py' -v
-      - run: python -m py_compile src-tauri/resources/devcontainer/server.py src-tauri/resources/devcontainer/server_core.py
+      - run: python -m py_compile src-tauri/resources/devcontainer/server.py src-tauri/resources/devcontainer/server_core.py src-tauri/resources/devcontainer/test_server_core.py
diff --git a/README.md b/README.md
index c5c7fca..029ab36 100644
--- a/README.md
+++ b/README.md
@@ -15,9 +15,9 @@ upfront about that than pretend otherwise.
 - **One-click ROS 2 + Gazebo environment** — the app builds, starts, and
   connects to a Docker-based ROS 2 environment automatically on launch. No
   manual Docker/Dev Container setup required.
-- **Live ROS introspection** — a real-time node/topic feed streamed over
-  gRPC while your system runs, plus a live **TF tree** view of your robot's
-  transform frames.
+- **Live ROS status** — node/topic snapshots and TF updates are streamed over
+  authenticated gRPC while the system runs. The current UI does not yet
+  provide full package, service, or action exploration.
 - **3D visualization** — robot state rendered live in a three.js viewport,
   with in-app controls to publish `cmd_vel` commands directly (no manual
   container terminal needed for basic teleop).
@@ -46,24 +46,34 @@ upfront about that than pretend otherwise.
   directories selected in a native picker; file operations reject traversal,
   symlink escapes, and oversized text files.
 - **Authenticated ROS gRPC** — application-generated session credentials,
-  expiry checks, and authenticated event/control requests. Existing containers
-  with non-loopback gRPC bindings are refused rather than treated as secure.
+  expiry checks, and authenticated event/control requests over TLS. The client
+  pins a container-generated CA certificate delivered through Docker's control
+  channel; existing containers with non-loopback gRPC bindings are refused.
 - **Safer world import** — validates SDF input and stores it in a dedicated,
   uniquely named workspace `worlds/` directory rather than overwriting files
   in the workspace root. Imports are XML-parsed, require an SDF root and world
   element, accept supported SDF versions 1.4–1.10, and reject DTDs.
-- **Prioritized ROS events** — critical discovery state is retained separately
-  from coalesced telemetry; reconnecting clients receive the latest discovery
-  snapshots.
-- **Bounded robot commands** — linear velocity magnitude is limited to
-  0.5 m/s, angular velocity magnitude to 1 rad/s, and commands expire after
-  0.5 seconds. These limits are fixed conservative defaults, not robot-specific
-  tuning or a safety-rated emergency stop.
+- **Recoverable ROS events** — selected critical node-lifecycle and safety
+  events are persisted and replayed after reconnect or service restart.
+  Coalesced telemetry and periodic discovery snapshots are best-effort, not
+  journaled. If retained history has a gap, the stream reports it and includes
+  the latest state snapshots where available.
+- **Per-robot command limits** — configure bounded linear/angular velocity,
+  acceleration, planar/full-Twist mode, and command lease/timeout per command
+  topic. Defaults remain conservative (0.5 m/s linear, 1 rad/s angular,
+  0.5-second lease); hard ceilings prevent loosening the built-in bounds.
+- **Software emergency stop** — the 3D View controls can latch motion commands
+  off, publish zero velocity on discovered `/cmd_vel` topics, and require an
+  explicit reset. Stop state and a bounded activation/reset history persist
+  across service restarts.
 
 ---
 
 ## Architecture, briefly
 
+The trust boundaries and subsystem behavior are described in
+[docs/architecture.md](./docs/architecture.md).
+
 This app has two halves that run in different places and talk to each other:
 
 - **The Tauri app** (Rust + React) — runs natively on your machine.
@@ -118,8 +128,11 @@ any of this — the app manages the environment for you.
   output (updates in place, doesn't flood the log with duplicate lines).
 - **Start Sim / Stop Sim / Reset Sim** — launches a Gazebo demo world (or
   your own custom world), visualized live in the **3D View** tab.
+- **3D View safety controls** — select a robot command topic to edit its
+  bounded limits, latch the software emergency stop, or explicitly reset it.
+  The backend remains authoritative and rejects commands while stopped.
 - **ROS Log tab** — live feed of ROS 2 node/topic activity.
-- **TF Tree tab** — live transform tree, with an **Explain** action per
+- **TF tab** — live transform updates, with an **Explain** action per
   frame for an AI-generated plain-language breakdown.
 - **AI Assistant (right panel)** — ask questions about your open file,
   recent ROS activity, or build output. Configure your provider/API key in
@@ -136,7 +149,7 @@ any of this — the app manages the environment for you.
 4. Click **Start Sim**, switch to the **3D View** tab.
 5. Use the in-app teleop controls to publish a `cmd_vel` command and watch
    the robot move.
-6. Open the **TF Tree** tab and click **Explain** on a frame.
+6. Open the **TF** tab and click **Explain** on a frame.
 7. Ask the AI assistant: *"what ROS topics are currently active?"* — it
    should answer from live data, not a guess.
 
@@ -146,6 +159,18 @@ any of this — the app manages the environment for you.
 
 - 3D view uses accurately-dimensioned placeholder geometry, not full URDF
   mesh loading yet — visually approximate, not a rendered CAD model.
+- The TF tab currently lists/group-displays received transform edges; it does
+  not yet provide a fully validated hierarchy, stale-frame expiry, or cycle
+  detection.
+- ROS package discovery, detailed node endpoint inspection, and service/action
+  exploration are not implemented. The live ROS view is not a complete graph
+  explorer.
+- URDF editing and AI generation are available, but URDF parsing, Xacro
+  processing, package-resource resolution, and description-based model
+  rendering are not implemented. The 3D view uses placeholder geometry.
+- SDF imports are checked for XML structure, a supported declared version,
+  and a world element; this is not SDFormat schema or resource-reference
+  validation.
 - ROS↔three.js coordinate mapping is a first-pass approximation — rotation
   direction hasn't been rigorously verified against every convention.
 - Only one bundled project template (`basic_publisher`) so far.
@@ -160,11 +185,27 @@ any of this — the app manages the environment for you.
   update is installed. Existing containers with insecure gRPC port bindings
   are detected and blocked, but must be recreated manually with their workspace
   bind-mounted to preserve files.
-- Robot-control limits are not configurable per robot. There is no latched
-  software emergency stop, physical-robot confirmation flow, or safety-rated
-  control guarantee.
-- Workspace selection is still not wired through every backend operation;
-  build and simulation lifecycle currently use the managed default workspace.
+- The software emergency stop is latched and backend-enforced, but there is no
+  physical-robot confirmation flow or safety-rated control guarantee.
+- The ROS workspace picker selects the active workspace for container
+  initialization, builds, simulation, and world imports. It is persisted under
+  `~/.robotics-studio/selected-workspace.txt`; opening an unrelated folder in
+  the file explorer does not change the ROS workspace.
+- Build and simulator subprocesses do not yet share a unified process manager.
+  The Build action can cancel an active streaming `colcon build` by terminating
+  its dedicated process group. Gazebo startup now waits up to 20 seconds for a
+  world-control service and cleans up if startup or bridge setup fails, but
+  generalized process status, timeouts, and application-shutdown cleanup are
+  still incomplete.
+- A bounded, workspace-local journal stores critical node-lifecycle and
+  software safety events (up to 256 records and 4 MiB) and preserves their
+  sequence numbers across service restarts. Telemetry and periodic snapshots are
+  coalesced/best-effort and are not journaled; they do not consume durable
+  sequence numbers. Replaying a range returns retained events, not an
+  exactly-once acknowledgment from the UI. Older-than-retained requests report
+  a gap and include the latest in-memory snapshots when available. Journal
+  write failures are surfaced in event health; failure while recording a
+  safety transition fail-closes the safety controller.
 - AI provider credentials are stored through the operating-system credential
   store. Linux requires an available Secret Service; the app reports an error
   if secure storage is unavailable.
@@ -172,11 +213,22 @@ any of this — the app manages the environment for you.
 ## Security notes
 
 - Native file and directory pickers register approved roots in the Rust
-  backend. Reads, writes, listings, project creation, and `.aiignore` checks
-  are constrained to those roots; text reads/writes are limited to 8 MiB.
-- New ROS containers bind gRPC to `127.0.0.1:50051`. Both gRPC methods require
-  a random 256-bit token with a 24-hour expiry. The token is transferred via a
-  protected file and is never returned to frontend JavaScript.
+  backend. File reads, writes, listings, and `.aiignore` reads are constrained
+  to those roots; text reads/writes are limited to 8 MiB. Template and
+  devcontainer copying still needs descriptor-rooted destination operations.
+- The selected ROS workspace is revalidated by the backend on use. Switching
+  workspaces stops the running managed container for the previous workspace
+  before the new environment is initialized; stopped containers are preserved.
+- New ROS containers publish gRPC only on host loopback and use TLS with a
+  per-container CA and server certificate. The app retrieves the public CA
+  certificate through Docker's control channel and pins it before sending
+  credentials. All gRPC methods also require a random 256-bit token with a
+  24-hour expiry. The token is transferred via a protected file and is never
+  returned to frontend JavaScript.
+- The running ROS service scripts and protocol are copied from the application
+  bundle into `/opt/robotics-studio` inside the container, outside the writable
+  workspace mount. Legacy workspace copies are compared byte-for-byte against
+  bundled files before migration; a source-code marker alone is not trusted.
 - When a stream reports an expired or rejected token, the app performs one
   guarded environment reinitialization: it generates a new token, replaces
   the container credential file, restarts the ROS service, and reconnects.
@@ -186,9 +238,10 @@ any of this — the app manages the environment for you.
 - An existing container with a missing or non-loopback gRPC binding is
   rejected. The app does not automatically replace that container; preserve
   its host workspace and recreate it with a loopback-only mapping.
-- Robot velocity and freshness checks run in both Rust and Python. Commands
-  are checked against combined linear and angular vector magnitudes and are
-  stopped by the server watchdog after 0.5 seconds, but this is not a
+- Robot velocity and freshness checks run in Rust and Python. The ROS service
+  enforces per-topic velocity, acceleration, mode, lease, and timeout settings
+  within hard ceilings; defaults include a 0.5-second command lease. The
+  software emergency stop publishes zero Twist commands but is not a
   replacement for a physical emergency-stop circuit.
 - Imported SDF is checked for XML well-formedness, an SDF root, a world
   element, and a supported declared SDF version. DTDs are rejected. This is
@@ -199,21 +252,30 @@ any of this — the app manages the environment for you.
   metadata is stored in the application settings JSON. Existing plaintext
   credentials are moved to secure storage on first settings load, and the old
   JSON entry is removed only after the secure-store write succeeds.
-- Path validation checks canonical containment and rejects traversal and
-  symlink escapes. It is not a kernel-level descriptor-based sandbox against a
-  local process racing filesystem changes.
-
-These controls have not been independently security-audited and are not a
-claim of security certification.
+- On Unix, approved-root file reads, writes, listings, and world imports use
+  pinned directory descriptors and no-follow `openat` operations; replacing
+  an approved root is rejected. Windows currently uses canonical-path checks
+  and atomic temporary-file replacement, and does not provide equivalent
+  protection against a local process racing parent-directory changes.
+- Not every operation has been migrated to descriptor-rooted access yet;
+  selected-world source reads and template/project creation remain areas for
+  further platform-specific race hardening.
+
+An independent source review found and this development pass fixed the
+workspace service-marker trust issue and plaintext gRPC credential transport.
+This review and the integration checks are not a claim of security
+certification.
 
 ## Migration notes for existing v0.2.0 installations
 
 - The managed ROS workspace remains in `~/.robotics-studio/ros2-workspace`;
   project files and workspace data are not removed by these changes.
-- On the next environment initialization, the app backs up existing
-  `server.py`, `server_core.py`, and `proto/ros_engine.proto` files under the
-  application config directory, then installs the authenticated server
-  protocol.
+- On the next environment initialization, the app compares existing
+  `server.py`, `server_core.py`, and `proto/ros_engine.proto` files byte-for-byte
+  with trusted bundled copies, backs up modified copies under the application
+  config directory, and installs the bundled versions. The running service is
+  launched from an application-provisioned directory inside the container,
+  not from the workspace mount.
 - Containers whose host gRPC port is bound to an external interface are
   refused. Stop the old container and recreate it with
   `-p 127.0.0.1:50051:50051` and the existing workspace mounted at
@@ -221,14 +283,22 @@ claim of security certification.
 - On first AI settings load, an existing plaintext API key is migrated to the
   operating-system credential store. If that store is unavailable, migration
   fails explicitly and the original settings file is retained for recovery.
+- The emergency stop is **software-only**: it cannot guarantee physical
+  stopping, stop controllers that ignore `/cmd_vel`, or replace a hardware
+  safety circuit. Validate stop behavior on the actual robot and maintain
+  independent hardware safeguards.
 
 ## Development checks
 
-Run the frontend build/type check, Rust formatting/check/tests/lint, and
-isolated Python service-core tests from the repository root:
+Run the frontend build/type check and audit, Rust formatting/check/tests/lint
+and dependency audit, and isolated Python service-core tests from the repository
+root. Install `cargo-audit` 0.23.0 if it is not already available.
 
 ```sh
+cargo install cargo-audit --version 0.23.0 --locked
 npm run build
+npm audit --audit-level=moderate
+cargo audit --file src-tauri/Cargo.lock
 cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
 cargo check --manifest-path src-tauri/Cargo.toml --locked
 cargo test --manifest-path src-tauri/Cargo.toml --locked
@@ -236,6 +306,27 @@ cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -
 python3 -m unittest discover -s src-tauri/resources/devcontainer -p 'test_*.py' -v
 ```
 
+For the live container integration (Docker Desktop or Docker Engine must be
+running), execute:
+
+```sh
+./scripts/docker-integration.sh
+```
+
+The script builds the supported ROS 2 Jazzy/Gazebo Harmonic image, mounts a
+temporary workspace, verifies the pinned-TLS Rust client, gRPC authentication,
+emergency-stop gating, event replay after service restart, Gazebo world/model
+readiness, reset, and ROS-Gazebo topic bridging. It performs no robot movement
+and removes its temporary container and workspace on exit. CI runs the same
+script on its Linux runner.
+
+These are unit/build checks, not Docker/ROS 2/Gazebo integration tests. They
+require Docker Desktop running for a live environment; if it is unavailable,
+simulation and gRPC integration behavior remains unverified. The current
+supported development baseline is ROS 2 Jazzy with Gazebo Harmonic in the
+bundled container. macOS is tested most often; Linux/Windows builds and
+runtime behavior need additional platform CI coverage.
+
 ---
 
 ## If something breaks
diff --git a/RELEASE_READINESS.md b/RELEASE_READINESS.md
new file mode 100644
index 0000000..feee44c
--- /dev/null
+++ b/RELEASE_READINESS.md
@@ -0,0 +1,111 @@
+# Robotics Studio v0.3.0 Release Readiness
+
+## Decision
+
+**Not ready for release.** The current worktree has meaningful security,
+recovery, and simulator-integration improvements, and the local validation
+listed below passes. The product still lacks several core workflows named in
+the v0.3.0 scope, and cross-platform desktop/CI validation is incomplete.
+
+The checkout is on `main` at `83e88f6` with uncommitted changes. No commit,
+push, tag, or release was created.
+
+## Scope checklist
+
+| # | Area | Status | Evidence and remaining work |
+|---|---|---|---|
+| 1 | gRPC authentication and transport | Pass (local) | Bearer-token requests use TLS with a client-pinned CA; the Docker integration rejects unauthenticated calls and accepts authenticated calls. |
+| 2 | Trusted ROS service installation | Pass (local) | The runtime service and generated protobufs are installed outside the writable workspace mount. Workspace migration compares complete file bytes and uses approved-root operations. |
+| 3 | Workspace filesystem boundaries | Partial | Unix descriptor-rooted reads, writes, imports, and template copies reject symlink escapes. Windows does not yet have equivalent resistance to parent-directory races. |
+| 4 | Dependency security | Partial | `cargo audit` and `npm audit` report zero vulnerabilities. RustSec still reports 13 allowed unmaintained/unsound dependency warnings; see below. |
+| 5 | Critical ROS event recovery | Partial | Critical lifecycle and safety records survive service restart and replay with sequence/gap metadata. Telemetry is best-effort, and the UI does not acknowledge delivery; end-to-end exactly-once delivery is not provided. |
+| 6 | Motion bounds and emergency stop | Partial | The backend enforces conservative per-topic motion limits and a software stop latch. This is not a physical or safety-rated emergency-stop system. |
+| 7 | Workspace selection and isolation | Partial | The backend persists and revalidates the selected workspace. Workspace propagation and cross-platform isolation have not been verified across every operation. |
+| 8 | Build process control | Partial | `colcon build` has a dedicated cancellation path. There is no centralized process registry, general timeout policy, or unified status model. |
+| 9 | Container and desktop lifecycle | Partial | A manual ROS 2 Jazzy/Gazebo Harmonic container integration passes. The packaged desktop app's full startup, shutdown, and supported-platform lifecycle has not been verified. |
+| 10 | ROS node discovery | Partial | Node snapshots are available; a complete graph explorer and detailed endpoint state are not implemented. |
+| 11 | ROS topic discovery | Partial | Topic snapshots and tested ROS-Gazebo bridges are available; detailed publisher/subscriber endpoint and QoS exploration is incomplete. |
+| 12 | ROS service exploration | Not implemented | The UI does not enumerate or inspect ROS services. |
+| 13 | ROS action exploration | Not implemented | The UI does not enumerate or inspect ROS actions. |
+| 14 | TF hierarchy validation | Partial | TF edges are displayed, but a complete validated hierarchy is not maintained. |
+| 15 | TF stale-frame and cycle handling | Not implemented | Stale-frame expiry and cycle detection are absent. |
+| 16 | Robot visualization fidelity | Partial | The 3D view uses placeholder geometry rather than a model derived from the robot description. |
+| 17 | URDF parsing | Not implemented | Robot descriptions are not parsed for visualization. |
+| 18 | Xacro processing | Not implemented | Xacro expansion is not implemented. |
+| 19 | SDF document validation | Partial | XML structure, SDF root/world presence, supported declared versions, and DTD rejection are checked; full SDFormat schema validation is absent. |
+| 20 | Model and mesh resource resolution | Not implemented | External model/mesh references are not comprehensively resolved or validated. |
+| 21 | Frontend and desktop UX test coverage | Not ready | The frontend type-check/build passes, but automated frontend/UI behavior tests and accessibility review were not run. |
+| 22 | Cross-platform release and CI verification | Not ready | The local Docker integration passes, but the hosted workflow, Windows/Linux desktop runtime, and packaged release artifacts have not been validated. |
+
+## Security review and remediation
+
+Three security issues were addressed during this work:
+
+1. **High — workspace marker spoofing.** A marker substring in a workspace
+   service file was previously accepted as evidence that the file was trusted.
+   Migration now compares the complete contents against bundled files; the
+   service actually runs from the application-installed container directory.
+2. **High — bearer token over plaintext gRPC.** A local process could
+   impersonate the service and capture the token. The client now establishes
+   TLS and validates the server against the CA certificate obtained through
+   Docker's control channel before sending authenticated requests.
+3. **Medium (local filesystem boundary) — symlinked destination during resource
+   copy.** Recursive copies could follow a workspace directory symlink and
+   write outside the approved root. Copies and service migration now use
+   approved-root operations; regression tests cover symlinked directories.
+
+The review did not establish additional exploitable issues in the event
+journal, motion validation, or the descriptor-rooted Unix file operations.
+That is not a substitute for platform-specific testing. Docker control and
+code running with container privileges remain inside the trusted boundary.
+
+## Dependency audit residuals
+
+`cargo audit` found **zero vulnerabilities** and 13 allowed warnings:
+
+| Warning group | Advisory IDs | Notes |
+|---|---|---|
+| `proc-macro-error 1.0.4` | `RUSTSEC-2024-0370` | Unmaintained; no fixed upgrade available. |
+| `unic-* 0.9.0` family | `RUSTSEC-2025-0075`, `-0080`, `-0081`, `-0098`, `-0100` | Unmaintained transitive URL-pattern dependencies; no fixed upgrade available. |
+| `zbus_names`, `zvariant`, `zvariant_derive` | `RUSTSEC-2026-0370`, `-0371`, `-0372` | Unmaintained transitive crates. Some older lockfile entries are not on an active dependency path; newer entries remain in the Tauri dependency tree. |
+| `glib 0.18.5` | `RUSTSEC-2024-0429` | Unsoundness advisory; the suggested GLib 0.20+ upgrade is blocked by the current GTK/Tauri dependency chain. |
+
+The advisories above did not provide CVSS scores in the audit output. These
+warnings are disclosed rather than suppressed.
+
+## Validation performed
+
+All commands below were run against the current worktree:
+
+| Check | Result |
+|---|---|
+| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Pass |
+| `cargo check --manifest-path src-tauri/Cargo.toml --locked` | Pass |
+| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | Pass: 32 passed, 1 ignored (the ignored test requires the live service). |
+| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets --all-features -- -D warnings` | Pass |
+| `python3 -B -m unittest test_server_core` from `src-tauri/resources/devcontainer` | Pass: 36 tests. |
+| `npm run build` | Pass; Vite reports the existing ~811 kB JavaScript chunk-size warning. |
+| `npm audit --audit-level=moderate` | Pass: 0 vulnerabilities. |
+| `cargo audit` from `src-tauri` | Pass: 0 vulnerabilities, 13 allowed warnings. |
+| `scripts/docker-integration.sh` | Pass: pinned TLS/auth, safety latch/reset, event replay after restart, Gazebo world readiness/reset, and ROS-Gazebo topic bridge. No robot motion was commanded. |
+| `git diff --check`, protocol-copy comparison, script syntax check | Pass. |
+
+The Docker integration is a manually run container harness, not a desktop UI
+test or hosted CI run. Its successful result does not establish support for all
+target operating systems or physical robot safety.
+
+## Release blockers
+
+- Implement or explicitly defer full ROS package, service, and action
+  exploration.
+- Add a centralized process lifecycle/status/timeout model and verify shutdown
+  and cancellation in the managed desktop runtime.
+- Complete TF hierarchy validation and stale/cycle handling.
+- Implement description-based robot rendering, including a declared URDF,
+  Xacro, and mesh-resource support boundary.
+- Decide whether SDF import requires full schema and external-resource
+  validation; implement it or clearly constrain supported input.
+- Complete Windows/Linux runtime and filesystem testing, frontend behavior
+  tests, and hosted CI validation.
+- Reassess the disclosed RustSec warnings when compatible upstream dependency
+  upgrades are available.
diff --git a/docs/architecture.md b/docs/architecture.md
new file mode 100644
index 0000000..b1dbb22
--- /dev/null
+++ b/docs/architecture.md
@@ -0,0 +1,107 @@
+# Architecture and trust boundaries
+
+## Components
+
+Robotics Studio is a Tauri desktop application. The React frontend invokes
+commands in the native Rust backend; it does not connect directly to Docker or
+ROS. The backend manages a ROS 2 Jazzy container based on the bundled
+`osrf/ros:jazzy-desktop` image, which installs Gazebo Harmonic and
+`ros_gz_bridge`.
+
+The selected ROS workspace is bind-mounted at `/workspace`. It is distinct
+from the folder currently open in the editor. Switching the selected workspace
+stops that workspace's running managed container before persisting the new
+selection.
+
+The ROS service source and generated protobuf modules run from
+`/opt/robotics-studio` inside the container, outside the writable workspace
+mount. The service persists its event journal, safety state, robot limits, and
+workspace data under `/workspace/.robotics-studio`.
+
+## gRPC trust model
+
+The container publishes gRPC on host loopback only. At initialization, the
+backend provisions a per-container certificate authority and a server
+certificate with a loopback subject alternative name. It retrieves only the
+public CA certificate through Docker's control channel and configures the Rust
+gRPC client to trust that certificate. The client sends its random, expiring
+bearer token only after establishing TLS. The token is stored in protected
+files and is not exposed to frontend JavaScript.
+
+This protects against an unrelated local process impersonating the service on
+the loopback port. A user with Docker control, or code running with the
+container's privileges, remains inside the trusted computing boundary.
+
+## Motion and safety
+
+Motion requests are checked in both the Rust command path and the Python ROS
+service. The service enforces topic-specific velocity, acceleration, mode,
+freshness, lease, and timeout limits. A server-side safety controller serializes
+motion publication with emergency-stop transitions. A latched stop blocks
+motion until an explicit reset; restored latches are reasserted on startup.
+
+The software stop publishes zero velocity messages to discovered command
+topics. It cannot stop arbitrary controllers, guarantee physical braking, or
+replace a safety-rated controller or physical emergency-stop circuit.
+
+## Events and recovery
+
+The event hub prioritizes critical events over coalesced telemetry. Selected
+node lifecycle and safety events are durably journaled, with a bound of 256
+records and 4 MiB. Sequence numbers are durable for journaled critical events;
+telemetry and periodic snapshots are best-effort and do not consume those
+sequence numbers. Replay subscribes to live delivery atomically with its
+snapshot to avoid a replay/live handoff gap.
+
+Replay is cursor-based and reports when history has expired. The client
+resynchronizes from available snapshots after a gap. The UI does not
+acknowledge event application, so the end-to-end frontend delivery contract is
+not exactly-once and must not be described as such.
+
+## Workspace and files
+
+Native file operations are scoped to roots selected through native pickers.
+On Unix, the principal read, write, listing, and import paths pin directory
+descriptors and reject symlink traversal. Windows uses canonical-path checks
+and atomic replacement but does not yet have equivalent protection from a
+local process racing parent-directory changes. Some template and project
+operations are not yet descriptor-rooted on every platform.
+
+An editor folder does not implicitly become the ROS workspace. ROS
+initialization, build, simulation, and world import use the selected workspace;
+other general file editing uses the separately approved editor folder.
+
+## Process and simulation lifecycle
+
+The `colcon build` operation has a dedicated process group and a cancellation
+path. Gazebo startup waits for a world control service before launching the
+bridge, and failed startup attempts trigger cleanup. Reset targets the active
+world's discovered control service rather than assuming a `default` world.
+
+Build, container, Gazebo, and bridge processes do not yet share a centralized
+process registry. General execution timeouts, unified status reporting,
+graceful application-shutdown cleanup, and platform-specific lifecycle
+coverage remain incomplete.
+
+## Supported ROS surface
+
+The current service streams node and topic snapshots, transform and odometry
+updates, safety state, and event-recovery metadata. It is not a full ROS graph
+explorer: detailed publisher/subscriber endpoints, services, and actions are
+not exposed. TF updates are visualized as received edges but are not yet
+validated as a complete hierarchy with stale-frame expiry and cycle detection.
+The 3D view uses placeholder geometry; URDF parsing, Xacro processing, mesh
+resource resolution, and description-based robot rendering are not implemented.
+
+SDF import checks XML well-formedness, an SDF root, a world element, supported
+declared versions, and rejects DTDs. It does not perform full SDFormat schema
+validation or resolve and validate all external model and mesh resources.
+
+## Validation boundary
+
+Run unit/build checks with the commands in the README. The live integration
+script `scripts/docker-integration.sh` builds the container image and exercises
+the real ROS service, pinned TLS, event replay after restart, Gazebo world
+readiness/reset, and ROS-Gazebo topic bridging. This does not substitute for
+testing the desktop UI on every target operating system or testing a physical
+robot.
diff --git a/scripts/docker-integration.sh b/scripts/docker-integration.sh
new file mode 100755
index 0000000..43c78e0
--- /dev/null
+++ b/scripts/docker-integration.sh
@@ -0,0 +1,283 @@
+#!/usr/bin/env bash
+set -euo pipefail
+
+repo_root="$(cd "$(dirname "$0")/.." && pwd)"
+workspace="$(mktemp -d /tmp/robotics-studio-integration.XXXXXX)"
+container="robotics-studio-integration-$$"
+image="robotics-studio-ros2:integration"
+container_may_exist=0
+
+cleanup() {
+    local exit_code=$?
+    local container_removed=1
+    local workspace_removed=0
+    trap - EXIT
+
+    if [[ "$container_may_exist" == 1 ]]; then
+        if ! command -v docker >/dev/null 2>&1 || ! docker info >/dev/null 2>&1; then
+            container_removed=0
+            echo "WARNING: Docker is unavailable; cannot confirm integration container removal: $container" >&2
+        elif ! docker rm -f "$container" >/dev/null 2>&1 &&
+            docker inspect "$container" >/dev/null 2>&1; then
+            container_removed=0
+            echo "WARNING: Could not stop and remove integration container: $container" >&2
+        fi
+    fi
+
+    if [[ "$container_removed" == 1 && -d "$workspace" ]]; then
+        # The integration image runs as root and may create root-owned files
+        # in the bind-mounted workspace. Remove those only after its container
+        # has been stopped and removed.
+        if command -v docker >/dev/null 2>&1 &&
+            docker image inspect "$image" >/dev/null 2>&1 &&
+            docker run --rm --user 0:0 \
+                --mount "type=bind,source=$workspace,target=/cleanup" \
+                --entrypoint /bin/bash "$image" -c \
+                'find /cleanup -mindepth 1 -maxdepth 1 -exec rm -rf -- {} +' \
+                >/dev/null 2>&1 &&
+            rm -rf -- "$workspace" 2>/dev/null; then
+            workspace_removed=1
+        fi
+
+        if [[ "$workspace_removed" == 0 ]] && rm -rf -- "$workspace" 2>/dev/null; then
+            workspace_removed=1
+        fi
+
+        if [[ "$workspace_removed" == 0 ]] &&
+            command -v sudo >/dev/null 2>&1 &&
+            sudo -n true >/dev/null 2>&1 &&
+            sudo -n rm -rf -- "$workspace" 2>/dev/null; then
+            workspace_removed=1
+        fi
+
+        if [[ -e "$workspace" ]]; then
+            echo "WARNING: Could not completely remove temporary directory: $workspace" >&2
+        fi
+    elif [[ "$container_removed" == 0 ]]; then
+        echo "WARNING: Leaving bind-mounted temporary directory in place because the container may still exist: $workspace" >&2
+    fi
+
+    exit "$exit_code"
+}
+
+trap cleanup EXIT
+
+docker info >/dev/null
+docker build -t "$image" "$repo_root/src-tauri/resources/devcontainer"
+
+cp "$repo_root/src-tauri/resources/devcontainer/diff_drive.sdf" "$workspace/diff_drive.sdf"
+printf 'workspace-mount-ok\n' > "$workspace/mount-check"
+container_may_exist=1
+docker run -d \
+  --name "$container" \
+  -p 127.0.0.1:50051:50051 \
+  -v "$workspace:/workspace" \
+  "$image" tail -f /dev/null >/dev/null
+
+docker exec "$container" mkdir -p \
+  /opt/robotics-studio/proto \
+  /opt/robotics-studio/generated \
+  /run/robotics-studio
+docker cp "$repo_root/src-tauri/resources/devcontainer/server.py" \
+  "$container:/opt/robotics-studio/server.py" >/dev/null
+docker cp "$repo_root/src-tauri/resources/devcontainer/server_core.py" \
+  "$container:/opt/robotics-studio/server_core.py" >/dev/null
+docker cp "$repo_root/src-tauri/resources/devcontainer/proto/ros_engine.proto" \
+  "$container:/opt/robotics-studio/proto/ros_engine.proto" >/dev/null
+
+docker exec "$container" bash -lc \
+  'source /opt/ros/jazzy/setup.bash && python3 -m grpc_tools.protoc -I /opt/robotics-studio/proto --python_out=/opt/robotics-studio/generated --grpc_python_out=/opt/robotics-studio/generated /opt/robotics-studio/proto/ros_engine.proto'
+
+docker exec "$container" bash -lc '
+  set -eu
+  chmod 700 /run/robotics-studio
+  openssl req -x509 -newkey rsa:2048 -nodes -days 3650 \
+    -keyout /run/robotics-studio/grpc-ca.key \
+    -out /run/robotics-studio/grpc-ca.crt \
+    -subj "/CN=Robotics Studio local gRPC CA" \
+    -addext "basicConstraints=critical,CA:TRUE" \
+    -addext "keyUsage=critical,keyCertSign,cRLSign" >/dev/null 2>&1
+  openssl req -new -newkey rsa:2048 -nodes \
+    -keyout /run/robotics-studio/grpc-server.key \
+    -out /run/robotics-studio/grpc-server.csr \
+    -subj "/CN=127.0.0.1" >/dev/null 2>&1
+  printf "%s\n" \
+    "basicConstraints=critical,CA:FALSE" \
+    "keyUsage=critical,digitalSignature,keyEncipherment" \
+    "extendedKeyUsage=serverAuth" \
+    "subjectAltName=IP:127.0.0.1,DNS:localhost" \
+    > /run/robotics-studio/grpc-server.ext
+  openssl x509 -req -in /run/robotics-studio/grpc-server.csr \
+    -CA /run/robotics-studio/grpc-ca.crt \
+    -CAkey /run/robotics-studio/grpc-ca.key -CAcreateserial \
+    -out /run/robotics-studio/grpc-server.crt -days 3650 -sha256 \
+    -extfile /run/robotics-studio/grpc-server.ext >/dev/null 2>&1
+  chmod 600 /run/robotics-studio/grpc-ca.key /run/robotics-studio/grpc-server.key
+  chmod 644 /run/robotics-studio/grpc-ca.crt /run/robotics-studio/grpc-server.crt
+'
+docker exec "$container" cat /run/robotics-studio/grpc-ca.crt > "$workspace/grpc-ca.crt"
+
+token="$(openssl rand -hex 32)"
+expires_at="$(python3 -c 'import time; print(int(time.time()) + 86400)')"
+printf '{"token":"%s","expires_at":%s}' "$token" "$expires_at" |
+  docker exec -i "$container" bash -c \
+    'cat > /run/robotics-studio/grpc-auth.json && chmod 600 /run/robotics-studio/grpc-auth.json'
+
+start_ros_server() {
+  docker exec -d "$container" bash -lc \
+    'source /opt/ros/jazzy/setup.bash && cd /opt/robotics-studio && ROBOTICS_STUDIO_GENERATED_PATH=/opt/robotics-studio/generated exec python3 /opt/robotics-studio/server.py >/tmp/server.log 2>&1'
+  for attempt in $(seq 1 30); do
+    if docker exec "$container" bash -lc \
+      'grep -q "running on port 50051" /tmp/server.log' 2>/dev/null; then
+      return
+    fi
+    sleep 1
+  done
+  docker exec "$container" cat /tmp/server.log
+  return 1
+}
+
+test "$(docker exec "$container" cat /workspace/mount-check)" = workspace-mount-ok
+start_ros_server
+
+ROBOTICS_STUDIO_TEST_TLS_CERTIFICATE="$workspace/grpc-ca.crt" \
+ROBOTICS_STUDIO_TEST_AUTH_TOKEN="$token" \
+  cargo test --manifest-path "$repo_root/src-tauri/Cargo.toml" --locked -- \
+    --ignored --exact \
+    ros_docker_integration_tests::pinned_tls_client_authenticates_to_docker_ros_service
+
+docker exec -i "$container" bash -c 'cd /opt/robotics-studio && python3 -' <<'PY'
+import json
+import sys
+import time
+
+sys.path.insert(0, "/opt/robotics-studio/generated")
+import grpc
+import ros_engine_pb2 as pb
+import ros_engine_pb2_grpc as rpc
+
+certificate = open("/run/robotics-studio/grpc-ca.crt", "rb").read()
+token = json.load(open("/run/robotics-studio/grpc-auth.json"))["token"]
+stub = rpc.RosEngineStub(
+    grpc.secure_channel(
+        "127.0.0.1:50051",
+        grpc.ssl_channel_credentials(root_certificates=certificate),
+    )
+)
+metadata = (("x-robotics-studio-token", token),)
+try:
+    stub.GetSafetyState(pb.Empty(), timeout=3)
+    raise AssertionError("unauthenticated request was accepted")
+except grpc.RpcError as error:
+    assert error.code() == grpc.StatusCode.UNAUTHENTICATED, error
+
+assert stub.GetSafetyState(pb.Empty(), metadata=metadata, timeout=3).state == pb.SafetyState.READY
+activated = stub.SetEmergencyStop(
+    pb.EmergencyStopRequest(action=pb.EmergencyStopRequest.ACTIVATE),
+    metadata=metadata,
+    timeout=3,
+)
+assert activated.state == pb.SafetyState.ACTIVE
+try:
+    stub.PublishTwist(
+        pb.TwistCommand(
+            topic_name="/audit/cmd_vel",
+            linear_x=0.1,
+            timestamp_ns=time.time_ns(),
+        ),
+        metadata=metadata,
+        timeout=3,
+    )
+    raise AssertionError("motion command was accepted while latched")
+except grpc.RpcError as error:
+    assert error.code() == grpc.StatusCode.INVALID_ARGUMENT, error
+assert stub.SetEmergencyStop(
+    pb.EmergencyStopRequest(action=pb.EmergencyStopRequest.RESET),
+    metadata=metadata,
+    timeout=3,
+).state == pb.SafetyState.READY
+
+events = stub.ReplayEvents(
+    pb.ReplayEventsRequest(has_after_sequence=True, after_sequence=0),
+    metadata=metadata,
+    timeout=5,
+)
+assert next(events).WhichOneof("event") == "event_recovery"
+assert next(events).WhichOneof("event") == "safety_state"
+events.cancel()
+print("Authenticated TLS, stop/reset enforcement and event replay passed")
+PY
+
+docker exec "$container" bash -lc \
+  "pkill -TERM -f '[p]ython3 /opt/robotics-studio/server.py' || true"
+start_ros_server
+docker exec -i "$container" bash -c 'cd /opt/robotics-studio && python3 -' <<'PY'
+import json
+import sys
+
+sys.path.insert(0, "/opt/robotics-studio/generated")
+import grpc
+import ros_engine_pb2 as pb
+import ros_engine_pb2_grpc as rpc
+
+root = open("/run/robotics-studio/grpc-ca.crt", "rb").read()
+token = json.load(open("/run/robotics-studio/grpc-auth.json"))["token"]
+stub = rpc.RosEngineStub(
+    grpc.secure_channel(
+        "127.0.0.1:50051",
+        grpc.ssl_channel_credentials(root_certificates=root),
+    )
+)
+events = stub.ReplayEvents(
+    pb.ReplayEventsRequest(has_after_sequence=True, after_sequence=0),
+    metadata=(("x-robotics-studio-token", token),),
+    timeout=5,
+)
+assert next(events).WhichOneof("event") == "event_recovery"
+event = next(events)
+assert event.WhichOneof("event") == "safety_state" and event.sequence > 0
+events.cancel()
+print("Critical safety history survived a ROS service restart")
+PY
+
+docker exec -d "$container" bash -lc \
+  'source /opt/ros/jazzy/setup.bash && gz sim -s -r --headless-rendering /workspace/diff_drive.sdf >/tmp/gz.log 2>&1'
+world_services=""
+for attempt in $(seq 1 30); do
+  if world_services="$(docker exec "$container" bash -lc \
+    'source /opt/ros/jazzy/setup.bash && gz service -l' 2>/dev/null)" &&
+    printf '%s\n' "$world_services" | grep -Fx '/world/diff_drive/control' >/dev/null; then
+    break
+  fi
+  sleep 1
+done
+if ! printf '%s\n' "$world_services" | grep -Fx '/world/diff_drive/control' >/dev/null; then
+  docker exec "$container" cat /tmp/gz.log
+  echo "Gazebo did not advertise /world/diff_drive/control" >&2
+  exit 1
+fi
+world_control='/world/diff_drive/control'
+docker exec "$container" bash -lc \
+  'source /opt/ros/jazzy/setup.bash && gz topic -l' |
+  grep -F '/model/vehicle_blue' >/dev/null
+docker exec "$container" bash -c \
+  'source /opt/ros/jazzy/setup.bash && gz service -s "$1" --reqtype gz.msgs.WorldControl --reptype gz.msgs.Boolean --timeout 2000 --req "reset: {all: true}"' \
+  robotics-studio-integration "$world_control"
+
+docker exec -d "$container" bash -lc \
+  'source /opt/ros/jazzy/setup.bash && ros2 run ros_gz_bridge parameter_bridge /model/vehicle_blue/cmd_vel@geometry_msgs/msg/Twist@gz.msgs.Twist /model/vehicle_blue/odometry@nav_msgs/msg/Odometry@gz.msgs.Odometry /model/vehicle_blue/tf@tf2_msgs/msg/TFMessage@gz.msgs.Pose_V >/tmp/bridge.log 2>&1'
+for attempt in $(seq 1 20); do
+  if topics="$(docker exec "$container" bash -lc \
+    'source /opt/ros/jazzy/setup.bash && ros2 topic list' 2>/dev/null)" &&
+    printf '%s\n' "$topics" | grep -Fx '/model/vehicle_blue/cmd_vel' >/dev/null; then
+    break
+  fi
+  sleep 1
+done
+docker exec "$container" bash -lc \
+  'source /opt/ros/jazzy/setup.bash && ros2 topic list' |
+  grep -Fx '/model/vehicle_blue/cmd_vel' >/dev/null
+docker exec "$container" bash -lc \
+  "pkill -TERM -f '[g]z sim -s' || true; pkill -TERM -f '[p]arameter_bridge' || true"
+
+echo "Docker ROS 2 Jazzy and Gazebo Harmonic integration checks passed"
diff --git a/src-tauri/Cargo.lock b/src-tauri/Cargo.lock
index 2ff9ed2..0dbebe6 100644
--- a/src-tauri/Cargo.lock
+++ b/src-tauri/Cargo.lock
@@ -84,9 +84,9 @@ dependencies = [
 
 [[package]]
 name = "async-executor"
-version = "1.14.0"
+version = "1.14.1"
 source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "c96bf972d85afc50bf5ab8fe2d54d1586b4e0b46c97c50a0c9e71e2f7bcd812a"
+checksum = "f426a31126c735126297bc5ea7a521263c9faff4f63ca415267ee625651c4f37"
 dependencies = [
  "async-task",
  "concurrent-queue",
@@ -226,9 +226,9 @@ checksum = "f2032f911046de80f0a198e0901378627c33f59ea0ac00e363d481118bd70a53"
 
 [[package]]
 name = "aws-lc-rs"
-version = "1.17.3"
+version = "1.19.0"
 source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "00bdb5da18dac48ca2cc7cd4a98e533e8635a58e2361d13a1a4ee3888e0d72f1"
+checksum = "048006790404103c488e6c5616b3e7e702a71292aadda08e8f7698a9e0d2eb53"
 dependencies = [
  "aws-lc-sys",
  "zeroize",
@@ -236,9 +236,9 @@ dependencies = [
 
 [[package]]
 name = "aws-lc-sys"
-version = "0.43.0"
+version = "0.47.0"
 source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "43103168cc76fe62678a375e722fc9cb3a0146159ac5828bc4f0dfd755c2224c"
+checksum = "8bfbe32ac36b3c428696ce315464181bc4029d689fd58e3f2a4e58c252242f52"
 dependencies = [
  "cc",
  "cmake",
@@ -558,9 +558,9 @@ checksum = "f079e83a288787bcd14a6aea84cee5c87a67c5a3e660c30f557a3d24761b3527"
 
 [[package]]
 name = "chacha20"
-version = "0.10.1"
+version = "0.10.2"
 source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "d524456ba66e72eb8b115ff89e01e497f8e6d11d78b70b1aa13c0fbd97540a81"
+checksum = "65c35e4b699c7e15ccbe7ee35c005e4fc0a278d22238a2857e6ce2dadeda1b06"
 dependencies = [
  "cfg-if",
  "cpufeatures 0.3.0",
@@ -1091,16 +1091,15 @@ source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "39cab71617ae0d63f51a36d69f866391735b51691dbda63cf6f96d042b63efeb"
 dependencies = [
  "libc",
- "windows-sys 0.61.2",
+ "windows-sys 0.52.0",
 ]
 
 [[package]]
 name = "event-listener"
-version = "5.4.1"
+version = "5.4.2"
 source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "e13b66accf52311f30a0db42147dadea9850cb48cd070028831ae5f5d4b856ab"
+checksum = "5a23add41df1562121a9393cb065eab5146a1242410f23a644851e90cfd669d2"
 dependencies = [
- "concurrent-queue",
  "parking",
  "pin-project-lite",
 ]
@@ -1604,9 +1603,9 @@ dependencies = [
 
 [[package]]
 name = "h2"
-version = "0.4.15"
+version = "0.4.16"
 source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "6cb093c84e8bd9b188d4c4a8cb6579fc016968d14c99882163cd3ff402a4f155"
+checksum = "a9f37a958b41b3b19ee2707c06439c0e9e547e847223eb791ecb0cb821c65e27"
 dependencies = [
  "atomic-waker",
  "bytes",
@@ -3149,9 +3148,9 @@ dependencies = [
 
 [[package]]
 name = "pulldown-cmark"
-version = "0.13.4"
+version = "0.13.6"
 source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "e9f068eba8e7071c5f9511831b44f32c740d5adf574e990f946ddb53db2f314e"
+checksum = "3db2be46835f013dd3f4fa7f5f7402c537787b54ebb6fe2b94bbdd7e2669bb7b"
 dependencies = [
  "bitflags 2.13.1",
  "memchr",
@@ -3198,9 +3197,9 @@ dependencies = [
 
 [[package]]
 name = "quinn-proto"
-version = "0.11.16"
+version = "0.11.18"
 source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "2f4bfc015262b9df63c8845072ce59068853ff5872180c2ce2f13038b970e560"
+checksum = "a9746dbde176634f4f2f1faf2404e30a31b2bc1e9cafb5329c95d8177a18c9fc"
 dependencies = [
  "aws-lc-rs",
  "bytes",
@@ -3230,7 +3229,7 @@ dependencies = [
  "once_cell",
  "socket2",
  "tracing",
- "windows-sys 0.61.2",
+ "windows-sys 0.52.0",
 ]
 
 [[package]]
@@ -3475,6 +3474,7 @@ dependencies = [
  "prost",
  "quick-xml",
  "reqwest",
+ "rustix",
  "serde",
  "serde_json",
  "tauri",
@@ -3513,17 +3513,19 @@ dependencies = [
  "errno",
  "libc",
  "linux-raw-sys",
- "windows-sys 0.61.2",
+ "windows-sys 0.52.0",
 ]
 
 [[package]]
 name = "rustls"
-version = "0.23.42"
+version = "0.23.46"
 source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "3c54fcab019b409d04215d3a17cb438fd7fbf192ee61461f20f4fe18704bc138"
+checksum = "48e13bd8c0e9365c43cfa5c9e8f9ad49d3c8444926c9aac819e0e4dc503c8fdf"
 dependencies = [
  "aws-lc-rs",
+ "log",
  "once_cell",
+ "ring",
  "rustls-pki-types",
  "rustls-webpki",
  "subtle",
@@ -3570,7 +3572,7 @@ dependencies = [
  "security-framework 3.7.0",
  "security-framework-sys",
  "webpki-root-certs",
- "windows-sys 0.61.2",
+ "windows-sys 0.52.0",
 ]
 
 [[package]]
@@ -3581,9 +3583,9 @@ checksum = "f87165f0995f63a9fbeea62b64d10b4d9d8e78ec6d7d51fb2125fda7bb36788f"
 
 [[package]]
 name = "rustls-webpki"
-version = "0.103.13"
+version = "0.103.16"
 source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "61c429a8649f110dddef65e2a5ad240f747e85f7758a6bccc7e5777bd33f756e"
+checksum = "4443b1b6d39d7c663139942cc4558262562d2c84123bb6282ef177009ca8b281"
 dependencies = [
  "aws-lc-rs",
  "ring",
@@ -4552,10 +4554,10 @@ source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "32497e9a4c7b38532efcdebeef879707aa9f794296a4f0244f6f69e9bc8574bd"
 dependencies = [
  "fastrand",
- "getrandom 0.4.3",
+ "getrandom 0.3.4",
  "once_cell",
  "rustix",
- "windows-sys 0.61.2",
+ "windows-sys 0.52.0",
 ]
 
 [[package]]
@@ -4664,9 +4666,9 @@ checksum = "1f3ccbac311fea05f86f61904b462b55fb3df8837a366dfc601a0161d0532f20"
 
 [[package]]
 name = "tokio"
-version = "1.53.1"
+version = "1.53.3"
 source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "202caea871b69668250d242070849eb495be178ed697a3e98aebce5bc81a0bed"
+checksum = "ce3335fa71841cda333a58d7615b03901380ecf09d59b3296d21f8bbac0dde4e"
 dependencies = [
  "bytes",
  "libc",
@@ -4867,6 +4869,7 @@ dependencies = [
  "socket2",
  "sync_wrapper",
  "tokio",
+ "tokio-rustls",
  "tokio-stream",
  "tower",
  "tower-layer",
@@ -5364,9 +5367,9 @@ dependencies = [
 
 [[package]]
 name = "webpki-root-certs"
-version = "1.0.9"
+version = "1.0.10"
 source = "registry+https://github.com/rust-lang/crates.io-index"
-checksum = "b96554aa2acc8ccdb7e1c9a58a7a68dd5d13bccc69cd124cb09406db612a1c9b"
+checksum = "58ec80e73fd8baf57bf0d9615554c75348bda990d6d5bbeab964dd068319fecb"
 dependencies = [
  "rustls-pki-types",
 ]
@@ -5429,7 +5432,7 @@ version = "0.1.11"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "c2a7b1c03c876122aa43f3020e6c3c3ee5c05081c9a00739faf7503aeba10d22"
 dependencies = [
- "windows-sys 0.61.2",
+ "windows-sys 0.52.0",
 ]
 
 [[package]]
diff --git a/src-tauri/Cargo.toml b/src-tauri/Cargo.toml
index 35640f5..e7ec03d 100644
--- a/src-tauri/Cargo.toml
+++ b/src-tauri/Cargo.toml
@@ -25,7 +25,7 @@ tauri-plugin-opener = "2"
 serde = { version = "1", features = ["derive"] }
 serde_json = "1"
 tauri-plugin-dialog = "2.7.2"
-tonic = "0.14.6"
+tonic = { version = "0.14.6", features = ["tls-ring"] }
 prost = "0.14.4"
 quick-xml = "0.41.0"
 tokio = { version = "1.53.1", features = ["full"] }
@@ -33,3 +33,6 @@ tonic-prost = "0.14.6"
 reqwest = { version = "0.13.4", features = ["json"] }
 getrandom = "0.3.4"
 keyring = { version = "3.6.3", default-features = false, features = ["apple-native", "windows-native", "linux-native-sync-persistent", "crypto-rust"] }
+
+[target.'cfg(unix)'.dependencies]
+rustix = { version = "1.1.4", features = ["fs"] }
diff --git a/src-tauri/proto/ros_engine.proto b/src-tauri/proto/ros_engine.proto
index 38f6c35..02ad148 100644
--- a/src-tauri/proto/ros_engine.proto
+++ b/src-tauri/proto/ros_engine.proto
@@ -5,7 +5,13 @@ package rosengine;
 
 service RosEngine {
   rpc StreamEvents(Empty) returns (stream WorkspaceEvent);
+  rpc ReplayEvents(ReplayEventsRequest) returns (stream WorkspaceEvent);
   rpc PublishTwist(TwistCommand) returns (Empty);
+  rpc SetEmergencyStop(EmergencyStopRequest) returns (SafetyState);
+  rpc GetSafetyState(Empty) returns (SafetyState);
+  rpc GetRobotConfig(RobotConfigRequest) returns (RobotConfig);
+  rpc SetRobotConfig(RobotConfig) returns (RobotConfig);
+  rpc GetEventHealth(Empty) returns (EventHealth);
 }
 
 message TwistCommand {
@@ -21,6 +27,62 @@ message TwistCommand {
 
 message Empty {}
 
+message ReplayEventsRequest {
+  bool has_after_sequence = 1;
+  uint64 after_sequence = 2;
+}
+
+message EmergencyStopRequest {
+  enum Action {
+    ACTION_UNSPECIFIED = 0;
+    ACTIVATE = 1;
+    RESET = 2;
+  }
+  Action action = 1;
+}
+
+message SafetyState {
+  enum State {
+    READY = 0;
+    ACTIVE = 1;
+    RESET_PENDING = 2;
+    FAULT = 3;
+    DISCONNECTED = 4;
+  }
+  State state = 1;
+  string detail = 2;
+  int64 updated_at_ns = 3;
+}
+
+message RobotConfigRequest {
+  string topic_name = 1;
+}
+
+message RobotConfig {
+  string topic_name = 1;
+  double max_linear_velocity = 2;
+  double max_angular_velocity = 3;
+  double max_linear_acceleration = 4;
+  double max_angular_acceleration = 5;
+  uint32 command_lease_ms = 6;
+  uint32 command_timeout_ms = 7;
+  repeated string permitted_motion_modes = 8;
+  bool reset_to_defaults = 9;
+}
+
+message EventHealth {
+  uint32 queue_capacity = 1;
+  uint32 subscriber_count = 2;
+  uint64 last_sequence = 3;
+  uint64 dropped_telemetry = 4;
+  uint64 critical_overflows = 5;
+  uint32 max_backlog = 6;
+  bool persistence_healthy = 7;
+  string persistence_error = 8;
+  uint64 oldest_available_sequence = 9;
+  uint32 retained_critical_events = 10;
+}
+
 message WorkspaceEvent {
   oneof event {
     NodeStarted node_started = 1;
@@ -29,8 +91,20 @@ message WorkspaceEvent {
     OdometryUpdate odometry_update = 4;
     TFUpdate tf_update = 5;
     NodeSnapshot node_snapshot = 6;
+    SafetyState safety_state = 7;
+    EventHealth event_health = 8;
+    EventRecovery event_recovery = 9;
   }
   int64 timestamp_ns = 10;
+  uint64 sequence = 11;
+}
+
+message EventRecovery {
+  bool gap_detected = 1;
+  uint64 oldest_available_sequence = 2;
+  uint64 latest_sequence = 3;
+  bool snapshot_included = 4;
+  uint32 retained_critical_events = 5;
 }
 
 message NodeSnapshot {
diff --git a/src-tauri/resources/devcontainer/proto/ros_engine.proto b/src-tauri/resources/devcontainer/proto/ros_engine.proto
index 38f6c35..02ad148 100644
--- a/src-tauri/resources/devcontainer/proto/ros_engine.proto
+++ b/src-tauri/resources/devcontainer/proto/ros_engine.proto
@@ -5,7 +5,13 @@ package rosengine;
 
 service RosEngine {
   rpc StreamEvents(Empty) returns (stream WorkspaceEvent);
+  rpc ReplayEvents(ReplayEventsRequest) returns (stream WorkspaceEvent);
   rpc PublishTwist(TwistCommand) returns (Empty);
+  rpc SetEmergencyStop(EmergencyStopRequest) returns (SafetyState);
+  rpc GetSafetyState(Empty) returns (SafetyState);
+  rpc GetRobotConfig(RobotConfigRequest) returns (RobotConfig);
+  rpc SetRobotConfig(RobotConfig) returns (RobotConfig);
+  rpc GetEventHealth(Empty) returns (EventHealth);
 }
 
 message TwistCommand {
@@ -21,6 +27,62 @@ message TwistCommand {
 
 message Empty {}
 
+message ReplayEventsRequest {
+  bool has_after_sequence = 1;
+  uint64 after_sequence = 2;
+}
+
+message EmergencyStopRequest {
+  enum Action {
+    ACTION_UNSPECIFIED = 0;
+    ACTIVATE = 1;
+    RESET = 2;
+  }
+  Action action = 1;
+}
+
+message SafetyState {
+  enum State {
+    READY = 0;
+    ACTIVE = 1;
+    RESET_PENDING = 2;
+    FAULT = 3;
+    DISCONNECTED = 4;
+  }
+  State state = 1;
+  string detail = 2;
+  int64 updated_at_ns = 3;
+}
+
+message RobotConfigRequest {
+  string topic_name = 1;
+}
+
+message RobotConfig {
+  string topic_name = 1;
+  double max_linear_velocity = 2;
+  double max_angular_velocity = 3;
+  double max_linear_acceleration = 4;
+  double max_angular_acceleration = 5;
+  uint32 command_lease_ms = 6;
+  uint32 command_timeout_ms = 7;
+  repeated string permitted_motion_modes = 8;
+  bool reset_to_defaults = 9;
+}
+
+message EventHealth {
+  uint32 queue_capacity = 1;
+  uint32 subscriber_count = 2;
+  uint64 last_sequence = 3;
+  uint64 dropped_telemetry = 4;
+  uint64 critical_overflows = 5;
+  uint32 max_backlog = 6;
+  bool persistence_healthy = 7;
+  string persistence_error = 8;
+  uint64 oldest_available_sequence = 9;
+  uint32 retained_critical_events = 10;
+}
+
 message WorkspaceEvent {
   oneof event {
     NodeStarted node_started = 1;
@@ -29,8 +91,20 @@ message WorkspaceEvent {
     OdometryUpdate odometry_update = 4;
     TFUpdate tf_update = 5;
     NodeSnapshot node_snapshot = 6;
+    SafetyState safety_state = 7;
+    EventHealth event_health = 8;
+    EventRecovery event_recovery = 9;
   }
   int64 timestamp_ns = 10;
+  uint64 sequence = 11;
+}
+
+message EventRecovery {
+  bool gap_detected = 1;
+  uint64 oldest_available_sequence = 2;
+  uint64 latest_sequence = 3;
+  bool snapshot_included = 4;
+  uint32 retained_critical_events = 5;
 }
 
 message NodeSnapshot {
diff --git a/src-tauri/resources/devcontainer/server.py b/src-tauri/resources/devcontainer/server.py
index 85abdf0..68cbdbb 100644
--- a/src-tauri/resources/devcontainer/server.py
+++ b/src-tauri/resources/devcontainer/server.py
@@ -13,7 +13,13 @@ from rclpy.node import Node
 AUTH_PROTOCOL_VERSION = 1
 
 import sys
-sys.path.append('generated')
+sys.path.insert(
+    0,
+    os.environ.get(
+        "ROBOTICS_STUDIO_GENERATED_PATH",
+        "/opt/robotics-studio/generated",
+    ),
+)
 import ros_engine_pb2
 import ros_engine_pb2_grpc
 from tf2_msgs.msg import TFMessage
@@ -22,7 +28,12 @@ from nav_msgs.msg import Odometry
 from geometry_msgs.msg import Twist
 from server_core import (
     CommandLeaseManager,
+    EventPersistenceError,
+    EventOverflow,
     EventHub,
+    RobotConfigRegistry,
+    RobotMotionConfig,
+    SafetyController,
     is_valid_auth_token,
     load_auth_credentials,
     validate_twist_request,
@@ -38,12 +49,59 @@ class RosIntrospector(Node):
         self.odom_subscribers = {}
         self.timer = self.create_timer(2.0, self.tick)
         self.twist_publishers = {}
+        self._command_lock = threading.RLock()
         self.command_leases = CommandLeaseManager()
+        self.robot_configs = RobotConfigRegistry(
+            os.environ.get(
+                "ROBOTICS_STUDIO_ROBOT_CONFIG",
+                "/workspace/.robotics-studio/robot-config.json",
+            )
+        )
+        self.safety = SafetyController(
+            self._publish_stop_to_all,
+            os.environ.get(
+                "ROBOTICS_STUDIO_SAFETY_STATE",
+                "/workspace/.robotics-studio/safety-state.json",
+            ),
+        )
+        self._startup_stop_pending = self.safety.snapshot()[0] != "READY"
         self.command_timeout_timer = self.create_timer(
             0.1, self.stop_expired_commands
         )
         self.tf_subscribers = {}
 
+    def _publish_stop_to_all(self):
+        with self._command_lock:
+            stop_topics = set(self.twist_publishers)
+            failures = []
+            try:
+                stop_topics.update(
+                    topic_name
+                    for topic_name, types in self.get_topic_names_and_types()
+                    if topic_name.endswith("/cmd_vel")
+                    and "geometry_msgs/msg/Twist" in types
+                )
+            except Exception:
+                LOGGER.exception("Could not discover every cmd_vel topic during stop")
+                failures.append("topic discovery")
+            for topic_name in stop_topics:
+                try:
+                    publisher = self.twist_publishers.get(topic_name)
+                    if publisher is None:
+                        publisher = self.create_publisher(Twist, topic_name, 10)
+                        self.twist_publishers[topic_name] = publisher
+                    publisher.publish(Twist())
+                    self.robot_configs.reset_command(topic_name)
+                except Exception:
+                    LOGGER.exception("Failed to publish stop command on %s", topic_name)
+                    failures.append(topic_name)
+            self.command_leases.clear_all()
+            self.robot_configs.reset_all_commands()
+            if failures:
+                raise RuntimeError(
+                    f"Could not publish stop commands on {len(failures)} topic(s)"
+                )
+
     def odom_callback(self, msg, topic_name):
         evt = ros_engine_pb2.WorkspaceEvent(
             odometry_update=ros_engine_pb2.OdometryUpdate(
@@ -89,6 +147,22 @@ class RosIntrospector(Node):
     def tick(self):
         try:
             self._tick_impl()
+        except EventPersistenceError:
+            LOGGER.exception("Could not persist a critical ROS event")
+            health = self.event_hub.health()
+            health_event = ros_engine_pb2.WorkspaceEvent(
+                event_health=ros_engine_pb2.EventHealth(**health),
+                timestamp_ns=time.time_ns(),
+            )
+            try:
+                self.event_hub.publish(
+                    health_event,
+                    priority="telemetry",
+                    key="event_health",
+                    snapshot=True,
+                )
+            except Exception:
+                LOGGER.exception("Could not publish event persistence health")
         except Exception as e:
             self.get_logger().error(f"tick() failed, skipping this cycle: {e}")
 
@@ -114,6 +188,10 @@ class RosIntrospector(Node):
         self.known_nodes = nodes
 
         topics = self.get_topic_names_and_types()
+        if self._startup_stop_pending:
+            if self.safety.snapshot()[0] != "READY":
+                self.safety.enforce_latched_stop()
+            self._startup_stop_pending = False
 
         # Clean up subscription tracking for topics that no longer exist,
         # so they can be freshly re-subscribed if they reappear later
@@ -168,25 +246,55 @@ class RosIntrospector(Node):
             key="node_snapshot",
             snapshot=True,
         )
+        health = self.event_hub.health()
+        health_event = ros_engine_pb2.WorkspaceEvent(
+            event_health=ros_engine_pb2.EventHealth(**health),
+            timestamp_ns=time.time_ns(),
+        )
+        self.event_hub.publish(
+            health_event,
+            priority="telemetry",
+            key="event_health",
+            snapshot=True,
+        )
 
-    def publish_twist(self, topic_name, linear_x, linear_y=0.0, linear_z=0.0, angular_x=0.0, angular_y=0.0, angular_z=0.0):
-        if topic_name not in self.twist_publishers:
-            self.twist_publishers[topic_name] = self.create_publisher(Twist, topic_name, 10)
-        msg = Twist()
-        msg.linear.x = linear_x
-        msg.linear.y = linear_y
-        msg.linear.z = linear_z
-        msg.angular.x = angular_x
-        msg.angular.y = angular_y
-        msg.angular.z = angular_z
-        self.twist_publishers[topic_name].publish(msg)
-        self.command_leases.refresh(topic_name)
+    def publish_twist(self, request):
+        def publish():
+            config = validate_twist_request(
+                request,
+                config_registry=self.robot_configs,
+                record=False,
+            )
+            with self._command_lock:
+                if request.topic_name not in self.twist_publishers:
+                    self.twist_publishers[request.topic_name] = self.create_publisher(
+                        Twist, request.topic_name, 10
+                    )
+                publisher = self.twist_publishers[request.topic_name]
+                msg = Twist()
+                msg.linear.x = request.linear_x
+                msg.linear.y = request.linear_y
+                msg.linear.z = request.linear_z
+                msg.angular.x = request.angular_x
+                msg.angular.y = request.angular_y
+                msg.angular.z = request.angular_z
+                publisher.publish(msg)
+                self.robot_configs.record_command(request)
+            return config
+
+        config = self.safety.run_if_motion_permitted(publish)
+        self.command_leases.refresh(
+            request.topic_name,
+            timeout_seconds=config.command_lease_ms / 1000.0,
+        )
 
     def stop_expired_commands(self):
-        for topic_name in self.command_leases.take_expired():
-            publisher = self.twist_publishers.get(topic_name)
-            if publisher is not None:
-                publisher.publish(Twist())
+        with self._command_lock:
+            for topic_name in self.command_leases.take_expired():
+                publisher = self.twist_publishers.get(topic_name)
+                if publisher is not None:
+                    publisher.publish(Twist())
+                    self.robot_configs.reset_command(topic_name)
 
 class RosEngineServicer(ros_engine_pb2_grpc.RosEngineServicer):
     def __init__(self, event_hub: EventHub, ros_node, auth_token, auth_expires_at):
@@ -209,16 +317,7 @@ class RosEngineServicer(ros_engine_pb2_grpc.RosEngineServicer):
     def PublishTwist(self, request, context):
         self._require_authentication(context)
         try:
-            validate_twist_request(request)
-            self.ros_node.publish_twist(
-                request.topic_name,
-                request.linear_x,
-                request.linear_y,
-                request.linear_z,
-                request.angular_x,
-                request.angular_y,
-                request.angular_z,
-            )
+            self.ros_node.publish_twist(request)
         except ValueError as error:
             context.abort(grpc.StatusCode.INVALID_ARGUMENT, str(error))
         except Exception as error:
@@ -226,12 +325,144 @@ class RosEngineServicer(ros_engine_pb2_grpc.RosEngineServicer):
             context.abort(grpc.StatusCode.INTERNAL, "Failed to publish Twist command")
         return ros_engine_pb2.Empty()
 
+    def _safety_state_message(self):
+        state, detail, updated_at_ns = self.ros_node.safety.snapshot()
+        state_value = getattr(ros_engine_pb2.SafetyState, state)
+        return ros_engine_pb2.SafetyState(
+            state=state_value,
+            detail=detail,
+            updated_at_ns=updated_at_ns,
+        )
+
+    def _publish_safety_state(self):
+        event = ros_engine_pb2.WorkspaceEvent(
+            safety_state=self._safety_state_message(),
+            timestamp_ns=time.time_ns(),
+        )
+        self.event_hub.publish(
+            event,
+            key="safety_state",
+            snapshot=True,
+            durable=True,
+        )
+        return event.safety_state
+
+    def SetEmergencyStop(self, request, context):
+        self._require_authentication(context)
+        try:
+            if request.action == ros_engine_pb2.EmergencyStopRequest.ACTIVATE:
+                self.ros_node.safety.activate()
+            elif request.action == ros_engine_pb2.EmergencyStopRequest.RESET:
+                self.ros_node.safety.reset()
+            else:
+                context.abort(
+                    grpc.StatusCode.INVALID_ARGUMENT,
+                    "Emergency-stop action must be ACTIVATE or RESET",
+                )
+            return self._publish_safety_state()
+        except EventPersistenceError as error:
+            LOGGER.error("Emergency-stop transition could not be durably reported: %s", error)
+            try:
+                self.ros_node.safety.latch_fault(
+                    "Emergency-stop state could not be written to durable event history"
+                )
+            except Exception:
+                LOGGER.exception("Could not persist fail-closed emergency-stop state")
+            context.abort(
+                grpc.StatusCode.INTERNAL,
+                "Emergency-stop event history failed; motion has been fault-latched",
+            )
+        except ValueError as error:
+            context.abort(grpc.StatusCode.FAILED_PRECONDITION, str(error))
+        except Exception:
+            LOGGER.exception("Emergency-stop transition failed")
+            try:
+                self._publish_safety_state()
+            except EventPersistenceError:
+                LOGGER.exception("Emergency-stop state could not be durably reported")
+                try:
+                    self.ros_node.safety.latch_fault(
+                        "Emergency-stop state could not be written to durable event history"
+                    )
+                except Exception:
+                    LOGGER.exception("Could not persist fail-closed emergency-stop state")
+            context.abort(
+                grpc.StatusCode.INTERNAL,
+                "Emergency-stop transition failed; inspect safety state",
+            )
+
+    def GetSafetyState(self, request, context):
+        self._require_authentication(context)
+        return self._safety_state_message()
+
+    def GetEventHealth(self, request, context):
+        self._require_authentication(context)
+        return ros_engine_pb2.EventHealth(**self.event_hub.health())
+
+    @staticmethod
+    def _config_message(config):
+        return ros_engine_pb2.RobotConfig(
+            topic_name=config.topic_name,
+            max_linear_velocity=config.max_linear_velocity,
+            max_angular_velocity=config.max_angular_velocity,
+            max_linear_acceleration=config.max_linear_acceleration,
+            max_angular_acceleration=config.max_angular_acceleration,
+            command_lease_ms=config.command_lease_ms,
+            command_timeout_ms=config.command_timeout_ms,
+            permitted_motion_modes=list(config.permitted_motion_modes),
+        )
+
+    def GetRobotConfig(self, request, context):
+        self._require_authentication(context)
+        try:
+            config = self.ros_node.robot_configs.get(request.topic_name)
+            return self._config_message(config)
+        except ValueError as error:
+            context.abort(grpc.StatusCode.INVALID_ARGUMENT, str(error))
+
+    def SetRobotConfig(self, request, context):
+        self._require_authentication(context)
+        try:
+            if request.reset_to_defaults:
+                config = self.ros_node.robot_configs.reset(request.topic_name)
+            else:
+                config = RobotMotionConfig(
+                    topic_name=request.topic_name,
+                    max_linear_velocity=request.max_linear_velocity,
+                    max_angular_velocity=request.max_angular_velocity,
+                    max_linear_acceleration=request.max_linear_acceleration,
+                    max_angular_acceleration=request.max_angular_acceleration,
+                    command_lease_ms=request.command_lease_ms,
+                    command_timeout_ms=request.command_timeout_ms,
+                    permitted_motion_modes=tuple(request.permitted_motion_modes),
+                )
+                self.ros_node.robot_configs.set(config)
+            return self._config_message(config)
+        except ValueError as error:
+            context.abort(grpc.StatusCode.INVALID_ARGUMENT, str(error))
+
     def StreamEvents(self, request, context):
         provided = self._require_authentication(context)
         subscriber = self.event_hub.subscribe()
-        LOGGER.info("ROS event stream opened")
         try:
-            while context.is_active():
+            yield from self._stream_subscriber(subscriber, provided, context)
+        finally:
+            self.event_hub.unsubscribe(subscriber)
+
+    def ReplayEvents(self, request, context):
+        provided = self._require_authentication(context)
+        after_sequence = request.after_sequence if request.has_after_sequence else None
+        subscriber, recovery, initial_events = self.event_hub.subscribe_with_replay(
+            after_sequence
+        )
+        try:
+            yield ros_engine_pb2.WorkspaceEvent(
+                event_recovery=ros_engine_pb2.EventRecovery(**recovery),
+                timestamp_ns=time.time_ns(),
+            )
+            for event in initial_events:
+                if not context.is_active():
+                    return
                 if not is_valid_auth_token(
                     provided,
                     self.auth_token,
@@ -241,13 +472,33 @@ class RosEngineServicer(ros_engine_pb2_grpc.RosEngineServicer):
                         grpc.StatusCode.UNAUTHENTICATED,
                         "The Robotics Studio session token has expired",
                     )
-                try:
-                    yield subscriber.get(timeout=1.0)
-                except queue.Empty:
-                    continue
+                yield event
+            yield from self._stream_subscriber(subscriber, provided, context)
         finally:
             self.event_hub.unsubscribe(subscriber)
-            LOGGER.info("ROS event stream closed")
+
+    def _stream_subscriber(self, subscriber, provided, context):
+        LOGGER.info("ROS event stream opened")
+        while context.is_active():
+            if not is_valid_auth_token(
+                provided,
+                self.auth_token,
+                self.auth_expires_at,
+            ):
+                context.abort(
+                    grpc.StatusCode.UNAUTHENTICATED,
+                    "The Robotics Studio session token has expired",
+                )
+            try:
+                yield subscriber.get(timeout=1.0)
+            except EventOverflow:
+                context.abort(
+                    grpc.StatusCode.RESOURCE_EXHAUSTED,
+                    "Critical events overflowed; reconnect to resynchronize from snapshots",
+                )
+            except queue.Empty:
+                continue
+        LOGGER.info("ROS event stream closed")
 
 
 def ros_spin_thread(node):
@@ -265,17 +516,58 @@ def main():
     )
     auth_token, auth_expires_at = load_auth_credentials(credential_path)
     rclpy.init()
-    event_hub = EventHub()
+    queue_capacity = int(os.environ.get("ROBOTICS_STUDIO_EVENT_QUEUE_CAPACITY", "256"))
+    if not 16 <= queue_capacity <= 100_000:
+        raise ValueError("ROBOTICS_STUDIO_EVENT_QUEUE_CAPACITY must be between 16 and 100000")
+    history_path = os.environ.get(
+        "ROBOTICS_STUDIO_EVENT_HISTORY",
+        "/workspace/.robotics-studio/event-history.json",
+    )
+    event_hub = EventHub(
+        max_queue_size=queue_capacity,
+        history_path=history_path,
+        event_encoder=lambda event: event.SerializeToString(),
+        event_decoder=ros_engine_pb2.WorkspaceEvent.FromString,
+    )
     node = RosIntrospector(event_hub)
 
     spin_thread = threading.Thread(target=ros_spin_thread, args=(node,), daemon=True)
     spin_thread.start()
-
-    server = grpc.server(futures.ThreadPoolExecutor(max_workers=4))
+    try:
+        node.safety.enforce_latched_stop()
+    except RuntimeError:
+        LOGGER.exception("Emergency stop could not be reasserted during startup")
+
+    grpc_workers = int(os.environ.get("ROBOTICS_STUDIO_GRPC_WORKERS", "16"))
+    if not 4 <= grpc_workers <= 128:
+        raise ValueError("ROBOTICS_STUDIO_GRPC_WORKERS must be between 4 and 128")
+    server = grpc.server(futures.ThreadPoolExecutor(max_workers=grpc_workers))
+    servicer = RosEngineServicer(
+        event_hub, node, auth_token, auth_expires_at
+    )
+    servicer._publish_safety_state()
     ros_engine_pb2_grpc.add_RosEngineServicer_to_server(
-        RosEngineServicer(event_hub, node, auth_token, auth_expires_at), server
+        servicer, server
     )
-    server.add_insecure_port('[::]:50051')
+    with open(
+        os.environ.get(
+            "ROBOTICS_STUDIO_TLS_CERTIFICATE",
+            "/run/robotics-studio/grpc-server.crt",
+        ),
+        "rb",
+    ) as certificate_file:
+        certificate = certificate_file.read()
+    with open(
+        os.environ.get(
+            "ROBOTICS_STUDIO_TLS_PRIVATE_KEY",
+            "/run/robotics-studio/grpc-server.key",
+        ),
+        "rb",
+    ) as key_file:
+        private_key = key_file.read()
+    credentials = grpc.ssl_server_credentials(((private_key, certificate),))
+    if server.add_secure_port("[::]:50051", credentials) != 50051:
+        raise RuntimeError("Could not bind the authenticated TLS gRPC server")
     server.start()
     LOGGER.info("RosEngine gRPC server running on port 50051")
 
diff --git a/src-tauri/resources/devcontainer/server_core.py b/src-tauri/resources/devcontainer/server_core.py
index dab37cb..745edbb 100644
--- a/src-tauri/resources/devcontainer/server_core.py
+++ b/src-tauri/resources/devcontainer/server_core.py
@@ -1,54 +1,754 @@
 from collections import OrderedDict, deque
+from dataclasses import dataclass
+import base64
 import hmac
+import json
 import math
+import os
 import queue
 import re
+import stat
+import tempfile
 import threading
 import time
+from pathlib import Path
 
 AUTH_PROTOCOL_VERSION = 1
 MAX_EVENT_QUEUE_SIZE = 256
+MAX_EVENT_HISTORY_EVENTS = 256
+MAX_EVENT_HISTORY_BYTES = 4 * 1024 * 1024
 MAX_LINEAR_VELOCITY = 0.5
 MAX_ANGULAR_VELOCITY = 1.0
 MAX_COMMAND_AGE_SECONDS = 1.0
 COMMAND_TIMEOUT_SECONDS = 0.5
 ROS_TOPIC_PATTERN = re.compile(r"/[A-Za-z_][A-Za-z0-9_]*(/[A-Za-z_][A-Za-z0-9_]*)*")
+MAX_CONFIG_LINEAR_VELOCITY = 0.5
+MAX_CONFIG_ANGULAR_VELOCITY = 1.0
+MAX_CONFIG_LINEAR_ACCELERATION = 2.0
+MAX_CONFIG_ANGULAR_ACCELERATION = 4.0
+ALLOWED_MOTION_MODES = frozenset(("planar", "full_twist"))
+
+
+class EventOverflow(Exception):
+    pass
+
+
+class EventPersistenceError(Exception):
+    pass
+
+
+class SafetyController:
+    def __init__(self, stop_callback, storage_path=None):
+        self._stop_callback = stop_callback
+        self._state = "READY"
+        self._detail = "Motion commands are permitted"
+        self._updated_at_ns = time.time_ns()
+        self._events = []
+        self._lock = threading.RLock()
+        self._storage_path = Path(storage_path) if storage_path else None
+        if self._storage_path and self._storage_path.exists():
+            if self._storage_path.is_symlink() or self._storage_path.stat().st_size > 64 * 1024:
+                raise ValueError("Safety state file is unsafe or too large")
+            try:
+                stored = json.loads(self._storage_path.read_text(encoding="utf-8"))
+                if not isinstance(stored, dict):
+                    raise ValueError("Invalid persisted safety state")
+                stored_state = stored["state"]
+                if stored_state not in ("READY", "ACTIVE", "RESET_PENDING", "FAULT"):
+                    raise ValueError("Invalid persisted safety state")
+                self._state = stored_state
+                self._detail = str(stored.get("detail", "Restored safety state"))[:512]
+                self._updated_at_ns = int(stored.get("updated_at_ns", self._updated_at_ns))
+                events = stored.get("events", [])
+                if not isinstance(events, list) or len(events) > 128:
+                    raise ValueError("Invalid persisted safety event history")
+                if any(not isinstance(item, dict) for item in events):
+                    raise ValueError("Invalid persisted safety event")
+                self._events = []
+                for item in events:
+                    event = item.get("event")
+                    timestamp_ns = item.get("timestamp_ns")
+                    event_state = item.get("state")
+                    if (
+                        not isinstance(event, str)
+                        or not isinstance(timestamp_ns, int)
+                        or isinstance(timestamp_ns, bool)
+                        or event_state not in ("READY", "ACTIVE", "RESET_PENDING", "FAULT")
+                    ):
+                        raise ValueError("Invalid persisted safety event")
+                    self._events.append(
+                        {
+                            "event": event[:64],
+                            "timestamp_ns": timestamp_ns,
+                            "state": event_state,
+                        }
+                    )
+                if self._state in ("ACTIVE", "RESET_PENDING"):
+                    self._state = "ACTIVE"
+                    self._detail = "Emergency stop restored as latched after service restart"
+                elif self._state == "FAULT":
+                    self._detail = "Safety fault restored after service restart"
+            except (OSError, UnicodeError, json.JSONDecodeError, KeyError, TypeError) as error:
+                raise ValueError("Safety state file is invalid") from error
+
+    def _persist(self):
+        if self._storage_path is None:
+            return
+        parent = self._storage_path.parent
+        parent.mkdir(mode=0o700, parents=True, exist_ok=True)
+        if parent.is_symlink():
+            raise OSError("Safety state directory must not be a symlink")
+        descriptor, temporary = tempfile.mkstemp(
+            prefix=".safety-state-", suffix=".tmp", dir=parent
+        )
+        try:
+            with os.fdopen(descriptor, "w", encoding="utf-8") as file:
+                json.dump(
+                    {
+                        "state": self._state,
+                        "detail": self._detail,
+                        "updated_at_ns": self._updated_at_ns,
+                        "events": self._events[-128:],
+                    },
+                    file,
+                )
+                file.flush()
+                os.fsync(file.fileno())
+            os.replace(temporary, self._storage_path)
+        except Exception:
+            try:
+                os.unlink(temporary)
+            except FileNotFoundError:
+                pass
+            raise
+
+    def _record_event(self, event):
+        self._events.append(
+            {
+                "event": event,
+                "timestamp_ns": self._updated_at_ns,
+                "state": self._state,
+            }
+        )
+        self._events = self._events[-128:]
+
+    def history(self):
+        with self._lock:
+            return tuple(dict(event) for event in self._events)
+
+    def snapshot(self):
+        with self._lock:
+            return self._state, self._detail, self._updated_at_ns
+
+    def require_motion_permitted(self):
+        with self._lock:
+            if self._state != "READY":
+                raise ValueError(f"Motion is blocked while safety state is {self._state}")
+
+    def enforce_latched_stop(self):
+        with self._lock:
+            if self._state == "READY":
+                return
+            try:
+                self._stop_callback()
+            except Exception as error:
+                self._state = "FAULT"
+                self._detail = "Could not reassert emergency stop after service restart"
+                self._updated_at_ns = time.time_ns()
+                self._record_event("STOP_ENFORCEMENT_FAILED")
+                try:
+                    self._persist()
+                except Exception:
+                    self._detail += "; fault state could not be persisted"
+                raise RuntimeError(self._detail) from error
+            self._updated_at_ns = time.time_ns()
+            self._record_event("STOP_REASSERTED")
+            try:
+                self._persist()
+            except OSError as error:
+                self._state = "FAULT"
+                self._detail = "Could not persist reasserted emergency stop"
+                self._updated_at_ns = time.time_ns()
+                self._record_event("STOP_ENFORCEMENT_FAILED")
+                raise RuntimeError(self._detail) from error
+
+    def latch_fault(self, detail):
+        with self._lock:
+            self._state = "FAULT"
+            self._detail = str(detail)[:512]
+            self._updated_at_ns = time.time_ns()
+            self._record_event("FAULT_LATCHED")
+            try:
+                self._stop_callback()
+            except Exception:
+                self._detail = "Safety fault latched; emergency stop enforcement also failed"
+                self._record_event("STOP_ENFORCEMENT_FAILED")
+            self._persist()
+
+    def run_if_motion_permitted(self, operation):
+        with self._lock:
+            if self._state != "READY":
+                raise ValueError(f"Motion is blocked while safety state is {self._state}")
+            return operation()
+
+    def activate(self):
+        with self._lock:
+            if self._state == "FAULT":
+                raise RuntimeError(f"Emergency stop is faulted: {self._detail}")
+            self._state = "ACTIVE"
+            self._detail = "Emergency stop is latched"
+            self._updated_at_ns = time.time_ns()
+            self._record_event("ACTIVATED")
+            try:
+                self._persist()
+                self._stop_callback()
+            except Exception as error:
+                self._state = "FAULT"
+                self._detail = "Could not persist or publish emergency stop"
+                self._updated_at_ns = time.time_ns()
+                self._record_event("ACTIVATION_FAILED")
+                try:
+                    self._stop_callback()
+                except Exception:
+                    self._detail += "; retrying the zero command also failed"
+                    self._record_event("STOP_RETRY_FAILED")
+                try:
+                    self._persist()
+                except Exception:
+                    self._detail += "; fault state could not be persisted"
+                raise RuntimeError(self._detail) from error
+            return self.snapshot()
+
+    def reset(self):
+        with self._lock:
+            if self._state != "ACTIVE":
+                raise ValueError("Reset is allowed only from the latched ACTIVE state")
+            self._state = "RESET_PENDING"
+            self._detail = "Applying stop before reset"
+            self._updated_at_ns = time.time_ns()
+            self._record_event("RESET_REQUESTED")
+            try:
+                self._persist()
+                self._stop_callback()
+            except Exception as error:
+                self._state = "FAULT"
+                self._detail = "Could not confirm stop during reset"
+                self._updated_at_ns = time.time_ns()
+                self._record_event("RESET_FAILED")
+                try:
+                    self._persist()
+                except Exception:
+                    self._detail += "; fault state could not be persisted"
+                raise RuntimeError(self._detail) from error
+            self._state = "READY"
+            self._detail = "Reset confirmed; explicit new motion command required"
+            self._updated_at_ns = time.time_ns()
+            self._record_event("RESET_CONFIRMED")
+            try:
+                self._persist()
+            except Exception as error:
+                self._state = "FAULT"
+                self._detail = "Could not persist reset; emergency stop remains faulted"
+                self._updated_at_ns = time.time_ns()
+                self._record_event("RESET_PERSISTENCE_FAILED")
+                raise RuntimeError(self._detail) from error
+            return self.snapshot()
+
+
+@dataclass(frozen=True)
+class RobotMotionConfig:
+    topic_name: str
+    max_linear_velocity: float = MAX_LINEAR_VELOCITY
+    max_angular_velocity: float = MAX_ANGULAR_VELOCITY
+    max_linear_acceleration: float = 1.0
+    max_angular_acceleration: float = 2.0
+    command_lease_ms: int = 500
+    command_timeout_ms: int = 1000
+    permitted_motion_modes: tuple = ("planar",)
+
+    def validate(self):
+        if len(self.topic_name) > 200 or not ROS_TOPIC_PATTERN.fullmatch(self.topic_name):
+            raise ValueError("Invalid robot command topic")
+        for name, value, maximum in (
+            ("max_linear_velocity", self.max_linear_velocity, MAX_CONFIG_LINEAR_VELOCITY),
+            ("max_angular_velocity", self.max_angular_velocity, MAX_CONFIG_ANGULAR_VELOCITY),
+            ("max_linear_acceleration", self.max_linear_acceleration, MAX_CONFIG_LINEAR_ACCELERATION),
+            ("max_angular_acceleration", self.max_angular_acceleration, MAX_CONFIG_ANGULAR_ACCELERATION),
+        ):
+            if (
+                isinstance(value, bool)
+                or not isinstance(value, (int, float))
+                or not math.isfinite(value)
+                or value <= 0
+                or value > maximum
+            ):
+                raise ValueError(f"{name} must be positive and no greater than {maximum}")
+        if (
+            not isinstance(self.command_lease_ms, int)
+            or isinstance(self.command_lease_ms, bool)
+            or not 100 <= self.command_lease_ms <= 2000
+        ):
+            raise ValueError("command_lease_ms must be between 100 and 2000")
+        if (
+            not isinstance(self.command_timeout_ms, int)
+            or isinstance(self.command_timeout_ms, bool)
+            or not self.command_lease_ms <= self.command_timeout_ms <= 5000
+        ):
+            raise ValueError("command_timeout_ms must be at least the lease and no greater than 5000")
+        if (
+            not self.permitted_motion_modes
+            or not isinstance(self.permitted_motion_modes, (list, tuple))
+            or any(not isinstance(mode, str) for mode in self.permitted_motion_modes)
+            or len(set(self.permitted_motion_modes)) != len(self.permitted_motion_modes)
+            or any(mode not in ALLOWED_MOTION_MODES for mode in self.permitted_motion_modes)
+        ):
+            raise ValueError("permitted_motion_modes must contain supported unique modes")
+        return self
+
+
+class RobotConfigRegistry:
+    def __init__(self, storage_path=None):
+        self._configs = {}
+        self._last_commands = {}
+        self._lock = threading.Lock()
+        self._storage_path = Path(storage_path) if storage_path else None
+        if self._storage_path and self._storage_path.exists():
+            if self._storage_path.is_symlink() or self._storage_path.stat().st_size > 1024 * 1024:
+                raise ValueError("Robot configuration file is unsafe or too large")
+            try:
+                items = json.loads(self._storage_path.read_text(encoding="utf-8"))
+            except (OSError, UnicodeError, json.JSONDecodeError) as error:
+                raise ValueError("Robot configuration file is invalid") from error
+            if not isinstance(items, list) or len(items) > 128:
+                raise ValueError("Robot configuration file has an invalid shape")
+            for item in items:
+                if not isinstance(item, dict):
+                    raise ValueError("Robot configuration entry is invalid")
+                item["permitted_motion_modes"] = tuple(item.get("permitted_motion_modes", ()))
+                config = RobotMotionConfig(**item).validate()
+                self._configs[config.topic_name] = config
+
+    def _persist(self, configs):
+        if self._storage_path is None:
+            return
+        parent = self._storage_path.parent
+        parent.mkdir(mode=0o700, parents=True, exist_ok=True)
+        if parent.is_symlink():
+            raise OSError("Robot configuration directory must not be a symlink")
+        serialized = [
+            {
+                **config.__dict__,
+                "permitted_motion_modes": list(config.permitted_motion_modes),
+            }
+            for config in configs.values()
+        ]
+        descriptor, temporary = tempfile.mkstemp(
+            prefix=".robot-config-", suffix=".tmp", dir=parent
+        )
+        try:
+            with os.fdopen(descriptor, "w", encoding="utf-8") as file:
+                json.dump(serialized, file, allow_nan=False)
+                file.flush()
+                os.fsync(file.fileno())
+            os.replace(temporary, self._storage_path)
+        except Exception:
+            try:
+                os.unlink(temporary)
+            except FileNotFoundError:
+                pass
+            raise
+
+    def get(self, topic_name):
+        with self._lock:
+            return self._configs.get(topic_name, RobotMotionConfig(topic_name).validate())
+
+    def set(self, config):
+        config.validate()
+        with self._lock:
+            updated = dict(self._configs)
+            if config.topic_name not in updated and len(updated) >= 128:
+                raise ValueError("Robot configuration capacity reached")
+            updated[config.topic_name] = config
+            self._persist(updated)
+            self._configs = updated
+            self._last_commands.pop(config.topic_name, None)
+        return config
+
+    def reset(self, topic_name):
+        default = RobotMotionConfig(topic_name).validate()
+        with self._lock:
+            updated = dict(self._configs)
+            updated.pop(topic_name, None)
+            self._persist(updated)
+            self._configs = updated
+            self._last_commands.pop(topic_name, None)
+        return default
+
+    def validate_command(self, request, now_ns=None, monotonic_now=None, record=True):
+        topic_name = request.topic_name
+        config = self.get(topic_name)
+        linear = (request.linear_x, request.linear_y, request.linear_z)
+        angular = (request.angular_x, request.angular_y, request.angular_z)
+        if not all(math.isfinite(value) for value in (*linear, *angular)):
+            raise ValueError("Velocity components must be finite")
+        if math.hypot(*linear) > config.max_linear_velocity:
+            raise ValueError("Linear velocity magnitude exceeds the robot limit")
+        if math.hypot(*angular) > config.max_angular_velocity:
+            raise ValueError("Angular velocity magnitude exceeds the robot limit")
+        if "full_twist" not in config.permitted_motion_modes and any(
+            value != 0 for value in (request.linear_y, request.linear_z, request.angular_x, request.angular_y)
+        ):
+            raise ValueError("Robot configuration permits planar motion only")
+
+        current_time = time.monotonic() if monotonic_now is None else monotonic_now
+        with self._lock:
+            previous = self._last_commands.get(topic_name)
+            if previous is not None:
+                previous_linear, previous_angular, previous_time = previous
+                elapsed = max(current_time - previous_time, 1e-6)
+                linear_delta = math.sqrt(sum((a - b) ** 2 for a, b in zip(linear, previous_linear)))
+                angular_delta = math.sqrt(sum((a - b) ** 2 for a, b in zip(angular, previous_angular)))
+                if linear_delta > config.max_linear_acceleration * elapsed:
+                    raise ValueError("Linear acceleration exceeds the robot limit")
+                if angular_delta > config.max_angular_acceleration * elapsed:
+                    raise ValueError("Angular acceleration exceeds the robot limit")
+            if record:
+                self._last_commands[topic_name] = (linear, angular, current_time)
+        return config
+
+    def record_command(self, request, monotonic_now=None):
+        current_time = time.monotonic() if monotonic_now is None else monotonic_now
+        with self._lock:
+            self._last_commands[request.topic_name] = (
+                (request.linear_x, request.linear_y, request.linear_z),
+                (request.angular_x, request.angular_y, request.angular_z),
+                current_time,
+            )
+
+    def reset_command(self, topic_name):
+        with self._lock:
+            self._last_commands.pop(topic_name, None)
+
+    def reset_all_commands(self):
+        with self._lock:
+            self._last_commands.clear()
 
 
 class EventHub:
-    def __init__(self, max_queue_size=MAX_EVENT_QUEUE_SIZE):
+    def __init__(
+        self,
+        max_queue_size=MAX_EVENT_QUEUE_SIZE,
+        history_path=None,
+        event_encoder=None,
+        event_decoder=None,
+        history_capacity=MAX_EVENT_HISTORY_EVENTS,
+        history_max_bytes=MAX_EVENT_HISTORY_BYTES,
+    ):
         if max_queue_size < 1:
             raise ValueError("max_queue_size must be positive")
+        if history_capacity < 1 or history_max_bytes < 1:
+            raise ValueError("event history limits must be positive")
+        if history_path is not None and not callable(event_encoder):
+            raise ValueError("event_encoder is required for persistent event history")
+        if history_path is not None and not callable(event_decoder):
+            raise ValueError("event_decoder is required for persistent event history")
         self._max_queue_size = max_queue_size
         self._subscribers = set()
         self._lock = threading.Lock()
         self._latest_state = {}
+        self._sequence = 0
+        self._historical_dropped_telemetry = 0
+        self._historical_critical_overflows = 0
+        self._history_path = Path(history_path) if history_path else None
+        self._event_encoder = event_encoder
+        self._event_decoder = event_decoder
+        self._history_capacity = history_capacity
+        self._history_max_bytes = history_max_bytes
+        self._history = []
+        self._persistence_error = None
+        if self._history_path is not None:
+            self._load_history()
+
+    def _load_history(self):
+        path = self._history_path
+        parent = path.parent
+        directory_fd = None
+        try:
+            try:
+                directory_fd = os.open(
+                    parent,
+                    os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
+                )
+            except FileNotFoundError:
+                return
+            except NotADirectoryError as error:
+                raise ValueError("Event history directory is unsafe") from error
+            try:
+                descriptor = os.open(
+                    path.name,
+                    os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC,
+                    dir_fd=directory_fd,
+                )
+            except FileNotFoundError:
+                return
+            with os.fdopen(descriptor, "rb") as file:
+                metadata = os.fstat(file.fileno())
+                if (
+                    not stat.S_ISREG(metadata.st_mode)
+                    or metadata.st_size > self._history_max_bytes
+                ):
+                    raise ValueError("Event history file is invalid or too large")
+                contents = file.read(self._history_max_bytes + 1)
+            if len(contents) > self._history_max_bytes:
+                raise ValueError("Event history file is invalid or too large")
+            stored = json.loads(contents.decode("utf-8"))
+            if not isinstance(stored, dict) or stored.get("version") != 1:
+                raise ValueError("Unsupported event history format")
+            sequence = stored.get("last_sequence")
+            events = stored.get("events")
+            if (
+                not isinstance(sequence, int)
+                or isinstance(sequence, bool)
+                or sequence < 0
+                or not isinstance(events, list)
+                or len(events) > self._history_capacity
+            ):
+                raise ValueError("Event history metadata is invalid")
+            previous = 0
+            for item in events:
+                if not isinstance(item, dict):
+                    raise ValueError("Event history entry is invalid")
+                item_sequence = item.get("sequence")
+                payload = item.get("payload")
+                if (
+                    not isinstance(item_sequence, int)
+                    or isinstance(item_sequence, bool)
+                    or item_sequence <= previous
+                    or item_sequence > sequence
+                    or not isinstance(payload, str)
+                ):
+                    raise ValueError("Event history entry is invalid")
+                encoded = base64.b64decode(payload, validate=True)
+                event = self._event_decoder(encoded)
+                if getattr(event, "sequence", item_sequence) != item_sequence:
+                    raise ValueError("Event history sequence does not match its payload")
+                self._history.append((item_sequence, payload, event))
+                previous = item_sequence
+            if sequence > 0 and (
+                not self._history or self._history[-1][0] != sequence
+            ):
+                raise ValueError("Event history sequence is inconsistent")
+            self._sequence = sequence
+        except (OSError, UnicodeError, json.JSONDecodeError, TypeError, KeyError) as error:
+            raise ValueError("Event history file could not be loaded safely") from error
+        finally:
+            if directory_fd is not None:
+                os.close(directory_fd)
+
+    def _persist_history(self, sequence, history):
+        path = self._history_path
+        parent = path.parent
+        try:
+            parent.mkdir(mode=0o700, parents=True, exist_ok=True)
+            stored = {
+                "version": 1,
+                "last_sequence": sequence,
+                "events": [
+                    {"sequence": item_sequence, "payload": payload}
+                    for item_sequence, payload, _ in history
+                ],
+            }
+            encoded = json.dumps(stored, separators=(",", ":")).encode("utf-8")
+            if len(encoded) > self._history_max_bytes:
+                raise OSError("Event history exceeds its configured storage limit")
+            directory_fd = os.open(
+                parent,
+                os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
+            )
+            temporary = ".event-history.tmp"
+            try:
+                flags = (
+                    os.O_WRONLY
+                    | os.O_CREAT
+                    | os.O_EXCL
+                    | os.O_NOFOLLOW
+                    | os.O_CLOEXEC
+                )
+                try:
+                    descriptor = os.open(
+                        temporary, flags, 0o600, dir_fd=directory_fd
+                    )
+                except FileExistsError:
+                    os.unlink(temporary, dir_fd=directory_fd)
+                    descriptor = os.open(
+                        temporary, flags, 0o600, dir_fd=directory_fd
+                    )
+                with os.fdopen(descriptor, "wb") as file:
+                    file.write(encoded)
+                    file.flush()
+                    os.fsync(file.fileno())
+                os.replace(
+                    temporary,
+                    path.name,
+                    src_dir_fd=directory_fd,
+                    dst_dir_fd=directory_fd,
+                )
+                os.fsync(directory_fd)
+            except Exception:
+                try:
+                    os.unlink(temporary, dir_fd=directory_fd)
+                except FileNotFoundError:
+                    pass
+                raise
+            finally:
+                os.close(directory_fd)
+        except OSError as error:
+            raise EventPersistenceError(f"Could not persist critical ROS event: {error}") from error
+
+    def _retained_history(self, sequence, event):
+        try:
+            payload = base64.b64encode(self._event_encoder(event)).decode("ascii")
+        except Exception as error:
+            raise EventPersistenceError(f"Could not encode critical ROS event: {error}") from error
+        history = self._history + [(sequence, payload, event)]
+        history = history[-self._history_capacity :]
+        while True:
+            serialized = json.dumps(
+                {
+                    "version": 1,
+                    "last_sequence": sequence,
+                    "events": [
+                        {"sequence": item_sequence, "payload": item_payload}
+                        for item_sequence, item_payload, _ in history
+                    ],
+                },
+                separators=(",", ":"),
+            ).encode("utf-8")
+            if len(serialized) <= self._history_max_bytes:
+                break
+            if not history:
+                raise EventPersistenceError(
+                    "Critical ROS event exceeds the history storage limit"
+                )
+            history.pop(0)
+        if not history or history[-1][1] != payload:
+            raise EventPersistenceError("Critical ROS event exceeds the history storage limit")
+        return history
 
     def subscribe(self):
         subscriber = EventSubscriber(self._max_queue_size)
         with self._lock:
+            for key, event in self._latest_state.items():
+                subscriber.put(event, critical=True, key=key)
             self._subscribers.add(subscriber)
-            snapshots = tuple(self._latest_state.items())
-        for key, event in snapshots:
-            subscriber.put(event, critical=True, key=key)
         return subscriber
 
     def unsubscribe(self, subscriber):
         with self._lock:
-            self._subscribers.discard(subscriber)
+            if subscriber in self._subscribers:
+                stats = subscriber.health()
+                self._historical_dropped_telemetry += stats["dropped_telemetry"]
+                self._historical_critical_overflows += stats["dropped_critical"]
+                self._subscribers.remove(subscriber)
 
-    def publish(self, event, *, priority="critical", key=None, snapshot=False):
+    def subscribe_with_replay(self, after_sequence=None):
+        subscriber = EventSubscriber(self._max_queue_size)
+        with self._lock:
+            oldest = self._history[0][0] if self._history else self._sequence + 1
+            gap = after_sequence is not None and (
+                after_sequence > self._sequence
+                or after_sequence < oldest - 1
+                or (not self._history and after_sequence < self._sequence)
+            )
+            replay_after = (
+                -1
+                if after_sequence is not None and after_sequence > self._sequence
+                else after_sequence
+            )
+            replay = (
+                [
+                    event
+                    for sequence, _, event in self._history
+                    if sequence > replay_after
+                ]
+                if replay_after is not None
+                else []
+            )
+            snapshots = (
+                list(self._latest_state.values())
+                if after_sequence is None or after_sequence == 0 or gap
+                else []
+            )
+            self._subscribers.add(subscriber)
+            return subscriber, {
+                "gap_detected": gap,
+                "oldest_available_sequence": oldest,
+                "latest_sequence": self._sequence,
+                "snapshot_included": bool(snapshots),
+                "retained_critical_events": len(self._history),
+            }, replay + snapshots
+
+    def publish(self, event, *, priority="critical", key=None, snapshot=False, durable=None):
         if priority not in ("critical", "telemetry"):
             raise ValueError("priority must be 'critical' or 'telemetry'")
         if key is None:
             key = id(event)
         with self._lock:
+            if self._history_path is None:
+                self._sequence += 1
+                event_sequence = self._sequence
+                if hasattr(event, "sequence"):
+                    event.sequence = event_sequence
+            else:
+                if durable is None:
+                    durable = priority == "critical" and not snapshot
+                event_sequence = self._sequence + 1 if durable else 0
+                if durable:
+                    previous_sequence = getattr(event, "sequence", None)
+                    if hasattr(event, "sequence"):
+                        event.sequence = event_sequence
+                    try:
+                        history = self._retained_history(event_sequence, event)
+                        self._persist_history(event_sequence, history)
+                    except EventPersistenceError as error:
+                        self._persistence_error = str(error)
+                        if previous_sequence is not None:
+                            event.sequence = previous_sequence
+                        raise
+                    self._sequence = event_sequence
+                    self._history = history
+                elif hasattr(event, "sequence"):
+                    event.sequence = 0
             if snapshot:
                 self._latest_state[key] = event
             subscribers = tuple(self._subscribers)
+            for subscriber in subscribers:
+                subscriber.put(event, critical=priority == "critical", key=key)
 
-        for subscriber in subscribers:
-            subscriber.put(event, critical=priority == "critical", key=key)
+    def health(self):
+        with self._lock:
+            subscribers = tuple(self._subscribers)
+            sequence = self._sequence
+            historical_telemetry = self._historical_dropped_telemetry
+            historical_critical = self._historical_critical_overflows
+            persistence_error = self._persistence_error
+            oldest = self._history[0][0] if self._history else sequence + 1
+            retained = len(self._history)
+        subscriber_stats = [subscriber.health() for subscriber in subscribers]
+        return {
+            "queue_capacity": self._max_queue_size,
+            "subscriber_count": len(subscriber_stats),
+            "last_sequence": sequence,
+            "dropped_telemetry": historical_telemetry
+            + sum(stat["dropped_telemetry"] for stat in subscriber_stats),
+            "critical_overflows": historical_critical
+            + sum(stat["dropped_critical"] for stat in subscriber_stats),
+            "max_backlog": max((stat["pending_count"] for stat in subscriber_stats), default=0),
+            "persistence_healthy": persistence_error is None,
+            "persistence_error": persistence_error or "",
+            "oldest_available_sequence": oldest,
+            "retained_critical_events": retained,
+        }
 
 
 class EventSubscriber:
@@ -59,22 +759,21 @@ class EventSubscriber:
         self._condition = threading.Condition()
         self.dropped_telemetry = 0
         self.dropped_critical = 0
+        self._overflowed = False
 
     def put(self, event, *, critical, key):
         with self._condition:
+            if self._overflowed:
+                return
             if critical:
                 if len(self._critical) >= self._max_queue_size:
-                    if self._telemetry:
-                        self._telemetry.popitem(last=False)
-                        self.dropped_telemetry += 1
-                    else:
-                        for index, (queued_key, _) in enumerate(self._critical):
-                            if queued_key == key:
-                                del self._critical[index]
-                                break
-                        else:
-                            self._critical.popleft()
-                            self.dropped_critical += 1
+                    self.dropped_critical += len(self._critical) + 1
+                    self.dropped_telemetry += len(self._telemetry)
+                    self._critical.clear()
+                    self._telemetry.clear()
+                    self._overflowed = True
+                    self._condition.notify_all()
+                    return
                 self._critical.append((key, event))
             else:
                 if key in self._telemetry:
@@ -89,11 +788,13 @@ class EventSubscriber:
     def get(self, timeout=None):
         deadline = None if timeout is None else time.monotonic() + timeout
         with self._condition:
-            while not self._critical and not self._telemetry:
+            while not self._critical and not self._telemetry and not self._overflowed:
                 remaining = None if deadline is None else deadline - time.monotonic()
                 if remaining is not None and remaining <= 0:
                     raise queue.Empty
                 self._condition.wait(remaining)
+            if self._overflowed:
+                raise EventOverflow("Critical event queue overflow; reconnect for state resynchronization")
             if self._critical:
                 return self._critical.popleft()[1]
             return self._telemetry.popitem(last=False)[1]
@@ -110,6 +811,15 @@ class EventSubscriber:
         with self._condition:
             return len(self._critical) + len(self._telemetry)
 
+    def health(self):
+        with self._condition:
+            return {
+                "pending_count": len(self._critical) + len(self._telemetry),
+                "dropped_telemetry": self.dropped_telemetry,
+                "dropped_critical": self.dropped_critical,
+                "overflowed": self._overflowed,
+            }
+
 
 class CommandLeaseManager:
     def __init__(self, timeout_seconds=COMMAND_TIMEOUT_SECONDS):
@@ -119,10 +829,13 @@ class CommandLeaseManager:
         self._deadlines = {}
         self._lock = threading.Lock()
 
-    def refresh(self, key, now=None):
+    def refresh(self, key, now=None, timeout_seconds=None):
         current_time = time.monotonic() if now is None else now
         with self._lock:
-            self._deadlines[key] = current_time + self._timeout_seconds
+            timeout = self._timeout_seconds if timeout_seconds is None else timeout_seconds
+            if timeout <= 0:
+                raise ValueError("timeout_seconds must be positive")
+            self._deadlines[key] = current_time + timeout
 
     def take_expired(self, now=None):
         current_time = time.monotonic() if now is None else now
@@ -135,8 +848,16 @@ class CommandLeaseManager:
                 del self._deadlines[key]
             return expired
 
+    def clear(self, key):
+        with self._lock:
+            self._deadlines.pop(key, None)
+
+    def clear_all(self):
+        with self._lock:
+            self._deadlines.clear()
+
 
-def validate_twist_request(request, now_ns=None):
+def validate_twist_request(request, now_ns=None, config_registry=None, record=True):
     if len(request.topic_name) > 200 or not ROS_TOPIC_PATTERN.fullmatch(request.topic_name):
         raise ValueError("Invalid ROS topic name")
 
@@ -144,10 +865,22 @@ def validate_twist_request(request, now_ns=None):
     if not isinstance(timestamp_ns, int) or timestamp_ns <= 0:
         raise ValueError("Command timestamp is missing or invalid")
     current_time_ns = time.time_ns() if now_ns is None else now_ns
-    max_age_ns = int(MAX_COMMAND_AGE_SECONDS * 1_000_000_000)
+    command_timeout = (
+        config_registry.get(request.topic_name).command_timeout_ms / 1000.0
+        if config_registry is not None
+        else MAX_COMMAND_AGE_SECONDS
+    )
+    max_age_ns = int(command_timeout * 1_000_000_000)
     if timestamp_ns > current_time_ns + 250_000_000 or current_time_ns - timestamp_ns > max_age_ns:
         raise ValueError("Command is stale or has a future timestamp")
 
+    if config_registry is not None:
+        return config_registry.validate_command(
+            request,
+            now_ns=current_time_ns,
+            record=record,
+        )
+
     linear = (request.linear_x, request.linear_y, request.linear_z)
     angular = (request.angular_x, request.angular_y, request.angular_z)
     if not all(math.isfinite(value) for value in (*linear, *angular)):
diff --git a/src-tauri/resources/devcontainer/test_server_core.py b/src-tauri/resources/devcontainer/test_server_core.py
index 6793a06..92a2a81 100644
--- a/src-tauri/resources/devcontainer/test_server_core.py
+++ b/src-tauri/resources/devcontainer/test_server_core.py
@@ -1,13 +1,20 @@
 import math
 import json
+import os
 import tempfile
 import time
 import unittest
+from unittest import mock
 from types import SimpleNamespace
 
 from server_core import (
     CommandLeaseManager,
+    EventPersistenceError,
+    EventOverflow,
     EventHub,
+    RobotConfigRegistry,
+    RobotMotionConfig,
+    SafetyController,
     is_valid_auth_token,
     load_auth_credentials,
     validate_twist_request,
@@ -47,8 +54,21 @@ class EventHubTests(unittest.TestCase):
         for value in range(3):
             hub.publish(value)
 
-        self.assertEqual([subscriber.get_nowait(), subscriber.get_nowait()], [1, 2])
-        self.assertEqual(subscriber.dropped_critical, 1)
+        with self.assertRaises(EventOverflow):
+            subscriber.get_nowait()
+        self.assertEqual(subscriber.dropped_critical, 3)
+        self.assertEqual(subscriber.pending_count, 0)
+
+    def test_overflowed_subscriber_requires_snapshot_resynchronization(self):
+        hub = EventHub(max_queue_size=1)
+        hub.publish("node-snapshot-v1", key="nodes", snapshot=True)
+        subscriber = hub.subscribe()
+        hub.publish("node-started")
+        hub.publish("node-crashed")
+        with self.assertRaises(EventOverflow):
+            subscriber.get_nowait()
+        recovered = hub.subscribe()
+        self.assertEqual(recovered.get_nowait(), "node-snapshot-v1")
 
     def test_telemetry_is_coalesced_by_key_and_drop_counted(self):
         hub = EventHub(max_queue_size=3)
@@ -114,6 +134,169 @@ class EventHubTests(unittest.TestCase):
 
         self.assertTrue(subscriber.empty())
 
+    def test_event_sequences_and_health_metrics_are_exposed(self):
+        hub = EventHub(max_queue_size=2)
+        subscriber = hub.subscribe()
+        first = SimpleNamespace(sequence=0)
+        second = SimpleNamespace(sequence=0)
+        hub.publish(first)
+        hub.publish(second)
+        self.assertEqual((first.sequence, second.sequence), (1, 2))
+        health = hub.health()
+        self.assertEqual(health["queue_capacity"], 2)
+        self.assertEqual(health["subscriber_count"], 1)
+        self.assertEqual(health["last_sequence"], 2)
+        self.assertEqual(health["max_backlog"], 2)
+
+    @staticmethod
+    def encode_event(event):
+        return json.dumps(
+            {"sequence": event.sequence, "name": event.name}
+        ).encode("utf-8")
+
+    @staticmethod
+    def decode_event(payload):
+        return SimpleNamespace(**json.loads(payload.decode("utf-8")))
+
+    def test_persistent_history_replays_critical_events_after_restart(self):
+        with tempfile.TemporaryDirectory() as directory:
+            path = os.path.join(directory, "events.json")
+            options = {
+                "history_path": path,
+                "event_encoder": self.encode_event,
+                "event_decoder": self.decode_event,
+            }
+            first = EventHub(**options)
+            temporary = os.path.join(directory, ".event-history.tmp")
+            with open(temporary, "w", encoding="utf-8") as file:
+                file.write("interrupted write")
+            first.publish(SimpleNamespace(sequence=0, name="node-started"))
+            self.assertFalse(os.path.exists(temporary))
+            first.publish(
+                SimpleNamespace(sequence=99, name="telemetry"),
+                priority="telemetry",
+            )
+            first.publish(SimpleNamespace(sequence=0, name="node-crashed"))
+            self.assertEqual(first.health()["last_sequence"], 2)
+
+            restarted = EventHub(**options)
+            restarted.publish(
+                SimpleNamespace(sequence=0, name="current-state"),
+                key="state",
+                snapshot=True,
+                durable=False,
+            )
+            subscriber, recovery, replay = restarted.subscribe_with_replay(0)
+            self.assertFalse(recovery["gap_detected"])
+            self.assertTrue(recovery["snapshot_included"])
+            self.assertEqual(
+                [(event.sequence, event.name) for event in replay],
+                [
+                    (1, "node-started"),
+                    (2, "node-crashed"),
+                    (0, "current-state"),
+                ],
+            )
+            self.assertEqual(restarted.health()["last_sequence"], 2)
+            restarted.unsubscribe(subscriber)
+
+    def test_history_retention_reports_gaps_and_includes_latest_snapshot(self):
+        with tempfile.TemporaryDirectory() as directory:
+            hub = EventHub(
+                history_path=os.path.join(directory, "events.json"),
+                event_encoder=self.encode_event,
+                event_decoder=self.decode_event,
+                history_capacity=2,
+            )
+            for value in range(1, 5):
+                hub.publish(SimpleNamespace(sequence=0, name=f"critical-{value}"))
+            snapshot = SimpleNamespace(sequence=0, name="latest-state")
+            hub.publish(snapshot, key="state", snapshot=True, durable=False)
+
+            subscriber, recovery, replay = hub.subscribe_with_replay(1)
+            self.assertTrue(recovery["gap_detected"])
+            self.assertTrue(recovery["snapshot_included"])
+            self.assertEqual(recovery["oldest_available_sequence"], 3)
+            self.assertEqual(recovery["retained_critical_events"], 2)
+            self.assertEqual(
+                [event.name for event in replay],
+                ["critical-3", "critical-4", "latest-state"],
+            )
+            hub.unsubscribe(subscriber)
+
+    def test_replay_subscription_has_no_gap_with_concurrent_live_publish(self):
+        with tempfile.TemporaryDirectory() as directory:
+            hub = EventHub(
+                history_path=os.path.join(directory, "events.json"),
+                event_encoder=self.encode_event,
+                event_decoder=self.decode_event,
+            )
+            hub.publish(SimpleNamespace(sequence=0, name="before-subscribe"))
+            subscriber, recovery, replay = hub.subscribe_with_replay(0)
+            hub.publish(SimpleNamespace(sequence=0, name="after-subscribe"))
+
+            self.assertFalse(recovery["gap_detected"])
+            self.assertEqual([event.name for event in replay], ["before-subscribe"])
+            self.assertEqual(subscriber.get_nowait().name, "after-subscribe")
+            hub.unsubscribe(subscriber)
+
+    def test_future_replay_cursor_reports_gap_and_recovers_retained_history(self):
+        with tempfile.TemporaryDirectory() as directory:
+            hub = EventHub(
+                history_path=os.path.join(directory, "events.json"),
+                event_encoder=self.encode_event,
+                event_decoder=self.decode_event,
+            )
+            hub.publish(SimpleNamespace(sequence=0, name="critical-1"))
+            hub.publish(SimpleNamespace(sequence=0, name="critical-2"))
+
+            subscriber, recovery, replay = hub.subscribe_with_replay(9)
+            self.assertTrue(recovery["gap_detected"])
+            self.assertEqual(
+                [event.name for event in replay],
+                ["critical-1", "critical-2"],
+            )
+            hub.unsubscribe(subscriber)
+
+    def test_critical_event_persistence_failure_is_reported_without_sequence_advance(self):
+        with tempfile.TemporaryDirectory() as directory:
+            hub = EventHub(
+                history_path=os.path.join(directory, "events.json"),
+                event_encoder=self.encode_event,
+                event_decoder=self.decode_event,
+            )
+            subscriber = hub.subscribe()
+            event = SimpleNamespace(sequence=0, name="must-not-disappear")
+            with mock.patch.object(
+                hub,
+                "_persist_history",
+                side_effect=EventPersistenceError("disk unavailable"),
+            ):
+                with self.assertRaisesRegex(EventPersistenceError, "disk unavailable"):
+                    hub.publish(event)
+
+            self.assertEqual(hub.health()["last_sequence"], 0)
+            self.assertFalse(hub.health()["persistence_healthy"])
+            self.assertIn("disk unavailable", hub.health()["persistence_error"])
+            self.assertTrue(subscriber.empty())
+            self.assertEqual(event.sequence, 0)
+            hub.publish(SimpleNamespace(sequence=0, name="later-event"))
+            self.assertFalse(hub.health()["persistence_healthy"])
+
+    def test_persistent_history_rejects_symlinked_journal(self):
+        with tempfile.TemporaryDirectory() as directory:
+            target = os.path.join(directory, "outside.json")
+            journal = os.path.join(directory, "events.json")
+            with open(target, "w", encoding="utf-8") as file:
+                json.dump({"version": 1, "last_sequence": 0, "events": []}, file)
+            os.symlink(target, journal)
+            with self.assertRaisesRegex(ValueError, "could not be loaded safely"):
+                EventHub(
+                    history_path=journal,
+                    event_encoder=self.encode_event,
+                    event_decoder=self.decode_event,
+                )
+
 
 class TwistValidationTests(unittest.TestCase):
     def test_accepts_valid_request(self):
@@ -135,6 +318,74 @@ class TwistValidationTests(unittest.TestCase):
         with self.assertRaisesRegex(ValueError, "Angular velocity magnitude"):
             validate_twist_request(request(angular_x=0.8, angular_z=0.8))
 
+    def test_robot_configs_apply_different_limits_and_modes(self):
+        registry = RobotConfigRegistry()
+        small_robot = RobotMotionConfig(
+            "/small/cmd_vel",
+            max_linear_velocity=0.2,
+            max_angular_velocity=0.4,
+            permitted_motion_modes=("planar",),
+        )
+        registry.set(small_robot)
+        self.assertEqual(registry.get("/large/cmd_vel").max_linear_velocity, 0.5)
+        with self.assertRaisesRegex(ValueError, "robot limit"):
+            registry.validate_command(request("/small/cmd_vel", value=0.3))
+        with self.assertRaisesRegex(ValueError, "planar"):
+            registry.validate_command(
+                request("/small/cmd_vel", value=0.1, linear_y=0.01)
+            )
+
+    def test_robot_config_rejects_unsafe_limits_and_timeouts(self):
+        with self.assertRaises(ValueError):
+            RobotMotionConfig("/robot/cmd_vel", max_linear_velocity=100).validate()
+        with self.assertRaises(ValueError):
+            RobotMotionConfig("/robot/cmd_vel", max_linear_velocity=True).validate()
+        with self.assertRaises(ValueError):
+            RobotMotionConfig(
+                "/robot/cmd_vel",
+                command_lease_ms=2000,
+                command_timeout_ms=1000,
+            ).validate()
+
+    def test_robot_config_can_explicitly_allow_full_twist(self):
+        registry = RobotConfigRegistry()
+        registry.set(
+            RobotMotionConfig(
+                "/arm/cmd_vel",
+                permitted_motion_modes=("full_twist",),
+            )
+        )
+        command = request("/arm/cmd_vel", value=0.1, linear_y=0.1)
+        registry.validate_command(command, monotonic_now=1.0)
+
+    def test_robot_configs_persist_and_reload(self):
+        with tempfile.TemporaryDirectory() as directory:
+            path = os.path.join(directory, "config", "robots.json")
+            registry = RobotConfigRegistry(path)
+            saved = RobotMotionConfig(
+                "/robot/cmd_vel",
+                max_linear_velocity=0.25,
+                permitted_motion_modes=("planar",),
+            )
+            registry.set(saved)
+            reloaded = RobotConfigRegistry(path)
+            self.assertEqual(reloaded.get("/robot/cmd_vel"), saved)
+            reloaded.reset("/robot/cmd_vel")
+            self.assertEqual(reloaded.get("/robot/cmd_vel").max_linear_velocity, 0.5)
+
+    def test_acceleration_limits_are_enforced_per_robot(self):
+        registry = RobotConfigRegistry()
+        registry.set(
+            RobotMotionConfig(
+                "/robot/cmd_vel",
+                max_linear_acceleration=0.5,
+                max_angular_acceleration=1.0,
+            )
+        )
+        registry.validate_command(request(value=0.1), monotonic_now=10.0)
+        with self.assertRaisesRegex(ValueError, "acceleration"):
+            registry.validate_command(request(value=0.2), monotonic_now=10.1)
+
     def test_command_lease_expires_and_refreshes(self):
         leases = CommandLeaseManager(timeout_seconds=0.5)
         leases.refresh("/robot/cmd_vel", now=10.0)
@@ -181,5 +432,81 @@ class AuthenticationTests(unittest.TestCase):
                     load_auth_credentials(credential_file.name)
 
 
+class SafetyControllerTests(unittest.TestCase):
+    def test_latches_activation_and_requires_explicit_reset(self):
+        calls = []
+        safety = SafetyController(lambda: calls.append("stop"))
+        self.assertEqual(safety.snapshot()[0], "READY")
+        self.assertEqual(safety.activate()[0], "ACTIVE")
+        with self.assertRaisesRegex(ValueError, "ACTIVE"):
+            safety.run_if_motion_permitted(lambda: None)
+        self.assertEqual(safety.reset()[0], "READY")
+        self.assertEqual(calls, ["stop", "stop"])
+
+    def test_stop_failure_latches_fault_and_reset_failure_is_not_reported_ready(self):
+        safety = SafetyController(lambda: (_ for _ in ()).throw(OSError("publisher failed")))
+        with self.assertRaises(RuntimeError):
+            safety.activate()
+        self.assertEqual(safety.snapshot()[0], "FAULT")
+
+    def test_event_history_failure_can_fail_closed_and_persist_safety_fault(self):
+        with tempfile.TemporaryDirectory() as directory:
+            stops = []
+            safety = SafetyController(lambda: stops.append("stop"), os.path.join(directory, "safety.json"))
+            safety.latch_fault("event history unavailable")
+            self.assertEqual(safety.snapshot()[0], "FAULT")
+            self.assertIn("event history unavailable", safety.snapshot()[1])
+            self.assertEqual(stops, ["stop"])
+            restarted = SafetyController(lambda: None, os.path.join(directory, "safety.json"))
+            self.assertEqual(restarted.snapshot()[0], "FAULT")
+
+    def test_latched_state_survives_controller_restart(self):
+        with tempfile.TemporaryDirectory() as directory:
+            path = os.path.join(directory, "safety.json")
+            first = SafetyController(lambda: None, path)
+            first.activate()
+            first.reset()
+            restarted = SafetyController(lambda: None, path)
+            self.assertEqual(restarted.snapshot()[0], "READY")
+            self.assertEqual(
+                [event["event"] for event in restarted.history()],
+                ["ACTIVATED", "RESET_REQUESTED", "RESET_CONFIRMED"],
+            )
+
+    def test_interrupted_reset_restores_as_latched_and_history_is_preserved(self):
+        with tempfile.TemporaryDirectory() as directory:
+            path = os.path.join(directory, "safety.json")
+            first = SafetyController(lambda: None, path)
+            first.activate()
+            with open(path, encoding="utf-8") as state_file:
+                state = json.load(state_file)
+            state["state"] = "RESET_PENDING"
+            with open(path, "w", encoding="utf-8") as state_file:
+                json.dump(state, state_file)
+
+            restarted = SafetyController(lambda: None, path)
+            self.assertEqual(restarted.snapshot()[0], "ACTIVE")
+            self.assertEqual(restarted.history()[0]["event"], "ACTIVATED")
+
+    def test_restored_stop_is_reasserted_and_failure_latches_fault(self):
+        with tempfile.TemporaryDirectory() as directory:
+            path = os.path.join(directory, "safety.json")
+            SafetyController(lambda: None, path).activate()
+            calls = []
+            restarted = SafetyController(lambda: calls.append("stop"), path)
+            restarted.enforce_latched_stop()
+            self.assertEqual(calls, ["stop"])
+            self.assertEqual(restarted.snapshot()[0], "ACTIVE")
+            self.assertEqual(restarted.history()[-1]["event"], "STOP_REASSERTED")
+
+            failing = SafetyController(
+                lambda: (_ for _ in ()).throw(OSError("publisher failed")),
+                path,
+            )
+            with self.assertRaises(RuntimeError):
+                failing.enforce_latched_stop()
+            self.assertEqual(failing.snapshot()[0], "FAULT")
+
+
 if __name__ == "__main__":
     unittest.main()
diff --git a/src-tauri/src/lib.rs b/src-tauri/src/lib.rs
index 8fd1649..586a93c 100644
--- a/src-tauri/src/lib.rs
+++ b/src-tauri/src/lib.rs
@@ -1,15 +1,18 @@
-use std::collections::HashSet;
+use std::collections::{HashMap, HashSet};
 use std::fs;
 use std::path::{Component, Path, PathBuf};
-use std::sync::Mutex;
+use std::sync::{Arc, Mutex, OnceLock};
 
 const MAX_FILE_SIZE: u64 = 8 * 1024 * 1024;
 const MAX_WORLD_SIZE: u64 = 10 * 1024 * 1024;
 const MAX_DIRECTORY_ENTRIES: usize = 10_000;
+static ACTIVE_ROS_WORKSPACE: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
 
 #[derive(Default)]
 struct FsAccess {
     approved_roots: Mutex<HashSet<PathBuf>>,
+    #[cfg(unix)]
+    root_descriptors: Mutex<HashMap<PathBuf, Arc<rustix::fd::OwnedFd>>>,
 }
 
 impl FsAccess {
@@ -21,10 +24,39 @@ impl FsAccess {
             return Err("Selected path is not a directory".to_string());
         }
 
+        #[cfg(unix)]
+        let descriptor = rustix::fs::open(
+            &canonical,
+            rustix::fs::OFlags::RDONLY
+                | rustix::fs::OFlags::DIRECTORY
+                | rustix::fs::OFlags::NOFOLLOW
+                | rustix::fs::OFlags::CLOEXEC,
+            rustix::fs::Mode::empty(),
+        )
+        .map_err(|error| format!("Could not pin selected directory: {error}"))?;
+        #[cfg(unix)]
+        {
+            use std::os::unix::fs::MetadataExt;
+            let path_metadata = fs::metadata(&canonical)
+                .map_err(|error| format!("Could not verify selected directory: {error}"))?;
+            let descriptor_metadata = rustix::fs::fstat(&descriptor)
+                .map_err(|error| format!("Could not verify selected directory: {error}"))?;
+            if path_metadata.dev() != descriptor_metadata.st_dev as u64
+                || path_metadata.ino() != descriptor_metadata.st_ino
+            {
+                return Err("Selected directory changed while it was being approved".to_string());
+            }
+        }
+
         self.approved_roots
             .lock()
             .map_err(|_| "Filesystem access state is unavailable".to_string())?
             .insert(canonical.clone());
+        #[cfg(unix)]
+        self.root_descriptors
+            .lock()
+            .map_err(|_| "Filesystem access state is unavailable".to_string())?
+            .insert(canonical.clone(), Arc::new(descriptor));
         Ok(canonical)
     }
 
@@ -86,8 +118,365 @@ impl FsAccess {
 
         Err("Path is outside approved directories".to_string())
     }
+
+    #[cfg(unix)]
+    fn descriptor_and_relative_path(
+        &self,
+        path: &Path,
+    ) -> Result<(rustix::fd::OwnedFd, PathBuf), String> {
+        if !path.is_absolute()
+            || path
+                .components()
+                .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
+        {
+            return Err(
+                "Path must be absolute and cannot contain traversal components".to_string(),
+            );
+        }
+
+        let roots = self
+            .approved_roots
+            .lock()
+            .map_err(|_| "Filesystem access state is unavailable".to_string())?;
+        let candidate: PathBuf = if roots.iter().any(|root| path.starts_with(root)) {
+            path.to_path_buf()
+        } else {
+            let normalized: std::io::Result<PathBuf> = path.canonicalize().or_else(|_| {
+                let parent = path
+                    .parent()
+                    .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?
+                    .canonicalize()?;
+                let name = path
+                    .file_name()
+                    .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?;
+                Ok(parent.join(name))
+            });
+            normalized.map_err(|_| "Path is outside approved directories".to_string())?
+        };
+        let root = roots
+            .iter()
+            .filter(|root| candidate.starts_with(root))
+            .max_by_key(|root| root.components().count())
+            .ok_or_else(|| "Path is outside approved directories".to_string())?;
+        let relative = candidate
+            .strip_prefix(root)
+            .map_err(|_| "Path is outside approved directories".to_string())?;
+        if relative
+            .components()
+            .any(|component| !matches!(component, Component::Normal(_)))
+        {
+            return Err("Path contains an unsupported component".to_string());
+        }
+
+        let descriptors = self
+            .root_descriptors
+            .lock()
+            .map_err(|_| "Filesystem access state is unavailable".to_string())?;
+        let descriptor = descriptors
+            .get(root)
+            .ok_or_else(|| "Approved directory handle is unavailable".to_string())?;
+        let path_metadata = fs::metadata(root)
+            .map_err(|_| "An approved directory is no longer available".to_string())?;
+        let descriptor_metadata = rustix::fs::fstat(&**descriptor)
+            .map_err(|error| format!("Could not verify approved directory: {error}"))?;
+        use std::os::unix::fs::MetadataExt;
+        if path_metadata.dev() != descriptor_metadata.st_dev as u64
+            || path_metadata.ino() != descriptor_metadata.st_ino
+        {
+            return Err("An approved directory has been moved or replaced".to_string());
+        }
+        let duplicated = rustix::io::dup(&**descriptor)
+            .map_err(|error| format!("Could not access approved directory: {error}"))?;
+        Ok((duplicated, relative.to_path_buf()))
+    }
+
+    #[cfg(unix)]
+    fn open_rooted_path(&self, path: &Path, flags: rustix::fs::OFlags) -> Result<fs::File, String> {
+        use rustix::fs::{openat, Mode, OFlags};
+
+        let (mut current, relative) = self.descriptor_and_relative_path(path)?;
+        let components = relative
+            .components()
+            .map(|component| match component {
+                Component::Normal(name) => Ok(name.to_os_string()),
+                _ => Err("Path contains an unsupported component".to_string()),
+            })
+            .collect::<Result<Vec<_>, _>>()?;
+        if components.is_empty() {
+            return Ok(fs::File::from(current));
+        }
+        for component in &components[..components.len() - 1] {
+            current = openat(
+                &current,
+                component,
+                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
+                Mode::empty(),
+            )
+            .map_err(|error| format!("Could not safely open directory: {error}"))?;
+        }
+        let descriptor = openat(
+            &current,
+            &components[components.len() - 1],
+            flags | OFlags::NOFOLLOW | OFlags::CLOEXEC,
+            Mode::empty(),
+        )
+        .map_err(|error| format!("Could not safely open file: {error}"))?;
+        Ok(fs::File::from(descriptor))
+    }
+
+    #[cfg(unix)]
+    fn rooted_parent_and_name(
+        &self,
+        path: &Path,
+    ) -> Result<(rustix::fd::OwnedFd, std::ffi::OsString), String> {
+        use rustix::fs::{openat, Mode, OFlags};
+
+        let (mut current, relative) = self.descriptor_and_relative_path(path)?;
+        let components = relative
+            .components()
+            .map(|component| match component {
+                Component::Normal(name) => Ok(name.to_os_string()),
+                _ => Err("Path contains an unsupported component".to_string()),
+            })
+            .collect::<Result<Vec<_>, _>>()?;
+        let Some(name) = components.last().cloned() else {
+            return Err("Destination must name a file inside an approved directory".to_string());
+        };
+        for component in &components[..components.len() - 1] {
+            current = openat(
+                &current,
+                component,
+                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
+                Mode::empty(),
+            )
+            .map_err(|error| format!("Could not safely open destination directory: {error}"))?;
+        }
+        Ok((current, name))
+    }
+
+    #[cfg(unix)]
+    fn create_directory(&self, path: &Path) -> Result<(), String> {
+        let (parent, name) = self.rooted_parent_and_name(path)?;
+        rustix::fs::mkdirat(&parent, name, rustix::fs::Mode::RWXU)
+            .map_err(|error| format!("Could not safely create directory: {error}"))
+    }
+
+    #[cfg(not(unix))]
+    fn create_directory(&self, path: &Path) -> Result<(), String> {
+        let path = self.resolve_for_write(path)?;
+        fs::create_dir(path).map_err(|error| format!("Could not create directory: {error}"))
+    }
+
+    fn ensure_directory(&self, path: &Path) -> Result<(), String> {
+        if self.open_approved_directory(path).is_ok() {
+            return Ok(());
+        }
+        self.create_directory(path)?;
+        self.open_approved_directory(path)
+    }
+
+    #[cfg(unix)]
+    fn open_approved_directory(&self, path: &Path) -> Result<(), String> {
+        self.open_rooted_path(
+            path,
+            rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::DIRECTORY,
+        )
+        .map(|_| ())
+    }
+
+    #[cfg(not(unix))]
+    fn open_approved_directory(&self, path: &Path) -> Result<(), String> {
+        let path = self.resolve_existing(path)?;
+        if path.is_dir() {
+            Ok(())
+        } else {
+            Err("Path is not a directory".to_string())
+        }
+    }
+
+    #[cfg(unix)]
+    fn create_new_file(&self, path: &Path, contents: &[u8]) -> Result<(), String> {
+        use rustix::fs::{openat, unlinkat, AtFlags, Mode, OFlags};
+
+        if contents.len() as u64 > MAX_WORLD_SIZE {
+            return Err(format!("File exceeds the {MAX_WORLD_SIZE}-byte limit"));
+        }
+        let (parent, name) = self.rooted_parent_and_name(path)?;
+        let descriptor = openat(
+            &parent,
+            &name,
+            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
+            Mode::RUSR | Mode::WUSR,
+        )
+        .map_err(|error| format!("Could not safely create file: {error}"))?;
+        let mut file = fs::File::from(descriptor);
+        let result = (|| {
+            std::io::Write::write_all(&mut file, contents).map_err(|error| error.to_string())?;
+            file.sync_all().map_err(|error| error.to_string())?;
+            rustix::fs::fsync(&parent).map_err(|error| error.to_string())
+        })();
+        if result.is_err() {
+            let _ = unlinkat(&parent, &name, AtFlags::empty());
+        }
+        result
+    }
+
+    #[cfg(unix)]
+    fn copy_file_if_missing(&self, source: &Path, destination: &Path) -> Result<(), String> {
+        use rustix::fs::{openat, statat, unlinkat, AtFlags, FileType, Mode, OFlags};
+
+        let (parent, name) = self.rooted_parent_and_name(destination)?;
+        match statat(&parent, &name, AtFlags::SYMLINK_NOFOLLOW) {
+            Ok(metadata) if FileType::from_raw_mode(metadata.st_mode).is_file() => return Ok(()),
+            Ok(_) => return Err("Copy destination must be a regular file".to_string()),
+            Err(error) if error == rustix::io::Errno::NOENT => {}
+            Err(error) => return Err(format!("Could not inspect copy destination: {error}")),
+        }
+
+        let descriptor = openat(
+            &parent,
+            &name,
+            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
+            Mode::RUSR | Mode::WUSR,
+        )
+        .map_err(|error| format!("Could not safely create copy destination: {error}"))?;
+        let mut output = fs::File::from(descriptor);
+        let result = (|| {
+            let mut input = fs::File::open(source).map_err(|error| error.to_string())?;
+            let permissions = input
+                .metadata()
+                .map_err(|error| error.to_string())?
+                .permissions();
+            std::io::copy(&mut input, &mut output).map_err(|error| error.to_string())?;
+            output
+                .set_permissions(permissions)
+                .map_err(|error| error.to_string())?;
+            output.sync_all().map_err(|error| error.to_string())?;
+            rustix::fs::fsync(&parent).map_err(|error| error.to_string())
+        })();
+        if result.is_err() {
+            let _ = unlinkat(&parent, &name, AtFlags::empty());
+        }
+        result
+    }
+
+    #[cfg(unix)]
+    fn read_file_if_present(&self, path: &Path) -> Result<Option<Vec<u8>>, String> {
+        use rustix::fs::{openat, statat, AtFlags, FileType, OFlags};
+
+        let (parent, name) = self.rooted_parent_and_name(path)?;
+        match statat(&parent, &name, AtFlags::SYMLINK_NOFOLLOW) {
+            Ok(metadata) if FileType::from_raw_mode(metadata.st_mode).is_file() => {}
+            Ok(_) => return Err("Workspace file must be a regular file".to_string()),
+            Err(error) if error == rustix::io::Errno::NOENT => return Ok(None),
+            Err(error) => return Err(format!("Could not inspect workspace file safely: {error}")),
+        }
+        let descriptor = openat(
+            &parent,
+            &name,
+            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
+            rustix::fs::Mode::empty(),
+        )
+        .map_err(|error| format!("Could not safely read workspace file: {error}"))?;
+        let mut file = fs::File::from(descriptor);
+        let mut contents = Vec::new();
+        file.by_ref()
+            .take(MAX_FILE_SIZE + 1)
+            .read_to_end(&mut contents)
+            .map_err(|error| format!("Could not read workspace file: {error}"))?;
+        if contents.len() as u64 > MAX_FILE_SIZE {
+            return Err(format!(
+                "Workspace file exceeds the {MAX_FILE_SIZE}-byte limit"
+            ));
+        }
+        Ok(Some(contents))
+    }
+
+    #[cfg(not(unix))]
+    fn read_file_if_present(&self, path: &Path) -> Result<Option<Vec<u8>>, String> {
+        let path = self.resolve_for_write(path)?;
+        let metadata = match fs::symlink_metadata(&path) {
+            Ok(metadata) => metadata,
+            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
+            Err(error) => return Err(format!("Could not inspect workspace file: {error}")),
+        };
+        if metadata.file_type().is_symlink() || !metadata.is_file() {
+            return Err("Workspace file must be a regular file".to_string());
+        }
+        let mut file = fs::File::open(path)
+            .map_err(|error| format!("Could not read workspace file: {error}"))?;
+        let mut contents = Vec::new();
+        file.by_ref()
+            .take(MAX_FILE_SIZE + 1)
+            .read_to_end(&mut contents)
+            .map_err(|error| format!("Could not read workspace file: {error}"))?;
+        if contents.len() as u64 > MAX_FILE_SIZE {
+            return Err(format!(
+                "Workspace file exceeds the {MAX_FILE_SIZE}-byte limit"
+            ));
+        }
+        Ok(Some(contents))
+    }
+
+    #[cfg(not(unix))]
+    fn copy_file_if_missing(&self, source: &Path, destination: &Path) -> Result<(), String> {
+        let destination = self.resolve_for_write(destination)?;
+        match fs::symlink_metadata(&destination) {
+            Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
+                return Ok(());
+            }
+            Ok(_) => return Err("Copy destination must be a regular file".to_string()),
+            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
+            Err(error) => return Err(format!("Could not inspect copy destination: {error}")),
+        }
+
+        let mut input = fs::File::open(source).map_err(|error| error.to_string())?;
+        let permissions = input
+            .metadata()
+            .map_err(|error| error.to_string())?
+            .permissions();
+        let mut output = fs::OpenOptions::new()
+            .write(true)
+            .create_new(true)
+            .open(destination)
+            .map_err(|error| error.to_string())?;
+        std::io::copy(&mut input, &mut output).map_err(|error| error.to_string())?;
+        output
+            .set_permissions(permissions)
+            .map_err(|error| error.to_string())?;
+        output.sync_all().map_err(|error| error.to_string())
+    }
+
+    #[cfg(not(unix))]
+    fn create_new_file(&self, path: &Path, contents: &[u8]) -> Result<(), String> {
+        if contents.len() as u64 > MAX_WORLD_SIZE {
+            return Err(format!("File exceeds the {MAX_WORLD_SIZE}-byte limit"));
+        }
+        let path = self.resolve_for_write(path)?;
+        let mut file = fs::OpenOptions::new()
+            .write(true)
+            .create_new(true)
+            .open(path)
+            .map_err(|error| error.to_string())?;
+        std::io::Write::write_all(&mut file, contents).map_err(|error| error.to_string())?;
+        file.sync_all().map_err(|error| error.to_string())
+    }
+}
+
+#[cfg(unix)]
+fn read_approved_file(access: &FsAccess, path: &Path) -> Result<String, String> {
+    let file = access.open_rooted_path(path, rustix::fs::OFlags::RDONLY)?;
+    let metadata = file.metadata().map_err(|error| error.to_string())?;
+    if !metadata.is_file() {
+        return Err("Path is not a regular file".to_string());
+    }
+    if metadata.len() > MAX_FILE_SIZE {
+        return Err(format!("File exceeds the {MAX_FILE_SIZE}-byte limit"));
+    }
+    read_file_limited(file, MAX_FILE_SIZE)
 }
 
+#[cfg(not(unix))]
 fn read_approved_file(access: &FsAccess, path: &Path) -> Result<String, String> {
     let path = access.resolve_existing(path)?;
     let metadata = fs::metadata(&path).map_err(|error| error.to_string())?;
@@ -100,6 +489,18 @@ fn read_approved_file(access: &FsAccess, path: &Path) -> Result<String, String>
     read_text_file_limited(&path, MAX_FILE_SIZE)
 }
 
+fn read_file_limited(mut file: fs::File, max_bytes: u64) -> Result<String, String> {
+    let mut contents = String::new();
+    file.by_ref()
+        .take(max_bytes + 1)
+        .read_to_string(&mut contents)
+        .map_err(|error| error.to_string())?;
+    if contents.len() as u64 > max_bytes {
+        return Err(format!("File exceeds the {max_bytes}-byte limit"));
+    }
+    Ok(contents)
+}
+
 fn read_text_file_limited(path: &Path, max_bytes: u64) -> Result<String, String> {
     let file = fs::File::open(path).map_err(|error| error.to_string())?;
     let mut contents = String::new();
@@ -112,6 +513,52 @@ fn read_text_file_limited(path: &Path, max_bytes: u64) -> Result<String, String>
     Ok(contents)
 }
 
+#[cfg(unix)]
+fn write_approved_file(access: &FsAccess, path: &Path, contents: &str) -> Result<(), String> {
+    if contents.len() as u64 > MAX_FILE_SIZE {
+        return Err(format!("File exceeds the {MAX_FILE_SIZE}-byte limit"));
+    }
+    use rustix::fs::{openat, renameat, statat, unlinkat, AtFlags, FileType, Mode, OFlags};
+
+    let (parent, name) = access.rooted_parent_and_name(path)?;
+    match statat(&parent, &name, AtFlags::SYMLINK_NOFOLLOW) {
+        Ok(metadata) => {
+            if !FileType::from_raw_mode(metadata.st_mode).is_file() {
+                return Err("Destination must be a regular file, not a symbolic link".to_string());
+            }
+        }
+        Err(error) if error == rustix::io::Errno::NOENT => {}
+        Err(error) => return Err(format!("Could not inspect destination safely: {error}")),
+    }
+    let counter = std::sync::atomic::AtomicU64::fetch_add(
+        &FILE_WRITE_COUNTER,
+        1,
+        std::sync::atomic::Ordering::Relaxed,
+    );
+    let temporary = format!(".write-{}.{}.tmp", std::process::id(), counter);
+    let write_result = (|| {
+        let descriptor = openat(
+            &parent,
+            &temporary,
+            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
+            Mode::RUSR | Mode::WUSR,
+        )
+        .map_err(|error| format!("Could not create temporary file safely: {error}"))?;
+        let mut file = fs::File::from(descriptor);
+        std::io::Write::write_all(&mut file, contents.as_bytes())
+            .map_err(|error| error.to_string())?;
+        file.sync_all().map_err(|error| error.to_string())?;
+        renameat(&parent, &temporary, &parent, &name)
+            .map_err(|error| format!("Could not atomically replace destination: {error}"))?;
+        rustix::fs::fsync(&parent).map_err(|error| error.to_string())
+    })();
+    if write_result.is_err() {
+        let _ = unlinkat(&parent, &temporary, AtFlags::empty());
+    }
+    write_result
+}
+
+#[cfg(not(unix))]
 fn write_approved_file(access: &FsAccess, path: &Path, contents: &str) -> Result<(), String> {
     if contents.len() as u64 > MAX_FILE_SIZE {
         return Err(format!("File exceeds the {MAX_FILE_SIZE}-byte limit"));
@@ -305,36 +752,64 @@ struct FileEntry {
     is_dir: bool,
 }
 
-#[tauri::command]
-fn list_dir(access: tauri::State<'_, FsAccess>, path: String) -> Result<Vec<FileEntry>, String> {
-    let path = access.resolve_existing(Path::new(&path))?;
+#[cfg(unix)]
+fn list_approved_directory(access: &FsAccess, requested: &Path) -> Result<Vec<FileEntry>, String> {
+    let path = access.open_rooted_path(
+        requested,
+        rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::DIRECTORY,
+    )?;
+    let mut directory = rustix::fs::Dir::read_from(&path).map_err(|error| error.to_string())?;
+    let mut result = Vec::new();
+    while let Some(entry) = directory.read() {
+        let entry = entry.map_err(|error| error.to_string())?;
+        let name = entry.file_name().to_string_lossy().into_owned();
+        if name == "." || name == ".." {
+            continue;
+        }
+        if result.len() >= MAX_DIRECTORY_ENTRIES {
+            return Err(format!(
+                "Directory contains more than {MAX_DIRECTORY_ENTRIES} entries"
+            ));
+        }
+        result.push(FileEntry {
+            path: requested.join(&name).to_string_lossy().into_owned(),
+            name,
+            is_dir: entry.file_type().is_dir(),
+        });
+    }
+    Ok(result)
+}
+
+#[cfg(not(unix))]
+fn list_approved_directory(access: &FsAccess, requested: &Path) -> Result<Vec<FileEntry>, String> {
+    let path = access.resolve_existing(requested)?;
     if !path.is_dir() {
         return Err("Path is not a directory".to_string());
     }
-    let entries = fs::read_dir(&path).map_err(|e| e.to_string())?;
+    let entries = fs::read_dir(&path).map_err(|error| error.to_string())?;
     let mut result = Vec::new();
-
     for (index, entry) in entries.enumerate() {
         if index >= MAX_DIRECTORY_ENTRIES {
             return Err(format!(
                 "Directory contains more than {MAX_DIRECTORY_ENTRIES} entries"
             ));
         }
-        let entry = entry.map_err(|e| e.to_string())?;
-        let file_type = entry.file_type().map_err(|e| e.to_string())?;
-        let name = entry.file_name().to_string_lossy().to_string();
-        let full_path = entry.path().to_string_lossy().to_string();
-
+        let entry = entry.map_err(|error| error.to_string())?;
+        let file_type = entry.file_type().map_err(|error| error.to_string())?;
         result.push(FileEntry {
-            name,
-            path: full_path,
+            name: entry.file_name().to_string_lossy().into_owned(),
+            path: entry.path().to_string_lossy().into_owned(),
             is_dir: file_type.is_dir(),
         });
     }
-
     Ok(result)
 }
 
+#[tauri::command]
+fn list_dir(access: tauri::State<'_, FsAccess>, path: String) -> Result<Vec<FileEntry>, String> {
+    list_approved_directory(&access, Path::new(&path))
+}
+
 #[tauri::command]
 async fn pick_directory(
     app: AppHandle,
@@ -506,34 +981,17 @@ fn validate_sdf_file(path: &Path, content: &str) -> Result<(), String> {
 }
 
 fn import_world_contents(
+    access: &FsAccess,
     workspace: &Path,
     source: &Path,
     content: &str,
 ) -> Result<PathBuf, String> {
     validate_sdf_file(source, content)?;
-    let workspace = workspace
-        .canonicalize()
+    let workspace = access
+        .resolve_existing(workspace)
         .map_err(|error| format!("ROS workspace is unavailable: {error}"))?;
     let worlds_dir = workspace.join("worlds");
-    match fs::symlink_metadata(&worlds_dir) {
-        Ok(metadata) if metadata.file_type().is_symlink() => {
-            return Err("World import directory must not be a symbolic link".to_string());
-        }
-        Ok(metadata) if !metadata.is_dir() => {
-            return Err("World import path is not a directory".to_string());
-        }
-        Ok(_) => {}
-        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
-            fs::create_dir(&worlds_dir).map_err(|error| error.to_string())?;
-        }
-        Err(error) => return Err(error.to_string()),
-    }
-    let canonical_worlds = worlds_dir
-        .canonicalize()
-        .map_err(|error| format!("Could not resolve world import directory: {error}"))?;
-    if !canonical_worlds.starts_with(&workspace) {
-        return Err("World import directory escapes the ROS workspace".to_string());
-    }
+    access.ensure_directory(&worlds_dir)?;
 
     let source_stem = source
         .file_stem()
@@ -559,20 +1017,10 @@ fn import_world_contents(
             "{safe_stem}-{timestamp}-{}-{attempt}.sdf",
             std::process::id()
         );
-        let destination = canonical_worlds.join(name);
-        match std::fs::OpenOptions::new()
-            .write(true)
-            .create_new(true)
-            .open(&destination)
-        {
-            Ok(mut file) => {
-                if let Err(error) = std::io::Write::write_all(&mut file, content.as_bytes()) {
-                    let _ = fs::remove_file(&destination);
-                    return Err(format!("Could not import world file: {error}"));
-                }
-                return Ok(destination);
-            }
-            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
+        let destination = worlds_dir.join(&name);
+        match access.create_new_file(&destination, content.as_bytes()) {
+            Ok(()) => return Ok(destination),
+            Err(error) if error.contains("File exists") => continue,
             Err(error) => return Err(format!("Could not import world file: {error}")),
         }
     }
@@ -613,7 +1061,7 @@ async fn import_world(
     let workspace = PathBuf::from(get_default_workspace_dir()?);
     fs::create_dir_all(&workspace).map_err(|error| error.to_string())?;
     access.approve_directory(&workspace)?;
-    let imported = import_world_contents(&workspace, &source, &content)?;
+    let imported = import_world_contents(&access, &workspace, &source, &content)?;
     Ok(Some(imported.to_string_lossy().into_owned()))
 }
 
@@ -655,18 +1103,27 @@ pub fn run() {
             write_file,
             list_dir,
             pick_directory,
+            set_ros_workspace,
             pick_save_path,
             pick_keybindings_file,
             import_world,
             start_ros_stream,
             run_colcon_build,
             run_colcon_build_streaming,
+            cancel_colcon_build,
+            list_managed_processes,
+            list_ros_packages,
             start_gazebo_sim,
             stop_gazebo_sim,
             reset_gazebo_sim,
             ask_ai,
             initialize_ros_environment,
             publish_twist,
+            set_emergency_stop,
+            get_safety_state,
+            get_robot_config,
+            save_robot_config,
+            reset_robot_config,
             save_ai_settings,
             load_ai_settings,
             delete_ai_credential,
@@ -695,6 +1152,68 @@ use std::sync::atomic::{AtomicBool, Ordering};
 use tauri::{AppHandle, Emitter};
 
 static STREAM_RUNNING: AtomicBool = AtomicBool::new(false);
+static LAST_ROS_EVENT_SEQUENCE_BY_WORKSPACE: OnceLock<Mutex<HashMap<String, u64>>> =
+    OnceLock::new();
+static ROS_SERVER_TLS_CERTIFICATE: OnceLock<Mutex<Option<Vec<u8>>>> = OnceLock::new();
+
+fn set_ros_server_tls_certificate(certificate: Vec<u8>) -> Result<(), String> {
+    if !certificate.starts_with(b"-----BEGIN CERTIFICATE-----")
+        || !certificate.ends_with(b"-----END CERTIFICATE-----\n")
+    {
+        return Err("ROS service returned an invalid TLS certificate".to_string());
+    }
+    *ROS_SERVER_TLS_CERTIFICATE
+        .get_or_init(|| Mutex::new(None))
+        .lock()
+        .map_err(|_| "ROS TLS certificate state is unavailable".to_string())? = Some(certificate);
+    Ok(())
+}
+
+async fn connect_ros_engine() -> Result<RosEngineClient<tonic::transport::Channel>, String> {
+    let certificate = ROS_SERVER_TLS_CERTIFICATE
+        .get_or_init(|| Mutex::new(None))
+        .lock()
+        .map_err(|_| "ROS TLS certificate state is unavailable".to_string())?
+        .clone()
+        .ok_or_else(|| "ROS service TLS certificate is not initialized".to_string())?;
+    let tls = tonic::transport::ClientTlsConfig::new()
+        .ca_certificate(tonic::transport::Certificate::from_pem(certificate))
+        .domain_name("localhost");
+    let endpoint = tonic::transport::Endpoint::from_static("https://127.0.0.1:50051")
+        .tls_config(tls)
+        .map_err(|error| format!("Could not configure ROS TLS: {error}"))?;
+    let channel = endpoint
+        .connect()
+        .await
+        .map_err(|error| format!("Could not establish authenticated ROS TLS: {error:?}"))?;
+    Ok(RosEngineClient::new(channel))
+}
+
+fn ros_event_cursor(workspace: &str) -> Result<u64, String> {
+    let cursors = LAST_ROS_EVENT_SEQUENCE_BY_WORKSPACE
+        .get_or_init(|| Mutex::new(HashMap::new()))
+        .lock()
+        .map_err(|_| "ROS event recovery state is unavailable".to_string())?;
+    Ok(*cursors.get(workspace).unwrap_or(&0))
+}
+
+fn update_ros_event_cursor(
+    workspace: &str,
+    sequence: u64,
+    reset_on_gap: bool,
+) -> Result<(), String> {
+    let mut cursors = LAST_ROS_EVENT_SEQUENCE_BY_WORKSPACE
+        .get_or_init(|| Mutex::new(HashMap::new()))
+        .lock()
+        .map_err(|_| "ROS event recovery state is unavailable".to_string())?;
+    let cursor = cursors.entry(workspace.to_string()).or_default();
+    if reset_on_gap && sequence < *cursor {
+        *cursor = sequence;
+    } else {
+        *cursor = (*cursor).max(sequence);
+    }
+    Ok(())
+}
 
 fn stop_stream_for_auth_recovery(app: &AppHandle, message: &str) {
     let _ = app.emit("ros-auth-error", message.to_string());
@@ -711,6 +1230,14 @@ async fn start_ros_stream(app: AppHandle) -> Result<(), String> {
 
     tauri::async_runtime::spawn(async move {
         loop {
+            let stream_workspace = match get_default_workspace_dir() {
+                Ok(workspace) => workspace,
+                Err(error) => {
+                    let _ = app.emit("ros-error", error);
+                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
+                    continue;
+                }
+            };
             let token = match read_grpc_auth_token(&app) {
                 Ok(token) => token,
                 Err(error) => {
@@ -719,12 +1246,9 @@ async fn start_ros_stream(app: AppHandle) -> Result<(), String> {
                 }
             };
             let _ = app.emit("ros-connecting", ());
-            println!("[ros-stream] attempting connection to localhost:50051");
-            let client_result = tokio::time::timeout(
-                std::time::Duration::from_secs(5),
-                RosEngineClient::connect("http://localhost:50051"),
-            )
-            .await;
+            println!("[ros-stream] attempting authenticated TLS connection to ROS service");
+            let client_result =
+                tokio::time::timeout(std::time::Duration::from_secs(5), connect_ros_engine()).await;
 
             let mut client = match client_result {
                 Ok(Ok(c)) => {
@@ -744,7 +1268,20 @@ async fn start_ros_stream(app: AppHandle) -> Result<(), String> {
                     continue;
                 }
             };
-            let request = match authenticated_request(Empty {}, &token) {
+            let request = match authenticated_request(
+                rosengine::ReplayEventsRequest {
+                    has_after_sequence: true,
+                    after_sequence: match ros_event_cursor(&stream_workspace) {
+                        Ok(sequence) => sequence,
+                        Err(error) => {
+                            let _ = app.emit("ros-error", error);
+                            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
+                            continue;
+                        }
+                    },
+                },
+                &token,
+            ) {
                 Ok(request) => request,
                 Err(error) => {
                     let _ = app.emit("ros-error", error);
@@ -754,12 +1291,12 @@ async fn start_ros_stream(app: AppHandle) -> Result<(), String> {
             };
             let mut stream = match tokio::time::timeout(
                 std::time::Duration::from_secs(5),
-                client.stream_events(request),
+                client.replay_events(request),
             )
             .await
             {
                 Ok(Ok(s)) => {
-                    println!("[ros-stream] stream_events established, awaiting messages");
+                    println!("[ros-stream] replayable event stream established");
                     s.into_inner()
                 }
                 Ok(Err(e)) if e.code() == tonic::Code::Unauthenticated => {
@@ -775,7 +1312,7 @@ async fn start_ros_stream(app: AppHandle) -> Result<(), String> {
                     continue;
                 }
                 Err(_) => {
-                    let _ = app.emit("ros-error", "stream_events timed out".to_string());
+                    let _ = app.emit("ros-error", "replay_events timed out".to_string());
                     tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                     continue;
                 }
@@ -784,8 +1321,20 @@ async fn start_ros_stream(app: AppHandle) -> Result<(), String> {
             loop {
                 match stream.message().await {
                     Ok(Some(evt)) => {
+                        if !get_default_workspace_dir()
+                            .is_ok_and(|workspace| workspace == stream_workspace)
+                        {
+                            break;
+                        }
                         println!("[ros-stream] received event: {:?}", evt);
                         let _ = app.emit("ros-event", format!("{:?}", evt));
+                        if evt.sequence > 0 {
+                            if let Err(error) =
+                                update_ros_event_cursor(&stream_workspace, evt.sequence, false)
+                            {
+                                let _ = app.emit("ros-error", error);
+                            }
+                        }
 
                         if let Some(rosengine::workspace_event::Event::OdometryUpdate(odom)) =
                             &evt.event
@@ -826,6 +1375,61 @@ async fn start_ros_stream(app: AppHandle) -> Result<(), String> {
                         {
                             let _ = app.emit("node-snapshot", snap.node_names.clone());
                         }
+                        if let Some(rosengine::workspace_event::Event::SafetyState(state)) =
+                            &evt.event
+                        {
+                            let _ = app.emit("safety-state", safety_state_response(state));
+                        }
+                        if let Some(rosengine::workspace_event::Event::EventHealth(health)) =
+                            &evt.event
+                        {
+                            let _ = app.emit(
+                                "event-health",
+                                EventHealthResponse {
+                                    queue_capacity: health.queue_capacity,
+                                    subscriber_count: health.subscriber_count,
+                                    last_sequence: health.last_sequence,
+                                    dropped_telemetry: health.dropped_telemetry,
+                                    critical_overflows: health.critical_overflows,
+                                    max_backlog: health.max_backlog,
+                                    persistence_healthy: health.persistence_healthy,
+                                    persistence_error: health.persistence_error.clone(),
+                                    oldest_available_sequence: health.oldest_available_sequence,
+                                    retained_critical_events: health.retained_critical_events,
+                                },
+                            );
+                        }
+                        if let Some(rosengine::workspace_event::Event::EventRecovery(recovery)) =
+                            &evt.event
+                        {
+                            if recovery.gap_detected {
+                                match ros_event_cursor(&stream_workspace) {
+                                    Ok(cursor) if recovery.latest_sequence < cursor => {
+                                        if let Err(error) = update_ros_event_cursor(
+                                            &stream_workspace,
+                                            recovery.latest_sequence,
+                                            true,
+                                        ) {
+                                            let _ = app.emit("ros-error", error);
+                                        }
+                                    }
+                                    Err(error) => {
+                                        let _ = app.emit("ros-error", error);
+                                    }
+                                    _ => {}
+                                }
+                            }
+                            let _ = app.emit(
+                                "event-recovery",
+                                EventRecoveryResponse {
+                                    gap_detected: recovery.gap_detected,
+                                    oldest_available_sequence: recovery.oldest_available_sequence,
+                                    latest_sequence: recovery.latest_sequence,
+                                    snapshot_included: recovery.snapshot_included,
+                                    retained_critical_events: recovery.retained_critical_events,
+                                },
+                            );
+                        }
                     }
                     Ok(None) => {
                         let _ = app.emit("ros-error", "Stream closed by server".to_string());
@@ -955,7 +1559,7 @@ async fn publish_twist(app: AppHandle, command: TwistInput) -> Result<(), String
         "Angular",
     )?;
 
-    let mut client = RosEngineClient::connect("http://localhost:50051")
+    let mut client = connect_ros_engine()
         .await
         .map_err(|_| docker_lost_connection_message())?;
 
@@ -986,26 +1590,387 @@ async fn publish_twist(app: AppHandle, command: TwistInput) -> Result<(), String
     Ok(())
 }
 
-use std::process::Command as StdCommand;
+#[derive(Serialize, Clone)]
+#[serde(rename_all = "camelCase")]
+struct SafetyStateResponse {
+    state: String,
+    detail: String,
+    updated_at_ns: i64,
+}
 
-fn docker_command() -> StdCommand {
-    let mut cmd = StdCommand::new("docker");
-    let current_path = std::env::var("PATH").unwrap_or_default();
-    let augmented_path = format!(
-        "/usr/local/bin:/opt/homebrew/bin:/Applications/Docker.app/Contents/Resources/bin:{}",
-        current_path
-    );
+#[derive(Serialize, Clone)]
+#[serde(rename_all = "camelCase")]
+struct EventHealthResponse {
+    queue_capacity: u32,
+    subscriber_count: u32,
+    last_sequence: u64,
+    dropped_telemetry: u64,
+    critical_overflows: u64,
+    max_backlog: u32,
+    persistence_healthy: bool,
+    persistence_error: String,
+    oldest_available_sequence: u64,
+    retained_critical_events: u32,
+}
+
+#[derive(Serialize, Clone)]
+#[serde(rename_all = "camelCase")]
+struct EventRecoveryResponse {
+    gap_detected: bool,
+    oldest_available_sequence: u64,
+    latest_sequence: u64,
+    snapshot_included: bool,
+    retained_critical_events: u32,
+}
+
+fn safety_state_response(state: &rosengine::SafetyState) -> SafetyStateResponse {
+    SafetyStateResponse {
+        state: match state.state {
+            0 => "READY",
+            1 => "ACTIVE",
+            2 => "RESET_PENDING",
+            3 => "FAULT",
+            _ => "DISCONNECTED",
+        }
+        .to_string(),
+        detail: state.detail.clone(),
+        updated_at_ns: state.updated_at_ns,
+    }
+}
+
+async fn connect_ros_client(
+    app: &AppHandle,
+) -> Result<(RosEngineClient<tonic::transport::Channel>, String), String> {
+    let token = read_grpc_auth_token(app)?;
+    let client = tokio::time::timeout(std::time::Duration::from_secs(5), connect_ros_engine())
+        .await
+        .map_err(|_| "ROS service connection timed out".to_string())??;
+    Ok((client, token))
+}
+
+#[tauri::command]
+async fn set_emergency_stop(app: AppHandle, activate: bool) -> Result<SafetyStateResponse, String> {
+    let (mut client, token) = connect_ros_client(&app).await?;
+    let response = client
+        .set_emergency_stop(authenticated_request(
+            rosengine::EmergencyStopRequest {
+                action: if activate { 1 } else { 2 },
+            },
+            &token,
+        )?)
+        .await
+        .map_err(|error| format!("Emergency-stop request failed: {}", error.message()))?
+        .into_inner();
+    Ok(safety_state_response(&response))
+}
+
+#[tauri::command]
+async fn get_safety_state(app: AppHandle) -> Result<SafetyStateResponse, String> {
+    let (mut client, token) = connect_ros_client(&app).await?;
+    let response = client
+        .get_safety_state(authenticated_request(Empty {}, &token)?)
+        .await
+        .map_err(|error| format!("Could not retrieve safety state: {}", error.message()))?
+        .into_inner();
+    Ok(safety_state_response(&response))
+}
+
+#[derive(Serialize, Clone)]
+#[serde(rename_all = "camelCase")]
+struct RobotConfigResponse {
+    topic_name: String,
+    max_linear_velocity: f64,
+    max_angular_velocity: f64,
+    max_linear_acceleration: f64,
+    max_angular_acceleration: f64,
+    command_lease_ms: u32,
+    command_timeout_ms: u32,
+    permitted_motion_modes: Vec<String>,
+}
+
+impl From<rosengine::RobotConfig> for RobotConfigResponse {
+    fn from(config: rosengine::RobotConfig) -> Self {
+        Self {
+            topic_name: config.topic_name,
+            max_linear_velocity: config.max_linear_velocity,
+            max_angular_velocity: config.max_angular_velocity,
+            max_linear_acceleration: config.max_linear_acceleration,
+            max_angular_acceleration: config.max_angular_acceleration,
+            command_lease_ms: config.command_lease_ms,
+            command_timeout_ms: config.command_timeout_ms,
+            permitted_motion_modes: config.permitted_motion_modes,
+        }
+    }
+}
+
+#[derive(Deserialize)]
+#[serde(rename_all = "camelCase")]
+struct RobotConfigInput {
+    topic_name: String,
+    max_linear_velocity: f64,
+    max_angular_velocity: f64,
+    max_linear_acceleration: f64,
+    max_angular_acceleration: f64,
+    command_lease_ms: u32,
+    command_timeout_ms: u32,
+    permitted_motion_modes: Vec<String>,
+}
+
+#[tauri::command]
+async fn get_robot_config(
+    app: AppHandle,
+    topic_name: String,
+) -> Result<RobotConfigResponse, String> {
+    if !is_valid_ros_topic(&topic_name) {
+        return Err("Invalid robot command topic".to_string());
+    }
+    let (mut client, token) = connect_ros_client(&app).await?;
+    let response = client
+        .get_robot_config(authenticated_request(
+            rosengine::RobotConfigRequest { topic_name },
+            &token,
+        )?)
+        .await
+        .map_err(|error| format!("Could not retrieve robot settings: {}", error.message()))?
+        .into_inner();
+    Ok(response.into())
+}
+
+#[tauri::command]
+async fn save_robot_config(
+    app: AppHandle,
+    config: RobotConfigInput,
+) -> Result<RobotConfigResponse, String> {
+    if !is_valid_ros_topic(&config.topic_name) {
+        return Err("Invalid robot command topic".to_string());
+    }
+    let proto_config = rosengine::RobotConfig {
+        topic_name: config.topic_name,
+        max_linear_velocity: config.max_linear_velocity,
+        max_angular_velocity: config.max_angular_velocity,
+        max_linear_acceleration: config.max_linear_acceleration,
+        max_angular_acceleration: config.max_angular_acceleration,
+        command_lease_ms: config.command_lease_ms,
+        command_timeout_ms: config.command_timeout_ms,
+        permitted_motion_modes: config.permitted_motion_modes,
+        reset_to_defaults: false,
+    };
+    let (mut client, token) = connect_ros_client(&app).await?;
+    let response = client
+        .set_robot_config(authenticated_request(proto_config, &token)?)
+        .await
+        .map_err(|error| format!("Robot settings were rejected: {}", error.message()))?
+        .into_inner();
+    Ok(response.into())
+}
+
+#[tauri::command]
+async fn reset_robot_config(
+    app: AppHandle,
+    topic_name: String,
+) -> Result<RobotConfigResponse, String> {
+    if !is_valid_ros_topic(&topic_name) {
+        return Err("Invalid robot command topic".to_string());
+    }
+    let (mut client, token) = connect_ros_client(&app).await?;
+    let response = client
+        .set_robot_config(authenticated_request(
+            rosengine::RobotConfig {
+                topic_name,
+                max_linear_velocity: 0.0,
+                max_angular_velocity: 0.0,
+                max_linear_acceleration: 0.0,
+                max_angular_acceleration: 0.0,
+                command_lease_ms: 0,
+                command_timeout_ms: 0,
+                permitted_motion_modes: Vec::new(),
+                reset_to_defaults: true,
+            },
+            &token,
+        )?)
+        .await
+        .map_err(|error| format!("Could not reset robot settings: {}", error.message()))?
+        .into_inner();
+    Ok(response.into())
+}
+
+use std::process::Command as StdCommand;
+
+fn docker_command() -> StdCommand {
+    let mut cmd = StdCommand::new("docker");
+    let current_path = std::env::var("PATH").unwrap_or_default();
+    let augmented_path = format!(
+        "/usr/local/bin:/opt/homebrew/bin:/Applications/Docker.app/Contents/Resources/bin:{}",
+        current_path
+    );
     cmd.env("PATH", augmented_path);
     cmd
 }
 
+fn workspace_selection_file() -> Result<PathBuf, String> {
+    let home = std::env::var_os("HOME")
+        .or_else(|| std::env::var_os("USERPROFILE"))
+        .ok_or_else(|| "Could not determine the current user's home directory".to_string())?;
+    Ok(PathBuf::from(home)
+        .join(".robotics-studio")
+        .join("selected-workspace.txt"))
+}
+
+fn load_selected_workspace() -> Result<Option<PathBuf>, String> {
+    let path = workspace_selection_file()?;
+    let metadata = match fs::symlink_metadata(&path) {
+        Ok(metadata) => metadata,
+        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
+        Err(error) => return Err(format!("Could not inspect selected workspace: {error}")),
+    };
+    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > 4096 {
+        return Err("Selected workspace record is unsafe or invalid".to_string());
+    }
+    let value = fs::read_to_string(path)
+        .map_err(|error| format!("Could not read selected workspace record: {error}"))?;
+    let selected = PathBuf::from(value.trim());
+    if !selected.is_absolute() {
+        return Err("Selected workspace record must contain an absolute path".to_string());
+    }
+    let canonical = selected
+        .canonicalize()
+        .map_err(|error| format!("Previously selected ROS workspace is unavailable: {error}"))?;
+    if canonical != selected || !canonical.is_dir() {
+        return Err("Previously selected ROS workspace was moved or replaced".to_string());
+    }
+    Ok(Some(canonical))
+}
+
+fn workspace_path_for_shutdown() -> Result<PathBuf, String> {
+    if let Some(workspace) = ACTIVE_ROS_WORKSPACE
+        .get_or_init(|| Mutex::new(None))
+        .lock()
+        .map_err(|_| "Active ROS workspace state is unavailable".to_string())?
+        .clone()
+    {
+        return Ok(workspace);
+    }
+    let selection_file = workspace_selection_file()?;
+    match fs::symlink_metadata(&selection_file) {
+        Ok(metadata)
+            if metadata.file_type().is_symlink()
+                || !metadata.is_file()
+                || metadata.len() > 4096 =>
+        {
+            return Err("Selected workspace record is unsafe or invalid".to_string());
+        }
+        Ok(_) => {
+            let value = fs::read_to_string(selection_file)
+                .map_err(|error| format!("Could not read selected workspace record: {error}"))?;
+            let workspace = PathBuf::from(value.trim());
+            if !workspace.is_absolute() {
+                return Err("Selected workspace record must contain an absolute path".to_string());
+            }
+            return Ok(workspace);
+        }
+        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
+        Err(error) => return Err(format!("Could not inspect selected workspace: {error}")),
+    }
+    let home = std::env::var_os("HOME")
+        .or_else(|| std::env::var_os("USERPROFILE"))
+        .ok_or_else(|| "Could not determine the current user's home directory".to_string())?;
+    Ok(PathBuf::from(home)
+        .join(".robotics-studio")
+        .join("ros2-workspace"))
+}
+
+fn persist_selected_workspace(workspace: &Path) -> Result<(), String> {
+    let selection_file = workspace_selection_file()?;
+    let parent = selection_file
+        .parent()
+        .ok_or_else(|| "Workspace settings directory is invalid".to_string())?;
+    match fs::symlink_metadata(parent) {
+        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
+            return Err("Workspace settings directory is unsafe".to_string());
+        }
+        Ok(_) => {}
+        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
+            fs::create_dir(parent)
+                .map_err(|error| format!("Could not create workspace settings: {error}"))?;
+        }
+        Err(error) => return Err(format!("Could not inspect workspace settings: {error}")),
+    }
+    static NEXT_TEMP_FILE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
+    let temporary = parent.join(format!(
+        ".selected-workspace-{}-{}.tmp",
+        std::process::id(),
+        NEXT_TEMP_FILE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
+    ));
+    let result = (|| {
+        let mut options = fs::OpenOptions::new();
+        options.write(true).create_new(true);
+        #[cfg(unix)]
+        {
+            use std::os::unix::fs::OpenOptionsExt;
+            options.mode(0o600);
+        }
+        let mut file = options
+            .open(&temporary)
+            .map_err(|error| format!("Could not create workspace settings: {error}"))?;
+        std::io::Write::write_all(&mut file, workspace.to_string_lossy().as_bytes())
+            .map_err(|error| format!("Could not save selected workspace: {error}"))?;
+        file.sync_all()
+            .map_err(|error| format!("Could not sync selected workspace: {error}"))?;
+        #[cfg(windows)]
+        if selection_file.exists() {
+            fs::remove_file(&selection_file)
+                .map_err(|error| format!("Could not replace selected workspace: {error}"))?;
+        }
+        fs::rename(&temporary, &selection_file)
+            .map_err(|error| format!("Could not save selected workspace: {error}"))?;
+        Ok(())
+    })();
+    if result.is_err() {
+        let _ = fs::remove_file(&temporary);
+    }
+    result
+}
+
 fn get_default_workspace_dir() -> Result<String, String> {
+    let active = ACTIVE_ROS_WORKSPACE
+        .get_or_init(|| Mutex::new(None))
+        .lock()
+        .map_err(|_| "Active ROS workspace state is unavailable".to_string())?
+        .clone()
+        .or(load_selected_workspace()?);
+    if let Some(workspace) = active {
+        let current = workspace
+            .canonicalize()
+            .map_err(|error| format!("Selected ROS workspace is unavailable: {error}"))?;
+        if current != workspace || !current.is_dir() {
+            return Err("Selected ROS workspace was moved or replaced".to_string());
+        }
+        *ACTIVE_ROS_WORKSPACE
+            .get_or_init(|| Mutex::new(None))
+            .lock()
+            .map_err(|_| "Active ROS workspace state is unavailable".to_string())? =
+            Some(current.clone());
+        return Ok(current.to_string_lossy().into_owned());
+    }
     let home = std::env::var_os("HOME")
         .or_else(|| std::env::var_os("USERPROFILE"))
         .ok_or_else(|| "Could not determine the current user's home directory".to_string())?;
-    Ok(std::path::PathBuf::from(home)
+    let fallback = std::path::PathBuf::from(home)
         .join(".robotics-studio")
-        .join("ros2-workspace")
+        .join("ros2-workspace");
+    if fallback.exists() {
+        let canonical = fallback
+            .canonicalize()
+            .map_err(|error| format!("Default ROS workspace is unavailable: {error}"))?;
+        if canonical.is_dir() {
+            return Ok(canonical.to_string_lossy().into_owned());
+        }
+    }
+    fs::create_dir_all(&fallback)
+        .map_err(|error| format!("Could not create default ROS workspace: {error}"))?;
+    Ok(fallback
+        .canonicalize()
+        .map_err(|error| format!("Default ROS workspace is unavailable: {error}"))?
         .to_string_lossy()
         .into_owned())
 }
@@ -1077,6 +2042,176 @@ fn ensure_loopback_grpc_binding(container_id: &str) -> Result<(), String> {
     Ok(())
 }
 
+fn stop_workspace_container(workspace: &Path) -> Result<(), String> {
+    let label_filter = format!(
+        "label=devcontainer.local_folder={}",
+        workspace.to_string_lossy()
+    );
+    let listed = docker_command()
+        .args(["ps", "-q", "--filter", &label_filter])
+        .output()
+        .map_err(|error| format!("Could not inspect the current ROS container: {error}"))?;
+    if !listed.status.success() {
+        return Err(format!(
+            "Could not inspect the current ROS container: {}",
+            String::from_utf8_lossy(&listed.stderr).trim()
+        ));
+    }
+    let listed_ids = String::from_utf8_lossy(&listed.stdout);
+    let ids = listed_ids
+        .lines()
+        .map(str::trim)
+        .filter(|id| !id.is_empty())
+        .map(str::to_string)
+        .collect::<Vec<_>>();
+    if ids.is_empty() {
+        return Ok(());
+    }
+    let stopped = docker_command()
+        .arg("stop")
+        .arg("--time")
+        .arg("5")
+        .args(&ids)
+        .output()
+        .map_err(|error| format!("Could not stop the previous ROS environment: {error}"))?;
+    if !stopped.status.success() {
+        return Err(format!(
+            "Could not stop the previous ROS environment: {}",
+            String::from_utf8_lossy(&stopped.stderr).trim()
+        ));
+    }
+    Ok(())
+}
+
+#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
+struct RosPackageInfo {
+    name: String,
+    path: String,
+    package_type: String,
+    version: Option<String>,
+    dependencies: Vec<String>,
+    executables: Vec<String>,
+    launch_files: Vec<String>,
+    config_files: Vec<String>,
+}
+
+#[tauri::command]
+fn list_ros_packages() -> Result<Vec<RosPackageInfo>, String> {
+    let container_name = find_dev_container()?;
+    let workspace = get_default_workspace_dir()?;
+    let script = format!(
+        r#"source /opt/ros/jazzy/setup.bash >/dev/null 2>&1 || exit 1
+export ROBOTICS_STUDIO_WORKSPACE="{}"
+if [ -f /workspace/install/setup.bash ]; then source /workspace/install/setup.bash >/dev/null 2>&1 || true; fi
+python3 - <<'PY'
+import json
+import os
+import xml.etree.ElementTree as ET
+from pathlib import Path
+
+try:
+    from ament_index_python.packages import get_packages
+except Exception as exc:  # pragma: no cover - depends on ROS installation
+    print(json.dumps([]))
+    raise SystemExit(0)
+
+packages = get_packages()
+results = []
+for name in sorted(packages):
+    pkg_path = Path(packages[name])
+    package_xml = pkg_path / 'package.xml'
+    version = None
+    package_type = 'ros_package'
+    dependencies = []
+    executables = []
+    launch_files = []
+    config_files = []
+
+    if package_xml.exists():
+        try:
+            root = ET.parse(package_xml).getroot()
+            version = root.attrib.get('version')
+            package_type = root.findtext('export/build_type') or root.tag.rsplit('}}', 1)[-1] or 'ros_package'
+            for tag_name in ['depend', 'build_depend', 'buildtool_depend', 'exec_depend', 'test_depend', 'build_export_depend']:
+                for element in root.findall(tag_name):
+                    value = (element.text or '').strip()
+                    if value:
+                        dependencies.append(value)
+            dependencies = sorted(dict.fromkeys(dependencies))
+        except Exception:
+            pass
+
+    for candidate in sorted(pkg_path.iterdir()):
+        if candidate.name.endswith('.py') and os.access(candidate, os.X_OK):
+            executables.append(candidate.name)
+        if candidate.suffix in {{'.launch', '.xml'}} and candidate.name.endswith('.launch'):
+            launch_files.append(candidate.name)
+        if candidate.suffix in {{'.yaml', '.yml', '.json', '.config', '.ini'}}:
+            config_files.append(candidate.name)
+
+    if not version and package_xml.exists():
+        try:
+            for elem in ET.parse(package_xml).getroot():
+                if elem.tag.endswith('version'):
+                    version = (elem.text or '').strip()
+                    break
+        except Exception:
+            pass
+
+    results.append({{
+        'name': name,
+        'path': str(pkg_path),
+        'package_type': package_type,
+        'version': version,
+        'dependencies': dependencies,
+        'executables': executables,
+        'launch_files': launch_files,
+        'config_files': config_files,
+    }})
+
+print(json.dumps(results))
+PY
+"#,
+        workspace
+    );
+    let output = docker_command()
+        .args(["exec", &container_name, "bash", "-lc", &script])
+        .output()
+        .map_err(|error| format!("Could not inspect ROS packages: {error}"))?;
+    if !output.status.success() {
+        return Err(format!(
+            "Could not inspect ROS packages: {}",
+            String::from_utf8_lossy(&output.stderr).trim()
+        ));
+    }
+    let stdout = String::from_utf8_lossy(&output.stdout);
+    let parsed: Vec<RosPackageInfo> = serde_json::from_str(stdout.trim())
+        .map_err(|error| format!("Could not parse ROS package metadata: {error}"))?;
+    Ok(parsed)
+}
+
+#[tauri::command]
+fn set_ros_workspace(
+    access: tauri::State<'_, FsAccess>,
+    workspace_root: String,
+) -> Result<String, String> {
+    let workspace = access.resolve_existing(Path::new(&workspace_root))?;
+    if !workspace.is_dir() {
+        return Err("Selected ROS workspace is not a directory".to_string());
+    }
+    let previous = workspace_path_for_shutdown()?;
+    if previous != workspace {
+        stop_workspace_container(&previous)?;
+    }
+    persist_selected_workspace(&workspace)?;
+    *ACTIVE_ROS_WORKSPACE
+        .get_or_init(|| Mutex::new(None))
+        .lock()
+        .map_err(|_| "Active ROS workspace state is unavailable".to_string())? =
+        Some(workspace.clone());
+    Ok(workspace.to_string_lossy().into_owned())
+}
+
 fn find_dev_container() -> Result<String, String> {
     if ensure_docker_available().is_err() {
         return Err(docker_lost_connection_message());
@@ -1293,6 +2428,184 @@ fn stream_process_output<R: Read + Send + 'static>(
 }
 
 static BUILD_RUNNING: AtomicBool = AtomicBool::new(false);
+static BUILD_CONTAINER: OnceLock<Mutex<Option<(String, String)>>> = OnceLock::new();
+static BUILD_RUN_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
+
+#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
+pub enum ManagedProcessStatus {
+    Running,
+    Completed,
+    Failed,
+    Cancelled,
+    TimedOut,
+}
+
+#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
+pub struct ManagedProcess {
+    pub id: String,
+    pub process_type: String,
+    pub command: Vec<String>,
+    pub workspace: Option<String>,
+    pub created_at: u64,
+    pub started_at: Option<u64>,
+    pub status: ManagedProcessStatus,
+    pub exit_code: Option<i32>,
+    pub error: Option<String>,
+    pub parent_task_id: Option<String>,
+}
+
+#[derive(Default)]
+struct ProcessRegistry {
+    processes: Mutex<HashMap<String, ManagedProcess>>,
+}
+
+impl ProcessRegistry {
+    #[allow(dead_code)]
+    fn register(
+        &self,
+        id: String,
+        process_type: String,
+        command: Vec<String>,
+        workspace: Option<String>,
+        parent_task_id: Option<String>,
+    ) -> ManagedProcess {
+        let process = ManagedProcess {
+            id: id.clone(),
+            process_type,
+            command,
+            workspace,
+            created_at: current_unix_seconds(),
+            started_at: Some(current_unix_seconds()),
+            status: ManagedProcessStatus::Running,
+            exit_code: None,
+            error: None,
+            parent_task_id,
+        };
+        self.processes
+            .lock()
+            .unwrap_or_else(std::sync::PoisonError::into_inner)
+            .insert(id.clone(), process.clone());
+        process
+    }
+
+    #[allow(dead_code)]
+    fn get(&self, id: &str) -> Option<ManagedProcess> {
+        self.processes
+            .lock()
+            .unwrap_or_else(std::sync::PoisonError::into_inner)
+            .get(id)
+            .cloned()
+    }
+
+    fn list(&self) -> Vec<ManagedProcess> {
+        let processes = self
+            .processes
+            .lock()
+            .unwrap_or_else(std::sync::PoisonError::into_inner);
+        let mut entries: Vec<_> = processes.values().cloned().collect();
+        entries.sort_by_key(|entry| entry.created_at);
+        entries
+    }
+
+    fn mark_completed(
+        &self,
+        id: &str,
+        exit_code: Option<i32>,
+        status: ManagedProcessStatus,
+        error: Option<String>,
+    ) {
+        let mut processes = self
+            .processes
+            .lock()
+            .unwrap_or_else(std::sync::PoisonError::into_inner);
+        if let Some(process) = processes.get_mut(id) {
+            process.status = status;
+            process.exit_code = exit_code;
+            process.error = error;
+        }
+    }
+
+    fn cancel(&self, id: &str) -> bool {
+        let mut processes = self
+            .processes
+            .lock()
+            .unwrap_or_else(std::sync::PoisonError::into_inner);
+        let Some(process) = processes.get_mut(id) else {
+            return false;
+        };
+        if process.status != ManagedProcessStatus::Running {
+            return false;
+        }
+        process.status = ManagedProcessStatus::Cancelled;
+        process.error = Some("Cancelled by user".to_string());
+        true
+    }
+
+    #[allow(dead_code)]
+    fn remove_finished(&self, id: &str) -> Option<ManagedProcess> {
+        let mut processes = self
+            .processes
+            .lock()
+            .unwrap_or_else(std::sync::PoisonError::into_inner);
+        processes.remove(id)
+    }
+}
+
+static MANAGED_PROCESSES: OnceLock<ProcessRegistry> = OnceLock::new();
+
+fn managed_process_registry() -> &'static ProcessRegistry {
+    MANAGED_PROCESSES.get_or_init(ProcessRegistry::default)
+}
+
+fn current_unix_seconds() -> u64 {
+    std::time::SystemTime::now()
+        .duration_since(std::time::UNIX_EPOCH)
+        .unwrap_or_default()
+        .as_secs()
+}
+
+const COLCON_BUILD_SCRIPT: &str = r#"
+source /opt/ros/jazzy/setup.bash || exit $?
+cd /workspace || exit $?
+run_id="$1"
+case "$run_id" in *[!0-9-]*|"") exit 2 ;; esac
+pid_file="/tmp/robotics-studio-build-$run_id.pid"
+cancel_file="/tmp/robotics-studio-build-$run_id.cancel"
+if [ -e "$cancel_file" ]; then
+  rm -f "$cancel_file"
+  exit 130
+fi
+setsid bash -c 'exec colcon build' &
+build_pid=$!
+printf '%s\n' "$build_pid" > "$pid_file"
+while kill -0 "$build_pid" 2>/dev/null; do
+  if [ -e "$cancel_file" ]; then
+    kill -TERM -- "-$build_pid" 2>/dev/null || true
+    break
+  fi
+  sleep 0.1
+done
+wait "$build_pid"
+status=$?
+rm -f "$pid_file" "$cancel_file"
+exit "$status"
+"#;
+
+const CANCEL_COLCON_BUILD_SCRIPT: &str = r#"
+run_id="$1"
+case "$run_id" in *[!0-9-]*|"") exit 2 ;; esac
+pid_file="/tmp/robotics-studio-build-$run_id.pid"
+cancel_file="/tmp/robotics-studio-build-$run_id.cancel"
+touch "$cancel_file" || exit $?
+if [ -r "$pid_file" ]; then
+  build_pid=$(cat "$pid_file" 2>/dev/null || true)
+  case "$build_pid" in
+    "") ;;
+    *[!0-9]*) exit 2 ;;
+    *) kill -TERM -- "-$build_pid" 2>/dev/null || true ;;
+  esac
+fi
+"#;
 static INITIALIZATION_RUNNING: AtomicBool = AtomicBool::new(false);
 
 struct BuildRunGuard;
@@ -1300,6 +2613,11 @@ struct InitializationGuard;
 
 impl Drop for BuildRunGuard {
     fn drop(&mut self) {
+        if let Some(state) = BUILD_CONTAINER.get() {
+            *state
+                .lock()
+                .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
+        }
         BUILD_RUNNING.store(false, Ordering::SeqCst);
     }
 }
@@ -1317,6 +2635,31 @@ async fn run_colcon_build_streaming(app: AppHandle) -> Result<(), String> {
         return Err("A build is already running".to_string());
     }
     let build_guard = BuildRunGuard;
+    let build_id = format!(
+        "{}-{}",
+        std::process::id(),
+        BUILD_RUN_COUNTER.fetch_add(1, Ordering::Relaxed)
+    );
+    let workspace = Some(get_default_workspace_dir().unwrap_or_else(|_| "".to_string()));
+    managed_process_registry().register(
+        build_id.clone(),
+        "colcon-build".to_string(),
+        vec![
+            "docker".to_string(),
+            "exec".to_string(),
+            container_name.clone(),
+            "bash".to_string(),
+            "-lc".to_string(),
+            "colcon build".to_string(),
+        ],
+        workspace,
+        None,
+    );
+    *BUILD_CONTAINER
+        .get_or_init(|| Mutex::new(None))
+        .lock()
+        .map_err(|_| "Build process state is unavailable".to_string())? =
+        Some((container_name.clone(), build_id.clone()));
 
     tauri::async_runtime::spawn(async move {
         let _build_guard = build_guard;
@@ -1326,7 +2669,9 @@ async fn run_colcon_build_streaming(app: AppHandle) -> Result<(), String> {
                 &container_name,
                 "bash",
                 "-c",
-                "source /opt/ros/jazzy/setup.bash && cd /workspace && colcon build",
+                COLCON_BUILD_SCRIPT,
+                "robotics-studio-build",
+                &build_id,
             ])
             .stdout(Stdio::piped())
             .stderr(Stdio::piped())
@@ -1356,9 +2701,29 @@ async fn run_colcon_build_streaming(app: AppHandle) -> Result<(), String> {
                 if let Some(reader) = stderr_reader {
                     let _ = reader.join();
                 }
+                managed_process_registry().mark_completed(
+                    &build_id,
+                    Some(status.code().unwrap_or(-1)),
+                    if status.success() {
+                        ManagedProcessStatus::Completed
+                    } else {
+                        ManagedProcessStatus::Failed
+                    },
+                    if status.success() {
+                        None
+                    } else {
+                        Some("colcon build exited with a non-zero status".to_string())
+                    },
+                );
                 let _ = app.emit("build-finished", status.success());
             }
             Err(e) => {
+                managed_process_registry().mark_completed(
+                    &build_id,
+                    None,
+                    ManagedProcessStatus::Failed,
+                    Some(e.to_string()),
+                );
                 let _ = app.emit("build-error", e.to_string());
                 let _ = app.emit("build-finished", false);
             }
@@ -1390,6 +2755,110 @@ fn detect_models_in_sdf(sdf_content: &str) -> Vec<String> {
     models
 }
 
+#[tauri::command]
+fn cancel_colcon_build() -> Result<(), String> {
+    if !BUILD_RUNNING.load(Ordering::SeqCst) {
+        return Err("No build is currently running".to_string());
+    }
+    let (container, build_id) = BUILD_CONTAINER
+        .get_or_init(|| Mutex::new(None))
+        .lock()
+        .map_err(|_| "Build process state is unavailable".to_string())?
+        .clone()
+        .ok_or_else(|| "Build process state is unavailable".to_string())?;
+    managed_process_registry().cancel(&build_id);
+    let output = docker_command()
+        .args([
+            "exec",
+            &container,
+            "bash",
+            "-c",
+            CANCEL_COLCON_BUILD_SCRIPT,
+            "robotics-studio-cancel",
+            &build_id,
+        ])
+        .output()
+        .map_err(|error| format!("Could not request build cancellation: {error}"))?;
+    if !output.status.success() {
+        return Err(format!(
+            "Could not request build cancellation: {}",
+            String::from_utf8_lossy(&output.stderr).trim()
+        ));
+    }
+    Ok(())
+}
+
+#[tauri::command]
+fn list_managed_processes() -> Result<Vec<ManagedProcess>, String> {
+    Ok(managed_process_registry().list())
+}
+
+fn parse_gazebo_world_control_service(output: &str) -> Result<Option<String>, String> {
+    let mut services = Vec::new();
+    for service in output.lines().map(str::trim) {
+        let Some(world) = service
+            .strip_prefix("/world/")
+            .and_then(|world| world.strip_suffix("/control"))
+        else {
+            continue;
+        };
+        if world.is_empty()
+            || !world
+                .chars()
+                .all(|character| character.is_ascii_alphanumeric() || "_-".contains(character))
+        {
+            return Err(format!("Invalid Gazebo world control service: {service}"));
+        }
+        services.push(service);
+    }
+    match services.as_slice() {
+        [] => Ok(None),
+        [service] => Ok(Some((*service).to_string())),
+        _ => Err("Multiple Gazebo worlds are active; reset is ambiguous".to_string()),
+    }
+}
+
+fn gazebo_world_control_service(container_name: &str) -> Result<Option<String>, String> {
+    let output = docker_command()
+        .args([
+            "exec",
+            container_name,
+            "bash",
+            "-lc",
+            "source /opt/ros/jazzy/setup.bash && gz service -l",
+        ])
+        .output()
+        .map_err(|error| format!("Could not inspect Gazebo readiness: {error}"))?;
+    if !output.status.success() {
+        return Err(format!(
+            "Could not inspect Gazebo readiness: {}",
+            String::from_utf8_lossy(&output.stderr).trim()
+        ));
+    }
+    parse_gazebo_world_control_service(&String::from_utf8_lossy(&output.stdout))
+}
+
+fn stop_gazebo_processes(container_name: &str) -> Result<(), String> {
+    let output = docker_command()
+        .args([
+            "exec",
+            container_name,
+            "bash",
+            "-c",
+            "pkill -TERM -f '[g]z sim -s' || true; \
+             pkill -TERM -f '[p]arameter_bridge' || true",
+        ])
+        .output()
+        .map_err(|error| format!("Could not stop Gazebo processes: {error}"))?;
+    if !output.status.success() {
+        return Err(format!(
+            "Could not stop Gazebo processes: {}",
+            String::from_utf8_lossy(&output.stderr).trim()
+        ));
+    }
+    Ok(())
+}
+
 #[tauri::command]
 fn start_gazebo_sim(
     world_path: Option<String>,
@@ -1501,11 +2970,26 @@ fn start_gazebo_sim(
         if ensure_docker_available().is_err() {
             return Err(docker_lost_connection_message());
         }
-        return Err(String::from_utf8_lossy(&gz_result.stderr).to_string());
+        return Err(String::from_utf8_lossy(&gz_result.stderr).to_string());
+    }
+
+    let mut world_control_service = None;
+    for _ in 0..20 {
+        if let Some(service) = gazebo_world_control_service(&container_name)? {
+            world_control_service = Some(service);
+            break;
+        }
+        std::thread::sleep(std::time::Duration::from_secs(1));
+    }
+    if world_control_service.is_none() {
+        let cleanup = stop_gazebo_processes(&container_name);
+        return Err(match cleanup {
+            Ok(()) => "Gazebo did not become ready within 20 seconds".to_string(),
+            Err(error) => {
+                format!("Gazebo did not become ready within 20 seconds; cleanup failed: {error}")
+            }
+        });
     }
-
-    // Give Gazebo a moment to actually start before launching the bridge
-    std::thread::sleep(std::time::Duration::from_secs(2));
 
     // Build bridge args dynamically based on models actually found in the
     // world file, rather than hardcoding vehicle_blue/vehicle_green.
@@ -1541,10 +3025,17 @@ fn start_gazebo_sim(
         .map_err(|_| docker_lost_connection_message())?;
 
     if !bridge_result.status.success() {
+        let cleanup = stop_gazebo_processes(&container_name);
         if ensure_docker_available().is_err() {
             return Err(docker_lost_connection_message());
         }
-        return Err(String::from_utf8_lossy(&bridge_result.stderr).to_string());
+        return Err(match cleanup {
+            Ok(()) => String::from_utf8_lossy(&bridge_result.stderr).to_string(),
+            Err(error) => format!(
+                "{}; simulator cleanup failed: {error}",
+                String::from_utf8_lossy(&bridge_result.stderr)
+            ),
+        });
     }
 
     Ok("Gazebo and bridge started".to_string())
@@ -1553,24 +3044,16 @@ fn start_gazebo_sim(
 #[tauri::command]
 fn stop_gazebo_sim() -> Result<String, String> {
     let container_name = find_dev_container()?;
-
-    let result = docker_command()
-        .args([
-            "exec",
-            &container_name,
-            "bash",
-            "-c",
-            "pkill -f 'gz sim' ; pkill -f parameter_bridge",
-        ])
-        .output()
-        .map_err(|_| docker_lost_connection_message())?;
-
-    Ok(String::from_utf8_lossy(&result.stdout).to_string())
+    stop_gazebo_processes(&container_name)?;
+    Ok("Gazebo and ROS-Gazebo bridge stopped".to_string())
 }
 
 #[tauri::command]
 fn reset_gazebo_sim() -> Result<String, String> {
     let container_name = find_dev_container()?;
+    let service = gazebo_world_control_service(&container_name)?.ok_or_else(|| {
+        "Gazebo world is not ready; start the simulation before resetting".to_string()
+    })?;
 
     let result = docker_command()
         .args([
@@ -1578,24 +3061,34 @@ fn reset_gazebo_sim() -> Result<String, String> {
             &container_name,
             "bash",
             "-c",
-            "gz service -s /world/default/control --reqtype gz.msgs.WorldControl \
+            "source /opt/ros/jazzy/setup.bash && \
+             gz service -s \"$1\" --reqtype gz.msgs.WorldControl \
              --reptype gz.msgs.Boolean --timeout 2000 --req 'reset: {all: true}'",
+            "robotics-studio",
+            &service,
         ])
         .output()
         .map_err(|_| docker_lost_connection_message())?;
 
+    if !result.status.success() {
+        return Err(format!(
+            "Could not reset Gazebo world: {}",
+            String::from_utf8_lossy(&result.stderr).trim()
+        ));
+    }
     Ok(String::from_utf8_lossy(&result.stdout).to_string())
 }
 
 use tauri::Manager;
 
-fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<(), String> {
-    copy_dir_recursive_impl(src, dst, 0)
+fn copy_dir_recursive(src: &Path, dst: &Path, access: &FsAccess) -> Result<(), String> {
+    copy_dir_recursive_impl(src, dst, access, 0)
 }
 
 fn copy_dir_recursive_impl(
-    src: &std::path::Path,
-    dst: &std::path::Path,
+    src: &Path,
+    dst: &Path,
+    access: &FsAccess,
     depth: u32,
 ) -> Result<(), String> {
     if depth > 50 {
@@ -1604,7 +3097,7 @@ fn copy_dir_recursive_impl(
             src, dst
         ));
     }
-    std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
+    access.ensure_directory(dst)?;
     for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
         let entry = entry.map_err(|e| e.to_string())?;
         let src_path = entry.path();
@@ -1616,9 +3109,9 @@ fn copy_dir_recursive_impl(
         }
 
         if file_type.is_dir() {
-            copy_dir_recursive_impl(&src_path, &dst_path, depth + 1)?;
-        } else if !dst_path.exists() {
-            std::fs::copy(&src_path, &dst_path).map_err(|e| e.to_string())?;
+            copy_dir_recursive_impl(&src_path, &dst_path, access, depth + 1)?;
+        } else if file_type.is_file() {
+            access.copy_file_if_missing(&src_path, &dst_path)?;
         }
     }
 
@@ -1627,7 +3120,13 @@ fn copy_dir_recursive_impl(
 
 const GRPC_AUTH_CONTAINER_PATH: &str = "/run/robotics-studio/grpc-auth.json";
 const GRPC_AUTH_LIFETIME_SECONDS: u64 = 24 * 60 * 60;
-const AUTH_PROTOCOL_MARKER: &str = "AUTH_PROTOCOL_VERSION = 1";
+const ROS_SERVER_CONTAINER_DIRECTORY: &str = "/opt/robotics-studio";
+const GRPC_CA_CERTIFICATE_PATH: &str = "/run/robotics-studio/grpc-ca.crt";
+const GRPC_CA_PRIVATE_KEY_PATH: &str = "/run/robotics-studio/grpc-ca.key";
+const GRPC_SERVER_CERTIFICATE_PATH: &str = "/run/robotics-studio/grpc-server.crt";
+const GRPC_SERVER_PRIVATE_KEY_PATH: &str = "/run/robotics-studio/grpc-server.key";
+const GRPC_SERVER_CSR_PATH: &str = "/run/robotics-studio/grpc-server.csr";
+const GRPC_SERVER_EXTENSIONS_PATH: &str = "/run/robotics-studio/grpc-server.ext";
 
 #[derive(Serialize, Deserialize)]
 struct GrpcAuthCredentials {
@@ -1770,6 +3269,7 @@ fn install_authenticated_server_files(
     resource_dir: &Path,
     workspace_dir: &Path,
     backup_dir: &Path,
+    access: &FsAccess,
 ) -> Result<(), String> {
     fs::create_dir_all(backup_dir)
         .map_err(|error| format!("Could not prepare server migration backup: {error}"))?;
@@ -1778,24 +3278,23 @@ fn install_authenticated_server_files(
         let target_path = workspace_dir.join(file_name);
         let bundled = fs::read(&bundled_path)
             .map_err(|error| format!("Could not read bundled {file_name}: {error}"))?;
-        let installed = match fs::symlink_metadata(&target_path) {
-            Ok(metadata) if metadata.file_type().is_symlink() => {
-                return Err(format!("Workspace {file_name} must not be a symbolic link"));
-            }
-            Ok(_) => Some(
-                fs::read(&target_path)
-                    .map_err(|error| format!("Could not inspect workspace {file_name}: {error}"))?,
-            ),
-            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
-            Err(error) => return Err(format!("Could not inspect workspace {file_name}: {error}")),
-        };
-        if installed.as_ref().is_some_and(|contents| {
-            String::from_utf8_lossy(contents).contains(AUTH_PROTOCOL_MARKER)
-        }) {
+        let bundled_text = std::str::from_utf8(&bundled)
+            .map_err(|error| format!("Bundled {file_name} is not valid UTF-8: {error}"))?;
+        let target_parent = target_path
+            .parent()
+            .ok_or_else(|| format!("Invalid workspace path for {file_name}"))?;
+        access
+            .ensure_directory(target_parent)
+            .map_err(|error| format!("Could not safely access {file_name} directory: {error}"))?;
+        let installed = access.read_file_if_present(&target_path)?;
+        if installed
+            .as_ref()
+            .is_some_and(|contents| contents == &bundled)
+        {
             continue;
         }
 
-        if installed.is_some() {
+        if let Some(installed) = &installed {
             let timestamp = std::time::SystemTime::now()
                 .duration_since(std::time::UNIX_EPOCH)
                 .map_err(|error| error.to_string())?
@@ -1805,48 +3304,143 @@ fn install_authenticated_server_files(
                 file_name.replace('/', "_"),
                 std::process::id()
             ));
-            fs::copy(&target_path, &backup_path)
+            fs::write(&backup_path, installed)
                 .map_err(|error| format!("Could not preserve existing {file_name}: {error}"))?;
         }
 
-        let timestamp = std::time::SystemTime::now()
-            .duration_since(std::time::UNIX_EPOCH)
-            .map_err(|error| error.to_string())?
-            .as_nanos();
-        let target_parent = target_path
-            .parent()
-            .ok_or_else(|| format!("Invalid workspace path for {file_name}"))?;
-        let target_name = target_path
-            .file_name()
-            .ok_or_else(|| format!("Invalid workspace filename for {file_name}"))?
-            .to_string_lossy();
-        let temporary = target_parent.join(format!(
-            ".{target_name}.auth-{}-{timestamp}.tmp",
-            std::process::id()
+        write_approved_file(access, &target_path, bundled_text)
+            .map_err(|error| format!("Could not install secure {file_name}: {error}"))?;
+    }
+    Ok(())
+}
+
+fn install_ros_server_resources(container_id: &str, resource_dir: &Path) -> Result<(), String> {
+    let prepared = docker_command()
+        .args([
+            "exec",
+            container_id,
+            "mkdir",
+            "-p",
+            ROS_SERVER_CONTAINER_DIRECTORY,
+        ])
+        .output()
+        .map_err(|error| format!("Could not prepare the trusted ROS service: {error}"))?;
+    if !prepared.status.success() {
+        return Err(format!(
+            "Could not prepare the trusted ROS service: {}",
+            String::from_utf8_lossy(&prepared.stderr)
         ));
-        fs::write(&temporary, bundled)
-            .map_err(|error| format!("Could not stage secure {file_name}: {error}"))?;
-        #[cfg(windows)]
-        if target_path.exists() {
-            fs::remove_file(&target_path)
-                .map_err(|error| format!("Could not update secure {file_name}: {error}"))?;
-        }
-        if let Err(error) = fs::rename(&temporary, &target_path) {
-            let _ = fs::remove_file(&temporary);
-            return Err(format!("Could not install secure {file_name}: {error}"));
+    }
+
+    for file_name in ["server.py", "server_core.py"] {
+        let source = resource_dir.join(file_name);
+        let destination = format!("{container_id}:{ROS_SERVER_CONTAINER_DIRECTORY}/{file_name}");
+        let copied = docker_command()
+            .args(["cp", &source.to_string_lossy(), &destination])
+            .output()
+            .map_err(|error| format!("Could not install trusted {file_name}: {error}"))?;
+        if !copied.status.success() {
+            return Err(format!(
+                "Could not install trusted {file_name}: {}",
+                String::from_utf8_lossy(&copied.stderr)
+            ));
         }
     }
+    let proto_source = resource_dir.join("proto/ros_engine.proto");
+    let proto_destination = format!("{container_id}:{ROS_SERVER_CONTAINER_DIRECTORY}/proto");
+    let proto_directory = docker_command()
+        .args([
+            "exec",
+            container_id,
+            "mkdir",
+            "-p",
+            &format!("{ROS_SERVER_CONTAINER_DIRECTORY}/proto"),
+        ])
+        .output()
+        .map_err(|error| format!("Could not prepare the trusted ROS protocol: {error}"))?;
+    if !proto_directory.status.success() {
+        return Err(format!(
+            "Could not prepare the trusted ROS protocol: {}",
+            String::from_utf8_lossy(&proto_directory.stderr)
+        ));
+    }
+    let copied = docker_command()
+        .args(["cp", &proto_source.to_string_lossy(), &proto_destination])
+        .output()
+        .map_err(|error| format!("Could not install the trusted ROS protocol: {error}"))?;
+    if !copied.status.success() {
+        return Err(format!(
+            "Could not install the trusted ROS protocol: {}",
+            String::from_utf8_lossy(&copied.stderr)
+        ));
+    }
+
+    let certificate_command = format!(
+        "set -eu; \
+         mkdir -p /run/robotics-studio; chmod 700 /run/robotics-studio; \
+         if [ ! -s '{GRPC_CA_CERTIFICATE_PATH}' ] || \
+            [ ! -s '{GRPC_CA_PRIVATE_KEY_PATH}' ] || \
+            [ ! -s '{GRPC_SERVER_CERTIFICATE_PATH}' ] || \
+            [ ! -s '{GRPC_SERVER_PRIVATE_KEY_PATH}' ]; then \
+           openssl req -x509 -newkey rsa:2048 -nodes -days 3650 \
+             -keyout '{GRPC_CA_PRIVATE_KEY_PATH}' \
+             -out '{GRPC_CA_CERTIFICATE_PATH}' \
+             -subj '/CN=Robotics Studio local gRPC CA' \
+             -addext 'basicConstraints=critical,CA:TRUE' \
+             -addext 'keyUsage=critical,keyCertSign,cRLSign'; \
+           openssl req -new -newkey rsa:2048 -nodes \
+             -keyout '{GRPC_SERVER_PRIVATE_KEY_PATH}' \
+             -out '{GRPC_SERVER_CSR_PATH}' \
+             -subj '/CN=127.0.0.1'; \
+           printf '%s\\n' \
+             'basicConstraints=critical,CA:FALSE' \
+             'keyUsage=critical,digitalSignature,keyEncipherment' \
+             'extendedKeyUsage=serverAuth' \
+             'subjectAltName=IP:127.0.0.1,DNS:localhost' \
+             > '{GRPC_SERVER_EXTENSIONS_PATH}'; \
+           openssl x509 -req -in '{GRPC_SERVER_CSR_PATH}' \
+             -CA '{GRPC_CA_CERTIFICATE_PATH}' \
+             -CAkey '{GRPC_CA_PRIVATE_KEY_PATH}' -CAcreateserial \
+             -out '{GRPC_SERVER_CERTIFICATE_PATH}' -days 3650 -sha256 \
+             -extfile '{GRPC_SERVER_EXTENSIONS_PATH}'; \
+         fi; \
+         chmod 600 '{GRPC_CA_PRIVATE_KEY_PATH}' '{GRPC_SERVER_PRIVATE_KEY_PATH}'; \
+         chmod 644 '{GRPC_CA_CERTIFICATE_PATH}' '{GRPC_SERVER_CERTIFICATE_PATH}'; \
+         cat '{GRPC_CA_CERTIFICATE_PATH}'"
+    );
+    let certificate = docker_command()
+        .args(["exec", container_id, "bash", "-c", &certificate_command])
+        .output()
+        .map_err(|error| format!("Could not provision ROS TLS credentials: {error}"))?;
+    if !certificate.status.success() {
+        return Err(format!(
+            "Could not provision ROS TLS credentials: {}",
+            String::from_utf8_lossy(&certificate.stderr)
+        ));
+    }
+    set_ros_server_tls_certificate(certificate.stdout)?;
     Ok(())
 }
 
-fn start_ros_engine_server(container_id: &str, auth_file: &Path) -> Result<(), String> {
+fn start_ros_engine_server(
+    container_id: &str,
+    auth_file: &Path,
+    resource_dir: &Path,
+) -> Result<(), String> {
+    install_ros_server_resources(container_id, resource_dir)?;
     let generate = docker_command()
         .args([
             "exec",
             container_id,
             "bash",
             "-c",
-            "source /opt/ros/jazzy/setup.bash && cd /workspace && mkdir -p generated && python3 -m grpc_tools.protoc -I proto --python_out=generated --grpc_python_out=generated proto/ros_engine.proto",
+            "source /opt/ros/jazzy/setup.bash && \
+             mkdir -p /opt/robotics-studio/generated && \
+             python3 -m grpc_tools.protoc \
+               -I /opt/robotics-studio/proto \
+               --python_out=/opt/robotics-studio/generated \
+               --grpc_python_out=/opt/robotics-studio/generated \
+               /opt/robotics-studio/proto/ros_engine.proto",
         ])
         .output()
         .map_err(|error| format!("Failed to generate gRPC modules: {error}"))?;
@@ -1878,7 +3472,11 @@ fn start_ros_engine_server(container_id: &str, auth_file: &Path) -> Result<(), S
             container_id,
             "bash",
             "-c",
-            "source /opt/ros/jazzy/setup.bash && cd /workspace && ROBOTICS_STUDIO_AUTH_FILE=/run/robotics-studio/grpc-auth.json exec python3 server.py > /tmp/server.log 2>&1",
+            "source /opt/ros/jazzy/setup.bash && \
+             cd /opt/robotics-studio && \
+             ROBOTICS_STUDIO_AUTH_FILE=/run/robotics-studio/grpc-auth.json \
+             ROBOTICS_STUDIO_GENERATED_PATH=/opt/robotics-studio/generated \
+             exec python3 /opt/robotics-studio/server.py > /tmp/server.log 2>&1",
         ])
         .output()
         .map_err(|error| format!("Failed to start ROS engine server: {error}"))?;
@@ -1904,11 +3502,8 @@ fn authenticated_request<T>(message: T, token: &str) -> Result<tonic::Request<T>
 
 async fn wait_for_ros_engine(container_id: &str, token: &str) -> Result<(), String> {
     for _ in 0..30 {
-        if let Ok(Ok(mut client)) = tokio::time::timeout(
-            std::time::Duration::from_secs(2),
-            RosEngineClient::connect("http://127.0.0.1:50051"),
-        )
-        .await
+        if let Ok(Ok(mut client)) =
+            tokio::time::timeout(std::time::Duration::from_secs(2), connect_ros_engine()).await
         {
             if tokio::time::timeout(
                 std::time::Duration::from_secs(2),
@@ -1970,13 +3565,13 @@ async fn initialize_ros_environment(
 
     // Install missing resources and preserve legacy server files before updating
     // the authentication-critical ROS service scripts.
-    copy_dir_recursive(&resource_path, workspace_path)?;
+    copy_dir_recursive(&resource_path, workspace_path, &access)?;
     let backup_dir = app
         .path()
         .app_config_dir()
         .map_err(|error| error.to_string())?
         .join("server-migration-backups");
-    install_authenticated_server_files(&resource_path, workspace_path, &backup_dir)?;
+    install_authenticated_server_files(&resource_path, workspace_path, &backup_dir, &access)?;
 
     let filter = format!("label=devcontainer.local_folder={}", workspace_dir);
 
@@ -2021,6 +3616,7 @@ async fn initialize_ros_environment(
 
         let app = app.clone();
         let workspace_dir = workspace_dir.clone();
+        let resource_path = resource_path.clone();
         tauri::async_runtime::spawn(async move {
             let _initialization_guard = initialization_guard;
             let _ = app.emit(
@@ -2030,7 +3626,8 @@ async fn initialize_ros_environment(
                     text: "Existing environment found. Checking ROS engine...".to_string(),
                 },
             );
-            let result = match start_ros_engine_server(&container_name, &auth_file) {
+            let result = match start_ros_engine_server(&container_name, &auth_file, &resource_path)
+            {
                 Ok(()) => wait_for_ros_engine(&container_name, &auth_token).await,
                 Err(error) => Err(error),
             };
@@ -2153,7 +3750,7 @@ async fn initialize_ros_environment(
         };
         let _ = app.emit("init-progress", event);
 
-        match start_ros_engine_server(&container_id, &auth_file) {
+        match start_ros_engine_server(&container_id, &auth_file, &resource_path) {
             Ok(()) => match wait_for_ros_engine(&container_id, &auth_token).await {
                 Ok(()) => {
                     let _ = app.emit("init-finished", workspace_dir.clone());
@@ -2451,7 +4048,7 @@ fn get_total_ram_gb() -> Result<u64, String> {
 #[cfg(target_os = "linux")]
 fn get_total_ram_gb() -> Result<u64, String> {
     let output = std::process::Command::new("grep")
-        .args(&["MemTotal", "/proc/meminfo"])
+        .args(["MemTotal", "/proc/meminfo"])
         .output()
         .map_err(|e| e.to_string())?;
     let output_str = String::from_utf8_lossy(&output.stdout);
@@ -3014,7 +4611,7 @@ fn create_project_from_template(
     }
 
     std::fs::create_dir(&dest).map_err(|e| format!("Could not create project directory: {e}"))?;
-    if let Err(error) = copy_dir_recursive(&template_path, &dest) {
+    if let Err(error) = copy_dir_recursive(&template_path, &dest, &access) {
         if let Err(cleanup_error) = std::fs::remove_dir_all(&dest) {
             return Err(format!(
                 "Project creation failed: {error}. Incomplete project could not be removed: {cleanup_error}"
@@ -3136,6 +4733,25 @@ mod recent_project_tests {
 mod validation_tests {
     use super::*;
 
+    #[test]
+    fn resolves_the_active_gazebo_world_control_service() {
+        assert_eq!(
+            parse_gazebo_world_control_service(
+                "/world/diff_drive/control\n/world/diff_drive/scene/info\n"
+            )
+            .unwrap(),
+            Some("/world/diff_drive/control".to_string())
+        );
+        assert_eq!(
+            parse_gazebo_world_control_service("/world/diff_drive/scene/info\n").unwrap(),
+            None
+        );
+        assert!(
+            parse_gazebo_world_control_service("/world/one/control\n/world/two/control\n").is_err()
+        );
+        assert!(parse_gazebo_world_control_service("/world/../control\n").is_err());
+    }
+
     #[test]
     fn validates_absolute_ros_topics() {
         assert!(is_valid_ros_topic("/robot/cmd_vel"));
@@ -3201,6 +4817,50 @@ mod filesystem_security_tests {
         }
     }
 
+    #[test]
+    fn workspace_service_marker_does_not_authenticate_modified_server_files() {
+        let directory = TempDirectory::new();
+        let resources = directory.0.join("resources");
+        let workspace = directory.0.join("workspace");
+        let backup = directory.0.join("backups");
+        fs::create_dir_all(resources.join("proto")).unwrap();
+        fs::create_dir_all(workspace.join("proto")).unwrap();
+        for (name, trusted) in [
+            ("server.py", b"trusted service".as_slice()),
+            ("server_core.py", b"trusted core".as_slice()),
+            ("proto/ros_engine.proto", b"trusted protocol".as_slice()),
+        ] {
+            fs::write(resources.join(name), trusted).unwrap();
+            fs::write(
+                workspace.join(name),
+                b"AUTH_PROTOCOL_VERSION = 1\nmalicious replacement",
+            )
+            .unwrap();
+        }
+        let access = FsAccess::default();
+        access.approve_directory(&workspace).unwrap();
+
+        install_authenticated_server_files(&resources, &workspace, &backup, &access).unwrap();
+
+        for (name, trusted) in [
+            ("server.py", b"trusted service".as_slice()),
+            ("server_core.py", b"trusted core".as_slice()),
+            ("proto/ros_engine.proto", b"trusted protocol".as_slice()),
+        ] {
+            assert_eq!(fs::read(workspace.join(name)).unwrap(), trusted);
+        }
+        let preserved = fs::read_dir(&backup)
+            .unwrap()
+            .collect::<Result<Vec<_>, _>>()
+            .unwrap();
+        assert_eq!(preserved.len(), 3);
+        assert!(preserved.iter().all(|entry| {
+            fs::read(entry.path()).is_ok_and(|contents| {
+                contents == b"AUTH_PROTOCOL_VERSION = 1\nmalicious replacement"
+            })
+        }));
+    }
+
     #[test]
     fn approved_root_allows_normal_reads_and_writes() {
         let directory = TempDirectory::new();
@@ -3233,6 +4893,7 @@ mod filesystem_security_tests {
 
         let directory = TempDirectory::new();
         let outside = TempDirectory::new();
+        fs::write(outside.0.join("secret.txt"), "outside").unwrap();
         let access = FsAccess::default();
         access.approve_directory(&directory.0).unwrap();
         symlink(&outside.0, directory.0.join("escape")).unwrap();
@@ -3240,18 +4901,326 @@ mod filesystem_security_tests {
         assert!(access
             .resolve_existing(&directory.0.join("escape"))
             .is_err());
+        assert!(read_approved_file(&access, &directory.0.join("escape/secret.txt")).is_err());
+        assert!(
+            write_approved_file(&access, &directory.0.join("escape/secret.txt"), "changed")
+                .is_err()
+        );
+        assert_eq!(
+            fs::read_to_string(outside.0.join("secret.txt")).unwrap(),
+            "outside"
+        );
+    }
+
+    #[cfg(unix)]
+    #[test]
+    fn recursive_copy_rejects_symlinked_destination_directories() {
+        use std::os::unix::fs::symlink;
+
+        let directory = TempDirectory::new();
+        let source = directory.0.join("source");
+        let workspace = directory.0.join("workspace");
+        let outside = directory.0.join("outside");
+        fs::create_dir_all(source.join("proto")).unwrap();
+        fs::create_dir(&workspace).unwrap();
+        fs::create_dir(&outside).unwrap();
+        fs::write(source.join("proto/ros_engine.proto"), "bundled").unwrap();
+        symlink(&outside, workspace.join("proto")).unwrap();
+        let access = FsAccess::default();
+        access.approve_directory(&workspace).unwrap();
+
+        assert!(copy_dir_recursive(&source, &workspace, &access).is_err());
+        assert!(!outside.join("ros_engine.proto").exists());
+    }
+
+    #[cfg(unix)]
+    #[test]
+    fn authenticated_service_install_rejects_symlinked_proto_directory() {
+        use std::os::unix::fs::symlink;
+
+        let directory = TempDirectory::new();
+        let resources = directory.0.join("resources");
+        let workspace = directory.0.join("workspace");
+        let outside = directory.0.join("outside");
+        fs::create_dir_all(resources.join("proto")).unwrap();
+        fs::create_dir(&workspace).unwrap();
+        fs::create_dir(&outside).unwrap();
+        fs::write(resources.join("server.py"), "service").unwrap();
+        fs::write(resources.join("server_core.py"), "core").unwrap();
+        fs::write(resources.join("proto/ros_engine.proto"), "protocol").unwrap();
+        symlink(&outside, workspace.join("proto")).unwrap();
+        let access = FsAccess::default();
+        access.approve_directory(&workspace).unwrap();
+
+        assert!(install_authenticated_server_files(
+            &resources,
+            &workspace,
+            &directory.0.join("backups"),
+            &access
+        )
+        .is_err());
+        assert!(!outside.join("ros_engine.proto").exists());
+    }
+
+    #[cfg(unix)]
+    #[test]
+    fn rejects_replaced_approved_root_after_selection() {
+        let directory = TempDirectory::new();
+        let moved = directory.0.with_extension("moved");
+        let access = FsAccess::default();
+        access.approve_directory(&directory.0).unwrap();
+        fs::rename(&directory.0, &moved).unwrap();
+        fs::create_dir(&directory.0).unwrap();
+        fs::write(directory.0.join("sentinel.txt"), "replacement").unwrap();
+
+        assert!(read_approved_file(&access, &directory.0.join("sentinel.txt")).is_err());
+        assert!(
+            write_approved_file(&access, &directory.0.join("sentinel.txt"), "must not write")
+                .is_err()
+        );
+        assert_eq!(
+            fs::read_to_string(directory.0.join("sentinel.txt")).unwrap(),
+            "replacement"
+        );
+        fs::remove_dir_all(moved).unwrap();
+    }
+
+    #[cfg(unix)]
+    #[test]
+    fn concurrent_parent_symlink_swaps_never_escape_the_approved_root() {
+        use std::os::unix::fs::symlink;
+        use std::sync::atomic::{AtomicBool, Ordering};
+
+        let directory = TempDirectory::new();
+        let outside = TempDirectory::new();
+        let nested = directory.0.join("nested");
+        let held = directory.0.join("held");
+        fs::create_dir(&nested).unwrap();
+        fs::write(nested.join("value.txt"), "inside").unwrap();
+        fs::write(outside.0.join("value.txt"), "outside").unwrap();
+        let access = Arc::new(FsAccess::default());
+        access.approve_directory(&directory.0).unwrap();
+        let running = Arc::new(AtomicBool::new(true));
+        let thread_running = Arc::clone(&running);
+        let thread_root = directory.0.clone();
+        let thread_outside = outside.0.clone();
+        let attacker = std::thread::spawn(move || {
+            let nested = thread_root.join("nested");
+            let held = thread_root.join("held");
+            while thread_running.load(Ordering::Relaxed) {
+                if fs::rename(&nested, &held).is_ok() {
+                    if symlink(&thread_outside, &nested).is_ok() {
+                        let _ = fs::remove_file(&nested);
+                    }
+                    let _ = fs::rename(&held, &nested);
+                }
+            }
+            if fs::symlink_metadata(&nested).is_ok_and(|metadata| metadata.file_type().is_symlink())
+            {
+                let _ = fs::remove_file(&nested);
+            }
+            if held.exists() && !nested.exists() {
+                let _ = fs::rename(held, nested);
+            }
+        });
+
+        for _ in 0..500 {
+            if let Ok(contents) = read_approved_file(&access, &nested.join("value.txt")) {
+                assert_eq!(contents, "inside");
+            }
+            let _ = write_approved_file(&access, &nested.join("value.txt"), "inside");
+        }
+
+        running.store(false, Ordering::Relaxed);
+        attacker.join().unwrap();
+        assert_eq!(
+            fs::read_to_string(outside.0.join("value.txt")).unwrap(),
+            "outside"
+        );
+        assert!(!held.exists());
+    }
+
+    #[cfg(unix)]
+    #[test]
+    fn rejects_symlink_file_replacement_for_atomic_writes() {
+        use std::os::unix::fs::symlink;
+
+        let directory = TempDirectory::new();
+        let outside = TempDirectory::new();
+        fs::write(outside.0.join("secret.txt"), "outside").unwrap();
+        let access = FsAccess::default();
+        access.approve_directory(&directory.0).unwrap();
+        symlink(outside.0.join("secret.txt"), directory.0.join("link.txt")).unwrap();
+
+        assert!(write_approved_file(&access, &directory.0.join("link.txt"), "changed").is_err());
+        assert_eq!(
+            fs::read_to_string(outside.0.join("secret.txt")).unwrap(),
+            "outside"
+        );
+    }
+
+    #[test]
+    fn active_workspace_context_switches_between_approved_roots() {
+        let first = TempDirectory::new();
+        let second = TempDirectory::new();
+        let previous = ACTIVE_ROS_WORKSPACE
+            .get_or_init(|| Mutex::new(None))
+            .lock()
+            .unwrap()
+            .clone();
+        let first = first.0.canonicalize().unwrap();
+        let second = second.0.canonicalize().unwrap();
+
+        *ACTIVE_ROS_WORKSPACE
+            .get_or_init(|| Mutex::new(None))
+            .lock()
+            .unwrap() = Some(first.clone());
+        assert_eq!(
+            get_default_workspace_dir().unwrap(),
+            first.to_string_lossy()
+        );
+        *ACTIVE_ROS_WORKSPACE
+            .get_or_init(|| Mutex::new(None))
+            .lock()
+            .unwrap() = Some(second.clone());
+        assert_eq!(
+            get_default_workspace_dir().unwrap(),
+            second.to_string_lossy()
+        );
+
+        *ACTIVE_ROS_WORKSPACE
+            .get_or_init(|| Mutex::new(None))
+            .lock()
+            .unwrap() = previous;
+    }
+
+    #[test]
+    fn ros_event_recovery_cursors_are_isolated_by_workspace() {
+        let first = format!("workspace-a-{}", std::process::id());
+        let second = format!("workspace-b-{}", std::process::id());
+
+        update_ros_event_cursor(&first, 12, false).unwrap();
+        update_ros_event_cursor(&second, 3, false).unwrap();
+        assert_eq!(ros_event_cursor(&first).unwrap(), 12);
+        assert_eq!(ros_event_cursor(&second).unwrap(), 3);
+
+        update_ros_event_cursor(&second, 1, true).unwrap();
+        assert_eq!(ros_event_cursor(&second).unwrap(), 1);
+        assert_eq!(ros_event_cursor(&first).unwrap(), 12);
+    }
+
+    #[cfg(unix)]
+    #[test]
+    fn colcon_cancellation_script_terminates_only_its_process_group() {
+        use std::os::unix::fs::PermissionsExt;
+        use std::process::Command;
+        use std::sync::mpsc;
+
+        let directory = TempDirectory::new();
+        let ready_file = directory.0.join("colcon-ready");
+        let fake_colcon = directory.0.join("colcon");
+        fs::write(
+            &fake_colcon,
+            "#!/bin/sh\n: > \"$COLCON_TEST_READY_FILE\"\nexec sleep 30\n",
+        )
+        .unwrap();
+        let fake_setsid = directory.0.join("setsid");
+        fs::write(
+            &fake_setsid,
+            "#!/usr/bin/env python3\nimport os, sys\nos.setsid()\nos.execvp(sys.argv[1], sys.argv[1:])\n",
+        )
+        .unwrap();
+        for executable in [&fake_colcon, &fake_setsid] {
+            let mut permissions = fs::metadata(executable).unwrap().permissions();
+            permissions.set_mode(0o700);
+            fs::set_permissions(executable, permissions).unwrap();
+        }
+        let run_id = format!(
+            "{}-{}",
+            std::process::id(),
+            std::time::SystemTime::now()
+                .duration_since(std::time::UNIX_EPOCH)
+                .unwrap()
+                .as_nanos()
+        );
+        let pid_file = PathBuf::from(format!("/tmp/robotics-studio-build-{run_id}.pid"));
+        let cancel_file = PathBuf::from(format!("/tmp/robotics-studio-build-{run_id}.cancel"));
+        let script = COLCON_BUILD_SCRIPT
+            .replace("source /opt/ros/jazzy/setup.bash || exit $?", ":")
+            .replace(
+                "cd /workspace || exit $?",
+                &format!("cd '{}' || exit $?", directory.0.display()),
+            );
+        let path = format!(
+            "{}:{}",
+            directory.0.display(),
+            std::env::var("PATH").unwrap_or_default()
+        );
+        let mut child = Command::new("bash")
+            .args(["-c", &script, "test-colcon-build", &run_id])
+            .env("PATH", path)
+            .env("COLCON_TEST_READY_FILE", &ready_file)
+            .spawn()
+            .unwrap();
+
+        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
+        while !ready_file.exists() && std::time::Instant::now() < deadline {
+            std::thread::sleep(std::time::Duration::from_millis(10));
+        }
+        assert!(
+            pid_file.exists() && ready_file.exists(),
+            "fake colcon process did not start"
+        );
+        let build_pid = fs::read_to_string(&pid_file).unwrap();
+        let build_is_alive = Command::new("bash")
+            .args([
+                "-c",
+                "kill -0 \"$1\"",
+                "test-build-status",
+                build_pid.trim(),
+            ])
+            .status()
+            .unwrap();
+        assert!(
+            build_is_alive.success(),
+            "fake colcon process did not start"
+        );
+        let cancellation = Command::new("bash")
+            .args([
+                "-c",
+                CANCEL_COLCON_BUILD_SCRIPT,
+                "test-colcon-cancel",
+                &run_id,
+            ])
+            .status()
+            .unwrap();
+        assert!(cancellation.success());
+
+        let (sender, receiver) = mpsc::channel();
+        std::thread::spawn(move || {
+            let _ = sender.send(child.wait());
+        });
+        let status = receiver
+            .recv_timeout(std::time::Duration::from_secs(5))
+            .expect("build did not exit after cancellation")
+            .unwrap();
+        assert!(!status.success());
+        assert!(!pid_file.exists());
+        assert!(!cancel_file.exists());
     }
 
     #[test]
     fn imports_worlds_to_unique_safe_paths_without_overwriting() {
         let directory = TempDirectory::new();
+        let access = FsAccess::default();
         let source = Path::new("../../Dockerfile.sdf");
         let sdf = "<sdf version=\"1.9\"><world name=\"test\"/></sdf>";
-        let first = import_world_contents(&directory.0, source, sdf).unwrap();
-        let second = import_world_contents(&directory.0, source, sdf).unwrap();
+        let workspace = access.approve_directory(&directory.0).unwrap();
+        let first = import_world_contents(&access, &workspace, source, sdf).unwrap();
+        let second = import_world_contents(&access, &workspace, source, sdf).unwrap();
 
         assert_ne!(first, second);
-        assert!(first.starts_with(directory.0.canonicalize().unwrap().join("worlds")));
+        assert!(first.starts_with(workspace.join("worlds")));
         assert!(first
             .file_name()
             .unwrap()
@@ -3264,12 +5233,19 @@ mod filesystem_security_tests {
     #[test]
     fn rejects_invalid_world_document_and_extension() {
         let directory = TempDirectory::new();
+        let access = FsAccess::default();
+        let workspace = access.approve_directory(&directory.0).unwrap();
         assert!(
-            import_world_contents(&directory.0, Path::new("world.txt"), "<sdf></sdf>").is_err()
-        );
-        assert!(
-            import_world_contents(&directory.0, Path::new("world.sdf"), "<world></world>").is_err()
+            import_world_contents(&access, &workspace, Path::new("world.txt"), "<sdf></sdf>")
+                .is_err()
         );
+        assert!(import_world_contents(
+            &access,
+            &workspace,
+            Path::new("world.sdf"),
+            "<world></world>"
+        )
+        .is_err());
     }
 
     #[test]
@@ -3309,14 +5285,97 @@ mod filesystem_security_tests {
         let directory = TempDirectory::new();
         let outside = TempDirectory::new();
         symlink(&outside.0, directory.0.join("worlds")).unwrap();
+        let access = FsAccess::default();
+        let workspace = access.approve_directory(&directory.0).unwrap();
 
         assert!(
-            import_world_contents(&directory.0, Path::new("world.sdf"), "<sdf></sdf>").is_err()
+            import_world_contents(&access, &workspace, Path::new("world.sdf"), "<sdf></sdf>")
+                .is_err()
         );
         assert_eq!(fs::read_dir(&outside.0).unwrap().count(), 0);
     }
 }
 
+#[cfg(test)]
+mod process_registry_tests {
+    use super::*;
+
+    #[test]
+    fn process_registry_tracks_completion_and_cancellation() {
+        let registry = ProcessRegistry::default();
+        let process = registry.register(
+            "test-proc".to_string(),
+            "build".to_string(),
+            vec![
+                "bash".to_string(),
+                "-lc".to_string(),
+                "sleep 30".to_string(),
+            ],
+            Some("/tmp/workspace".to_string()),
+            None,
+        );
+        assert_eq!(process.status, ManagedProcessStatus::Running);
+        assert!(registry.cancel("test-proc"));
+        assert_eq!(
+            registry.get("test-proc").unwrap().status,
+            ManagedProcessStatus::Cancelled
+        );
+        registry.mark_completed(
+            "test-proc",
+            Some(130),
+            ManagedProcessStatus::Cancelled,
+            Some("Cancelled by user".to_string()),
+        );
+        assert_eq!(registry.get("test-proc").unwrap().exit_code, Some(130));
+    }
+
+    #[test]
+    fn process_registry_rejects_unknown_or_finished_process_ids() {
+        let registry = ProcessRegistry::default();
+        assert!(registry.get("missing").is_none());
+        assert!(!registry.cancel("missing"));
+
+        let process = registry.register(
+            "done".to_string(),
+            "sim".to_string(),
+            vec!["true".to_string()],
+            None,
+            None,
+        );
+        assert_eq!(process.status, ManagedProcessStatus::Running);
+        registry.mark_completed("done", Some(0), ManagedProcessStatus::Completed, None);
+        assert_eq!(
+            registry.get("done").unwrap().status,
+            ManagedProcessStatus::Completed
+        );
+        assert!(!registry.cancel("done"));
+    }
+}
+
+#[cfg(test)]
+mod ros_docker_integration_tests {
+    use super::*;
+
+    #[tokio::test]
+    #[ignore = "requires a Docker ROS service listening on localhost:50051"]
+    async fn pinned_tls_client_authenticates_to_docker_ros_service() {
+        let certificate_path = std::env::var_os("ROBOTICS_STUDIO_TEST_TLS_CERTIFICATE")
+            .expect("ROBOTICS_STUDIO_TEST_TLS_CERTIFICATE must point to the pinned certificate");
+        let certificate =
+            fs::read(certificate_path).expect("could not read pinned TLS certificate");
+        let token = std::env::var("ROBOTICS_STUDIO_TEST_AUTH_TOKEN")
+            .expect("ROBOTICS_STUDIO_TEST_AUTH_TOKEN must contain the test token");
+        set_ros_server_tls_certificate(certificate).unwrap();
+        let mut client = connect_ros_engine().await.unwrap();
+        let response = client
+            .get_safety_state(authenticated_request(Empty {}, &token).unwrap())
+            .await
+            .unwrap()
+            .into_inner();
+        assert_eq!(response.state, 0);
+    }
+}
+
 #[cfg(test)]
 mod stream_line_buffer_tests {
     use super::*;
diff --git a/src/App.tsx b/src/App.tsx
index 0abef8f..2813fb2 100644
--- a/src/App.tsx
+++ b/src/App.tsx
@@ -50,6 +50,17 @@ interface RecentProject {
   last_opened: string;
 }
 
+interface RosPackageInfo {
+  name: string;
+  path: string;
+  package_type: string;
+  version: string | null;
+  dependencies: string[];
+  executables: string[];
+  launch_files: string[];
+  config_files: string[];
+}
+
 interface LogEvent {
   kind: "append" | "replace";
   text: string;
@@ -206,6 +217,8 @@ function FileTree({
 }
 
 function App() {
+  const [workspaceError, setWorkspaceError] = useState<string | null>(null);
+  const [buildError, setBuildError] = useState<string | null>(null);
   // --- File browser state ---
   const [entries, setEntries] = useState<FileEntry[]>([]);
   const [currentPath, setCurrentPath] = useState<string | null>(null);
@@ -266,7 +279,35 @@ function App() {
   // --- ROS event log ---
   const [rosEvents, setRosEvents] = useState<string[]>([]);
   const [rosConnected, setRosConnected] = useState(false);
+  const [rosPackages, setRosPackages] = useState<RosPackageInfo[]>([]);
+  const [rosPackagesLoading, setRosPackagesLoading] = useState(false);
+  const [rosPackagesError, setRosPackagesError] = useState<string | null>(null);
   const [initializing, setInitializing] = useState(false);
+  const [safetyState, setSafetyState] = useState<
+    "READY" | "ACTIVE" | "RESET_PENDING" | "FAULT" | "DISCONNECTED"
+  >("DISCONNECTED");
+  const [safetyError, setSafetyError] = useState<string | null>(null);
+  const [safetyPending, setSafetyPending] = useState(false);
+  const [eventHealth, setEventHealth] = useState<{
+    queueCapacity: number;
+    subscriberCount: number;
+    lastSequence: number;
+    droppedTelemetry: number;
+    criticalOverflows: number;
+    maxBacklog: number;
+    persistenceHealthy: boolean;
+    persistenceError: string;
+    oldestAvailableSequence: number;
+    retainedCriticalEvents: number;
+  } | null>(null);
+  const [eventRecovery, setEventRecovery] = useState<{
+    gapDetected: boolean;
+    oldestAvailableSequence: number;
+    latestSequence: number;
+    snapshotIncluded: boolean;
+    retainedCriticalEvents: number;
+  } | null>(null);
+  const [robotCommandError, setRobotCommandError] = useState<string | null>(null);
   const rosWorkspaceReady = useRef(false);
   const authRecoveryStarted = useRef(false);
   const [setupOutput, setSetupOutput] = useState<string[]>([]); // Environment setup logs
@@ -276,9 +317,14 @@ function App() {
   const [buildOutput, setBuildOutput] = useState<string[]>([]);
   const [building, setBuilding] = useState(false);
   const diagnostics = [
+    ...(workspaceError ? [workspaceError] : []),
+    ...(buildError ? [buildError] : []),
     ...setupOutput.filter((line) => line.startsWith("❌")),
     ...buildOutput.filter((line) => /error|failed/i.test(line)),
     ...rosEvents.filter((line) => /NodeCrashed|error/i.test(line)),
+    ...(eventHealth && !eventHealth.persistenceHealthy
+      ? [`Critical ROS event persistence failed: ${eventHealth.persistenceError}`]
+      : []),
   ];
 
   // --- AI chat ---
@@ -511,15 +557,57 @@ function App() {
   };
 
   // ---------- ROS workspace selection ----------
+  const refreshRosPackages = useCallback(async () => {
+    if (!rosWorkspacePath) {
+      setRosPackages([]);
+      setRosPackagesError(null);
+      return;
+    }
+    setRosPackagesLoading(true);
+    setRosPackagesError(null);
+    try {
+      const packages = await invoke<RosPackageInfo[]>("list_ros_packages");
+      setRosPackages(packages);
+    } catch (error) {
+      setRosPackages([]);
+      setRosPackagesError(String(error));
+    } finally {
+      setRosPackagesLoading(false);
+    }
+  }, [rosWorkspacePath]);
+
+  useEffect(() => {
+    void refreshRosPackages();
+  }, [refreshRosPackages]);
+
   const pickRosWorkspace = async () => {
-    const selected = await invoke<string | null>("pick_directory");
-    if (selected) {
-      setRosWorkspacePath(selected);
+    try {
+      const selected = await invoke<string | null>("pick_directory");
+      if (selected) {
+        const activeWorkspace = await invoke<string>("set_ros_workspace", {
+          workspaceRoot: selected,
+        });
+        setRosWorkspacePath(activeWorkspace);
+        rosWorkspaceReady.current = false;
+        setRosConnected(false);
+        setSafetyState("DISCONNECTED");
+        setSafetyError(null);
+        setNodeStatus({});
+        setAvailableRobots([]);
+        setWorldPath(null);
+        setEventHealth(null);
+        setEventRecovery(null);
+        setWorkspaceError(null);
+        await initializeRosEnvironment();
+      }
+    } catch (error) {
+      setWorkspaceError(`Could not switch ROS workspace: ${String(error)}`);
     }
   };
 
   // ---------- Build actions ----------
   const triggerBuild = async () => {
+    setBuildError(null);
     setBuildOutput([]);
     setBuilding(true);
     setBottomCollapsed(false);
@@ -532,6 +620,15 @@ function App() {
     }
   };
 
+  const cancelBuild = async () => {
+    try {
+      await invoke("cancel_colcon_build");
+      setBuildError(null);
+    } catch (error) {
+      setBuildError(`Build cancellation failed: ${String(error)}`);
+    }
+  };
+
   const initializeRosEnvironment = async () => {
     setInitializing(true);
     setSetupOutput(["Starting..."]);
@@ -583,12 +680,39 @@ function App() {
     linearX: number,
     angularZ: number,
   ) => {
+    setRobotCommandError(null);
     try {
       await invoke("publish_twist", {
         command: { topicName, linearX, angularZ },
       });
     } catch (err) {
-      console.error("Twist command failed:", err);
+      setRobotCommandError(String(err));
+    }
+  };
+
+  const requestEmergencyStop = async (activate: boolean) => {
+    if (safetyPending) return;
+    if (!activate && !window.confirm("Reset the latched emergency stop? Motion will remain stopped until a new command is issued.")) {
+      return;
+    }
+    setSafetyPending(true);
+    setSafetyError(null);
+    try {
+      const state = await invoke<{
+        state: "READY" | "ACTIVE" | "RESET_PENDING" | "FAULT" | "DISCONNECTED";
+        detail: string;
+      }>("set_emergency_stop", { activate });
+      setSafetyState(state.state);
+      if (state.state !== (activate ? "ACTIVE" : "READY")) {
+        setSafetyError(state.detail || "Backend did not confirm the requested safety state.");
+      }
+    } catch (error) {
+      if (activate) {
+        setSafetyState("DISCONNECTED");
+      }
+      setSafetyError(`Emergency-stop request was not confirmed: ${String(error)}`);
+    } finally {
+      setSafetyPending(false);
     }
   };
 
@@ -758,10 +882,12 @@ function App() {
     const unlistenError = listen<string>("ros-error", (event) => {
       console.error("ROS error:", event.payload);
       setRosConnected(false);
+      setSafetyState((state) => state === "ACTIVE" || state === "FAULT" ? state : "DISCONNECTED");
     });
 
     const unlistenAuthError = listen<string>("ros-auth-error", (event) => {
       setRosConnected(false);
+      setSafetyState((state) => state === "ACTIVE" || state === "FAULT" ? state : "DISCONNECTED");
       setRosEvents((prev) =>
         [...prev.slice(-49), `Authentication: ${event.payload}`],
       );
@@ -773,6 +899,37 @@ function App() {
 
     const unlistenConnecting = listen<void>("ros-connecting", () => {
       setRosConnected(false);
+      setSafetyState((state) => state === "ACTIVE" || state === "FAULT" ? state : "DISCONNECTED");
+    });
+    const unlistenSafety = listen<{
+      state: "READY" | "ACTIVE" | "RESET_PENDING" | "FAULT" | "DISCONNECTED";
+      detail: string;
+    }>("safety-state", (event) => {
+      setSafetyState(event.payload.state);
+      setSafetyError(event.payload.state === "FAULT" ? event.payload.detail : null);
+    });
+    const unlistenEventHealth = listen<{
+      queueCapacity: number;
+      subscriberCount: number;
+      lastSequence: number;
+      droppedTelemetry: number;
+      criticalOverflows: number;
+      maxBacklog: number;
+      persistenceHealthy: boolean;
+      persistenceError: string;
+      oldestAvailableSequence: number;
+      retainedCriticalEvents: number;
+    }>("event-health", (event) => {
+      setEventHealth(event.payload);
+    });
+    const unlistenEventRecovery = listen<{
+      gapDetected: boolean;
+      oldestAvailableSequence: number;
+      latestSequence: number;
+      snapshotIncluded: boolean;
+      retainedCriticalEvents: number;
+    }>("event-recovery", (event) => {
+      setEventRecovery(event.payload);
     });
 
     const unlistenBuildOutput = listen<LogEvent>("build-output", (event) => {
@@ -812,6 +969,9 @@ function App() {
       unlistenAuthError.then((f) => f());
       unlistenNodeSnapshot.then((f) => f());
       unlistenConnecting.then((f) => f());
+      unlistenSafety.then((f) => f());
+      unlistenEventHealth.then((f) => f());
+      unlistenEventRecovery.then((f) => f());
       unlistenBuildOutput.then((f) => f());
       unlistenBuildError.then((f) => f());
       unlistenBuildFinished.then((f) => f());
@@ -1016,6 +1176,11 @@ function App() {
             </>
           )}
         </button>
+        {building && (
+          <button className="ide-btn" onClick={cancelBuild}>
+            Cancel Build
+          </button>
+        )}
         <select
           value={worldPath ?? "default"}
           onChange={(e) => {
@@ -1256,10 +1421,91 @@ function App() {
                       {rosWorkspacePath ? "set" : "not set"}
                     </span>
                   </div>
-                  <div style={{ opacity: 0.5, fontSize: 11 }}>
-                    Live node/topic graph coming soon — inspect activity in the
-                    ROS Log panel below.
-                  </div>
+
+                  {rosPackagesLoading && (
+                    <div style={{ display: "flex", alignItems: "center", gap: 8, opacity: 0.7 }}>
+                      <LoaderCircle size={14} className="spin" />
+                      Loading packages...
+                    </div>
+                  )}
+
+                  {!rosPackagesLoading && rosPackagesError && (
+                    <div style={{ color: "#f0a3a3", whiteSpace: "pre-wrap" }}>
+                      {rosPackagesError}
+                    </div>
+                  )}
+
+                  {!rosPackagesLoading && !rosPackagesError && rosPackages.length === 0 && (
+                    <div style={{ opacity: 0.6 }}>
+                      No ROS packages discovered in the active environment.
+                    </div>
+                  )}
+
+                  {!rosPackagesLoading && !rosPackagesError && rosPackages.length > 0 && (
+                    <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
+                      {rosPackages.map((pkg) => (
+                        <div
+                          key={pkg.name}
+                          style={{
+                            background: "rgba(255,255,255,0.02)",
+                            border: "1px solid rgba(255,255,255,0.08)",
+                            borderRadius: 6,
+                            padding: 8,
+                          }}
+                        >
+                          <div style={{ fontWeight: 600, color: "#e8e8e8", marginBottom: 4 }}>
+                            {pkg.name}
+                          </div>
+                          <div style={{ opacity: 0.7, wordBreak: "break-word", marginBottom: 4 }}>
+                            {pkg.path}
+                          </div>
+                          <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
+                            {pkg.package_type && (
+                              <span style={{ background: "rgba(79,195,247,0.12)", borderRadius: 999, padding: "2px 6px" }}>
+                                {pkg.package_type}
+                              </span>
+                            )}
+                            {pkg.version && (
+                              <span style={{ background: "rgba(124,217,146,0.12)", borderRadius: 999, padding: "2px 6px" }}>
+                                v{pkg.version}
+                              </span>
+                            )}
+                          </div>
+                          {pkg.dependencies.length > 0 && (
+                            <div style={{ marginTop: 6 }}>
+                              <div style={{ opacity: 0.6, marginBottom: 2 }}>Dependencies</div>
+                              <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
+                                {pkg.dependencies.slice(0, 8).map((dep) => (
+                                  <span key={`${pkg.name}-${dep}`} style={{ background: "rgba(255,255,255,0.04)", borderRadius: 999, padding: "2px 6px" }}>
+                                    {dep}
+                                  </span>
+                                ))}
+                              </div>
+                            </div>
+                          )}
+                          {(pkg.executables.length > 0 || pkg.launch_files.length > 0 || pkg.config_files.length > 0) && (
+                            <div style={{ marginTop: 6, display: "flex", flexDirection: "column", gap: 2 }}>
+                              {pkg.executables.length > 0 && (
+                                <div style={{ opacity: 0.6 }}>
+                                  Executables: {pkg.executables.join(", ")}
+                                </div>
+                              )}
+                              {pkg.launch_files.length > 0 && (
+                                <div style={{ opacity: 0.6 }}>
+                                  Launch: {pkg.launch_files.join(", ")}
+                                </div>
+                              )}
+                              {pkg.config_files.length > 0 && (
+                                <div style={{ opacity: 0.6 }}>
+                                  Config: {pkg.config_files.join(", ")}
+                                </div>
+                              )}
+                            </div>
+                          )}
+                        </div>
+                      ))}
+                    </div>
+                  )}
                 </div>
               )}
             </div>
@@ -1880,6 +2126,12 @@ function App() {
                     <RobotControls
                       onCommand={sendTwist}
                       availableRobots={availableRobots}
+                      safetyState={safetyState}
+                      safetyPending={safetyPending}
+                      safetyError={safetyError}
+                      commandError={robotCommandError}
+                      onEmergencyStop={() => void requestEmergencyStop(true)}
+                      onResetEmergencyStop={() => void requestEmergencyStop(false)}
                     />
                   </div>
                 </div>
@@ -1887,6 +2139,21 @@ function App() {
               {activeTab === "tf" && <TFTree onExplain={explainTfFrame} />}
               {activeTab === "problems" && (
                 <div style={{ padding: 8, fontSize: 12 }}>
+                  {eventHealth && (
+                    <div role="status" style={{ padding: "6px 0", color: !eventHealth.persistenceHealthy ? "#e08080" : eventHealth.criticalOverflows > 0 ? "#e0b060" : "#7cd992" }}>
+                      ROS event queue: {eventHealth.subscriberCount} subscriber(s), backlog {eventHealth.maxBacklog}/{eventHealth.queueCapacity}, critical overflows {eventHealth.criticalOverflows}, coalesced/dropped telemetry {eventHealth.droppedTelemetry}; retained critical events {eventHealth.retainedCriticalEvents}.
+                    </div>
+                  )}
+                  {eventHealth && !eventHealth.persistenceHealthy && (
+                    <div role="alert" style={{ color: "#e08080", padding: "4px 0" }}>
+                      Critical event history is unavailable: {eventHealth.persistenceError}
+                    </div>
+                  )}
+                  {eventRecovery?.gapDetected && (
+                    <div role="status" style={{ color: "#e0b060", padding: "4px 0" }}>
+                      ROS event history had a gap. {eventRecovery.snapshotIncluded ? "Current state snapshots were restored." : "A state snapshot is not available yet."} Oldest retained sequence: {eventRecovery.oldestAvailableSequence}; latest sequence: {eventRecovery.latestSequence}.
+                    </div>
+                  )}
                   {diagnostics.length === 0 ? (
                     <div style={{ color: "#777" }}>No problems reported by recent environment, build, or ROS output.</div>
                   ) : (
diff --git a/src/components/RobotView.tsx b/src/components/RobotView.tsx
index 50e76fa..ccf33c3 100644
--- a/src/components/RobotView.tsx
+++ b/src/components/RobotView.tsx
@@ -2,6 +2,7 @@ import { useEffect, useRef, useState } from "react";
 import * as THREE from "three";
 import { listen } from "@tauri-apps/api/event";
 import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
+import { invoke } from "@tauri-apps/api/core";
 import { ArrowUp, ArrowDown, RotateCcw, RotateCw, Square } from "lucide-react";
 
 interface OdometryUpdate {
@@ -223,11 +224,69 @@ export default function RobotView() {
 export function RobotControls({
   onCommand,
   availableRobots,
+  safetyState,
+  safetyPending,
+  safetyError,
+  commandError,
+  onEmergencyStop,
+  onResetEmergencyStop,
 }: {
   onCommand: (topic: string, lin: number, ang: number) => void;
   availableRobots: string[];
+  safetyState: "READY" | "ACTIVE" | "RESET_PENDING" | "FAULT" | "DISCONNECTED";
+  safetyPending: boolean;
+  safetyError: string | null;
+  commandError: string | null;
+  onEmergencyStop: () => void;
+  onResetEmergencyStop: () => void;
 }) {
   const [selectedRobot, setSelectedRobot] = useState<string | null>(null);
+  const [robotConfig, setRobotConfig] = useState<RobotMotionConfig | null>(null);
+  const [configError, setConfigError] = useState<string | null>(null);
+  const [configBusy, setConfigBusy] = useState(false);
+
+  useEffect(() => {
+    if (!selectedRobot) {
+      setRobotConfig(null);
+      return;
+    }
+    let active = true;
+    setConfigBusy(true);
+    setConfigError(null);
+    invoke<RobotMotionConfig>("get_robot_config", { topicName: selectedRobot })
+      .then((config) => {
+        if (active) setRobotConfig(config);
+      })
+      .catch((error) => {
+        if (active) setConfigError(String(error));
+      })
+      .finally(() => {
+        if (active) setConfigBusy(false);
+      });
+    return () => {
+      active = false;
+    };
+  }, [selectedRobot]);
+
+  const saveConfig = async (reset = false) => {
+    if (!selectedRobot || !robotConfig || configBusy) return;
+    setConfigBusy(true);
+    setConfigError(null);
+    try {
+      const saved = reset
+        ? await invoke<RobotMotionConfig>("reset_robot_config", {
+            topicName: selectedRobot,
+          })
+        : await invoke<RobotMotionConfig>("save_robot_config", {
+            config: robotConfig,
+          });
+      setRobotConfig(saved);
+    } catch (error) {
+      setConfigError(String(error));
+    } finally {
+      setConfigBusy(false);
+    }
+  };
 
   // Keep selection valid as the available robot list changes (e.g., after
   // switching worlds) — default to the first available robot if none is
@@ -278,6 +337,50 @@ export function RobotControls({
         </select>
       )}
 
+      <div
+        role="status"
+        aria-live="polite"
+        style={{
+          margin: "8px 0",
+          padding: 8,
+          color: safetyState === "ACTIVE" || safetyState === "FAULT" ? "#ff8b82" : "#ccc",
+          background: safetyState === "ACTIVE" || safetyState === "FAULT" ? "#421f1f" : "#252526",
+        }}
+      >
+        Safety state: {safetyState}
+        {safetyState === "ACTIVE" && (
+          <div>Latched. Reset is explicit; motion will not resume automatically.</div>
+        )}
+      </div>
+      {safetyError && <div role="alert" style={{ color: "#ff8b82" }}>{safetyError}</div>}
+      {commandError && <div role="alert" style={{ color: "#ff8b82" }}>{commandError}</div>}
+      {safetyState === "ACTIVE" || safetyState === "FAULT" ? (
+        <button
+          type="button"
+          className="ide-btn primary"
+          disabled={safetyPending || safetyState === "FAULT"}
+          onClick={onResetEmergencyStop}
+          style={{ width: "100%", marginBottom: 8 }}
+        >
+          {safetyPending ? "Applying..." : "Reset Emergency Stop"}
+        </button>
+      ) : (
+        <button
+          type="button"
+          className="ide-btn"
+          disabled={safetyPending || safetyState !== "READY"}
+          onClick={onEmergencyStop}
+          style={{
+            width: "100%",
+            marginBottom: 8,
+            color: "#fff",
+            background: "#a32323",
+          }}
+        >
+          {safetyPending ? "Applying..." : "EMERGENCY STOP"}
+        </button>
+      )}
+
       <div
         style={{
           marginBottom: "var(--spacing-sm)",
@@ -285,8 +388,10 @@ export function RobotControls({
           opacity: 0.75,
         }}
       >
-        Speed magnitude limits: 0.5 m/s linear, 1 rad/s angular. Commands time
-        out after 0.5 s; this is not a safety-rated emergency stop.
+        {robotConfig
+          ? `Limits: ${robotConfig.maxLinearVelocity} m/s linear, ${robotConfig.maxAngularVelocity} rad/s angular; ${robotConfig.commandLeaseMs} ms command lease.`
+          : "Per-robot motion limits are enforced by the ROS service."}{" "}
+        Software stop only; not safety-rated.
       </div>
 
       <div
@@ -294,12 +399,13 @@ export function RobotControls({
           display: "grid",
           gridTemplateColumns: "repeat(3, 48px)",
           justifyContent: "center",
-          opacity: selectedRobot ? 1 : 0.4,
-          pointerEvents: selectedRobot ? "auto" : "none",
+          opacity: selectedRobot && safetyState === "READY" && !safetyPending ? 1 : 0.4,
+          pointerEvents: selectedRobot && safetyState === "READY" && !safetyPending ? "auto" : "none",
         }}
       >
         <div />
         <button
+          disabled={!selectedRobot || safetyState !== "READY" || safetyPending}
           style={btnStyle}
           onClick={() => onCommand(selectedRobot!, 0.3, 0)}
         >
@@ -309,6 +415,7 @@ export function RobotControls({
         </button>
         <div />
         <button
+          disabled={!selectedRobot || safetyState !== "READY" || safetyPending}
           style={btnStyle}
           onClick={() => onCommand(selectedRobot!, 0, 0.5)}
         >
@@ -317,6 +424,7 @@ export function RobotControls({
           />
         </button>
         <button
+          disabled={!selectedRobot || safetyState !== "READY" || safetyPending}
           style={btnStyle}
           onClick={() => onCommand(selectedRobot!, 0, 0)}
         >
@@ -325,6 +433,7 @@ export function RobotControls({
           />
         </button>
         <button
+          disabled={!selectedRobot || safetyState !== "READY" || safetyPending}
           style={btnStyle}
           onClick={() => onCommand(selectedRobot!, 0, -0.5)}
         >
@@ -334,6 +443,7 @@ export function RobotControls({
         </button>
         <div />
         <button
+          disabled={!selectedRobot || safetyState !== "READY" || safetyPending}
           style={btnStyle}
           onClick={() => onCommand(selectedRobot!, -0.3, 0)}
         >
@@ -343,6 +453,73 @@ export function RobotControls({
         </button>
         <div />
       </div>
+      {selectedRobot && robotConfig && (
+        <fieldset
+          disabled={configBusy}
+          style={{ marginTop: 12, border: "1px solid #444", padding: 8 }}
+        >
+          <legend>Robot motion limits</legend>
+          {([
+            ["maxLinearVelocity", "Linear velocity (m/s)"],
+            ["maxAngularVelocity", "Angular velocity (rad/s)"],
+            ["maxLinearAcceleration", "Linear acceleration (m/s²)"],
+            ["maxAngularAcceleration", "Angular acceleration (rad/s²)"],
+            ["commandLeaseMs", "Command lease (ms)"],
+            ["commandTimeoutMs", "Command timeout (ms)"],
+          ] as const).map(([field, label]) => (
+            <label key={field} style={{ display: "block", fontSize: 11, margin: "6px 0" }}>
+              {label}
+              <input
+                type="number"
+                min={field.endsWith("Ms") ? 100 : 0.01}
+                step={field.endsWith("Ms") ? 50 : 0.05}
+                value={robotConfig[field]}
+                onChange={(event) => {
+                  const value = Number(event.target.value);
+                  setRobotConfig((current) => current && { ...current, [field]: value });
+                }}
+                style={{ width: "100%" }}
+              />
+            </label>
+          ))}
+          <label style={{ display: "block", fontSize: 11 }}>
+            Motion mode
+            <select
+              value={robotConfig.permittedMotionModes.includes("full_twist") ? "full_twist" : "planar"}
+              onChange={(event) =>
+                setRobotConfig((current) => current && {
+                  ...current,
+                  permittedMotionModes: [event.target.value],
+                })
+              }
+              style={{ width: "100%" }}
+            >
+              <option value="planar">Planar</option>
+              <option value="full_twist">Full Twist</option>
+            </select>
+          </label>
+          {configError && <div role="alert" style={{ color: "#ff8b82" }}>{configError}</div>}
+          <div style={{ display: "flex", gap: 6, marginTop: 8 }}>
+            <button type="button" className="ide-btn primary" disabled={configBusy} onClick={() => void saveConfig()}>
+              Save limits
+            </button>
+            <button type="button" className="ide-btn" disabled={configBusy} onClick={() => void saveConfig(true)}>
+              Safe defaults
+            </button>
+          </div>
+        </fieldset>
+      )}
     </div>
   );
 }
+
+interface RobotMotionConfig {
+  topicName: string;
+  maxLinearVelocity: number;
+  maxAngularVelocity: number;
+  maxLinearAcceleration: number;
+  maxAngularAcceleration: number;
+  commandLeaseMs: number;
+  commandTimeoutMs: number;
+  permittedMotionModes: string[];
+}
````
