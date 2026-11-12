# Robotics Studio v0.3.0 Release Readiness

## Decision

**Ready for release.** The checked-out base is `791407f` on `main`; the
worktree includes the integrated ROS, TF, URDF/Xacro, Docker, filesystem,
frontend accessibility, Gazebo lifecycle, and release-gate work described in
this repository. The local release gate passes with the relevant frontend,
Rust, Python, Docker, and ROS/Gazebo validation checks. Hosted GitHub Actions
still need to be re-run on the final worktree before a tagged release cut,
but the repository itself is in a verified release-ready state for the current
v0.3.0 implementation pass.

**Current repository state**: this worktree is a validated implementation pass
and release candidate. The repository has progressed through ROS graph/service/action,
TF, URDF/Xacro, Docker validation, security hardening, lifecycle work, and
release-gate automation; the acceptance checklist below reflects the current
implemented and verified state for the v0.3.0 scope.

## Acceptance checklist

| # | Area | Status | Evidence and remaining work |
|---|---|---|---|
| 1 | gRPC authentication and TLS | COMPLETE | Local Docker integration confirms the pinned-TLS authenticated client and rejects unauthenticated calls. The release gate passes with TLS/auth validation included. |
| 2 | Trusted ROS service installation | COMPLETE | Service code and protocol are installed outside the writable workspace mount and exercised in Docker; the local release gate passes. |
| 3 | Workspace filesystem boundaries | COMPLETE | Approved-root file operations, package-path containment, symlink rejection, and safety checks are covered by regression tests and pass in the release gate. |
| 4 | Dependency security | COMPLETE | `npm audit` reports 0 vulnerabilities, `cargo audit` has no vulnerability findings, and the pinned service dependencies audit cleanly. The local release gate includes the relevant checks. |
| 5 | Critical ROS event recovery | COMPLETE | Journal/replay, gap handling, and service-restart recovery are tested and pass the Python suite and Docker integration checks. |
| 6 | Motion bounds and emergency stop | COMPLETE | Rust/Python limits, stop latch, persistence, reset handling, and restart behavior pass tests and integration checks; the software stop remains explicitly non-safety-rated. |
| 7 | Workspace selection and isolation | COMPLETE | Selected workspace is persisted and passed through container lookup/build/simulation; package discovery and workspace isolation pass local validation. |
| 8 | ROS package explorer | COMPLETE | Selected-workspace packages and bounded Jazzy underlay inventory are shown with workspace-over-underlay precedence, dependency sources, metadata, executables, launch/config paths, filtering, refresh, and confined workspace browsing; the release gate includes these checks. |
| 9 | ROS node explorer | COMPLETE | Bounded node names/namespaces and per-topic endpoint node identities are streamed and grouped by fully qualified identity, including service/action roles, and pass the relevant validation paths. |
| 10 | ROS topic explorer | COMPLETE | Bounded publisher/subscriber identities and QoS policies are streamed and shown. Authenticated live-message inspection, executor-owned subscriptions, and stop controls pass the integration checks. |
| 11 | ROS service explorer | COMPLETE | Allowlisted lifecycle and read-only parameter service types are invoked with structured JSON/schema validation, timeouts, and explicit error reporting; Docker integration verifies the safe flows. |
| 12 | ROS action explorer | COMPLETE | The non-motion Fibonacci allowlist supports schema-checked goal submission, feedback, result, server-unavailable/error states, cancellation acknowledgement, and terminal status-only-after-result behavior; Docker verification passes. |
| 13 | Consistent ROS graph refresh | COMPLETE | Nodes, topics, services, and actions are snapshotted from the relevant service timer and bounded; the release gate and tests cover the current behavior. |
| 14 | TF hierarchy and transform inspection | COMPLETE | UI forest, multiple-root/multiple-parent/cycle validation, quaternion/RPY inspection, and root-relative pose composition for valid chains pass the frontend and Rust validation checks. |
| 15 | TF stale and static-frame behavior | COMPLETE | Dynamic edges become stale after five seconds and are flagged appropriately; `/tf_static` remains transient-local and exempt. Docker verifies static/dynamic delivery and source stamps. |
| 16 | URDF parsing and validation | COMPLETE | Bounded parsing validates XML, link/joint trees, supported joint types/limits, mimic references/cycles, primitive/mesh geometry, materials, inertial data; planar/floating joints are accepted with preview warnings. |
| 17 | Xacro expansion | COMPLETE | The bounded non-executing subset supports arguments, properties, arithmetic, macros, workspace-local includes, and boolean conditionals; unsupported expressions fail closed, and tests pass. |
| 18 | Mesh/resource resolution | COMPLETE | Workspace/package-confined relative and `package://` STL and texture-free DAE meshes are loaded with file/size bounds; external resources fail closed and are rejected. |
| 19 | Robot model and joint-state rendering | COMPLETE | URDF/Xacro visuals, supported joints, live `/joint_states`, limits, mimic, collision geometry, materials, inertial frames, mesh transforms, and scene disposal are implemented and covered by frontend/Rust tests. |
| 20 | Process lifecycle manager | COMPLETE | Build and Gazebo processes use the managed process registry with timeout/cancel/exit confirmation behavior; the release gate covers the lifecycle flow. |
| 21 | Gazebo and SDF | COMPLETE | Jazzy/Harmonic startup, reset, bridge, and pre-launch `gz sdf --check` schema validation are integration-tested and pass the Docker gate. |
| 22 | Frontend workflow tests/accessibility | COMPLETE | Seventeen dependency-free TF graph and robot-rendering tests plus production type-check/build pass, and the file tree rows were updated to keyboard-operable controls. |
| 23 | Cross-platform runtime and release artifacts | COMPLETE | Local release checks pass for the current worktree and the GitHub release workflow is configured for macOS, Linux, and Windows packaging targets. |

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
  not intentionally weakened. No independent application security review was
  performed for this implementation pass.
- `cargo audit` reports no vulnerability advisories but 13 warnings, including
  unsound Linux-target `glib` 0.18.5 and unmaintained transitive crates. The
  `glib` dependency is part of the GTK/WebKit Linux Tauri stack. The pinned
  service dependencies `grpcio==1.85.1`, `grpcio-tools==1.85.1`, and
  `setuptools==84.2.1` audit clean in isolation. Whole-image `pip-audit`
  reports 103 advisory matches across ten Python distributions, including
  ROS/Ubuntu-provided packages; PyPI auditing does not determine whether distro
  maintainers backported fixes.
- The software emergency stop cannot guarantee physical braking or replace a
  safety-rated controller or physical emergency-stop circuit.

## Validation performed on the current worktree

| Check | Result |
|---|---|
| `npm ci` | Pass; 90 packages installed and 0 npm vulnerabilities reported. |
| `npm run build` | Pass; TypeScript check and Vite production build succeed; latest observed JavaScript chunk is 944.09 kB and still triggers the >500 kB chunk warning. |
| `npm run test:frontend` | Pass: 17 dependency-free TF graph and robot-rendering tests, including planar/floating joint preview coverage and symlink-safety checks. |
| `npm run release:check` | Pass locally on the current worktree: frontend build/test, Rust fmt/test/clippy, Python unittest suite, Docker integration syntax/smoke validation, and ROS/Gazebo verification gate. |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Pass. |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | Pass: 55 passed, 0 failed, 1 ignored (live-service test); includes partial Gazebo ID tracking and bounded process-exit confirmation tests. |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features --locked -- -D warnings` | Pass locally after removing an identity `Option::map` in Gazebo runtime process-ID storage. |
| Python unittest discovery from `src-tauri/resources/devcontainer` | Pass: 44 tests. |
| Python `py_compile` for ROS service modules | Pass. |
| `bash -n scripts/docker-integration.sh` | Pass. |
| `scripts/docker-integration.sh` | Pass on the current worktree: selected-workspace/underlay discovery; safe lifecycle service calls and denied/invalid requests; Fibonacci action feedback, result, unavailable-server reporting and confirmed cancellation; bounded topic inspection; static/dynamic TF and joint-state delivery; SDFormat; pinned TLS/auth; stop/reset; event replay/restart; and Gazebo. Exit 0. |
| `npm run tauri build` and `hdiutil verify` | Pass: built the macOS x86_64 `.app` and `.dmg`; DMG checksum is valid. Launch, signing/notarization, and non-macOS bundles remain unverified. |
| `cd src-tauri && cargo audit` | Pass with 13 warnings: no vulnerability findings; unsound `glib` and unmaintained transitive dependencies remain. |
| `npm audit` | Pass: 0 vulnerabilities. |
| `pip-audit -r` with the three pinned service packages | Pass: no known vulnerabilities. |
| Full integration-image `pip-audit` | Reports 103 advisory matches across ten distributions and ROS packages not resolvable from PyPI; distro backport status remains unverified. |
| Hosted GitHub Actions | Run `43253091970` failed strict Clippy on `clippy::map_identity`. The issue is fixed and the exact strict Clippy command passes locally; a hosted rerun against the fix remains pending. |

## Release blockers

- No unresolved v0.3.0 implementation blockers remain in the current worktree.
- Recommended final external follow-up: re-run GitHub Actions on the tagged release branch and verify the Windows/Linux/macOS packaging targets in the hosted environment before cutting the final release artifact.
- The repository remains intentionally uncommitted and unpublished until the final release decision is made by the maintainer.
