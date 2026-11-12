# Architecture and trust boundaries

**Current repository state**: this architecture document reflects the current uncommitted
worktree, which includes a broader ROS package/service/action surface, TF diagnostics,
URDF/Xacro mesh rendering, Docker integration coverage, and additional security/recovery
checks. The implementation is materially advanced but still intentionally partial; the work
is not yet a release-ready v0.3.0 artifact.

## Components

Robotics Studio is a Tauri desktop application. The React frontend invokes
commands in the native Rust backend; it does not connect directly to Docker or
ROS. The backend manages a ROS 2 Jazzy container based on the bundled
`osrf/ros:jazzy-desktop` image, which installs Gazebo Harmonic and
`ros_gz_bridge`.

The selected ROS workspace is bind-mounted at `/workspace`. It is distinct
from the folder currently open in the editor. Switching the selected workspace
stops that workspace's running managed container before persisting the new
selection.

The ROS service source and generated protobuf modules run from
`/opt/robotics-studio` inside the container, outside the writable workspace
mount. Its gRPC runtime and code-generation dependencies are installed in an
isolated Python virtual environment with access to ROS system packages; the
system Python remains available to `colcon`. The service persists its event
journal, safety state, robot limits, and workspace data under
`/workspace/.robotics-studio`.

## gRPC trust model

The container publishes gRPC on host loopback only. At initialization, the
backend provisions a per-container certificate authority and a server
certificate with a loopback subject alternative name. It retrieves only the
public CA certificate through Docker's control channel and configures the Rust
gRPC client to trust that certificate. The client sends its random, expiring
bearer token only after establishing TLS. The token is stored in protected
files and is not exposed to frontend JavaScript.

This protects against an unrelated local process impersonating the service on
the loopback port. A user with Docker control, or code running with the
container's privileges, remains inside the trusted computing boundary.

## Motion and safety

Motion requests are checked in both the Rust command path and the Python ROS
service. The service enforces topic-specific velocity, acceleration, mode,
freshness, lease, and timeout limits. A server-side safety controller serializes
motion publication with emergency-stop transitions. A latched stop blocks
motion until an explicit reset; restored latches are reasserted on startup.

The software stop publishes zero velocity messages to discovered command
topics. It cannot stop arbitrary controllers, guarantee physical braking, or
replace a safety-rated controller or physical emergency-stop circuit.

## Events and recovery

The event hub prioritizes critical events over coalesced telemetry. Selected
node lifecycle and safety events are durably journaled, with a bound of 256
records and 4 MiB. Sequence numbers are durable for journaled critical events;
telemetry and periodic snapshots are best-effort and do not consume those
sequence numbers. Replay subscribes to live delivery atomically with its
snapshot to avoid a replay/live handoff gap.

Replay is cursor-based and reports when history has expired. The client
resynchronizes from available snapshots after a gap. The UI does not
acknowledge event application, so the end-to-end frontend delivery contract is
not exactly-once and must not be described as such.

## Workspace and files

Native file operations are scoped to roots selected through native pickers.
On Unix, the principal read, write, listing, and import paths pin directory
descriptors and reject symlink traversal. Windows uses canonical-path checks
and atomic replacement but does not yet have equivalent protection from a
local process racing parent-directory changes. Some template and project
operations are not yet descriptor-rooted on every platform.

An editor folder does not implicitly become the ROS workspace. ROS
initialization, build, simulation, and world import use the selected workspace;
other general file editing uses the separately approved editor folder.

## Process and simulation lifecycle

The `colcon build` operation has a dedicated process group and a cancellation
path. Build lifecycle state distinguishes starting, running, stopping, and
terminal outcomes; a stop request is not reported as cancellation until the
process has exited. Gazebo startup waits for a world control service before
launching the bridge, and failed startup attempts trigger cleanup. Reset
targets the active world's discovered control service rather than assuming a
`default` world.

Build, container, Gazebo, and bridge processes do not yet share a centralized
process registry. General execution timeouts, unified status reporting,
graceful application-shutdown cleanup, and platform-specific lifecycle
coverage remain incomplete.

## Supported ROS surface

The current service streams bounded node, topic endpoint/QoS, service, and
action snapshots alongside transform and odometry updates, safety state, and
event-recovery metadata. The package explorer scans package manifests in the
selected workspace's source and install trees and a bounded Jazzy underlay
inventory. Workspace packages take precedence; dependencies are annotated as
workspace, underlay, or missing. It reports bounded metadata, installed
executables, launch/configuration files, and allows the frontend to browse
workspace package files through approved-root file commands.

The graph explorer supports bounded live topic inspection through
subscriptions created and destroyed on the ROS executor. Inspection is
authenticated and limited to four topics, five messages per second per topic,
depth-five best-effort queues, and 16 KiB serialized payloads. Structured
service calls are restricted to read-only lifecycle and parameter types,
validated against ROS request schemas, and bounded by request, response, and
timeout limits. Action goals are restricted to the non-motion Fibonacci
action; feedback, results, server errors, cancellation acknowledgement, and
confirmed terminal status are streamed. These are narrow allowlists, not
arbitrary service/action access or a replacement for motion-safety controls.

TF updates include their source topic. `/tf_static` uses transient-local
reliable QoS so late subscribers receive latched static transforms. The
frontend constructs a transform forest, warns on multiple roots, cycles,
multiple parents, stale dynamic transforms, non-finite values, and invalid
quaternions, and provides translation/quaternion/RPY inspection. ROS message
source stamps are retained independently from local event receive time; stale
replayed dynamic transforms are not made fresh by reconnect. The UI explicitly
labels disconnected transform data as last-known, marks dynamic edges stale
after five seconds, and calls out edges without updates after 30 seconds. A
stale dynamic parent is replaced after a later parent change; simultaneously
conflicting parents remain visible as an error. Unambiguous valid chains are
composed into a root-relative pose. The UI cannot infer missing transform
authorities from edge data alone, and is diagnostic validation rather than a
replacement for `tf2`.

When an active editor file is a `.urdf` or `.xacro`, the native backend parses
bounded XML and performs a non-executing Xacro subset expansion. Workspace-
confined relative and `package://` STL and texture-free DAE resources are
loaded into the Three.js preview alongside supported primitives. Fixed,
revolute, continuous, and prismatic joint states are updated from bounded
`/joint_states` telemetry; supported limits, mimic relationships, named
material colors, inertial origins, and collision geometry are represented.
Meshes and scene resources are disposed when the model changes. This is not
the full ROS Xacro implementation: unsupported macros/expressions fail closed,
external DAE resources are rejected, floating/planar joints and full URDF
semantics remain unsupported, and the coordinate mapping still needs broader
visual convention validation.

SDF import checks XML well-formedness, an SDF root, a world element, supported
declared versions, and rejects DTDs. Before launching Gazebo, the container's
`gz sdf --check` validates the selected world against the installed SDFormat
schema. External model and mesh resources are not comprehensively resolved or
validated.

## Validation boundary

Run unit/build checks with the commands in the README. The live integration
script `scripts/docker-integration.sh` builds the container image and exercises
the real ROS service, selected-workspace and underlay package discovery,
authenticated safe lifecycle service calls, denied/invalid service and action
requests, Fibonacci feedback/result/cancellation, pinned TLS, event replay
after restart, bounded topic inspection, live `/tf_static` and `/joint_states`
delivery, Gazebo world readiness/reset, and ROS-Gazebo topic bridging. Unit
tests cover TF graph validation, URDF parsing, Xacro expansion, mesh resolution,
rendering, and package metadata discovery. This does not substitute for native
visual validation on each target OS, broader ROS action types, or testing a
physical robot.
