# Robotics Studio v0.3.0 Release Readiness

## Decision

**Not ready for release.** The current worktree has meaningful security,
recovery, and simulator-integration improvements, and the local validation
listed below passes. The product still lacks several core workflows named in
the v0.3.0 scope, and cross-platform desktop/CI validation is incomplete.

The checkout is on `main` at `83e88f6` with uncommitted changes. No commit,
push, tag, or release was created.

## Scope checklist

| # | Area | Status | Evidence and remaining work |
|---|---|---|---|
| 1 | gRPC authentication and transport | Pass (local) | Bearer-token requests use TLS with a client-pinned CA; the Docker integration rejects unauthenticated calls and accepts authenticated calls. |
| 2 | Trusted ROS service installation | Pass (local) | The runtime service and generated protobufs are installed outside the writable workspace mount. Workspace migration compares complete file bytes and uses approved-root operations. |
| 3 | Workspace filesystem boundaries | Partial | Unix descriptor-rooted reads, writes, imports, and template copies reject symlink escapes. Windows does not yet have equivalent resistance to parent-directory races. |
| 4 | Dependency security | Partial | `cargo audit` and `npm audit` report zero vulnerabilities. RustSec still reports 13 allowed unmaintained/unsound dependency warnings; see below. |
| 5 | Critical ROS event recovery | Partial | Critical lifecycle and safety records survive service restart and replay with sequence/gap metadata. Telemetry is best-effort, and the UI does not acknowledge delivery; end-to-end exactly-once delivery is not provided. |
| 6 | Motion bounds and emergency stop | Partial | The backend enforces conservative per-topic motion limits and a software stop latch. This is not a physical or safety-rated emergency-stop system. |
| 7 | Workspace selection and isolation | Partial | The backend persists and revalidates the selected workspace. Workspace propagation and cross-platform isolation have not been verified across every operation. |
| 8 | Build process control | Partial | `colcon build` has a dedicated cancellation path. There is no centralized process registry, general timeout policy, or unified status model. |
| 9 | Container and desktop lifecycle | Partial | A manual ROS 2 Jazzy/Gazebo Harmonic container integration passes. The packaged desktop app's full startup, shutdown, and supported-platform lifecycle has not been verified. |
| 10 | ROS node discovery | Partial | Node snapshots are available; a complete graph explorer and detailed endpoint state are not implemented. |
| 11 | ROS topic discovery | Partial | Topic snapshots and tested ROS-Gazebo bridges are available; detailed publisher/subscriber endpoint and QoS exploration is incomplete. |
| 12 | ROS service exploration | Not implemented | The UI does not enumerate or inspect ROS services. |
| 13 | ROS action exploration | Not implemented | The UI does not enumerate or inspect ROS actions. |
| 14 | TF hierarchy validation | Partial | TF edges are displayed, but a complete validated hierarchy is not maintained. |
| 15 | TF stale-frame and cycle handling | Not implemented | Stale-frame expiry and cycle detection are absent. |
| 16 | Robot visualization fidelity | Partial | The 3D view uses placeholder geometry rather than a model derived from the robot description. |
| 17 | URDF parsing | Not implemented | Robot descriptions are not parsed for visualization. |
| 18 | Xacro processing | Not implemented | Xacro expansion is not implemented. |
| 19 | SDF document validation | Partial | XML structure, SDF root/world presence, supported declared versions, and DTD rejection are checked; full SDFormat schema validation is absent. |
| 20 | Model and mesh resource resolution | Not implemented | External model/mesh references are not comprehensively resolved or validated. |
| 21 | Frontend and desktop UX test coverage | Not ready | The frontend type-check/build passes, but automated frontend/UI behavior tests and accessibility review were not run. |
| 22 | Cross-platform release and CI verification | Not ready | The local Docker integration passes, but the hosted workflow, Windows/Linux desktop runtime, and packaged release artifacts have not been validated. |

## Security review and remediation

Three security issues were addressed during this work:

1. **High — workspace marker spoofing.** A marker substring in a workspace
   service file was previously accepted as evidence that the file was trusted.
   Migration now compares the complete contents against bundled files; the
   service actually runs from the application-installed container directory.
2. **High — bearer token over plaintext gRPC.** A local process could
   impersonate the service and capture the token. The client now establishes
   TLS and validates the server against the CA certificate obtained through
   Docker's control channel before sending authenticated requests.
3. **Medium (local filesystem boundary) — symlinked destination during resource
   copy.** Recursive copies could follow a workspace directory symlink and
   write outside the approved root. Copies and service migration now use
   approved-root operations; regression tests cover symlinked directories.

The review did not establish additional exploitable issues in the event
journal, motion validation, or the descriptor-rooted Unix file operations.
That is not a substitute for platform-specific testing. Docker control and
code running with container privileges remain inside the trusted boundary.

## Dependency audit residuals

`cargo audit` found **zero vulnerabilities** and 13 allowed warnings:

| Warning group | Advisory IDs | Notes |
|---|---|---|
| `proc-macro-error 1.0.4` | `RUSTSEC-2024-0370` | Unmaintained; no fixed upgrade available. |
| `unic-* 0.9.0` family | `RUSTSEC-2025-0075`, `-0080`, `-0081`, `-0098`, `-0100` | Unmaintained transitive URL-pattern dependencies; no fixed upgrade available. |
| `zbus_names`, `zvariant`, `zvariant_derive` | `RUSTSEC-2026-0370`, `-0371`, `-0372` | Unmaintained transitive crates. Some older lockfile entries are not on an active dependency path; newer entries remain in the Tauri dependency tree. |
| `glib 0.18.5` | `RUSTSEC-2024-0429` | Unsoundness advisory; the suggested GLib 0.20+ upgrade is blocked by the current GTK/Tauri dependency chain. |

The advisories above did not provide CVSS scores in the audit output. These
warnings are disclosed rather than suppressed.

## Validation performed

All commands below were run against the current worktree:

| Check | Result |
|---|---|
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Pass |
| `cargo check --manifest-path src-tauri/Cargo.toml --locked` | Pass |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | Pass: 32 passed, 1 ignored (the ignored test requires the live service). |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets --all-features -- -D warnings` | Pass |
| `python3 -B -m unittest test_server_core` from `src-tauri/resources/devcontainer` | Pass: 36 tests. |
| `npm run build` | Pass; Vite reports the existing ~811 kB JavaScript chunk-size warning. |
| `npm audit --audit-level=moderate` | Pass: 0 vulnerabilities. |
| `cargo audit` from `src-tauri` | Pass: 0 vulnerabilities, 13 allowed warnings. |
| `scripts/docker-integration.sh` | Pass: pinned TLS/auth, safety latch/reset, event replay after restart, Gazebo world readiness/reset, and ROS-Gazebo topic bridge. No robot motion was commanded. |
| `git diff --check`, protocol-copy comparison, script syntax check | Pass. |

The Docker integration is a manually run container harness, not a desktop UI
test or hosted CI run. Its successful result does not establish support for all
target operating systems or physical robot safety.

## Release blockers

- Implement or explicitly defer full ROS package, service, and action
  exploration.
- Add a centralized process lifecycle/status/timeout model and verify shutdown
  and cancellation in the managed desktop runtime.
- Complete TF hierarchy validation and stale/cycle handling.
- Implement description-based robot rendering, including a declared URDF,
  Xacro, and mesh-resource support boundary.
- Decide whether SDF import requires full schema and external-resource
  validation; implement it or clearly constrain supported input.
- Complete Windows/Linux runtime and filesystem testing, frontend behavior
  tests, and hosted CI validation.
- Reassess the disclosed RustSec warnings when compatible upstream dependency
  upgrades are available.
