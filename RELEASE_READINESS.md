# Robotics Studio v0.3.0 Release Readiness

## Decision

**Not ready for release.** The worktree is based on `59d8467` on `main` and
contains uncommitted ROS package, service/action, Xacro/mesh rendering, TF,
filesystem, and process-related changes. Local validation and the complete
Jazzy/Harmonic Docker integration pass, but substantial v0.3.0 acceptance
criteria remain incomplete. Hosted run
[#43253091970](https://github.com/NEXE-sudo/robotics-studio/actions/runs/43253091970)
failed strict Clippy on an identity `Option::map`; that was fixed locally and
the same strict Clippy command now passes. Hosted CI has not yet been rerun
after the fix. No commit, push, tag, or release was created.

**Current repository state**: this worktree is a release-candidate implementation pass,
not a release artifact. The repository has progressed through ROS graph/service/action,
TF, URDF/Xacro, Docker validation, and documentation updates, yet the remaining acceptance
areas are still partial and must be closed before a v0.3.0 release decision is made.

## Acceptance checklist

| # | Area | Status | Evidence and remaining work |
|---|---|---|---|
| 1 | gRPC authentication and TLS | PARTIAL | Local Docker integration confirms the pinned-TLS authenticated client and rejects unauthenticated calls. Cross-platform and packaged-app transport testing remains. |
| 2 | Trusted ROS service installation | PARTIAL | Service code and protocol are installed outside the writable workspace mount and exercised in Docker. Upgrade/migration paths need broader platform testing. |
| 3 | Workspace filesystem boundaries | PARTIAL | Unix descriptor-rooted file operations and package-path containment have regression tests. Windows race resistance and several other operations remain incomplete. |
| 4 | Dependency security | PARTIAL | `npm audit` reports 0 vulnerabilities. `cargo audit` reports no vulnerability findings but 13 warnings, including unsound Linux-target `glib` 0.18.5 and unmaintained transitive crates; current GTK/WebKit dependencies constrain the Linux stack. The three pinned PyPI service packages audit clean. A whole-image `pip-audit` finds 103 advisory matches across 10 Python distributions in the ROS base image; their Ubuntu/ROS backport status needs upstream package-level review. Hosted CI run `43253091970` ran on the current worktree and failed strict Clippy; the Clippy issue is fixed locally, with hosted rerun pending. |
| 5 | Critical ROS event recovery | PARTIAL | Journal/replay, gap handling, and service-restart recovery are tested. The UI does not acknowledge event application and telemetry is best-effort. |
| 6 | Motion bounds and emergency stop | PARTIAL | Rust/Python limits, stop latch, persistence, and restart behavior have tests and integration checks. This is software-only and not safety-rated. |
| 7 | Workspace selection and isolation | PARTIAL | Selected workspace is persisted and passed to container lookup/build/simulation. Package discovery now filters to source/install trees inside that mounted workspace; full operation/platform matrix remains unverified. |
| 8 | ROS package explorer | PARTIAL | Selected-workspace packages and a bounded Jazzy underlay inventory are shown with workspace-over-underlay precedence and dependency sources (`workspace`, `underlay`, or `missing`). Metadata, executables, launch/config paths, filtering, refresh, and confined workspace file browsing are tested. Full ROS package-index/underlay coverage and all content workflows remain. |
| 9 | ROS node explorer | PARTIAL | Bounded node names/namespaces and per-topic endpoint node identities are streamed and grouped by fully qualified identity, including service/action roles. Available lifecycle service state can be queried through the safe service flow; automatic lifecycle state discovery and refresh remain incomplete. |
| 10 | ROS topic explorer | PARTIAL | Bounded publisher/subscriber identities and QoS policies are streamed and shown. Authenticated live-message inspection is available with executor-owned subscriptions, four-topic/five-Hz/payload bounds, and stop controls. ROS topic type support depends on installed message packages; broader UX and lifecycle testing remain. |
| 11 | ROS service explorer | PARTIAL | Allowlisted lifecycle and read-only parameter service types can be invoked with structured JSON/schema validation, bounded inputs/responses, timeout and explicit error reporting. Docker verifies lifecycle `GetState`, denied mutating types, and invalid fields. Service cancellation and broader workflow testing remain. |
| 12 | ROS action explorer | PARTIAL | The non-motion Fibonacci allowlist supports schema-checked goal submission, feedback, result, server-unavailable/error states, cancellation acknowledgement, and terminal status only after a ROS result. Docker verifies success and cancellation; broader action types, reconnect behavior, and timeout recovery remain. |
| 13 | Consistent ROS graph refresh | PARTIAL | Nodes, topics, services, and actions are snapshotted from one service timer and bounded. Automatic client subscriptions and full graph transactionality remain absent. |
| 14 | TF hierarchy and transform inspection | PARTIAL | UI forest, multiple-root/multiple-parent/cycle validation, quaternion/RPY inspection, and root-relative pose composition for unambiguous valid chains are covered by unit tests. It is not authoritative `tf2` validation. |
| 15 | TF stale and static-frame behavior | PARTIAL | Dynamic edges become stale after five seconds and are flagged as having no updates after 30 seconds; stale dynamic parent edges are replaced after an observed parent change. `/tf_static` uses reliable transient-local QoS and is exempt. Docker verifies static/dynamic delivery and source stamps; live reconnection and frame-authority behavior remain incomplete. |
| 16 | URDF parsing and validation | PARTIAL | Bounded parsing validates XML, link/joint trees, supported joint types/limits, mimic references/cycles, primitive/mesh geometry, materials, and inertial data. Planar/floating joints are accepted with explicit best-effort preview warnings; full URDF semantics remain unsupported. |
| 17 | Xacro expansion | PARTIAL | A bounded non-executing subset supports arguments, properties, arithmetic, macros, workspace-local includes, and boolean conditionals, with cycle/expansion limits and fail-closed unsupported expressions. This is not controlled ROS `xacro` tooling or full Xacro compatibility. |
| 18 | Mesh/resource resolution | PARTIAL | Workspace/package-confined relative and `package://` STL and texture-free DAE meshes are loaded with file/size bounds. External DAE textures/references and broader resource semantics fail closed; visual desktop verification remains. |
| 19 | Robot model and joint-state rendering | PARTIAL | URDF/Xacro visuals, supported joints (including best-effort planar/floating axis translations), live `/joint_states`, limits, mimic, collision geometry, materials, inertial frames, mesh transforms, and scene disposal are implemented and covered by focused tests. Native preview and broader coordinate-convention verification remain unperformed. |
| 20 | Process lifecycle manager | PARTIAL | Build states distinguish starting/running/stopping/terminal outcomes; stop requests are only reported as cancellation after process exit. Docker, Gazebo, bridge, and ROS service processes still lack a shared lifecycle manager, generalized timeouts, and application-shutdown cleanup. |
| 21 | Gazebo and SDF | PARTIAL | Jazzy/Harmonic startup, reset, bridge, and pre-launch `gz sdf --check` schema validation are integration-tested. Comprehensive external resource resolution and validation remain absent. |
| 22 | Frontend workflow tests/accessibility | PARTIAL | Seventeen dependency-free TF graph and robot-rendering tests plus production type-check/build pass. React workflow tests, accessibility review, and coverage of workspace/ROS/safety UI remain. |
| 23 | Cross-platform runtime and release artifacts | PARTIAL | A local macOS x86_64 `.app` and `.dmg` were built successfully. They have not been launch-tested or notarized; macOS Apple Silicon, Linux, and Windows release builds/runtime and v0.2.0 upgrade verification remain. |

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
| `npm run build` | Pass; TypeScript check and Vite production build succeed; latest observed JavaScript chunk is 943.91 kB and still triggers the >500 kB chunk warning. |
| `npm run test:frontend` | Pass: 17 dependency-free TF graph and robot-rendering tests, including planar/floating joint preview coverage and symlink-safety checks. |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Pass. |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | Pass: 52 passed, 0 failed, 1 ignored (live-service test). |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features --locked -- -D warnings` | Pass locally after removing an identity `Option::map` in Gazebo runtime process-ID storage. |
| Python unittest discovery from `src-tauri/resources/devcontainer` | Pass: 44 tests. |
| Python `py_compile` for ROS service modules | Pass. |
| `bash -n scripts/docker-integration.sh` | Pass. |
| `scripts/docker-integration.sh` | Pass: selected-workspace/underlay discovery; safe lifecycle service calls and denied/invalid requests; Fibonacci action feedback, result, unavailable-server reporting and confirmed cancellation; bounded topic inspection; static/dynamic TF and joint-state delivery; SDFormat; pinned TLS/auth; stop/reset; event replay/restart; and Gazebo. Exit 0; no integration container or temporary directory remains. |
| `npm run tauri build` and `hdiutil verify` | Pass: built the macOS x86_64 `.app` and `.dmg`; DMG checksum is valid. Launch, signing/notarization, and non-macOS bundles remain unverified. |
| `cd src-tauri && cargo audit` | Pass with 13 warnings: no vulnerability findings; unsound `glib` and unmaintained transitive dependencies remain. |
| `npm audit` | Pass: 0 vulnerabilities. |
| `pip-audit -r` with the three pinned service packages | Pass: no known vulnerabilities. |
| Full integration-image `pip-audit` | Reports 103 advisory matches across ten distributions and ROS packages not resolvable from PyPI; distro backport status remains unverified. |
| Hosted GitHub Actions | Run `43253091970` failed strict Clippy on `clippy::map_identity`. The issue is fixed and the exact strict Clippy command passes locally; a hosted rerun against the fix remains pending. |

## Release blockers

- Complete automatic lifecycle-state inspection, graph refresh/subscription
  recovery, TF frame disappearance/relationship updates, and transform
  composition/validation against live ROS behavior.
- Keep extending the current best-effort robot-description preview for
  unsupported URDF semantics and finish native packaging/runtime validation
  before a v0.3.0 release decision.
- Expand the intentionally narrow safe service/action allowlists only with
  reviewed non-motion APIs; add service cancellation and frontend workflow
  tests.
- Integrate controlled ROS Xacro tooling or document and approve the current
  non-executing subset as intended product behavior. Complete remaining URDF
  semantics and coordinate-convention verification; textured/external mesh
  resources remain rejected.
- Unify application-managed process state, failure reporting, timeouts,
  cancellation confirmation, and shutdown cleanup across supported platforms.
- Complete filesystem race hardening for remaining operations and Windows.
- Add frontend workflow tests and conduct accessibility review.
- Run hosted CI on the modified worktree; verify Linux, Windows, macOS Intel,
  and Apple Silicon packaging/runtime paths and test upgrades from v0.2.0. The
  local macOS x86_64 bundle builds, but has not been launch-tested or notarized.
- Complete external SDF model/mesh resource resolution and validation.
- Resolve or document the RustSec warnings and establish whether the ROS base
  image's Python advisory findings are fixed by distro backports; do not
  suppress these findings without evidence.
- Prepare and verify unsigned/signed release artifacts as applicable; do not
  publish a tag or release until explicitly approved.
