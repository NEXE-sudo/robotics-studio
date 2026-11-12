# Robotics Studio

A native desktop IDE for ROS 2 development — edit code, build with `colcon`,
run live simulations in Gazebo, visualize robot state in real time, and get
AI assistance that actually understands your running ROS 2 system.

**Status**: v0.3.0 development target, early-stage. Built solo, in the open. Expect rough
edges — this is a real, working tool, not a polished 1.0, and I'd rather be
upfront about that than pretend otherwise.

**Current repo state**: The working tree contains the current uncommitted v0.3.0 implementation
pass across ROS, TF, URDF/Xacro, Docker integration, release docs, and verification updates.
The project remains explicitly not release-ready: the work is substantial and functionally
advanced, but the remaining acceptance areas are still partial and require broader platform,
upgrade, and hosted-CI validation before release.

---

## What's in the v0.3.0 development target

- **One-click ROS 2 + Gazebo environment** — the app builds, starts, and
  connects to a Docker-based ROS 2 environment automatically on launch. No
  manual Docker/Dev Container setup required.
- **Live ROS status** — node/topic snapshots and TF updates are streamed over
  authenticated gRPC while the system runs. The ROS sidebar also discovers
  source packages in the selected workspace, shows package metadata and
  resources, and lets you inspect package files. Underlay packages are
  identified separately from workspace packages, with dependency sources
  reported as workspace, underlay, or missing. The live graph shows topic
  publisher/subscriber QoS and bounded message inspection. It also exposes
  read-only lifecycle/parameter service calls and the allowlisted non-motion
  Fibonacci action workflow; arbitrary service types and motion-producing
  actions are not enabled.
- **3D visualization** — preview supported URDF and Xacro descriptions,
  primitive and workspace-confined STL/texture-free DAE visuals, supported
  joints, inertial frames, and collision geometry in the three.js viewport.
  Planar and floating joints are accepted in the preview with explicit
  best-effort axis translations, while unsupported external resources remain
  rejected. Streamed `/joint_states` update supported joints. Without an
  active robot description, the view keeps its placeholder robot. In-app
  controls publish bounded `cmd_vel` commands directly (no manual container
  terminal needed for basic teleop).
- **Custom Gazebo world support** — load your own world file instead of
  being limited to the bundled demo.
- **AI assistant with real context** — ask questions about your open file,
  recent ROS activity, or build output, with pluggable providers (Groq,
  Anthropic, OpenAI) configured entirely in-app via **Settings** — no
  environment variables required.
  - Generate ROS 2 node code or URDF files from a prompt.
  - Click **Explain** on any TF frame to get a plain-language breakdown of
    what it represents and whether its values look normal.
- **Project templates** — spin up a new ROS 2 package from a bundled
  template instead of writing boilerplate by hand.
- **`.aiignore` support** — exclude specific files (secrets, credentials)
  from ever being sent to the AI assistant.
- **Configurable keybindings** — rebind shortcuts in-app, or import your
  existing bindings from VS Code's `keybindings.json`.
- **System-aware offline model guidance** — Settings can detect your
  machine's RAM/CPU and suggest realistically-sized local model tiers if
  you're considering running AI features offline (this is guidance only —
  it doesn't run local models for you yet).
- **Responsive UI** — scales with window/screen size rather than staying
  pinned at a fixed layout.
- **Restricted native file access** — file browsing/editing is scoped to
  directories selected in a native picker; file operations reject traversal,
  symlink escapes anywhere in the requested path, and oversized text files.
- **Authenticated ROS gRPC** — application-generated session credentials,
  expiry checks, and authenticated event/control requests over TLS. The client
  pins a container-generated CA certificate delivered through Docker's control
  channel; existing containers with non-loopback gRPC bindings are refused.
- **Safer world import** — validates SDF input and stores it in a dedicated,
  uniquely named workspace `worlds/` directory rather than overwriting files
  in the workspace root. Imports are XML-parsed, require an SDF root and world
  element, accept supported SDF versions 1.4–1.10, and reject DTDs.
- **Recoverable ROS events** — selected critical node-lifecycle and safety
  events are persisted and replayed after reconnect or service restart.
  Coalesced telemetry and periodic discovery snapshots are best-effort, not
  journaled. If retained history has a gap, the stream reports it and includes
  the latest state snapshots where available.
- **Per-robot command limits** — configure bounded linear/angular velocity,
  acceleration, planar/full-Twist mode, and command lease/timeout per command
  topic. Defaults remain conservative (0.5 m/s linear, 1 rad/s angular,
  0.5-second lease); hard ceilings prevent loosening the built-in bounds.
- **Software emergency stop** — the 3D View controls can latch motion commands
  off, publish zero velocity on discovered `/cmd_vel` topics, and require an
  explicit reset. Stop state and a bounded activation/reset history persist
  across service restarts.

---

## Architecture, briefly

The trust boundaries and subsystem behavior are described in
[docs/architecture.md](./docs/architecture.md).

This app has two halves that run in different places and talk to each other:

- **The Tauri app** (Rust + React) — runs natively on your machine.
- **The ROS 2 environment** — runs inside a Docker container (needed on
  macOS, since ROS 2 doesn't run natively there; also used on Linux/Windows
  for a consistent, reproducible environment). The app talks to it via gRPC
  (for live ROS state) and `docker exec`/streamed process output (for
  builds and simulation control).

You don't need to touch Docker, VS Code Dev Containers, or a terminal for
any of this — the app manages the environment for you.

---

## Prerequisites

- **Docker Desktop** — [docker.com](https://docker.com) (must be installed
  and running before you launch the app)
- An API key for at least one supported AI provider if you want to use the
  AI assistant — **Groq** (free tier at
  [console.groq.com](https://console.groq.com)), Anthropic, or OpenAI.
  This is entirely optional; the IDE, build tooling, and simulation work
  without it.

---

## Installation

1. Download the latest release for your platform from the
   [Releases page](https://github.com/NEXE-sudo/robotics-studio/releases).
2. **macOS**: open the `.dmg`, drag **Robotics Studio** into Applications.
   On first launch, macOS will likely show *"Apple cannot verify this app is
   free of malware"* — this is expected for a small, unsigned, early-stage
   app. **Right-click the app → Open**, then click **Open** again in the
   dialog. You only need to do this once.
3. Launch the app. On first run, it will automatically build and start the
   Docker-based ROS 2 environment — you'll see a progress popup while it
   downloads and builds (this takes a few minutes the first time; future
   launches are much faster since the image is reused).
4. Open **Settings → AI Provider** in-app to add your API key if you want
   AI features. No environment variables or config files to edit by hand.

---

## Using the app

- **Open Folder** — browse and edit any files (not necessarily the ROS
  workspace — this can be any folder you want to edit).
- **New Project from Template** — scaffold a new ROS 2 package instead of
  writing boilerplate.
- **Build** — runs `colcon build` inside the container, with live streaming
  output (updates in place, doesn't flood the log with duplicate lines).
- **Start Sim / Stop Sim / Reset Sim** — launches a Gazebo demo world (or
  your own custom world), visualized live in the **3D View** tab.
- **3D View safety controls** — select a robot command topic to edit its
  bounded limits, latch the software emergency stop, or explicitly reset it.
  The backend remains authoritative and rejects commands while stopped.
- **ROS Log tab** — live feed of ROS 2 node/topic activity.
- **ROS Explorer sidebar** — filter packages from the selected workspace,
  inspect build type, version, description, dependencies, installed
  executables, launch/configuration files, and browse package files. Refresh
  after builds to update the workspace package inventory. The live graph also
  shows nodes, topic endpoints/QoS, services, and action endpoint metadata.
- **TF tab** — live transform updates, with an **Explain** action per
  frame for an AI-generated plain-language breakdown.
- **AI Assistant (right panel)** — ask questions about your open file,
  recent ROS activity, or build output. Configure your provider/API key in
  **Settings**. Files matching `.aiignore` are excluded from what it sees.
- **Settings** — AI provider config, keyboard shortcut customization
  (including VS Code import), and system-spec-based offline model guidance.

### A concrete first thing to try

1. Launch the app and wait for the environment-setup popup to finish (first
   run only takes a few minutes).
2. Click **New Project from Template** to scaffold a test package.
3. Click **Build** — confirm it builds successfully.
4. Click **Start Sim**, switch to the **3D View** tab.
5. Use the in-app teleop controls to publish a `cmd_vel` command and watch
   the robot move.
6. Open the **TF** tab and click **Explain** on a frame.
7. Ask the AI assistant: *"what ROS topics are currently active?"* — it
   should answer from live data, not a guess.

---

## Known limitations (please break things and tell me what you find)

- The URDF preview supports box, cylinder, sphere, STL, and texture-free DAE
  visuals plus fixed/revolute/continuous/prismatic joints. Xacro uses a
  non-executing subset (arguments, properties, arithmetic, macros, workspace-
  local includes, and boolean conditionals); unsupported constructs fail
  closed. `package://` and relative mesh paths are confined to the selected
  workspace and its discovered packages. DAE image/texture and external
  resource references are rejected. Joint limits, mimic relationships,
  named material colors, inertial origins, and supported primitive collision
  geometry are represented in the preview. Inertial frames can be shown with
  a scalar radius-of-gyration marker. Floating/planar joints and full URDF
  semantics are not supported. ROS fixed-axis RPY rotations and the Z-up to
  Three.js coordinate mapping are applied.
- The TF tab builds a live transform forest and warns about multiple roots,
  cycles, multiple parents, stale dynamic transforms, non-finite values, and
  invalid quaternions. It displays the ROS source stamp separately from the
  local receive age, so replayed old data does not appear freshly received.
  Stale dynamic edges are labeled after five seconds and reported as having
  no recent updates after 30 seconds; an old dynamic parent is replaced after
  a child changes parent and the old edge has gone stale. The inspector
  composes root-to-frame poses when the path is unambiguous and valid.
  Disconnection is explicit; static `/tf_static` edges use transient-local
  QoS and are not marked stale. This is diagnostic validation, not a
  substitute for `tf2` authority or full ROS graph validation.
- ROS graph snapshots are bounded. Live topic inspection is limited to four
  topics, five samples per second per topic, and bounded serialized payloads.
  Service invocation is restricted to read-only lifecycle/parameter types;
  action goals are restricted to the non-motion Fibonacci type. Arbitrary
  service types and motion-producing actions are not enabled. Package
  discovery reports bounded workspace and Jazzy-underlay packages, not every
  package installed in the container.
- SDF imports are structurally checked, and the world is validated against
  the installed SDFormat schema before Gazebo starts. This does not resolve or
  validate every external model/mesh resource.
- Only one bundled project template (`basic_publisher`) so far.
- Local/offline model *running* isn't implemented — Settings will suggest
  appropriately-sized models for your hardware, but you'll still need to
  run them yourself via something like Ollama for now.
- Tested most thoroughly on macOS; Linux/Windows should work given the
  Docker-based architecture, but haven't had the same depth of testing.
- Existing files in the managed ROS workspace are preserved rather than
  overwritten during startup, except that legacy ROS server/proto files are
  backed up under the application config directory before the authentication
  update is installed. Existing containers with insecure gRPC port bindings
  are detected and blocked, but must be recreated manually with their workspace
  bind-mounted to preserve files.
- The software emergency stop is latched and backend-enforced, but there is no
  physical-robot confirmation flow or safety-rated control guarantee.
- The ROS workspace picker selects the active workspace for container
  initialization, builds, simulation, and world imports. It is persisted under
  `~/.robotics-studio/selected-workspace.txt`; opening an unrelated folder in
  the file explorer does not change the ROS workspace.
- Build and simulator subprocesses do not yet share a unified process manager.
  Build status and cancellation requests are tracked, and cancellation is only
  reported after process exit is confirmed. Gazebo startup waits up to 20
  seconds for a world-control service and cleans up if startup or bridge setup
  fails, but generalized process status, timeouts, and application-shutdown
  cleanup are still incomplete.
- A bounded, workspace-local journal stores critical node-lifecycle and
  software safety events (up to 256 records and 4 MiB) and preserves their
  sequence numbers across service restarts. Telemetry and periodic snapshots are
  coalesced/best-effort and are not journaled; they do not consume durable
  sequence numbers. Replaying a range returns retained events, not an
  exactly-once acknowledgment from the UI. Older-than-retained requests report
  a gap and include the latest in-memory snapshots when available. Journal
  write failures are surfaced in event health; failure while recording a
  safety transition fail-closes the safety controller.
- AI provider credentials are stored through the operating-system credential
  store. Linux requires an available Secret Service; the app reports an error
  if secure storage is unavailable.

## Security notes

- Native file and directory pickers register approved roots in the Rust
  backend. File reads, writes, listings, and `.aiignore` reads are constrained
  to those roots; text reads/writes are limited to 8 MiB. Template and
  devcontainer copying still needs descriptor-rooted destination operations.
- The selected ROS workspace is revalidated by the backend on use. Switching
  workspaces stops the running managed container for the previous workspace
  before the new environment is initialized; stopped containers are preserved.
- New ROS containers publish gRPC only on host loopback and use TLS with a
  per-container CA and server certificate. The app retrieves the public CA
  certificate through Docker's control channel and pins it before sending
  credentials. All gRPC methods also require a random 256-bit token with a
  24-hour expiry. The token is transferred via a protected file and is never
  returned to frontend JavaScript.
- The running ROS service scripts and protocol are copied from the application
  bundle into `/opt/robotics-studio` inside the container, outside the writable
  workspace mount. Legacy workspace copies are compared byte-for-byte against
  bundled files before migration; a source-code marker alone is not trusted.
- When a stream reports an expired or rejected token, the app performs one
  guarded environment reinitialization: it generates a new token, replaces
  the container credential file, restarts the ROS service, and reconnects.
  The old token is invalidated when the server restarts. If recovery fails,
  the app stops automatic auth retries and reports the error. This is a
  service-restart-based rotation flow, not a separate refresh-token protocol.
- An existing container with a missing or non-loopback gRPC binding is
  rejected. The app does not automatically replace that container; preserve
  its host workspace and recreate it with a loopback-only mapping.
- Robot velocity and freshness checks run in Rust and Python. The ROS service
  enforces per-topic velocity, acceleration, mode, lease, and timeout settings
  within hard ceilings; defaults include a 0.5-second command lease. The
  software emergency stop publishes zero Twist commands but is not a
  replacement for a physical emergency-stop circuit.
- Imported SDF is checked for XML well-formedness, an SDF root, a world
  element, and a supported declared SDF version. DTDs are rejected. This is
  not full SDFormat schema validation and does not resolve or validate every
  referenced model or mesh.
- AI provider credentials are stored separately through macOS Keychain,
  Windows Credential Manager, or Linux Secret Service. Only provider/model
  metadata is stored in the application settings JSON. Existing plaintext
  credentials are moved to secure storage on first settings load, and the old
  JSON entry is removed only after the secure-store write succeeds.
- On Unix, approved-root file reads, writes, listings, and world imports use
  pinned directory descriptors and no-follow `openat` operations; replacing
  an approved root is rejected. Windows currently uses canonical-path checks
  and atomic temporary-file replacement, and does not provide equivalent
  protection against a local process racing parent-directory changes.
- Not every operation has been migrated to descriptor-rooted access yet;
  selected-world source reads and template/project creation remain areas for
  further platform-specific race hardening.

An independent source review found and this development pass fixed the
workspace service-marker trust issue and plaintext gRPC credential transport.
This review and the integration checks are not a claim of security
certification.

## Migration notes for existing v0.2.0 installations

- The managed ROS workspace remains in `~/.robotics-studio/ros2-workspace`;
  project files and workspace data are not removed by these changes.
- On the next environment initialization, the app compares existing
  `server.py`, `server_core.py`, and `proto/ros_engine.proto` files byte-for-byte
  with trusted bundled copies, backs up modified copies under the application
  config directory, and installs the bundled versions. The running service is
  launched from an application-provisioned directory inside the container,
  not from the workspace mount.
- Containers whose host gRPC port is bound to an external interface are
  refused. Stop the old container and recreate it with
  `-p 127.0.0.1:50051:50051` and the existing workspace mounted at
  `/workspace`. The application does not delete or automatically replace it.
- On first AI settings load, an existing plaintext API key is migrated to the
  operating-system credential store. If that store is unavailable, migration
  fails explicitly and the original settings file is retained for recovery.
- The emergency stop is **software-only**: it cannot guarantee physical
  stopping, stop controllers that ignore `/cmd_vel`, or replace a hardware
  safety circuit. Validate stop behavior on the actual robot and maintain
  independent hardware safeguards.

## Development checks

Run the frontend build/type check and audit, Rust formatting/check/tests/lint
and dependency audit, and isolated Python service-core tests from the repository
root. Install `cargo-audit` 0.23.0 if it is not already available.

```sh
npm ci
cargo install cargo-audit --version 0.23.0 --locked
npm run build
npm run test:frontend
npm audit
cargo audit --file src-tauri/Cargo.lock
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo check --manifest-path src-tauri/Cargo.toml --locked
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
python3 -m unittest discover -s src-tauri/resources/devcontainer -p 'test_*.py' -v
python3 -m py_compile src-tauri/resources/devcontainer/server.py src-tauri/resources/devcontainer/server_core.py
```

For the live container integration (Docker Desktop or Docker Engine must be
running), execute:

```sh
./scripts/docker-integration.sh
```

The script builds the supported ROS 2 Jazzy/Gazebo Harmonic image, mounts a
temporary workspace, verifies workspace/underlay package discovery, safe
lifecycle service calls, rejected unsafe requests, Fibonacci action feedback,
results and confirmed cancellation, bounded topic inspection, static/dynamic
TF and joint states, pinned-TLS authentication, emergency-stop gating, event
replay after service restart, Gazebo readiness/reset and ROS-Gazebo topic
bridging. It performs no robot movement and removes its temporary container
and workspace on exit. CI runs the same script on its Linux runner.

Unit/build checks are not a substitute for live Docker/ROS 2/Gazebo checks.
They require Docker Desktop running for an integration environment; if it is
unavailable, simulation and gRPC integration behavior remains unverified. The
current supported development baseline is ROS 2 Jazzy with Gazebo Harmonic in
the bundled container. macOS is tested most often; Linux/Windows runtime
behavior and packaged desktop lifecycle still need platform-specific testing.

---

## If something breaks

Check, in order:
1. Is Docker Desktop actually running?
2. Is the container running? (`docker ps` — look for `robotics-studio-ros2`)
3. Is the app's dev console (if running from source) showing an error?
4. Try restarting the app — environment setup safely re-runs and reuses
   the existing image/container rather than rebuilding from scratch.

If none of that helps, open an issue with exactly what you clicked, what
you expected, and what happened instead — that's the most useful thing you
can hand back.

---

## Contributing / following along

This is being built in the open, in public, with real bugs and real fixes
shared as they happen rather than only polished announcements. If you work
with ROS 2, or you're just curious about developer tooling for robotics,
issues, PRs, and feedback are genuinely welcome.
