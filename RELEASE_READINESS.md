# Robotics Studio v0.3.0 Release Readiness

## Decision

**Not ready for release.** The checkout starts at `b907550` on `main`; the
worktree now contains uncommitted ROS package, TF, graph, SDF-validation, and
initial URDF-preview improvements.
The hosted CI run for `b907550` passed all three jobs, but it does not include
the current uncommitted changes. Several major v0.3.0 acceptance criteria
remain incomplete. No commit, push, tag, or release was created.

## Acceptance checklist

| # | Area | Status | Evidence and remaining work |
|---|---|---|---|
| 1 | gRPC authentication and TLS | PARTIAL | Local Docker integration confirms the pinned-TLS authenticated client and rejects unauthenticated calls. Cross-platform and packaged-app transport testing remains. |
| 2 | Trusted ROS service installation | PARTIAL | Service code and protocol are installed outside the writable workspace mount and exercised in Docker. Upgrade/migration paths need broader platform testing. |
| 3 | Workspace filesystem boundaries | PARTIAL | Unix descriptor-rooted file operations and package-path containment have regression tests. Windows race resistance and several other operations remain incomplete. |
| 4 | Dependency security | PARTIAL | Current local `npm audit` reports no vulnerabilities and `cargo audit` reports no vulnerability findings, with 13 allowed warnings for unmaintained/unsound transitive dependencies. Hosted CI has not run on this worktree. |
| 5 | Critical ROS event recovery | PARTIAL | Journal/replay, gap handling, and service-restart recovery are tested. The UI does not acknowledge event application and telemetry is best-effort. |
| 6 | Motion bounds and emergency stop | PARTIAL | Rust/Python limits, stop latch, persistence, and restart behavior have tests and integration checks. This is software-only and not safety-rated. |
| 7 | Workspace selection and isolation | PARTIAL | Selected workspace is persisted and passed to container lookup/build/simulation. Package discovery now filters to source/install trees inside that mounted workspace; full operation/platform matrix remains unverified. |
| 8 | ROS package explorer | PARTIAL | Selected-workspace package metadata, build type, dependencies, executable outputs, launch/config files, filtering, refresh, and file browsing are implemented and tested. Underlay packages, dependency resolution, and full package content workflows remain out of scope. |
| 9 | ROS node explorer | PARTIAL | Bounded node names/namespaces and per-topic endpoint node identities are streamed and shown. Full node-specific endpoint grouping and lifecycle details remain. |
| 10 | ROS topic explorer | PARTIAL | Bounded publisher/subscriber identities and QoS policies are streamed and shown. Safe live-message inspection and subscription controls are not implemented. |
| 11 | ROS service explorer | PARTIAL | Bounded service names/types and discovered server/client nodes are shown read-only. Structured invocation, validation, timeout, and cancellation remain absent to avoid bypassing safety controls. |
| 12 | ROS action explorer | PARTIAL | Action names/types and discovered server/client nodes are shown read-only. Goal submission, feedback/results, and cancellation are not implemented. |
| 13 | Consistent ROS graph refresh | PARTIAL | Nodes, topics, services, and actions are snapshotted from one service timer and bounded. Automatic client subscriptions and full graph transactionality remain absent. |
| 14 | TF hierarchy and transform inspection | PARTIAL | A UI forest, translation/quaternion/RPY inspector, multiple-root/multiple-parent/cycle checks, and pure unit tests are implemented. It is not authoritative `tf2` validation. |
| 15 | TF stale and static-frame behavior | PARTIAL | Dynamic edges become stale after five seconds; `/tf_static` uses reliable transient-local QoS and is exempt. A live Docker integration publishes and verifies a static TF edge. Missing authorities cannot be inferred from edge data. |
| 16 | URDF parsing and validation | PARTIAL | A bounded parser rejects malformed/unsafe XML, validates link/joint-tree structure and primitive dimensions, and reports unsupported mesh visuals. Full URDF semantics, Xacro, and resource validation remain. |
| 17 | Xacro expansion | NOT STARTED | No controlled ROS Xacro expansion workflow exists. |
| 18 | Mesh/resource resolution | NOT STARTED | `package://`, relative model paths, and mesh formats are not resolved for rendering. |
| 19 | Robot model and joint-state rendering | PARTIAL | Active `.urdf` files render box/cylinder/sphere visuals and supported joint transforms; bounded `/joint_states` updates are covered by live Docker integration. No mesh/Xacro, coordinate-convention verification, or visual desktop test yet. |
| 20 | Process lifecycle manager | PARTIAL | Builds and process-registry data have dedicated support, but build, Docker, Gazebo, bridge, and ROS service processes do not share reliable lifecycle, timeout, cancellation, and shutdown handling. |
| 21 | Gazebo and SDF | PARTIAL | Jazzy/Harmonic startup, reset, bridge, and pre-launch `gz sdf --check` schema validation are integration-tested. Comprehensive external resource resolution and validation remain absent. |
| 22 | Frontend workflow tests/accessibility | PARTIAL | Dependency-free TF graph and robot-rendering tests plus production type-check/build pass. React workflow tests, accessibility review, and coverage of workspace/ROS/safety UI remain. |
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
- The software emergency stop cannot guarantee physical braking or replace a
  safety-rated controller or physical emergency-stop circuit.

## Validation performed on the current worktree

| Check | Result |
|---|---|
| `npm run build` | Pass; Vite reports an existing large JavaScript chunk warning (~820 kB). |
| `npm run test:frontend` | Pass: 7 TF graph and robot-description tests (Node 26.5.0). |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Pass. |
| `cargo check --manifest-path src-tauri/Cargo.toml --locked` | Pass. |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | Pass: 39 passed, 1 ignored (live-service test). |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features --locked -- -D warnings` | Pass. |
| `python3 -B -m unittest -v test_server_core` from `src-tauri/resources/devcontainer` | Pass: 42 tests, including package discovery/isolation and action endpoint parsing. |
| Python `py_compile` for service and tests | Pass. |
| `bash -n scripts/docker-integration.sh` | Pass. |
| `scripts/docker-integration.sh` | Pass: selected-workspace package discovery, live graph node/topic/service/action endpoints, `/tf_static` and `/joint_states` delivery, valid/invalid SDFormat checks, pinned TLS/auth, stop/reset enforcement, replay and safety history after restart, Gazebo readiness/reset and topic bridge. Cleanup exited 0 and left no integration container or temporary workspace. |
| Protocol-copy comparison | Pass: Rust and container protobuf definitions match. |
| `cargo audit --file src-tauri/Cargo.lock` | Pass with 13 allowed warnings: no vulnerability findings; remaining advisories are unmaintained/unsound transitive dependencies disclosed in the previous readiness review. Patched Rustix advisory was fixed by updating to 1.1.6. |
| `npm audit --audit-level=moderate` | Pass: 0 vulnerabilities. |
| Hosted GitHub Actions | Pass for baseline commit `b907550` in run `43087023826` (frontend, Rust/integration, and Python jobs); not run against this worktree's changes. |

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
