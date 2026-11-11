# Robotics Studio v0.3.0 Release Readiness

## Decision

**Not ready for release.** The worktree is based on `307f3ef` on `main` and
contains uncommitted ROS graph, TF integration, URDF preview, build lifecycle,
and package-discovery test changes. Local validation and Docker integration
pass, but substantial v0.3.0 acceptance criteria remain incomplete. Hosted CI
has not run against this worktree. No commit, push, tag, or release was
created.

## Acceptance checklist

| # | Area | Status | Evidence and remaining work |
|---|---|---|---|
| 1 | gRPC authentication and TLS | PARTIAL | Local Docker integration confirms the pinned-TLS authenticated client and rejects unauthenticated calls. Cross-platform and packaged-app transport testing remains. |
| 2 | Trusted ROS service installation | PARTIAL | Service code and protocol are installed outside the writable workspace mount and exercised in Docker. Upgrade/migration paths need broader platform testing. |
| 3 | Workspace filesystem boundaries | PARTIAL | Unix descriptor-rooted file operations and package-path containment have regression tests. Windows race resistance and several other operations remain incomplete. |
| 4 | Dependency security | PARTIAL | Current local `npm audit` reports no vulnerabilities. `cargo audit` reports no vulnerability findings and 13 warnings, including an unsound `glib` version and unmaintained transitive crates. `pip-audit` reports no known vulnerabilities in the integration image's isolated, pinned PyPI-installed packages; the full image inventory includes ROS distro packages not resolvable from PyPI. Hosted CI has not run on this worktree. |
| 5 | Critical ROS event recovery | PARTIAL | Journal/replay, gap handling, and service-restart recovery are tested. The UI does not acknowledge event application and telemetry is best-effort. |
| 6 | Motion bounds and emergency stop | PARTIAL | Rust/Python limits, stop latch, persistence, and restart behavior have tests and integration checks. This is software-only and not safety-rated. |
| 7 | Workspace selection and isolation | PARTIAL | Selected workspace is persisted and passed to container lookup/build/simulation. Package discovery now filters to source/install trees inside that mounted workspace; full operation/platform matrix remains unverified. |
| 8 | ROS package explorer | PARTIAL | Selected-workspace package metadata, build type, dependencies, executable outputs, launch/config files, filtering, refresh, and file browsing are implemented and tested. Underlay packages, dependency resolution, and full package content workflows remain out of scope. |
| 9 | ROS node explorer | PARTIAL | Bounded node names/namespaces and per-topic endpoint node identities are streamed and grouped in the UI by fully qualified node identity, including service/action roles. Full ROS lifecycle-state details remain. |
| 10 | ROS topic explorer | PARTIAL | Bounded publisher/subscriber identities and QoS policies are streamed and shown. Safe live-message inspection and subscription controls are not implemented. |
| 11 | ROS service explorer | PARTIAL | Bounded service names/types and discovered server/client nodes are shown read-only. Structured invocation, validation, timeout, and cancellation remain absent to avoid bypassing safety controls. |
| 12 | ROS action explorer | PARTIAL | Action names/types and discovered server/client nodes are shown read-only. Goal submission, feedback/results, and cancellation are not implemented. |
| 13 | Consistent ROS graph refresh | PARTIAL | Nodes, topics, services, and actions are snapshotted from one service timer and bounded. Automatic client subscriptions and full graph transactionality remain absent. |
| 14 | TF hierarchy and transform inspection | PARTIAL | A UI forest, translation/quaternion/RPY inspector, multiple-root/multiple-parent/cycle checks, and pure unit tests are implemented. It is not authoritative `tf2` validation. |
| 15 | TF stale and static-frame behavior | PARTIAL | Dynamic edges become stale after five seconds; `/tf_static` uses reliable transient-local QoS and is exempt. Docker integration publishes and verifies both static and dynamic TF. Missing authorities cannot be inferred from edge data. |
| 16 | URDF parsing and validation | PARTIAL | A bounded parser rejects malformed/unsafe XML, validates link/joint-tree structure, primitive dimensions, joint limits, and mimic references/cycles, and reports unsupported mesh visuals. Full URDF semantics, Xacro, and resource validation remain. |
| 17 | Xacro expansion | NOT STARTED | No controlled ROS Xacro expansion workflow exists. |
| 18 | Mesh/resource resolution | NOT STARTED | `package://`, relative model paths, and mesh formats are not resolved for rendering. |
| 19 | Robot model and joint-state rendering | PARTIAL | Active `.urdf` files render box/cylinder/sphere visuals and supported joint transforms; limits, mimic joints, and a collision-geometry toggle are supported. Bounded `/joint_states` updates are covered by Docker integration. No mesh/Xacro, coordinate-convention verification, or visual desktop test yet; inertial and named robot-level materials are not rendered. |
| 20 | Process lifecycle manager | PARTIAL | Build states distinguish starting/running/stopping/terminal outcomes; stop requests are only reported as cancellation after process exit. Docker, Gazebo, bridge, and ROS service processes still lack a shared lifecycle manager, generalized timeouts, and application-shutdown cleanup. |
| 21 | Gazebo and SDF | PARTIAL | Jazzy/Harmonic startup, reset, bridge, and pre-launch `gz sdf --check` schema validation are integration-tested. Comprehensive external resource resolution and validation remain absent. |
| 22 | Frontend workflow tests/accessibility | PARTIAL | Ten dependency-free TF graph and robot-rendering tests plus production type-check/build pass. React workflow tests, accessibility review, and coverage of workspace/ROS/safety UI remain. |
| 23 | Cross-platform runtime and release artifacts | BLOCKED | CI has Linux checks and a release workflow matrix for macOS/Linux/Windows, but no current packaged artifacts or runtime verification on every platform were produced. Signing/notarization and upgrade verification remain. |

## Security and safety notes

- Package discovery now runs from the application-installed `server_core.py`
  in the selected container. It scans only that container's `/workspace/src`
  and `/workspace/install`, bounds package/file counts and manifest sizes,
  rejects DTD/entity declarations, reports malformed manifests, and skips
  symlinked package directories. Rust canonicalizes returned relative paths
  and checks that they remain under the selected host workspace before
  exposing them to the approved-root file browser.
- The package command no longer interpolates a host-controlled path into a
  shell script. The active container is selected from the validated workspace
  label and uses its fixed `/workspace` mount.
- TF validation is diagnostic. It does not replace ROS `tf2`, identify
  transform authorities, or make motion-safety guarantees.
- Existing TLS/authentication, emergency-stop, and filesystem controls were
  not intentionally weakened. No new independent security audit was performed
  for this implementation pass.
- The Rust dependency audit reports an unsound `glib` dependency warning in
  addition to unmaintained transitive dependencies. `pip-audit` found no known
  vulnerabilities in `grpcio`, `grpcio-tools`, and `setuptools` as installed
  in an isolated virtual environment in the integration image. The Dockerfile
  pins these PyPI packages without replacing ROS/colcon's system Python
  dependencies. A whole-image pip audit cannot resolve ROS-distributed
  packages such as `action-msgs` from PyPI.
- The software emergency stop cannot guarantee physical braking or replace a
  safety-rated controller or physical emergency-stop circuit.

## Validation performed on the current worktree

| Check | Result |
|---|---|
| `npm ci` | Pass; 91 packages installed. |
| `npm run build` | Pass; Vite reports a large JavaScript chunk warning (~831 kB). |
| `npm run test:frontend` | Pass: 10 TF graph and robot-description tests. |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Pass. |
| `cargo check --manifest-path src-tauri/Cargo.toml --locked` | Pass. |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | Pass: 43 passed, 1 ignored (live-service test). |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features --locked -- -D warnings` | Pass. |
| Python unittest discovery from `src-tauri/resources/devcontainer` | Pass: 43 tests, including inaccessible workspace-tree reporting. |
| Python `py_compile` for service and tests | Pass. |
| `bash -n scripts/docker-integration.sh` | Pass. |
| `scripts/docker-integration.sh` | Pass: selected-workspace package discovery, live graph endpoints, static/dynamic TF and joint-state delivery, SDFormat checks, pinned TLS/auth, stop/reset enforcement, event replay/restart, and Gazebo. Cleanup exited 0; a follow-up check found no integration container or temporary directory. |
| `cargo audit --file src-tauri/Cargo.lock` | Pass with 13 allowed warnings: no vulnerability findings; warnings include unmaintained crates and unsound `glib`. |
| `npm audit --audit-level=moderate` | Pass: 0 vulnerabilities. |
| Python dependency audit | Pass: no known vulnerabilities in the isolated environment's pinned `grpcio==1.85.1`, `grpcio-tools==1.85.1`, `protobuf==7.36.2`, `setuptools==84.2.1`, and `typing_extensions==4.16.0`. The full system pip inventory could not be resolved because it includes ROS distro packages. |
| ROS/colcon Python compatibility | Pass: gRPC uses the isolated Python 3.12 environment, system Python retains `setuptools==68.1.2`, and a representative `colcon build` completes. |
| Hosted GitHub Actions | Not run against this uncommitted worktree. |

## Release blockers

- Complete live ROS node/topic endpoint details and implement service/action
  safe structured invocation/goal monitoring without bypassing the motion
  safety controls. Bounded read-only graph discovery is implemented.
- Validate TF reconnect/disappearance behavior against a live ROS publisher;
  static `/tf_static` delivery is covered by the Docker integration test.
- Complete URDF semantics and coordinate validation; implement Xacro expansion
  and safe package/model resource resolution. The current bounded parser,
  primitive preview, and live joint-state updates are partial support only.
- Unify application-managed process state, failure reporting, timeouts,
  cancellation confirmation, and shutdown cleanup across supported platforms.
- Complete filesystem race hardening for remaining operations and Windows.
- Add frontend workflow tests and conduct accessibility review.
- Run hosted CI on the modified worktree; verify Linux, Windows, macOS Intel,
  and Apple Silicon packaging/runtime paths and test upgrades from v0.2.0.
- Complete external SDF model/mesh resource resolution and validation.
- Re-run current dependency audits and reassess disclosed RustSec advisories.
- Prepare and verify unsigned/signed release artifacts as applicable; do not
  publish a tag or release until explicitly approved.
