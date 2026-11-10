# Robotics Studio

A native desktop IDE for ROS 2 development — edit code, build with `colcon`,
run live simulations in Gazebo, visualize robot state in real time, and get
AI assistance that actually understands your running ROS 2 system.

**Status**: v0.3.0 development target, early-stage. Built solo, in the open. Expect rough
edges — this is a real, working tool, not a polished 1.0, and I'd rather be
upfront about that than pretend otherwise.

---

## What's in the v0.3.0 development target

- **One-click ROS 2 + Gazebo environment** — the app builds, starts, and
  connects to a Docker-based ROS 2 environment automatically on launch. No
  manual Docker/Dev Container setup required.
- **Live ROS introspection** — a real-time node/topic feed streamed over
  gRPC while your system runs, plus a live **TF tree** view of your robot's
  transform frames.
- **3D visualization** — robot state rendered live in a three.js viewport,
  with in-app controls to publish `cmd_vel` commands directly (no manual
  container terminal needed for basic teleop).
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
  symlink escapes, and oversized text files.
- **Authenticated ROS gRPC** — application-generated session credentials,
  expiry checks, and authenticated event/control requests. Existing containers
  with non-loopback gRPC bindings are refused rather than treated as secure.
- **Safer world import** — validates SDF input and stores it in a dedicated,
  uniquely named workspace `worlds/` directory rather than overwriting files
  in the workspace root. Imports are XML-parsed, require an SDF root and world
  element, accept supported SDF versions 1.4–1.10, and reject DTDs.
- **Prioritized ROS events** — critical discovery state is retained separately
  from coalesced telemetry; reconnecting clients receive the latest discovery
  snapshots.
- **Bounded robot commands** — linear velocity magnitude is limited to
  0.5 m/s, angular velocity magnitude to 1 rad/s, and commands expire after
  0.5 seconds. These limits are fixed conservative defaults, not robot-specific
  tuning or a safety-rated emergency stop.

---

## Architecture, briefly

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
- **ROS Log tab** — live feed of ROS 2 node/topic activity.
- **TF Tree tab** — live transform tree, with an **Explain** action per
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
6. Open the **TF Tree** tab and click **Explain** on a frame.
7. Ask the AI assistant: *"what ROS topics are currently active?"* — it
   should answer from live data, not a guess.

---

## Known limitations (please break things and tell me what you find)

- 3D view uses accurately-dimensioned placeholder geometry, not full URDF
  mesh loading yet — visually approximate, not a rendered CAD model.
- ROS↔three.js coordinate mapping is a first-pass approximation — rotation
  direction hasn't been rigorously verified against every convention.
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
- Robot-control limits are not configurable per robot. There is no latched
  software emergency stop, physical-robot confirmation flow, or safety-rated
  control guarantee.
- Workspace selection is still not wired through every backend operation;
  build and simulation lifecycle currently use the managed default workspace.
- AI provider credentials are stored through the operating-system credential
  store. Linux requires an available Secret Service; the app reports an error
  if secure storage is unavailable.

## Security notes

- Native file and directory pickers register approved roots in the Rust
  backend. Reads, writes, listings, project creation, and `.aiignore` checks
  are constrained to those roots; text reads/writes are limited to 8 MiB.
- New ROS containers bind gRPC to `127.0.0.1:50051`. Both gRPC methods require
  a random 256-bit token with a 24-hour expiry. The token is transferred via a
  protected file and is never returned to frontend JavaScript.
- When a stream reports an expired or rejected token, the app performs one
  guarded environment reinitialization: it generates a new token, replaces
  the container credential file, restarts the ROS service, and reconnects.
  The old token is invalidated when the server restarts. If recovery fails,
  the app stops automatic auth retries and reports the error. This is a
  service-restart-based rotation flow, not a separate refresh-token protocol.
- An existing container with a missing or non-loopback gRPC binding is
  rejected. The app does not automatically replace that container; preserve
  its host workspace and recreate it with a loopback-only mapping.
- Robot velocity and freshness checks run in both Rust and Python. Commands
  are checked against combined linear and angular vector magnitudes and are
  stopped by the server watchdog after 0.5 seconds, but this is not a
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
- Path validation checks canonical containment and rejects traversal and
  symlink escapes. It is not a kernel-level descriptor-based sandbox against a
  local process racing filesystem changes.

These controls have not been independently security-audited and are not a
claim of security certification.

## Migration notes for existing v0.2.0 installations

- The managed ROS workspace remains in `~/.robotics-studio/ros2-workspace`;
  project files and workspace data are not removed by these changes.
- On the next environment initialization, the app backs up existing
  `server.py`, `server_core.py`, and `proto/ros_engine.proto` files under the
  application config directory, then installs the authenticated server
  protocol.
- Containers whose host gRPC port is bound to an external interface are
  refused. Stop the old container and recreate it with
  `-p 127.0.0.1:50051:50051` and the existing workspace mounted at
  `/workspace`. The application does not delete or automatically replace it.
- On first AI settings load, an existing plaintext API key is migrated to the
  operating-system credential store. If that store is unavailable, migration
  fails explicitly and the original settings file is retained for recovery.

## Development checks

Run the frontend build/type check, Rust formatting/check/tests/lint, and
isolated Python service-core tests from the repository root:

```sh
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo check --manifest-path src-tauri/Cargo.toml --locked
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features --locked -- -D warnings
python3 -m unittest discover -s src-tauri/resources/devcontainer -p 'test_*.py' -v
```

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
