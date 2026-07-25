# Robotics Studio (Prototype)

A native desktop IDE for ROS 2 development — edit code, build with `colcon`,
run live simulations in Gazebo, visualize robot state in real time, and get
AI assistance that actually understands your running ROS 2 system.

**Status**: early prototype (M1), built for internal testing and feedback.
Expect rough edges — this is a first pass, not a polished release.

---

## Architecture, briefly

This app has two halves that run in different places and talk to each other:

- **The Tauri app** (Rust + React) — runs natively on your machine (macOS,
  tested; should work on Linux/Windows with minor adjustments).
- **The ROS 2 environment** — runs inside a Docker container, since ROS 2
  doesn't support macOS natively. The app talks to it via gRPC (for live
  ROS state) and `docker exec` (for builds and simulation control).

The app sets up and manages the ROS 2 environment for you — you don't need
to touch Docker, VS Code Dev Containers, or a terminal for this part.

---

## Prerequisites

- **Docker Desktop** — [docker.com](https://docker.com) (must be installed
  and running before you launch the app)
- **Rust** (via [rustup.rs](https://rustup.rs)) and **Node.js** (v18+) —
  only needed to build the app itself; not needed once a packaged version
  exists
- **Xcode Command Line Tools** (macOS): `xcode-select --install`
- A **Groq API key** (free tier available at [console.groq.com](https://console.groq.com)) — used for the AI assistant panel

---

## One-time setup

### 1. Build and run the desktop app

```bash
cd robotics-studio-app/robotics-studio
export GROQ_API_KEY="your-key-here"
npm install
npm run tauri dev
```

The app window should open. First launch compiles Rust dependencies and may
take a few minutes.

### 2. Initialize the ROS environment

In the app's toolbar, click **🚀 Initialize ROS Environment**.

This will:
- Build a Docker image with ROS 2 and Gazebo (first run only — several
  minutes; you'll see live build progress in the app)
- Start the container
- Automatically launch the background ROS service the app talks to

You'll know it's done when the log shows **✅ ROS environment ready** and
the toolbar's connection indicator turns green (**● ROS Connected**).

On future launches, this step is much faster — it reuses the already-built
image and container instead of rebuilding from scratch.

### 3. (Optional) Create a test ROS package

The initialized environment starts with an empty workspace. To have
something to build and run, open a terminal into the running container:
```bash
docker exec -it robotics-studio-ros2 bash
```
Wait — actually easier: use the app itself. Click **Open Folder**, and once
you have a package to work with, **Build** will pick it up automatically.
If you don't have one yet, ask whoever shared this app with you for a
starter package, or create one via the container terminal above:
```bash
source /opt/ros/jazzy/setup.bash
cd /workspace/src
ros2 pkg create --build-type ament_python my_test_pkg
```

---

## Using the app

- **🚀 Initialize ROS Environment** — one-time (per machine) setup; sets
  everything up automatically. Safe to click again later — it won't rebuild
  if already set up, it'll just make sure things are running.
- **Open Folder** — browse and edit any files (not necessarily the ROS
  workspace — this can be any folder you want to edit).
- **Build** — runs `colcon build` inside the container, streams output live.
- **Start Sim / Stop Sim / Reset Sim** — launches a two-robot Gazebo demo
  world (`diff_drive`), visualized live in the **3D View** tab at the bottom.
- **ROS Log tab** — live feed of ROS 2 node/topic activity.
- **AI Assistant (right panel)** — ask questions about your open file,
  recent ROS activity, or build output. It only knows what's visible in
  the app right now (open file + recent logs), not your whole filesystem.

### A concrete first thing to try

1. Click **🚀 Initialize ROS Environment** and wait for it to finish.
2. Create or open a test package (see step 3 above).
3. Click **Build** — confirm it builds successfully.
4. Click **Start Sim**, switch to the **3D View** tab.
5. Open a terminal into the container (`docker exec -it robotics-studio-ros2 bash`) and run:
   ```bash
   ros2 topic pub /model/vehicle_blue/cmd_vel geometry_msgs/msg/Twist "{linear: {x: 0.3}}"
   ```
6. Watch the blue box move in the 3D view.
7. Ask the AI assistant: *"what ROS topics are currently active?"* — it
   should answer from live data, not a guess.

---

## Known limitations (please break things and tell me what you find)

- Only tested with the built-in `diff_drive` demo world — no custom world/
  robot loading yet.
- 3D view shows placeholder boxes, not real robot geometry (no URDF mesh
  loading yet).
- ROS↔three.js coordinate mapping is a first-pass approximation — rotation
  direction hasn't been rigorously verified.
- Sending simulation commands (like the `cmd_vel` example above) still
  requires a manual container terminal for now — no in-app way to do this yet.
- AI assistant requires a Groq API key set as an environment variable before
  launching the app — no in-app settings UI yet.
- First-time setup (Initialize ROS Environment) can take several minutes —
  this is expected, not a hang, as long as the progress log is still updating.

---

## If something breaks

Check, in order:
1. Is Docker Desktop actually running? (Look for its icon/menu bar app)
2. Is the container running? (`docker ps` — look for `robotics-studio-ros2`)
3. Is the app's dev console (Inspect Element → Console) showing an error?
4. Is the terminal running `npm run tauri dev` showing a Rust-side error?
5. Try clicking **Initialize ROS Environment** again — it's safe to re-run
   and may resolve a stuck state.

If none of that helps, note exactly what you clicked, what you expected,
and what happened instead — that's the most useful thing you can hand back.
