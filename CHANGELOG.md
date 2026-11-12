# Changelog

## [0.3.0] - Unreleased

This version is still in development and is not release-ready. See
[RELEASE_READINESS.md](./RELEASE_READINESS.md) for verified scope and blockers.

### Added

- Selected-workspace ROS package discovery with package build type, version,
  description, dependencies, installed executables, launch/configuration file
  paths, filtering, refresh, and approved-root package file browsing.
- TF parent-child forest and transform inspector with multiple-root,
  multiple-parent, cycle, stale dynamic-transform, quaternion, and finite-value
  diagnostics.
- Bounded read-only ROS graph snapshots with node namespaces, topic
  publisher/subscriber identities and QoS, service types/endpoints, and action
  server/client discovery, grouped by fully qualified node identity in the UI.
- Optional bounded live topic inspection with executor-owned subscriptions,
  authenticated start/stop controls, rate and payload limits, and serialized
  message previews.
- Reliable transient-local subscription to `/tf_static`.
- TF inspection preserves each transform's ROS source stamp separately from
  receive time; reconnect state shows disconnected last-known data and stale
  dynamic transforms.
- Bounded URDF parsing for primitive visuals and connected link trees, with
  safe rejection of malformed or unsupported descriptions.
- Three.js rendering for box, cylinder, and sphere visuals and supported
  fixed/revolute/continuous/prismatic joints; live `/joint_states` updates are
  streamed to the preview. Joint limits and mimic relationships are applied,
  and supported collision primitives can be toggled in the preview.
- Dependency-free frontend unit tests for TF graph construction and robot
  geometry/joint transforms.
- Docker integration coverage for selected-workspace package discovery,
  malformed package manifests, installed executables, workspace isolation,
  live ROS graph endpoints, static and dynamic TF plus `/joint_states`
  delivery, bounded live topic inspection, and valid/invalid SDFormat schema
  checks.
- Build-process state and cancellation feedback that distinguishes a stop
  request from confirmed process termination.

### Security and reliability

- Package discovery is restricted to package manifests and install outputs
  within the selected workspace. Discovery bounds scan sizes, avoids symlinked
  package directories, rejects DTD/entity declarations in package manifests,
  and surfaces malformed metadata.
- ROS package paths are returned as relative paths from the container and are
  canonicalized and confined to the selected host workspace before the
  frontend can inspect them.
- ROS package discovery no longer interpolates the host workspace path into a
  shell command.
- Service calls and action goals are intentionally not exposed by the graph
  explorer, avoiding a bypass of the backend's bounded motion/safety path.
- Gazebo validates the selected SDF against the installed SDFormat schema
  before startup; external model/mesh resource validation remains incomplete.
- Workspace package discovery reports inaccessible tree errors rather than
  silently returning an empty package inventory.
- The ROS container isolates its pinned Python gRPC runtime and code-generation
  packages in a virtual environment so ROS/colcon retain their system Python
  dependencies.
- Existing v0.3.0 protections include TLS-pinned authenticated gRPC,
  approved-root filesystem access, bounded motion commands, persistent
  software stop state, critical event replay, and structural SDF checks.

### Changed

- The ROS sidebar distinguishes source packages in the selected workspace
  from packages available in the ROS underlay.
- The TF panel displays hierarchy and transform values instead of only a
  flat/grouped edge list.
- Documentation now identifies package, TF, cross-platform, and release
  limitations based on the current verified implementation.

### Fixed

- Workspace package discovery no longer silently converts missing package
  tooling or malformed overall command output into an empty package list.
- Package file inspection uses the backend's approved-root file commands.

### Breaking changes

- No application-facing breaking change has been identified in this
  development pass. New protobuf graph and joint-state event fields are
  additive for protobuf readers; matching bundled protocol definitions must
  still be deployed together.

### Known limitations

- Structured service/action invocation and monitoring are not implemented;
  graph discovery remains read-only.
- TF diagnostics are not a substitute for `tf2` authority validation; missing
  frame authorities cannot be determined from received edges alone.
- URDF support is a bounded subset: Xacro expansion, safe mesh/model
  resolution, floating/planar joints, and full URDF semantics remain
  outstanding. Coordinate-convention validation remains incomplete.
- Build, simulation, and service processes do not yet share a complete
  lifecycle manager.
- Windows filesystem race resistance, full SDF schema/resource validation,
  frontend workflow tests, cross-platform runtime testing, and verified
  release artifacts remain outstanding.
