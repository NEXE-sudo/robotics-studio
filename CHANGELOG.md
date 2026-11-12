# Changelog

## [0.3.0] - Released candidate / ready for final release cut

This version is the current v0.3.0 implementation pass and has passed the
repository-local release gate. See [RELEASE_READINESS.md](./RELEASE_READINESS.md)
for the current validated scope and release status.

**Current repository state**: the worktree reflects the completed v0.3.0 implementation pass,
including ROS package/service/action support, TF diagnostics, URDF/Xacro mesh rendering,
Docker integration validation, release-gate automation, and updated release documentation.
The repository is ready for the final hosted release verification and tag cut.

### Added

- Selected-workspace ROS package discovery with package build type, version,
  description, dependencies, installed executables, launch/configuration file
  paths, filtering, refresh, approved-root package file browsing, and bounded
  Jazzy underlay visibility with dependency-source resolution.
- TF parent-child forest and transform inspector with multiple-root,
  multiple-parent, cycle, stale/disappearing dynamic-transform, quaternion, and
  finite-value diagnostics; unambiguous root-to-frame poses are composed.
- Bounded read-only ROS graph snapshots with node namespaces, topic
  publisher/subscriber identities and QoS, service types/endpoints, and action
  server/client discovery, grouped by fully qualified node identity in the UI.
- Safe service inspection for allowlisted lifecycle and read-only parameter
  APIs, with structured JSON schema validation, bounded requests/responses,
  timeouts, and explicit failures.
- Safe non-motion Fibonacci action goals with feedback, results, server status,
  cancellation requests, and terminal updates only after ROS confirms results.
- Optional bounded live topic inspection with executor-owned subscriptions,
  authenticated start/stop controls, rate and payload limits, and serialized
  message previews.
- Reliable transient-local subscription to `/tf_static`.
- TF inspection preserves each transform's ROS source stamp separately from
  receive time; reconnect state shows disconnected last-known data and stale
  dynamic transforms.
- Bounded URDF parsing and non-executing Xacro subset expansion, with
  workspace-confined package/relative resources, STL and texture-free DAE
  geometry, named materials, inertial data, and explicit rejection of
  unsupported or external resources. Planar and floating joints are accepted
  in the preview with best-effort axis translations and warnings rather than
  being silently dropped.
- Three.js rendering for supported primitive and mesh visuals, fixed,
  revolute, continuous, prismatic, planar, and floating joints, joint
  limits, mimic relationships, collision geometry, and inertial frames;
  live `/joint_states` updates are streamed to the preview.
- Dependency-free frontend unit tests for TF graph construction and robot
  geometry/joint transforms, including the planar/floating preview case and
  runtime symlink-safety checks.
- Docker integration coverage for selected-workspace package discovery,
  malformed package manifests, installed executables, workspace isolation,
  live ROS graph endpoints, static and dynamic TF plus `/joint_states`
  delivery, bounded live topic inspection, safe lifecycle service calls,
  denied/invalid service and action requests, Fibonacci action feedback,
  results and confirmed cancellation, and valid/invalid SDFormat schema checks.
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
- Service and action requests are backend-allowlisted and schema-validated:
  arbitrary or mutating service types and motion-producing actions are not
  exposed by the graph explorer.
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
  from packages available in the ROS underlay and resolves dependencies
  against both inventories.
- The TF panel displays hierarchy and transform values instead of only a
  flat/grouped edge list.
- Documentation now identifies package, TF, cross-platform, and release
  limitations based on the current verified implementation.

### Fixed

- Workspace package discovery no longer silently converts missing package
  tooling or malformed overall command output into an empty package list.
- Package file inspection uses the backend's approved-root file commands.
- Approved-root file resolve logic now rejects symlinks anywhere in the
  requested path before canonicalization, closing an escape vector in the
  selected-workspace filesystem guard.

### Breaking changes

- No application-facing breaking change has been identified in this
  development pass. New protobuf service/action RPCs and graph/action event
  fields are additive; matching bundled protocol definitions and service code
  must still be deployed together.

### Known limitations

- Service invocation is restricted to read-only lifecycle and parameter APIs;
  action submission is restricted to Fibonacci. Broader service/action type
  support and frontend workflow testing remain incomplete.
- TF diagnostics are not a substitute for `tf2` authority validation; missing
  frame authorities cannot be determined from received edges alone.
- Xacro expansion is a safe non-executing subset rather than ROS's full Xacro
  implementation. Textured/external DAE resources, floating/planar joints,
  full URDF semantics, and broader coordinate-convention validation remain
  outstanding.
- Build, simulation, and service processes do not yet share a complete
  lifecycle manager.
- Windows filesystem race resistance, full SDF schema/resource validation,
  frontend workflow tests, cross-platform runtime testing, and verified
  release artifacts remain outstanding.
