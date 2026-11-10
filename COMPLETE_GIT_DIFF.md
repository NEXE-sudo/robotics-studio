# Complete Git Diff

Snapshot of tracked changes against `HEAD` at generation time. The generated document itself is not part of this snapshot.

``````````diff
diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
index cb8d19c..3363c0f 100644
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -16,6 +16,7 @@ jobs:
           node-version: 22
           cache: npm
       - run: npm ci
+      - run: npm audit --audit-level=moderate
       - run: npm run build
 
   rust:
@@ -39,8 +40,10 @@ jobs:
       - uses: dtolnay/rust-toolchain@stable
         with:
           components: clippy
+      - run: cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
+      - run: cargo check --manifest-path src-tauri/Cargo.toml --locked
       - run: cargo test --manifest-path src-tauri/Cargo.toml --locked
-      - run: cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked
+      - run: cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features --locked -- -D warnings
 
   python:
     name: Python service-core tests
diff --git a/README.md b/README.md
index 9b8f02a..c5c7fca 100644
--- a/README.md
+++ b/README.md
@@ -4,13 +4,13 @@ A native desktop IDE for ROS 2 development — edit code, build with `colcon`,
 run live simulations in Gazebo, visualize robot state in real time, and get
 AI assistance that actually understands your running ROS 2 system.
 
-**Status**: v0.2.0, early-stage. Built solo, in the open. Expect rough
+**Status**: v0.3.0 development target, early-stage. Built solo, in the open. Expect rough
 edges — this is a real, working tool, not a polished 1.0, and I'd rather be
 upfront about that than pretend otherwise.
 
 ---
 
-## What's in v0.2.0
+## What's in the v0.3.0 development target
 
 - **One-click ROS 2 + Gazebo environment** — the app builds, starts, and
   connects to a Docker-based ROS 2 environment automatically on launch. No
@@ -42,6 +42,23 @@ upfront about that than pretend otherwise.
   it doesn't run local models for you yet).
 - **Responsive UI** — scales with window/screen size rather than staying
   pinned at a fixed layout.
+- **Restricted native file access** — file browsing/editing is scoped to
+  directories selected in a native picker; file operations reject traversal,
+  symlink escapes, and oversized text files.
+- **Authenticated ROS gRPC** — application-generated session credentials,
+  expiry checks, and authenticated event/control requests. Existing containers
+  with non-loopback gRPC bindings are refused rather than treated as secure.
+- **Safer world import** — validates SDF input and stores it in a dedicated,
+  uniquely named workspace `worlds/` directory rather than overwriting files
+  in the workspace root. Imports are XML-parsed, require an SDF root and world
+  element, accept supported SDF versions 1.4–1.10, and reject DTDs.
+- **Prioritized ROS events** — critical discovery state is retained separately
+  from coalesced telemetry; reconnecting clients receive the latest discovery
+  snapshots.
+- **Bounded robot commands** — linear velocity magnitude is limited to
+  0.5 m/s, angular velocity magnitude to 1 rad/s, and commands expire after
+  0.5 seconds. These limits are fixed conservative defaults, not robot-specific
+  tuning or a safety-rated emergency stop.
 
 ---
 
@@ -138,40 +155,84 @@ any of this — the app manages the environment for you.
 - Tested most thoroughly on macOS; Linux/Windows should work given the
   Docker-based architecture, but haven't had the same depth of testing.
 - Existing files in the managed ROS workspace are preserved rather than
-  overwritten during startup. Older customized Docker/server resources may
-  therefore need an explicit manual migration to receive newer resource
-  changes.
+  overwritten during startup, except that legacy ROS server/proto files are
+  backed up under the application config directory before the authentication
+  update is installed. Existing containers with insecure gRPC port bindings
+  are detected and blocked, but must be recreated manually with their workspace
+  bind-mounted to preserve files.
+- Robot-control limits are not configurable per robot. There is no latched
+  software emergency stop, physical-robot confirmation flow, or safety-rated
+  control guarantee.
+- Workspace selection is still not wired through every backend operation;
+  build and simulation lifecycle currently use the managed default workspace.
+- AI provider credentials are stored through the operating-system credential
+  store. Linux requires an available Secret Service; the app reports an error
+  if secure storage is unavailable.
 
 ## Security notes
 
-- New ROS containers publish gRPC on `127.0.0.1:50051`, not on all host
-  interfaces. The gRPC service does not yet authenticate callers; do not
-  forward or re-publish this port to another network.
-- Twist commands reject malformed topic names, non-finite values, and
-  linear or angular components outside `-5` to `5`.
-- AI provider credentials are currently stored in the application
-  configuration file and are not yet integrated with the operating
-  system's credential store. Avoid using the app with credentials on
-  shared or untrusted accounts.
-- File commands currently accept paths supplied by the frontend and do
-  not yet enforce a backend-maintained approved-root list. Only open
-  projects and files you trust.
-- Existing containers created before loopback-only port publishing may
-  retain their previous host binding. Check `docker port` before using an
-  existing installation on an untrusted network.
-
-These are known security limitations, not a claim of security
-certification.
+- Native file and directory pickers register approved roots in the Rust
+  backend. Reads, writes, listings, project creation, and `.aiignore` checks
+  are constrained to those roots; text reads/writes are limited to 8 MiB.
+- New ROS containers bind gRPC to `127.0.0.1:50051`. Both gRPC methods require
+  a random 256-bit token with a 24-hour expiry. The token is transferred via a
+  protected file and is never returned to frontend JavaScript.
+- When a stream reports an expired or rejected token, the app performs one
+  guarded environment reinitialization: it generates a new token, replaces
+  the container credential file, restarts the ROS service, and reconnects.
+  The old token is invalidated when the server restarts. If recovery fails,
+  the app stops automatic auth retries and reports the error. This is a
+  service-restart-based rotation flow, not a separate refresh-token protocol.
+- An existing container with a missing or non-loopback gRPC binding is
+  rejected. The app does not automatically replace that container; preserve
+  its host workspace and recreate it with a loopback-only mapping.
+- Robot velocity and freshness checks run in both Rust and Python. Commands
+  are checked against combined linear and angular vector magnitudes and are
+  stopped by the server watchdog after 0.5 seconds, but this is not a
+  replacement for a physical emergency-stop circuit.
+- Imported SDF is checked for XML well-formedness, an SDF root, a world
+  element, and a supported declared SDF version. DTDs are rejected. This is
+  not full SDFormat schema validation and does not resolve or validate every
+  referenced model or mesh.
+- AI provider credentials are stored separately through macOS Keychain,
+  Windows Credential Manager, or Linux Secret Service. Only provider/model
+  metadata is stored in the application settings JSON. Existing plaintext
+  credentials are moved to secure storage on first settings load, and the old
+  JSON entry is removed only after the secure-store write succeeds.
+- Path validation checks canonical containment and rejects traversal and
+  symlink escapes. It is not a kernel-level descriptor-based sandbox against a
+  local process racing filesystem changes.
+
+These controls have not been independently security-audited and are not a
+claim of security certification.
+
+## Migration notes for existing v0.2.0 installations
+
+- The managed ROS workspace remains in `~/.robotics-studio/ros2-workspace`;
+  project files and workspace data are not removed by these changes.
+- On the next environment initialization, the app backs up existing
+  `server.py`, `server_core.py`, and `proto/ros_engine.proto` files under the
+  application config directory, then installs the authenticated server
+  protocol.
+- Containers whose host gRPC port is bound to an external interface are
+  refused. Stop the old container and recreate it with
+  `-p 127.0.0.1:50051:50051` and the existing workspace mounted at
+  `/workspace`. The application does not delete or automatically replace it.
+- On first AI settings load, an existing plaintext API key is migrated to the
+  operating-system credential store. If that store is unavailable, migration
+  fails explicitly and the original settings file is retained for recovery.
 
 ## Development checks
 
-Run the frontend build/type check, Rust tests and lint, and isolated
-Python service-core tests from the repository root:
+Run the frontend build/type check, Rust formatting/check/tests/lint, and
+isolated Python service-core tests from the repository root:
 
 ```sh
 npm run build
+cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
+cargo check --manifest-path src-tauri/Cargo.toml --locked
 cargo test --manifest-path src-tauri/Cargo.toml --locked
-cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked
+cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features --locked -- -D warnings
 python3 -m unittest discover -s src-tauri/resources/devcontainer -p 'test_*.py' -v
 ```
 
diff --git a/package-lock.json b/package-lock.json
index 0f9a6b9..60a774d 100644
--- a/package-lock.json
+++ b/package-lock.json
@@ -1,12 +1,12 @@
 {
   "name": "robotics-studio",
-  "version": "0.2.0",
+  "version": "0.3.0",
   "lockfileVersion": 3,
   "requires": true,
   "packages": {
     "": {
       "name": "robotics-studio",
-      "version": "0.2.0",
+      "version": "0.3.0",
       "dependencies": {
         "@monaco-editor/react": "^4.7.0",
         "@tauri-apps/api": "^2",
@@ -1671,9 +1671,9 @@
       }
     },
     "node_modules/dompurify": {
-      "version": "3.4.8",
-      "resolved": "https://registry.npmjs.org/dompurify/-/dompurify-3.4.8.tgz",
-      "integrity": "sha512-yb1cEmaOum7wFvOCSQxyfgVlv5D47Rc30iZWoMpbDIWTnJ6grDDQyu2KFJzB2k7u0pMuJcQ1zphH//fFnw2tjQ==",
+      "version": "3.4.17",
+      "resolved": "https://registry.npmjs.org/dompurify/-/dompurify-3.4.17.tgz",
+      "integrity": "sha512-B/pSlYVHQ58kqISD/yDjuJ1yfzI34m1gF4lAod2bwSRhiN4K5Cl3H8Af7hqB2PmTL911tdEudnnXn5FnhRoz0w==",
       "license": "(MPL-2.0 OR Apache-2.0)",
       "peer": true,
       "optionalDependencies": {
@@ -1862,13 +1862,13 @@
       "license": "MIT"
     },
     "node_modules/monaco-editor": {
-      "version": "0.56.0",
-      "resolved": "https://registry.npmjs.org/monaco-editor/-/monaco-editor-0.56.0.tgz",
-      "integrity": "sha512-sXboRm3BeBeLm938eaiyLMe0OxzfXIlZvbv4ir/jVgQy1zDhWjgmny0WoN45fuDKhCCQsYMbBJrv/A6jd8aCUg==",
+      "version": "0.57.0",
+      "resolved": "https://registry.npmjs.org/monaco-editor/-/monaco-editor-0.57.0.tgz",
+      "integrity": "sha512-5BkI9KGoqrNvBGUe15/QlZq3OooZ8WLg1AxTpaqHRCP3HNpzPPZKE2EDz8M7c+VRmCeUw1Brp4cx/PWm3kI/5A==",
       "license": "MIT",
       "peer": true,
       "dependencies": {
-        "dompurify": "3.4.8",
+        "dompurify": "3.4.15",
         "marked": "14.0.0"
       }
     },
@@ -1880,9 +1880,9 @@
       "license": "MIT"
     },
     "node_modules/nanoid": {
-      "version": "3.3.16",
-      "resolved": "https://registry.npmjs.org/nanoid/-/nanoid-3.3.16.tgz",
-      "integrity": "sha512-bzlKTyNJ7+LdGIIwy8ijFpIqEQIvafahV7eYykJ8Cvh42EdJeODoJ6gUJXpQJvej1BddH8OqTXZNE/KfbWAu8Q==",
+      "version": "3.3.20",
+      "resolved": "https://registry.npmjs.org/nanoid/-/nanoid-3.3.20.tgz",
+      "integrity": "sha512-uKdg2G3GNCKQn9byYOpxbGqrT2fGO5KRt5J/8b3pok8rT6qxGWF6hxMyJiEYtAf+FVyYuD9hRaDqX5uPFYJ4ZQ==",
       "dev": true,
       "funding": [
         {
@@ -1929,9 +1929,9 @@
       }
     },
     "node_modules/postcss": {
-      "version": "8.5.22",
-      "resolved": "https://registry.npmjs.org/postcss/-/postcss-8.5.22.tgz",
-      "integrity": "sha512-KBDEIpLrvpv16pp3K0Fw+UCoZfopFjjgeB+0tA/aaThfEE74kKDLrgg603YvOWJyg3+WYtyq3xYsQWsIyZlPqQ==",
+      "version": "8.5.32",
+      "resolved": "https://registry.npmjs.org/postcss/-/postcss-8.5.32.tgz",
+      "integrity": "sha512-LJlKCCAUFKOuZxipUdYw1NzIITEsPcgyOG9LkvRJvuKj/bRzbuRY0ARXvL0pWGpsZvn8jku+LMep2eU0seHp8g==",
       "dev": true,
       "funding": [
         {
@@ -1949,9 +1949,9 @@
       ],
       "license": "MIT",
       "dependencies": {
-        "nanoid": "^3.3.16",
+        "nanoid": "^3.3.20",
         "picocolors": "^1.1.1",
-        "source-map-js": "^1.2.1"
+        "source-map-js": "^1.2.2"
       },
       "engines": {
         "node": "^10 || ^12 || >=14"
@@ -2050,9 +2050,9 @@
       }
     },
     "node_modules/source-map-js": {
-      "version": "1.2.1",
-      "resolved": "https://registry.npmjs.org/source-map-js/-/source-map-js-1.2.1.tgz",
-      "integrity": "sha512-UXWMKhLOwVKb728IUtQPXxfYU+usdybtUrK/8uGE8CQMvrhOpwvzDBwj0QhSL7MQc7vIsISBG8VQ8+IDQxpfQA==",
+      "version": "1.2.2",
+      "resolved": "https://registry.npmjs.org/source-map-js/-/source-map-js-1.2.2.tgz",
+      "integrity": "sha512-KGj/8Y43x35aZVDtt+J4mK1hoLGHULMYfSkODJNQjNDC3oW1PqPoxMwo0pLUsWM/UEGzON/NxeHywEfNXNP3Vw==",
       "dev": true,
       "license": "BSD-3-Clause",
       "engines": {
@@ -2134,9 +2134,9 @@
       }
     },
     "node_modules/vite": {
-      "version": "7.3.6",
-      "resolved": "https://registry.npmjs.org/vite/-/vite-7.3.6.tgz",
-      "integrity": "sha512-4XP60spRGjSZFf1qYH+dJIkK2znL3zQfl9KkOV9MkkRR/3Dls0dxaBsQPTloEc5BLXWPL9vsOxopxyKoMmDueg==",
+      "version": "7.3.8",
+      "resolved": "https://registry.npmjs.org/vite/-/vite-7.3.8.tgz",
+      "integrity": "sha512-3tTi78MAKhVk9h+YZVXMQ8POmneJuE97QUPAWtJghHzhH7BAyiqQOC2KZh1DWOsu/USB2cRXhznPByaabvYXfw==",
       "dev": true,
       "license": "MIT",
       "dependencies": {
diff --git a/package.json b/package.json
index 138a26b..4c6b264 100644
--- a/package.json
+++ b/package.json
@@ -1,7 +1,7 @@
 {
   "name": "robotics-studio",
   "private": true,
-  "version": "0.2.0",
+  "version": "0.3.0",
   "type": "module",
   "scripts": {
     "dev": "vite",
@@ -27,5 +27,8 @@
     "@vitejs/plugin-react": "^4.6.0",
     "typescript": "~5.8.3",
     "vite": "^7.0.4"
+  },
+  "overrides": {
+    "dompurify": "^3.4.17"
   }
 }
diff --git a/src-tauri/Cargo.lock b/src-tauri/Cargo.lock
index fd60458..2ff9ed2 100644
--- a/src-tauri/Cargo.lock
+++ b/src-tauri/Cargo.lock
@@ -8,6 +8,17 @@ version = "2.0.1"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "320119579fcad9c21884f5c4861d16174d0e06250625266f50fe6898340abefa"
 
+[[package]]
+name = "aes"
+version = "0.8.4"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "b169f7a6d4742236a0a00c541b845991d0ac43e546831af1249753ab4c3aa3a0"
+dependencies = [
+ "cfg-if",
+ "cipher",
+ "cpufeatures 0.2.17",
+]
+
 [[package]]
 name = "aho-corasick"
 version = "1.1.4"
@@ -330,6 +341,15 @@ dependencies = [
  "generic-array",
 ]
 
+[[package]]
+name = "block-padding"
+version = "0.3.3"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "a8894febbff9f758034a5b8e12d87918f56dfc64a8e1fe757d65e29041538d93"
+dependencies = [
+ "generic-array",
+]
+
 [[package]]
 name = "block2"
 version = "0.6.2"
@@ -476,6 +496,15 @@ dependencies = [
  "toml 0.9.12+spec-1.1.0",
 ]
 
+[[package]]
+name = "cbc"
+version = "0.1.2"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "26b52a9543ae338f279b96b0b9fed9c8093744685043739079ce85cd58f289a6"
+dependencies = [
+ "cipher",
+]
+
 [[package]]
 name = "cc"
 version = "1.3.0"
@@ -535,7 +564,7 @@ checksum = "d524456ba66e72eb8b115ff89e01e497f8e6d11d78b70b1aa13c0fbd97540a81"
 dependencies = [
  "cfg-if",
  "cpufeatures 0.3.0",
- "rand_core",
+ "rand_core 0.10.1",
 ]
 
 [[package]]
@@ -550,6 +579,16 @@ dependencies = [
  "windows-link 0.2.1",
 ]
 
+[[package]]
+name = "cipher"
+version = "0.4.4"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "773f3b9af64447d2ce9850330c473515014aa235e6a783b02db81ff39e4a3dad"
+dependencies = [
+ "crypto-common",
+ "inout",
+]
+
 [[package]]
 name = "cmake"
 version = "0.1.58"
@@ -774,6 +813,24 @@ dependencies = [
  "windows-sys 0.61.2",
 ]
 
+[[package]]
+name = "dbus-secret-service"
+version = "4.1.0"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "708b509edf7889e53d7efb0ffadd994cc6c2345ccb62f55cfd6b0682165e4fa6"
+dependencies = [
+ "aes",
+ "block-padding",
+ "cbc",
+ "dbus",
+ "fastrand",
+ "hkdf",
+ "num",
+ "once_cell",
+ "sha2",
+ "zeroize",
+]
+
 [[package]]
 name = "deranged"
 version = "0.5.8"
@@ -812,6 +869,7 @@ checksum = "9ed9a281f7bc9b7576e61468ba615a66a5c8cfdff42420a70aa82701a3b1e292"
 dependencies = [
  "block-buffer",
  "crypto-common",
+ "subtle",
 ]
 
 [[package]]
@@ -1392,7 +1450,7 @@ dependencies = [
  "js-sys",
  "libc",
  "r-efi 6.0.0",
- "rand_core",
+ "rand_core 0.10.1",
  "wasm-bindgen",
 ]
 
@@ -1608,6 +1666,24 @@ version = "0.4.3"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "7f24254aa9a54b5c858eaee2f5bccdb46aaf0e486a595ed5fd8f86ba55232a70"
 
+[[package]]
+name = "hkdf"
+version = "0.12.4"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "7b5f8eb2ad728638ea2c7d47a21db23b7b58a72ed6a38256b8a1849f15fbbdf7"
+dependencies = [
+ "hmac",
+]
+
+[[package]]
+name = "hmac"
+version = "0.12.1"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "6c49c37c09c17a53d937dfbb742eb3a961d65a994e6bcdcf37e7399d0cc8ab5e"
+dependencies = [
+ "digest",
+]
+
 [[package]]
 name = "html5ever"
 version = "0.38.0"
@@ -1913,6 +1989,16 @@ dependencies = [
  "cfb",
 ]
 
+[[package]]
+name = "inout"
+version = "0.1.4"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "879f10e63c20629ecabbb64a8010319738c66a5cd0c29b02d63d272b03751d01"
+dependencies = [
+ "block-padding",
+ "generic-array",
+]
+
 [[package]]
 name = "ipnet"
 version = "2.12.0"
@@ -2104,6 +2190,23 @@ dependencies = [
  "unicode-segmentation",
 ]
 
+[[package]]
+name = "keyring"
+version = "3.6.3"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "eebcc3aff044e5944a8fbaf69eb277d11986064cba30c468730e8b9909fb551c"
+dependencies = [
+ "byteorder",
+ "dbus-secret-service",
+ "linux-keyutils",
+ "log",
+ "secret-service",
+ "security-framework 2.11.1",
+ "security-framework 3.7.0",
+ "windows-sys 0.60.2",
+ "zeroize",
+]
+
 [[package]]
 name = "libappindicator"
 version = "0.9.0"
@@ -2162,6 +2265,16 @@ dependencies = [
  "libc",
 ]
 
+[[package]]
+name = "linux-keyutils"
+version = "0.2.5"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "83270a18e9f90d0707c41e9f35efada77b64c0e6f3f1810e71c8368a864d5590"
+dependencies = [
+ "bitflags 2.13.1",
+ "libc",
+]
+
 [[package]]
 name = "linux-raw-sys"
 version = "0.12.1"
@@ -2311,12 +2424,88 @@ version = "1.0.6"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "650eef8c711430f1a879fdd01d4745a7deea475becfb90269c06775983bbf086"
 
+[[package]]
+name = "nix"
+version = "0.29.0"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "71e2746dc3a24dd78b3cfcb7be93368c6de9963d30f43a6a73998a9cf4b17b46"
+dependencies = [
+ "bitflags 2.13.1",
+ "cfg-if",
+ "cfg_aliases",
+ "libc",
+ "memoffset",
+]
+
+[[package]]
+name = "num"
+version = "0.4.3"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "35bd024e8b2ff75562e5f34e7f4905839deb4b22955ef5e73d2fea1b9813cb23"
+dependencies = [
+ "num-bigint",
+ "num-complex",
+ "num-integer",
+ "num-iter",
+ "num-rational",
+ "num-traits",
+]
+
+[[package]]
+name = "num-bigint"
+version = "0.4.8"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "c89e69e7e0f03bea5ef08013795c25018e101932225a656383bd384495ecc367"
+dependencies = [
+ "num-integer",
+ "num-traits",
+]
+
+[[package]]
+name = "num-complex"
+version = "0.4.6"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "73f88a1307638156682bada9d7604135552957b7818057dcef22705b4d509495"
+dependencies = [
+ "num-traits",
+]
+
 [[package]]
 name = "num-conv"
 version = "0.2.2"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "521739c6d2bac4aa25192232afe6841231376b2b26d4d9fae5ecf8ca5772e441"
 
+[[package]]
+name = "num-integer"
+version = "0.1.47"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "7ce2d95d4b3734dc35aa2f45e1aa22cd416814592a4f9d9205e11affd5b8e10b"
+dependencies = [
+ "num-traits",
+]
+
+[[package]]
+name = "num-iter"
+version = "0.1.46"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "c92800bd69a1eac91786bcfe9da64a897eb72911b8dc3095decbd07429e8048b"
+dependencies = [
+ "num-integer",
+ "num-traits",
+]
+
+[[package]]
+name = "num-rational"
+version = "0.4.2"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "f83d14da390562dca69fc84082e73e548e1ad308d24accdedd2720017cb37824"
+dependencies = [
+ "num-bigint",
+ "num-integer",
+ "num-traits",
+]
+
 [[package]]
 name = "num-traits"
 version = "0.2.19"
@@ -2818,6 +3007,15 @@ version = "0.2.0"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "439ee305def115ba05938db6eb1644ff94165c5ab5e9420d1c1bcedbba909391"
 
+[[package]]
+name = "ppv-lite86"
+version = "0.2.21"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "85eae3c4ed2f50dcfe72643da4befc30deadb458a9b590d720cde2f2b1e97da9"
+dependencies = [
+ "zerocopy",
+]
+
 [[package]]
 name = "precomputed-hash"
 version = "0.1.1"
@@ -3008,7 +3206,7 @@ dependencies = [
  "bytes",
  "getrandom 0.4.3",
  "lru-slab",
- "rand",
+ "rand 0.10.2",
  "rand_pcg",
  "ring",
  "rustc-hash",
@@ -3032,7 +3230,7 @@ dependencies = [
  "once_cell",
  "socket2",
  "tracing",
- "windows-sys 0.59.0",
+ "windows-sys 0.61.2",
 ]
 
 [[package]]
@@ -3056,6 +3254,17 @@ version = "6.0.0"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "f8dcc9c7d52a811697d2151c701e0d08956f92b0e24136cf4cf27b57a6a0d9bf"
 
+[[package]]
+name = "rand"
+version = "0.8.8"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "e058c7de0b26af77780c769414d6257830bb240f3c38477dbc2c16e5f54d6d4c"
+dependencies = [
+ "libc",
+ "rand_chacha",
+ "rand_core 0.6.4",
+]
+
 [[package]]
 name = "rand"
 version = "0.10.2"
@@ -3064,7 +3273,26 @@ checksum = "c7f5fa3a058cd35567ef9bfa5e75732bee0f9e4c55fa90477bef2dfcdbc4be80"
 dependencies = [
  "chacha20",
  "getrandom 0.4.3",
- "rand_core",
+ "rand_core 0.10.1",
+]
+
+[[package]]
+name = "rand_chacha"
+version = "0.3.1"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "e6c10a63a0fa32252be49d21e7709d4d4baf8d231c2dbce1eaa8141b9b127d88"
+dependencies = [
+ "ppv-lite86",
+ "rand_core 0.6.4",
+]
+
+[[package]]
+name = "rand_core"
+version = "0.6.4"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "ec0be4795e2f6a28069bec0b5ff3e2ac9bafc99e6a9a7dc3547996c5c816922c"
+dependencies = [
+ "getrandom 0.2.17",
 ]
 
 [[package]]
@@ -3079,7 +3307,7 @@ version = "0.10.2"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "caa0f4137e1c0a72f4c651489402276c8e8e1cf081f3b0ba156d2cbeef09e86a"
 dependencies = [
- "rand_core",
+ "rand_core 0.10.1",
 ]
 
 [[package]]
@@ -3240,9 +3468,12 @@ dependencies = [
 
 [[package]]
 name = "robotics-studio"
-version = "0.1.0"
+version = "0.3.0"
 dependencies = [
+ "getrandom 0.3.4",
+ "keyring",
  "prost",
+ "quick-xml",
  "reqwest",
  "serde",
  "serde_json",
@@ -3308,7 +3539,7 @@ dependencies = [
  "openssl-probe",
  "rustls-pki-types",
  "schannel",
- "security-framework",
+ "security-framework 3.7.0",
 ]
 
 [[package]]
@@ -3336,10 +3567,10 @@ dependencies = [
  "rustls-native-certs",
  "rustls-platform-verifier-android",
  "rustls-webpki",
- "security-framework",
+ "security-framework 3.7.0",
  "security-framework-sys",
  "webpki-root-certs",
- "windows-sys 0.59.0",
+ "windows-sys 0.61.2",
 ]
 
 [[package]]
@@ -3441,6 +3672,38 @@ version = "1.2.0"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "94143f37725109f92c262ed2cf5e59bce7498c01bcc1502d7b9afe439a4e9f49"
 
+[[package]]
+name = "secret-service"
+version = "4.0.0"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "e4d35ad99a181be0a60ffcbe85d680d98f87bdc4d7644ade319b87076b9dbfd4"
+dependencies = [
+ "aes",
+ "cbc",
+ "futures-util",
+ "generic-array",
+ "hkdf",
+ "num",
+ "once_cell",
+ "rand 0.8.8",
+ "serde",
+ "sha2",
+ "zbus 4.4.0",
+]
+
+[[package]]
+name = "security-framework"
+version = "2.11.1"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "897b2245f0b511c87893af39b033e5ca9cce68824c4d7e7630b5a1d339658d02"
+dependencies = [
+ "bitflags 2.13.1",
+ "core-foundation 0.9.4",
+ "core-foundation-sys",
+ "libc",
+ "security-framework-sys",
+]
+
 [[package]]
 name = "security-framework"
 version = "3.7.0"
@@ -3651,6 +3914,17 @@ dependencies = [
  "stable_deref_trait",
 ]
 
+[[package]]
+name = "sha1"
+version = "0.10.7"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "a978451301f4db1d02937a4ab3ccce137717b81826e79b7d49ffe3244a13c3b8"
+dependencies = [
+ "cfg-if",
+ "cpufeatures 0.2.17",
+ "digest",
+]
+
 [[package]]
 name = "sha2"
 version = "0.10.9"
@@ -3782,6 +4056,12 @@ version = "1.2.1"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "6ce2be8dc25455e1f91df71bfa12ad37d7af1092ae736f3a6cd0e37bc7810596"
 
+[[package]]
+name = "static_assertions"
+version = "1.1.0"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "a2eb9349b6444b326872e140eb1cf5e7c522154d69e7a0ffb0fb81c06b37543f"
+
 [[package]]
 name = "string_cache"
 version = "0.9.0"
@@ -4162,7 +4442,7 @@ dependencies = [
  "thiserror 2.0.19",
  "url",
  "windows",
- "zbus",
+ "zbus 5.18.0",
 ]
 
 [[package]]
@@ -5683,6 +5963,16 @@ dependencies = [
  "pkg-config",
 ]
 
+[[package]]
+name = "xdg-home"
+version = "1.3.0"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "ec1cdab258fb55c0da61328dc52c8764709b249011b2cad0454c72f0bf10a1f6"
+dependencies = [
+ "libc",
+ "windows-sys 0.59.0",
+]
+
 [[package]]
 name = "yoke"
 version = "0.8.3"
@@ -5706,6 +5996,38 @@ dependencies = [
  "synstructure",
 ]
 
+[[package]]
+name = "zbus"
+version = "4.4.0"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "bb97012beadd29e654708a0fdb4c84bc046f537aecfde2c3ee0a9e4b4d48c725"
+dependencies = [
+ "async-broadcast",
+ "async-process",
+ "async-recursion",
+ "async-trait",
+ "enumflags2",
+ "event-listener",
+ "futures-core",
+ "futures-sink",
+ "futures-util",
+ "hex",
+ "nix",
+ "ordered-stream",
+ "rand 0.8.8",
+ "serde",
+ "serde_repr",
+ "sha1",
+ "static_assertions",
+ "tracing",
+ "uds_windows",
+ "windows-sys 0.52.0",
+ "xdg-home",
+ "zbus_macros 4.4.0",
+ "zbus_names 3.0.0",
+ "zvariant 4.2.0",
+]
+
 [[package]]
 name = "zbus"
 version = "5.18.0"
@@ -5736,9 +6058,22 @@ dependencies = [
  "uuid",
  "windows-sys 0.61.2",
  "winnow 1.0.4",
- "zbus_macros",
- "zbus_names",
- "zvariant",
+ "zbus_macros 5.18.0",
+ "zbus_names 4.3.4",
+ "zvariant 5.13.1",
+]
+
+[[package]]
+name = "zbus_macros"
+version = "4.4.0"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "267db9407081e90bbfa46d841d3cbc60f59c0351838c4bc65199ecd79ab1983e"
+dependencies = [
+ "proc-macro-crate 3.5.0",
+ "proc-macro2",
+ "quote",
+ "syn 2.0.119",
+ "zvariant_utils 2.1.0",
 ]
 
 [[package]]
@@ -5751,9 +6086,20 @@ dependencies = [
  "proc-macro2",
  "quote",
  "syn 2.0.119",
- "zbus_names",
- "zvariant",
- "zvariant_utils",
+ "zbus_names 4.3.4",
+ "zvariant 5.13.1",
+ "zvariant_utils 3.5.0",
+]
+
+[[package]]
+name = "zbus_names"
+version = "3.0.0"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "4b9b1fef7d021261cc16cba64c351d291b715febe0fa10dc3a443ac5a5022e6c"
+dependencies = [
+ "serde",
+ "static_assertions",
+ "zvariant 4.2.0",
 ]
 
 [[package]]
@@ -5764,7 +6110,27 @@ checksum = "d8bf88b4a3ff53e883001e0e0115b297a9d53c31b9c1edd2bfdd853e3428624e"
 dependencies = [
  "serde",
  "winnow 1.0.4",
- "zvariant",
+ "zvariant 5.13.1",
+]
+
+[[package]]
+name = "zerocopy"
+version = "0.8.63"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "e5fe1f8f1b06191a00962174c61aa5005e0bb391a6d80d07e24d115c01a92ed8"
+dependencies = [
+ "zerocopy-derive",
+]
+
+[[package]]
+name = "zerocopy-derive"
+version = "0.8.63"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "863ad3ac83293fb4d740aedbfdc9240dd8d1a50c1099acd76ce80ce7c7230c7f"
+dependencies = [
+ "proc-macro2",
+ "quote",
+ "syn 2.0.119",
 ]
 
 [[package]]
@@ -5793,6 +6159,20 @@ name = "zeroize"
 version = "1.9.0"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "e13c156562582aa81c60cb29407084cdb54c4164760106ab78e6c5b0858cf64e"
+dependencies = [
+ "zeroize_derive",
+]
+
+[[package]]
+name = "zeroize_derive"
+version = "1.5.0"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "3c50655cbb0fe3fc43170059e702f1ce5e19b84cec58dc87b037a09935c2f328"
+dependencies = [
+ "proc-macro2",
+ "quote",
+ "syn 2.0.119",
+]
 
 [[package]]
 name = "zerotrie"
@@ -5833,6 +6213,19 @@ version = "1.0.23"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "29666d0abbfad1e3dc4dcf6144730dd3a3ab225bbbdac83319345b1b44ccfc1b"
 
+[[package]]
+name = "zvariant"
+version = "4.2.0"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "2084290ab9a1c471c38fc524945837734fbf124487e105daec2bb57fd48c81fe"
+dependencies = [
+ "endi",
+ "enumflags2",
+ "serde",
+ "static_assertions",
+ "zvariant_derive 4.2.0",
+]
+
 [[package]]
 name = "zvariant"
 version = "5.13.1"
@@ -5843,8 +6236,21 @@ dependencies = [
  "enumflags2",
  "serde",
  "winnow 1.0.4",
- "zvariant_derive",
- "zvariant_utils",
+ "zvariant_derive 5.13.1",
+ "zvariant_utils 3.5.0",
+]
+
+[[package]]
+name = "zvariant_derive"
+version = "4.2.0"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "73e2ba546bda683a90652bac4a279bc146adad1386f25379cf73200d2002c449"
+dependencies = [
+ "proc-macro-crate 3.5.0",
+ "proc-macro2",
+ "quote",
+ "syn 2.0.119",
+ "zvariant_utils 2.1.0",
 ]
 
 [[package]]
@@ -5857,7 +6263,18 @@ dependencies = [
  "proc-macro2",
  "quote",
  "syn 2.0.119",
- "zvariant_utils",
+ "zvariant_utils 3.5.0",
+]
+
+[[package]]
+name = "zvariant_utils"
+version = "2.1.0"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "c51bcff7cc3dbb5055396bcf774748c3dab426b4b8659046963523cee4808340"
+dependencies = [
+ "proc-macro2",
+ "quote",
+ "syn 2.0.119",
 ]
 
 [[package]]
diff --git a/src-tauri/Cargo.toml b/src-tauri/Cargo.toml
index c34e540..35640f5 100644
--- a/src-tauri/Cargo.toml
+++ b/src-tauri/Cargo.toml
@@ -1,6 +1,6 @@
 [package]
 name = "robotics-studio"
-version = "0.2.0"
+version = "0.3.0"
 description = "A Tauri App"
 authors = ["you"]
 edition = "2021"
@@ -27,7 +27,9 @@ serde_json = "1"
 tauri-plugin-dialog = "2.7.2"
 tonic = "0.14.6"
 prost = "0.14.4"
+quick-xml = "0.41.0"
 tokio = { version = "1.53.1", features = ["full"] }
 tonic-prost = "0.14.6"
 reqwest = { version = "0.13.4", features = ["json"] }
-
+getrandom = "0.3.4"
+keyring = { version = "3.6.3", default-features = false, features = ["apple-native", "windows-native", "linux-native-sync-persistent", "crypto-rust"] }
diff --git a/src-tauri/build.rs b/src-tauri/build.rs
index 0845d3b..e09e33e 100644
--- a/src-tauri/build.rs
+++ b/src-tauri/build.rs
@@ -13,4 +13,4 @@ fn main() -> Result<(), Box<dyn std::error::Error>> {
         .compile_protos(&["proto/ros_engine.proto"], &["proto"])?;
     tauri_build::build();
     Ok(())
-}
\ No newline at end of file
+}
diff --git a/src-tauri/capabilities/default.json b/src-tauri/capabilities/default.json
index 7bfd476..6ec2990 100644
--- a/src-tauri/capabilities/default.json
+++ b/src-tauri/capabilities/default.json
@@ -6,8 +6,6 @@
   "permissions": [
     "core:default",
     "opener:default",
-    "dialog:default",
-    "dialog:allow-message",
     "core:event:default"
   ]
 }
diff --git a/src-tauri/proto/ros_engine.proto b/src-tauri/proto/ros_engine.proto
index 060d7e1..38f6c35 100644
--- a/src-tauri/proto/ros_engine.proto
+++ b/src-tauri/proto/ros_engine.proto
@@ -1,3 +1,4 @@
+// AUTH_PROTOCOL_VERSION = 1
 syntax = "proto3";
 
 package rosengine;
@@ -15,6 +16,7 @@ message TwistCommand {
   double linear_z = 5;
   double angular_x = 6;
   double angular_y = 7;
+  int64 timestamp_ns = 8;
 }
 
 message Empty {}
diff --git a/src-tauri/resources/devcontainer/proto/ros_engine.proto b/src-tauri/resources/devcontainer/proto/ros_engine.proto
index 060d7e1..38f6c35 100644
--- a/src-tauri/resources/devcontainer/proto/ros_engine.proto
+++ b/src-tauri/resources/devcontainer/proto/ros_engine.proto
@@ -1,3 +1,4 @@
+// AUTH_PROTOCOL_VERSION = 1
 syntax = "proto3";
 
 package rosengine;
@@ -15,6 +16,7 @@ message TwistCommand {
   double linear_z = 5;
   double angular_x = 6;
   double angular_y = 7;
+  int64 timestamp_ns = 8;
 }
 
 message Empty {}
diff --git a/src-tauri/resources/devcontainer/server.py b/src-tauri/resources/devcontainer/server.py
index dfbbc0b..85abdf0 100644
--- a/src-tauri/resources/devcontainer/server.py
+++ b/src-tauri/resources/devcontainer/server.py
@@ -2,6 +2,7 @@ import threading
 import queue
 import time
 import logging
+import os
 
 import grpc
 from concurrent import futures
@@ -9,6 +10,8 @@ from concurrent import futures
 import rclpy
 from rclpy.node import Node
 
+AUTH_PROTOCOL_VERSION = 1
+
 import sys
 sys.path.append('generated')
 import ros_engine_pb2
@@ -17,7 +20,13 @@ from tf2_msgs.msg import TFMessage
 
 from nav_msgs.msg import Odometry
 from geometry_msgs.msg import Twist
-from server_core import EventHub, validate_twist_request
+from server_core import (
+    CommandLeaseManager,
+    EventHub,
+    is_valid_auth_token,
+    load_auth_credentials,
+    validate_twist_request,
+)
 
 LOGGER = logging.getLogger("ros_engine")
 
@@ -29,6 +38,10 @@ class RosIntrospector(Node):
         self.odom_subscribers = {}
         self.timer = self.create_timer(2.0, self.tick)
         self.twist_publishers = {}
+        self.command_leases = CommandLeaseManager()
+        self.command_timeout_timer = self.create_timer(
+            0.1, self.stop_expired_commands
+        )
         self.tf_subscribers = {}
 
     def odom_callback(self, msg, topic_name):
@@ -45,7 +58,11 @@ class RosIntrospector(Node):
             ),
             timestamp_ns=time.time_ns()
         )
-        self.event_hub.publish(evt)
+        self.event_hub.publish(
+            evt,
+            priority="telemetry",
+            key=("odometry", topic_name),
+        )
 
     def tf_callback(self, msg, topic_name):
         for transform in msg.transforms:
@@ -63,7 +80,11 @@ class RosIntrospector(Node):
                 ),
                 timestamp_ns=time.time_ns()
             )
-            self.event_hub.publish(evt)
+            self.event_hub.publish(
+                evt,
+                priority="telemetry",
+                key=("tf", transform.child_frame_id),
+            )
 
     def tick(self):
         try:
@@ -81,14 +102,14 @@ class RosIntrospector(Node):
                 node_started=ros_engine_pb2.NodeStarted(node_name=name),
                 timestamp_ns=time.time_ns()
             )
-            self.event_hub.publish(evt)
+            self.event_hub.publish(evt, key=("node", name))
 
         for name in disappeared:
             evt = ros_engine_pb2.WorkspaceEvent(
                 node_crashed=ros_engine_pb2.NodeCrashed(node_name=name),
                 timestamp_ns=time.time_ns()
             )
-            self.event_hub.publish(evt)
+            self.event_hub.publish(evt, key=("node", name))
 
         self.known_nodes = nodes
 
@@ -132,13 +153,21 @@ class RosIntrospector(Node):
             topic_snapshot=ros_engine_pb2.TopicSnapshot(topics=topic_msgs),
             timestamp_ns=time.time_ns()
         )
-        self.event_hub.publish(evt)
+        self.event_hub.publish(
+            evt,
+            key="topic_snapshot",
+            snapshot=True,
+        )
 
         node_evt = ros_engine_pb2.WorkspaceEvent(
             node_snapshot=ros_engine_pb2.NodeSnapshot(node_names=list(nodes)),
             timestamp_ns=time.time_ns()
         )
-        self.event_hub.publish(node_evt)
+        self.event_hub.publish(
+            node_evt,
+            key="node_snapshot",
+            snapshot=True,
+        )
 
     def publish_twist(self, topic_name, linear_x, linear_y=0.0, linear_z=0.0, angular_x=0.0, angular_y=0.0, angular_z=0.0):
         if topic_name not in self.twist_publishers:
@@ -151,13 +180,34 @@ class RosIntrospector(Node):
         msg.angular.y = angular_y
         msg.angular.z = angular_z
         self.twist_publishers[topic_name].publish(msg)
+        self.command_leases.refresh(topic_name)
+
+    def stop_expired_commands(self):
+        for topic_name in self.command_leases.take_expired():
+            publisher = self.twist_publishers.get(topic_name)
+            if publisher is not None:
+                publisher.publish(Twist())
 
 class RosEngineServicer(ros_engine_pb2_grpc.RosEngineServicer):
-    def __init__(self, event_hub: EventHub, ros_node):
+    def __init__(self, event_hub: EventHub, ros_node, auth_token, auth_expires_at):
         self.event_hub = event_hub
         self.ros_node = ros_node
+        self.auth_token = auth_token
+        self.auth_expires_at = auth_expires_at
+
+    def _require_authentication(self, context):
+        provided = dict(context.invocation_metadata()).get(
+            "x-robotics-studio-token"
+        )
+        if not is_valid_auth_token(provided, self.auth_token, self.auth_expires_at):
+            context.abort(
+                grpc.StatusCode.UNAUTHENTICATED,
+                "A valid Robotics Studio session token is required",
+            )
+        return provided
 
     def PublishTwist(self, request, context):
+        self._require_authentication(context)
         try:
             validate_twist_request(request)
             self.ros_node.publish_twist(
@@ -177,10 +227,20 @@ class RosEngineServicer(ros_engine_pb2_grpc.RosEngineServicer):
         return ros_engine_pb2.Empty()
 
     def StreamEvents(self, request, context):
+        provided = self._require_authentication(context)
         subscriber = self.event_hub.subscribe()
         LOGGER.info("ROS event stream opened")
         try:
             while context.is_active():
+                if not is_valid_auth_token(
+                    provided,
+                    self.auth_token,
+                    self.auth_expires_at,
+                ):
+                    context.abort(
+                        grpc.StatusCode.UNAUTHENTICATED,
+                        "The Robotics Studio session token has expired",
+                    )
                 try:
                     yield subscriber.get(timeout=1.0)
                 except queue.Empty:
@@ -199,6 +259,11 @@ def main():
         level=logging.INFO,
         format="%(asctime)s %(levelname)s %(name)s: %(message)s",
     )
+    credential_path = os.environ.get(
+        "ROBOTICS_STUDIO_AUTH_FILE",
+        "/run/robotics-studio/grpc-auth.json",
+    )
+    auth_token, auth_expires_at = load_auth_credentials(credential_path)
     rclpy.init()
     event_hub = EventHub()
     node = RosIntrospector(event_hub)
@@ -208,7 +273,7 @@ def main():
 
     server = grpc.server(futures.ThreadPoolExecutor(max_workers=4))
     ros_engine_pb2_grpc.add_RosEngineServicer_to_server(
-        RosEngineServicer(event_hub, node), server
+        RosEngineServicer(event_hub, node, auth_token, auth_expires_at), server
     )
     server.add_insecure_port('[::]:50051')
     server.start()
diff --git a/src-tauri/resources/devcontainer/server_core.py b/src-tauri/resources/devcontainer/server_core.py
index 47d419f..dab37cb 100644
--- a/src-tauri/resources/devcontainer/server_core.py
+++ b/src-tauri/resources/devcontainer/server_core.py
@@ -1,11 +1,17 @@
+from collections import OrderedDict, deque
+import hmac
 import math
 import queue
 import re
 import threading
+import time
 
+AUTH_PROTOCOL_VERSION = 1
 MAX_EVENT_QUEUE_SIZE = 256
-MAX_LINEAR_VELOCITY = 5.0
-MAX_ANGULAR_VELOCITY = 5.0
+MAX_LINEAR_VELOCITY = 0.5
+MAX_ANGULAR_VELOCITY = 1.0
+MAX_COMMAND_AGE_SECONDS = 1.0
+COMMAND_TIMEOUT_SECONDS = 0.5
 ROS_TOPIC_PATTERN = re.compile(r"/[A-Za-z_][A-Za-z0-9_]*(/[A-Za-z_][A-Za-z0-9_]*)*")
 
 
@@ -16,46 +22,172 @@ class EventHub:
         self._max_queue_size = max_queue_size
         self._subscribers = set()
         self._lock = threading.Lock()
+        self._latest_state = {}
 
     def subscribe(self):
-        subscriber = queue.Queue(maxsize=self._max_queue_size)
+        subscriber = EventSubscriber(self._max_queue_size)
         with self._lock:
             self._subscribers.add(subscriber)
+            snapshots = tuple(self._latest_state.items())
+        for key, event in snapshots:
+            subscriber.put(event, critical=True, key=key)
         return subscriber
 
     def unsubscribe(self, subscriber):
         with self._lock:
             self._subscribers.discard(subscriber)
 
-    def publish(self, event):
+    def publish(self, event, *, priority="critical", key=None, snapshot=False):
+        if priority not in ("critical", "telemetry"):
+            raise ValueError("priority must be 'critical' or 'telemetry'")
+        if key is None:
+            key = id(event)
         with self._lock:
+            if snapshot:
+                self._latest_state[key] = event
             subscribers = tuple(self._subscribers)
 
         for subscriber in subscribers:
-            try:
-                subscriber.put_nowait(event)
-            except queue.Full:
-                try:
-                    subscriber.get_nowait()
-                except queue.Empty:
-                    pass
-                try:
-                    subscriber.put_nowait(event)
-                except queue.Full:
-                    pass
-
-
-def validate_twist_request(request):
+            subscriber.put(event, critical=priority == "critical", key=key)
+
+
+class EventSubscriber:
+    def __init__(self, max_queue_size):
+        self._max_queue_size = max_queue_size
+        self._critical = deque()
+        self._telemetry = OrderedDict()
+        self._condition = threading.Condition()
+        self.dropped_telemetry = 0
+        self.dropped_critical = 0
+
+    def put(self, event, *, critical, key):
+        with self._condition:
+            if critical:
+                if len(self._critical) >= self._max_queue_size:
+                    if self._telemetry:
+                        self._telemetry.popitem(last=False)
+                        self.dropped_telemetry += 1
+                    else:
+                        for index, (queued_key, _) in enumerate(self._critical):
+                            if queued_key == key:
+                                del self._critical[index]
+                                break
+                        else:
+                            self._critical.popleft()
+                            self.dropped_critical += 1
+                self._critical.append((key, event))
+            else:
+                if key in self._telemetry:
+                    self._telemetry.pop(key)
+                    self.dropped_telemetry += 1
+                elif len(self._telemetry) >= self._max_queue_size:
+                    self._telemetry.popitem(last=False)
+                    self.dropped_telemetry += 1
+                self._telemetry[key] = event
+            self._condition.notify()
+
+    def get(self, timeout=None):
+        deadline = None if timeout is None else time.monotonic() + timeout
+        with self._condition:
+            while not self._critical and not self._telemetry:
+                remaining = None if deadline is None else deadline - time.monotonic()
+                if remaining is not None and remaining <= 0:
+                    raise queue.Empty
+                self._condition.wait(remaining)
+            if self._critical:
+                return self._critical.popleft()[1]
+            return self._telemetry.popitem(last=False)[1]
+
+    def get_nowait(self):
+        return self.get(timeout=0)
+
+    def empty(self):
+        with self._condition:
+            return not self._critical and not self._telemetry
+
+    @property
+    def pending_count(self):
+        with self._condition:
+            return len(self._critical) + len(self._telemetry)
+
+
+class CommandLeaseManager:
+    def __init__(self, timeout_seconds=COMMAND_TIMEOUT_SECONDS):
+        if timeout_seconds <= 0:
+            raise ValueError("timeout_seconds must be positive")
+        self._timeout_seconds = timeout_seconds
+        self._deadlines = {}
+        self._lock = threading.Lock()
+
+    def refresh(self, key, now=None):
+        current_time = time.monotonic() if now is None else now
+        with self._lock:
+            self._deadlines[key] = current_time + self._timeout_seconds
+
+    def take_expired(self, now=None):
+        current_time = time.monotonic() if now is None else now
+        with self._lock:
+            expired = [
+                key for key, deadline in self._deadlines.items()
+                if current_time >= deadline
+            ]
+            for key in expired:
+                del self._deadlines[key]
+            return expired
+
+
+def validate_twist_request(request, now_ns=None):
     if len(request.topic_name) > 200 or not ROS_TOPIC_PATTERN.fullmatch(request.topic_name):
         raise ValueError("Invalid ROS topic name")
 
-    for name, value, limit in (
-        ("linear_x", request.linear_x, MAX_LINEAR_VELOCITY),
-        ("linear_y", request.linear_y, MAX_LINEAR_VELOCITY),
-        ("linear_z", request.linear_z, MAX_LINEAR_VELOCITY),
-        ("angular_x", request.angular_x, MAX_ANGULAR_VELOCITY),
-        ("angular_y", request.angular_y, MAX_ANGULAR_VELOCITY),
-        ("angular_z", request.angular_z, MAX_ANGULAR_VELOCITY),
+    timestamp_ns = getattr(request, "timestamp_ns", None)
+    if not isinstance(timestamp_ns, int) or timestamp_ns <= 0:
+        raise ValueError("Command timestamp is missing or invalid")
+    current_time_ns = time.time_ns() if now_ns is None else now_ns
+    max_age_ns = int(MAX_COMMAND_AGE_SECONDS * 1_000_000_000)
+    if timestamp_ns > current_time_ns + 250_000_000 or current_time_ns - timestamp_ns > max_age_ns:
+        raise ValueError("Command is stale or has a future timestamp")
+
+    linear = (request.linear_x, request.linear_y, request.linear_z)
+    angular = (request.angular_x, request.angular_y, request.angular_z)
+    if not all(math.isfinite(value) for value in (*linear, *angular)):
+        raise ValueError("Velocity components must be finite")
+    if math.hypot(*linear) > MAX_LINEAR_VELOCITY:
+        raise ValueError(
+            f"Linear velocity magnitude must not exceed {MAX_LINEAR_VELOCITY}"
+        )
+    if math.hypot(*angular) > MAX_ANGULAR_VELOCITY:
+        raise ValueError(
+            f"Angular velocity magnitude must not exceed {MAX_ANGULAR_VELOCITY}"
+        )
+
+
+def is_valid_auth_token(provided, expected, expires_at, now=None):
+    if not isinstance(provided, str) or not isinstance(expected, str):
+        return False
+    try:
+        expiration = float(expires_at)
+    except (TypeError, ValueError):
+        return False
+    current_time = time.time() if now is None else now
+    if not math.isfinite(expiration) or current_time >= expiration:
+        return False
+    return hmac.compare_digest(provided, expected)
+
+
+def load_auth_credentials(path):
+    import json
+
+    with open(path, encoding="utf-8") as credential_file:
+        credentials = json.load(credential_file)
+    token = credentials.get("token")
+    expires_at = credentials.get("expires_at")
+    if (
+        not isinstance(token, str)
+        or len(token) != 64
+        or any(character not in "0123456789abcdef" for character in token)
     ):
-        if not math.isfinite(value) or abs(value) > limit:
-            raise ValueError(f"{name} must be finite and between -{limit} and {limit}")
+        raise ValueError("Invalid gRPC credential file")
+    if not isinstance(expires_at, (int, float)) or not math.isfinite(expires_at):
+        raise ValueError("Invalid gRPC credential expiration")
+    return token, float(expires_at)
diff --git a/src-tauri/resources/devcontainer/test_server_core.py b/src-tauri/resources/devcontainer/test_server_core.py
index ca020ab..6793a06 100644
--- a/src-tauri/resources/devcontainer/test_server_core.py
+++ b/src-tauri/resources/devcontainer/test_server_core.py
@@ -1,19 +1,30 @@
 import math
+import json
+import tempfile
+import time
 import unittest
 from types import SimpleNamespace
 
-from server_core import EventHub, validate_twist_request
+from server_core import (
+    CommandLeaseManager,
+    EventHub,
+    is_valid_auth_token,
+    load_auth_credentials,
+    validate_twist_request,
+)
 
 
-def request(topic="/robot/cmd_vel", value=0.0):
+def request(topic="/robot/cmd_vel", value=0.0, timestamp_ns=None, linear_y=0.0,
+            angular_x=0.0, angular_z=0.0):
     return SimpleNamespace(
         topic_name=topic,
         linear_x=value,
-        linear_y=0.0,
+        linear_y=linear_y,
         linear_z=0.0,
-        angular_x=0.0,
+        angular_x=angular_x,
         angular_y=0.0,
-        angular_z=0.0,
+        angular_z=angular_z,
+        timestamp_ns=time.time_ns() if timestamp_ns is None else timestamp_ns,
     )
 
 
@@ -37,6 +48,62 @@ class EventHubTests(unittest.TestCase):
             hub.publish(value)
 
         self.assertEqual([subscriber.get_nowait(), subscriber.get_nowait()], [1, 2])
+        self.assertEqual(subscriber.dropped_critical, 1)
+
+    def test_telemetry_is_coalesced_by_key_and_drop_counted(self):
+        hub = EventHub(max_queue_size=3)
+        subscriber = hub.subscribe()
+
+        hub.publish("odom-1", priority="telemetry", key="odom")
+        hub.publish("odom-2", priority="telemetry", key="odom")
+        hub.publish("joint-1", priority="telemetry", key="joint")
+        hub.publish("tf-1", priority="telemetry", key="tf")
+
+        self.assertEqual(
+            [subscriber.get_nowait(), subscriber.get_nowait(), subscriber.get_nowait()],
+            ["odom-2", "joint-1", "tf-1"],
+        )
+        self.assertEqual(subscriber.dropped_telemetry, 1)
+
+    def test_critical_event_is_delivered_ahead_of_telemetry(self):
+        hub = EventHub(max_queue_size=1)
+        subscriber = hub.subscribe()
+
+        hub.publish("telemetry", priority="telemetry", key="odom")
+        hub.publish("node-crashed", key=("node", "camera"))
+
+        self.assertEqual(subscriber.get_nowait(), "node-crashed")
+        self.assertEqual(subscriber.get_nowait(), "telemetry")
+        self.assertEqual(subscriber.dropped_telemetry, 0)
+
+    def test_reconnecting_subscriber_receives_latest_snapshots(self):
+        hub = EventHub()
+        hub.publish("topics-v1", key="topics", snapshot=True)
+        hub.publish("topics-v2", key="topics", snapshot=True)
+        hub.publish("nodes", key="nodes", snapshot=True)
+
+        subscriber = hub.subscribe()
+
+        self.assertEqual(
+            {subscriber.get_nowait(), subscriber.get_nowait()},
+            {"topics-v2", "nodes"},
+        )
+
+    def test_slow_subscriber_memory_stays_bounded_under_telemetry_load(self):
+        hub = EventHub(max_queue_size=8)
+        slow = hub.subscribe()
+        another_client = hub.subscribe()
+
+        for value in range(10_000):
+            hub.publish(value, priority="telemetry", key=f"topic-{value % 4}")
+
+        self.assertLessEqual(slow.pending_count, 8)
+        self.assertLessEqual(another_client.pending_count, 8)
+        self.assertEqual(slow.dropped_telemetry, 9_996)
+        self.assertEqual(another_client.get_nowait(), 9_996)
+        self.assertEqual(another_client.get_nowait(), 9_997)
+        self.assertEqual(another_client.get_nowait(), 9_998)
+        self.assertEqual(another_client.get_nowait(), 9_999)
 
     def test_unsubscribed_client_receives_no_events(self):
         hub = EventHub()
@@ -51,16 +118,68 @@ class EventHubTests(unittest.TestCase):
 class TwistValidationTests(unittest.TestCase):
     def test_accepts_valid_request(self):
         validate_twist_request(request())
+        validate_twist_request(request(value=0.5))
 
     def test_rejects_malformed_topic(self):
         with self.assertRaises(ValueError):
             validate_twist_request(request("/robot/../cmd_vel"))
 
     def test_rejects_non_finite_or_unbounded_velocity(self):
-        for value in (math.nan, math.inf, 5.01):
+        for value in (math.nan, math.inf, 0.51):
             with self.subTest(value=value), self.assertRaises(ValueError):
                 validate_twist_request(request(value=value))
 
+    def test_rejects_velocity_vectors_over_magnitude_limits(self):
+        with self.assertRaisesRegex(ValueError, "Linear velocity magnitude"):
+            validate_twist_request(request(value=0.4, linear_y=0.4))
+        with self.assertRaisesRegex(ValueError, "Angular velocity magnitude"):
+            validate_twist_request(request(angular_x=0.8, angular_z=0.8))
+
+    def test_command_lease_expires_and_refreshes(self):
+        leases = CommandLeaseManager(timeout_seconds=0.5)
+        leases.refresh("/robot/cmd_vel", now=10.0)
+        leases.refresh("/robot/cmd_vel", now=10.25)
+
+        self.assertEqual(leases.take_expired(now=10.74), [])
+        self.assertEqual(leases.take_expired(now=10.75), ["/robot/cmd_vel"])
+        self.assertEqual(leases.take_expired(now=11.0), [])
+
+    def test_rejects_stale_or_future_commands(self):
+        now_ns = 10_000_000_000
+        with self.assertRaises(ValueError):
+            validate_twist_request(request(timestamp_ns=now_ns - 1_000_000_001), now_ns)
+        with self.assertRaises(ValueError):
+            validate_twist_request(request(timestamp_ns=now_ns + 250_000_001), now_ns)
+
+
+class AuthenticationTests(unittest.TestCase):
+    def test_accepts_valid_unexpired_token(self):
+        self.assertTrue(is_valid_auth_token("session-token", "session-token", 101, now=100))
+
+    def test_rejects_missing_invalid_expired_and_rotated_tokens(self):
+        self.assertFalse(is_valid_auth_token(None, "session-token", 101, now=100))
+        self.assertFalse(is_valid_auth_token("wrong", "session-token", 101, now=100))
+        self.assertFalse(is_valid_auth_token("session-token", "session-token", 100, now=100))
+        self.assertFalse(is_valid_auth_token("old-token", "new-token", 101, now=100))
+
+    def test_loads_credentials_without_logging_them(self):
+        token = "a" * 64
+        with tempfile.NamedTemporaryFile(mode="w+", encoding="utf-8") as credential_file:
+            json.dump({"token": token, "expires_at": 200}, credential_file)
+            credential_file.flush()
+
+            self.assertEqual(load_auth_credentials(credential_file.name), (token, 200.0))
+
+    def test_rejects_malformed_credentials(self):
+        for token in ("short", "g" * 64):
+            with self.subTest(token=token), tempfile.NamedTemporaryFile(
+                mode="w+", encoding="utf-8"
+            ) as credential_file:
+                json.dump({"token": token, "expires_at": 200}, credential_file)
+                credential_file.flush()
+                with self.assertRaises(ValueError):
+                    load_auth_credentials(credential_file.name)
+
 
 if __name__ == "__main__":
     unittest.main()
diff --git a/src-tauri/src/lib.rs b/src-tauri/src/lib.rs
index 1da586b..8fd1649 100644
--- a/src-tauri/src/lib.rs
+++ b/src-tauri/src/lib.rs
@@ -1,4 +1,167 @@
+use std::collections::HashSet;
 use std::fs;
+use std::path::{Component, Path, PathBuf};
+use std::sync::Mutex;
+
+const MAX_FILE_SIZE: u64 = 8 * 1024 * 1024;
+const MAX_WORLD_SIZE: u64 = 10 * 1024 * 1024;
+const MAX_DIRECTORY_ENTRIES: usize = 10_000;
+
+#[derive(Default)]
+struct FsAccess {
+    approved_roots: Mutex<HashSet<PathBuf>>,
+}
+
+impl FsAccess {
+    fn approve_directory(&self, path: &Path) -> Result<PathBuf, String> {
+        let canonical = path
+            .canonicalize()
+            .map_err(|error| format!("Could not resolve selected directory: {error}"))?;
+        if !canonical.is_dir() {
+            return Err("Selected path is not a directory".to_string());
+        }
+
+        self.approved_roots
+            .lock()
+            .map_err(|_| "Filesystem access state is unavailable".to_string())?
+            .insert(canonical.clone());
+        Ok(canonical)
+    }
+
+    fn resolve_existing(&self, path: &Path) -> Result<PathBuf, String> {
+        self.resolve(path, false)
+    }
+
+    fn resolve_for_write(&self, path: &Path) -> Result<PathBuf, String> {
+        self.resolve(path, true)
+    }
+
+    fn resolve(&self, path: &Path, allow_missing_leaf: bool) -> Result<PathBuf, String> {
+        if !path.is_absolute()
+            || path
+                .components()
+                .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
+        {
+            return Err(
+                "Path must be absolute and cannot contain traversal components".to_string(),
+            );
+        }
+
+        let candidate = if allow_missing_leaf {
+            let parent = path
+                .parent()
+                .ok_or_else(|| "Invalid destination path".to_string())?
+                .canonicalize()
+                .map_err(|error| format!("Invalid destination directory: {error}"))?;
+            parent.join(
+                path.file_name()
+                    .ok_or_else(|| "Invalid destination filename".to_string())?,
+            )
+        } else {
+            path.canonicalize()
+                .map_err(|error| format!("Could not resolve path: {error}"))?
+        };
+
+        let roots = self
+            .approved_roots
+            .lock()
+            .map_err(|_| "Filesystem access state is unavailable".to_string())?;
+        for root in roots.iter() {
+            let current_root = root
+                .canonicalize()
+                .map_err(|_| "An approved directory is no longer available".to_string())?;
+            if current_root == *root && candidate.starts_with(root) {
+                if allow_missing_leaf && candidate.exists() {
+                    let metadata =
+                        fs::symlink_metadata(&candidate).map_err(|error| error.to_string())?;
+                    if metadata.file_type().is_symlink() {
+                        return Err(
+                            "Symbolic links are not allowed as file destinations".to_string()
+                        );
+                    }
+                }
+                return Ok(candidate);
+            }
+        }
+
+        Err("Path is outside approved directories".to_string())
+    }
+}
+
+fn read_approved_file(access: &FsAccess, path: &Path) -> Result<String, String> {
+    let path = access.resolve_existing(path)?;
+    let metadata = fs::metadata(&path).map_err(|error| error.to_string())?;
+    if !metadata.is_file() {
+        return Err("Path is not a regular file".to_string());
+    }
+    if metadata.len() > MAX_FILE_SIZE {
+        return Err(format!("File exceeds the {MAX_FILE_SIZE}-byte limit"));
+    }
+    read_text_file_limited(&path, MAX_FILE_SIZE)
+}
+
+fn read_text_file_limited(path: &Path, max_bytes: u64) -> Result<String, String> {
+    let file = fs::File::open(path).map_err(|error| error.to_string())?;
+    let mut contents = String::new();
+    file.take(max_bytes + 1)
+        .read_to_string(&mut contents)
+        .map_err(|error| error.to_string())?;
+    if contents.len() as u64 > max_bytes {
+        return Err(format!("File exceeds the {max_bytes}-byte limit"));
+    }
+    Ok(contents)
+}
+
+fn write_approved_file(access: &FsAccess, path: &Path, contents: &str) -> Result<(), String> {
+    if contents.len() as u64 > MAX_FILE_SIZE {
+        return Err(format!("File exceeds the {MAX_FILE_SIZE}-byte limit"));
+    }
+    let path = access.resolve_for_write(path)?;
+    if path.exists() {
+        let metadata = fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
+        if metadata.file_type().is_symlink() || !metadata.is_file() {
+            return Err("Destination must be a regular file, not a symbolic link".to_string());
+        }
+    }
+    let parent = path
+        .parent()
+        .ok_or_else(|| "Destination has no parent directory".to_string())?;
+    let file_name = path
+        .file_name()
+        .ok_or_else(|| "Destination has no filename".to_string())?
+        .to_string_lossy();
+    let counter = std::sync::atomic::AtomicU64::fetch_add(
+        &FILE_WRITE_COUNTER,
+        1,
+        std::sync::atomic::Ordering::Relaxed,
+    );
+    let temporary = parent.join(format!(
+        ".{file_name}.{}.{}.tmp",
+        std::process::id(),
+        counter
+    ));
+    let write_result = (|| {
+        let mut file = std::fs::OpenOptions::new()
+            .write(true)
+            .create_new(true)
+            .open(&temporary)
+            .map_err(|error| error.to_string())?;
+        std::io::Write::write_all(&mut file, contents.as_bytes())
+            .map_err(|error| error.to_string())?;
+        file.sync_all().map_err(|error| error.to_string())?;
+        #[cfg(windows)]
+        if path.exists() {
+            fs::remove_file(&path).map_err(|error| error.to_string())?;
+        }
+        fs::rename(&temporary, &path).map_err(|error| error.to_string())
+    })();
+    if write_result.is_err() {
+        let _ = fs::remove_file(&temporary);
+    }
+    write_result
+}
+
+static FILE_WRITE_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
 
 /// Very small gitignore-style pattern matcher, supporting the common cases:
 ///   *.ext          — match by extension/suffix anywhere in the filename
@@ -18,16 +181,14 @@ fn matches_aiignore_pattern(rel_path: &str, pattern: &str) -> bool {
     }
 
     if let Some(dir) = pattern.strip_suffix('/') {
-        return rel_path
-            .split('/')
-            .any(|component| component == dir);
+        return rel_path.split('/').any(|component| component == dir);
     }
 
     if let Some(ext) = pattern.strip_prefix("*.") {
         return rel_path
             .rsplit('/')
             .next()
-            .map_or(false, |name| name.ends_with(&format!(".{}", ext)));
+            .is_some_and(|name| name.ends_with(&format!(".{}", ext)));
     }
 
     if pattern.contains('*') {
@@ -43,25 +204,40 @@ fn matches_aiignore_pattern(rel_path: &str, pattern: &str) -> bool {
 
 /// Loads `.aiignore` from the given workspace root, if present, and returns
 /// whether `file_path` (absolute) should be excluded from AI context.
-fn is_ignored_by_aiignore(workspace_root: &str, file_path: &str) -> bool {
-    let ignore_file = std::path::Path::new(workspace_root).join(".aiignore");
-    let Ok(contents) = fs::read_to_string(&ignore_file) else {
-        return false;
+fn is_ignored_by_aiignore(
+    access: &FsAccess,
+    workspace_root: &Path,
+    file_path: &Path,
+) -> Result<bool, String> {
+    let workspace_root = access.resolve_existing(workspace_root)?;
+    if !workspace_root.is_dir() {
+        return Err("Workspace path is not a directory".to_string());
+    }
+    let file_path = access.resolve_existing(file_path)?;
+    if !file_path.starts_with(&workspace_root) {
+        return Err("File is outside the selected workspace".to_string());
+    }
+    let ignore_file = workspace_root.join(".aiignore");
+    let contents = match access.resolve_existing(&ignore_file) {
+        Ok(path) => read_approved_file(access, &path)?,
+        Err(_) if !ignore_file.exists() => return Ok(false),
+        Err(error) => return Err(error),
     };
 
-    let rel_path = std::path::Path::new(file_path)
-        .strip_prefix(workspace_root)
-        .map(|p| p.to_string_lossy().to_string())
-        .unwrap_or_else(|_| file_path.to_string());
+    let rel_path = file_path
+        .strip_prefix(&workspace_root)
+        .map_err(|_| "File is outside the selected workspace".to_string())?
+        .to_string_lossy()
+        .to_string();
 
-    contents
+    Ok(contents
         .lines()
-        .any(|pattern| matches_aiignore_pattern(&rel_path, pattern))
+        .any(|pattern| matches_aiignore_pattern(&rel_path, pattern)))
 }
 
 #[tauri::command]
-fn read_file(path: String) -> Result<String, String> {
-    fs::read_to_string(&path).map_err(|e| e.to_string())
+fn read_file(access: tauri::State<'_, FsAccess>, path: String) -> Result<String, String> {
+    read_approved_file(&access, Path::new(&path))
 }
 
 #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
@@ -72,7 +248,8 @@ struct RecentProject {
 }
 
 fn normalize_recent_projects(mut projects: Vec<RecentProject>) -> Vec<RecentProject> {
-    let mut by_path: std::collections::HashMap<String, RecentProject> = std::collections::HashMap::new();
+    let mut by_path: std::collections::HashMap<String, RecentProject> =
+        std::collections::HashMap::new();
 
     for project in projects.drain(..) {
         let path = project.path.trim().to_string();
@@ -113,8 +290,12 @@ fn normalize_recent_projects(mut projects: Vec<RecentProject>) -> Vec<RecentProj
 }
 
 #[tauri::command]
-fn write_file(path: String, contents: String) -> Result<(), String> {
-    fs::write(&path, contents).map_err(|e| e.to_string())
+fn write_file(
+    access: tauri::State<'_, FsAccess>,
+    path: String,
+    contents: String,
+) -> Result<(), String> {
+    write_approved_file(&access, Path::new(&path), &contents)
 }
 
 #[derive(serde::Serialize)]
@@ -125,11 +306,20 @@ struct FileEntry {
 }
 
 #[tauri::command]
-fn list_dir(path: String) -> Result<Vec<FileEntry>, String> {
+fn list_dir(access: tauri::State<'_, FsAccess>, path: String) -> Result<Vec<FileEntry>, String> {
+    let path = access.resolve_existing(Path::new(&path))?;
+    if !path.is_dir() {
+        return Err("Path is not a directory".to_string());
+    }
     let entries = fs::read_dir(&path).map_err(|e| e.to_string())?;
     let mut result = Vec::new();
 
-    for entry in entries {
+    for (index, entry) in entries.enumerate() {
+        if index >= MAX_DIRECTORY_ENTRIES {
+            return Err(format!(
+                "Directory contains more than {MAX_DIRECTORY_ENTRIES} entries"
+            ));
+        }
         let entry = entry.map_err(|e| e.to_string())?;
         let file_type = entry.file_type().map_err(|e| e.to_string())?;
         let name = entry.file_name().to_string_lossy().to_string();
@@ -145,14 +335,350 @@ fn list_dir(path: String) -> Result<Vec<FileEntry>, String> {
     Ok(result)
 }
 
+#[tauri::command]
+async fn pick_directory(
+    app: AppHandle,
+    access: tauri::State<'_, FsAccess>,
+) -> Result<Option<String>, String> {
+    use tauri_plugin_dialog::DialogExt;
+
+    let selected =
+        tauri::async_runtime::spawn_blocking(move || app.dialog().file().blocking_pick_folder())
+            .await
+            .map_err(|error| format!("Directory picker failed: {error}"))?;
+    let Some(selected) = selected else {
+        return Ok(None);
+    };
+    let selected = selected
+        .into_path()
+        .map_err(|error| format!("Invalid selected directory: {error}"))?;
+    let approved = access.approve_directory(&selected)?;
+    Ok(Some(approved.to_string_lossy().into_owned()))
+}
+
+#[tauri::command]
+async fn pick_save_path(
+    app: AppHandle,
+    is_xml: bool,
+    access: tauri::State<'_, FsAccess>,
+) -> Result<Option<String>, String> {
+    use tauri_plugin_dialog::DialogExt;
+
+    let selected = tauri::async_runtime::spawn_blocking(move || {
+        let dialog = app.dialog().file().set_title("Save generated file");
+        if is_xml {
+            dialog
+                .add_filter("URDF/XML", &["urdf", "xacro", "xml"])
+                .blocking_save_file()
+        } else {
+            dialog.add_filter("Python", &["py"]).blocking_save_file()
+        }
+    })
+    .await
+    .map_err(|error| format!("Save dialog failed: {error}"))?;
+    let Some(selected) = selected else {
+        return Ok(None);
+    };
+    let selected = selected
+        .into_path()
+        .map_err(|error| format!("Invalid selected file: {error}"))?;
+    let parent = selected
+        .parent()
+        .ok_or_else(|| "Selected file has no parent directory".to_string())?;
+    access.approve_directory(parent)?;
+    let safe_path = access.resolve_for_write(&selected)?;
+    Ok(Some(safe_path.to_string_lossy().into_owned()))
+}
+
+fn validate_sdf_file(path: &Path, content: &str) -> Result<(), String> {
+    if path
+        .extension()
+        .and_then(|extension| extension.to_str())
+        .is_none_or(|extension| !extension.eq_ignore_ascii_case("sdf"))
+    {
+        return Err("World files must use the .sdf extension".to_string());
+    }
+    if content.len() as u64 > MAX_WORLD_SIZE {
+        return Err(format!(
+            "World file exceeds the {MAX_WORLD_SIZE}-byte limit"
+        ));
+    }
+    if content.contains('\0') {
+        return Err("World file contains a NUL character".to_string());
+    }
+
+    fn local_name(name: &[u8]) -> &[u8] {
+        name.rsplit(|byte| *byte == b':').next().unwrap_or(name)
+    }
+
+    fn version_from_attributes(
+        element: &quick_xml::events::BytesStart<'_>,
+        decoder: quick_xml::encoding::Decoder,
+    ) -> Result<String, String> {
+        for attribute in element.attributes().with_checks(true) {
+            let attribute =
+                attribute.map_err(|error| format!("Malformed SDF XML attribute: {error}"))?;
+            if attribute.key.as_ref() == b"version" {
+                return attribute
+                    .decoded_and_normalized_value(quick_xml::XmlVersion::Implicit1_0, decoder)
+                    .map(|value| value.into_owned())
+                    .map_err(|error| format!("Malformed SDF version attribute: {error}"));
+            }
+        }
+        Err("SDF root element is missing its version attribute".to_string())
+    }
+
+    let mut reader = quick_xml::Reader::from_str(content);
+    reader.config_mut().trim_text(true);
+    let mut root_seen = false;
+    let mut root_closed = false;
+    let mut world_seen = false;
+    let mut depth = 0usize;
+    let mut version = None;
+    loop {
+        match reader.read_event() {
+            Ok(quick_xml::events::Event::Start(element)) => {
+                if !root_seen {
+                    if local_name(element.name().as_ref()) != b"sdf" {
+                        return Err("World file root element must be <sdf>".to_string());
+                    }
+                    version = Some(version_from_attributes(&element, reader.decoder())?);
+                    root_seen = true;
+                } else if root_closed {
+                    return Err("SDF document contains content after its root element".to_string());
+                } else if depth == 1 && local_name(element.name().as_ref()) == b"world" {
+                    world_seen = true;
+                }
+                depth += 1;
+            }
+            Ok(quick_xml::events::Event::Empty(element)) => {
+                if !root_seen {
+                    if local_name(element.name().as_ref()) != b"sdf" {
+                        return Err("World file root element must be <sdf>".to_string());
+                    }
+                    version = Some(version_from_attributes(&element, reader.decoder())?);
+                    root_seen = true;
+                    root_closed = true;
+                } else if root_closed {
+                    return Err("SDF document contains content after its root element".to_string());
+                } else if depth == 1 && local_name(element.name().as_ref()) == b"world" {
+                    world_seen = true;
+                }
+            }
+            Ok(quick_xml::events::Event::End(_)) => {
+                depth = depth.saturating_sub(1);
+                if depth == 0 {
+                    root_closed = true;
+                }
+            }
+            Ok(quick_xml::events::Event::DocType(_)) => {
+                return Err("SDF documents must not contain a DTD".to_string());
+            }
+            Ok(quick_xml::events::Event::Text(text)) if depth == 0 => {
+                let text_bytes: &[u8] = text.as_ref();
+                if !text_bytes.iter().all(u8::is_ascii_whitespace) {
+                    return Err("SDF document contains text outside its root element".to_string());
+                }
+            }
+            Ok(quick_xml::events::Event::CData(_)) if depth == 0 => {
+                return Err("SDF document contains data outside its root element".to_string());
+            }
+            Ok(quick_xml::events::Event::Eof) => break,
+            Ok(_) => {}
+            Err(error) => return Err(format!("Malformed SDF XML: {error}")),
+        }
+    }
+
+    if !root_seen || !root_closed || depth != 0 {
+        return Err("SDF document is incomplete".to_string());
+    }
+    let version = version.ok_or_else(|| "SDF version is missing".to_string())?;
+    if !world_seen {
+        return Err("SDF world document must contain a <world> element".to_string());
+    }
+    if !matches!(
+        version.as_str(),
+        "1.4" | "1.5" | "1.6" | "1.7" | "1.8" | "1.9" | "1.10"
+    ) {
+        return Err(format!("Unsupported SDF version: {version}"));
+    }
+    Ok(())
+}
+
+fn import_world_contents(
+    workspace: &Path,
+    source: &Path,
+    content: &str,
+) -> Result<PathBuf, String> {
+    validate_sdf_file(source, content)?;
+    let workspace = workspace
+        .canonicalize()
+        .map_err(|error| format!("ROS workspace is unavailable: {error}"))?;
+    let worlds_dir = workspace.join("worlds");
+    match fs::symlink_metadata(&worlds_dir) {
+        Ok(metadata) if metadata.file_type().is_symlink() => {
+            return Err("World import directory must not be a symbolic link".to_string());
+        }
+        Ok(metadata) if !metadata.is_dir() => {
+            return Err("World import path is not a directory".to_string());
+        }
+        Ok(_) => {}
+        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
+            fs::create_dir(&worlds_dir).map_err(|error| error.to_string())?;
+        }
+        Err(error) => return Err(error.to_string()),
+    }
+    let canonical_worlds = worlds_dir
+        .canonicalize()
+        .map_err(|error| format!("Could not resolve world import directory: {error}"))?;
+    if !canonical_worlds.starts_with(&workspace) {
+        return Err("World import directory escapes the ROS workspace".to_string());
+    }
+
+    let source_stem = source
+        .file_stem()
+        .and_then(|stem| stem.to_str())
+        .unwrap_or("world");
+    let safe_stem = source_stem
+        .chars()
+        .filter(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
+        .take(48)
+        .collect::<String>();
+    let safe_stem = if safe_stem.is_empty() {
+        "world"
+    } else {
+        &safe_stem
+    };
+    let timestamp = std::time::SystemTime::now()
+        .duration_since(std::time::UNIX_EPOCH)
+        .map_err(|error| error.to_string())?
+        .as_nanos();
+
+    for attempt in 0..100u32 {
+        let name = format!(
+            "{safe_stem}-{timestamp}-{}-{attempt}.sdf",
+            std::process::id()
+        );
+        let destination = canonical_worlds.join(name);
+        match std::fs::OpenOptions::new()
+            .write(true)
+            .create_new(true)
+            .open(&destination)
+        {
+            Ok(mut file) => {
+                if let Err(error) = std::io::Write::write_all(&mut file, content.as_bytes()) {
+                    let _ = fs::remove_file(&destination);
+                    return Err(format!("Could not import world file: {error}"));
+                }
+                return Ok(destination);
+            }
+            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
+            Err(error) => return Err(format!("Could not import world file: {error}")),
+        }
+    }
+    Err("Could not allocate a unique world filename".to_string())
+}
+
+#[tauri::command]
+async fn import_world(
+    app: AppHandle,
+    access: tauri::State<'_, FsAccess>,
+) -> Result<Option<String>, String> {
+    use tauri_plugin_dialog::DialogExt;
+
+    let selected = tauri::async_runtime::spawn_blocking(move || {
+        app.dialog()
+            .file()
+            .add_filter("SDF World", &["sdf"])
+            .blocking_pick_file()
+    })
+    .await
+    .map_err(|error| format!("World picker failed: {error}"))?;
+    let Some(selected) = selected else {
+        return Ok(None);
+    };
+    let source = selected
+        .into_path()
+        .map_err(|error| format!("Invalid selected world file: {error}"))?;
+    let metadata = fs::metadata(&source).map_err(|error| error.to_string())?;
+    if !metadata.is_file() || metadata.len() > MAX_WORLD_SIZE {
+        return Err(format!(
+            "World file must be a regular file no larger than {MAX_WORLD_SIZE} bytes"
+        ));
+    }
+    let content = read_text_file_limited(&source, MAX_WORLD_SIZE)
+        .map_err(|error| format!("Could not read selected world file: {error}"))?;
+    validate_sdf_file(&source, &content)?;
+
+    let workspace = PathBuf::from(get_default_workspace_dir()?);
+    fs::create_dir_all(&workspace).map_err(|error| error.to_string())?;
+    access.approve_directory(&workspace)?;
+    let imported = import_world_contents(&workspace, &source, &content)?;
+    Ok(Some(imported.to_string_lossy().into_owned()))
+}
+
+#[tauri::command]
+async fn pick_keybindings_file(app: AppHandle) -> Result<Option<String>, String> {
+    use tauri_plugin_dialog::DialogExt;
+
+    let selected = tauri::async_runtime::spawn_blocking(move || {
+        app.dialog()
+            .file()
+            .add_filter("JSON", &["json"])
+            .blocking_pick_file()
+    })
+    .await
+    .map_err(|error| format!("File picker failed: {error}"))?;
+    let Some(selected) = selected else {
+        return Ok(None);
+    };
+    let path = selected
+        .into_path()
+        .map_err(|error| format!("Invalid selected file: {error}"))?;
+    let metadata = fs::metadata(&path).map_err(|error| error.to_string())?;
+    if !metadata.is_file() || metadata.len() > MAX_FILE_SIZE {
+        return Err("Selected file is not a regular file or exceeds the size limit".to_string());
+    }
+    read_text_file_limited(&path, MAX_FILE_SIZE)
+        .map(Some)
+        .map_err(|error| error.to_string())
+}
+
 #[cfg_attr(mobile, tauri::mobile_entry_point)]
 pub fn run() {
     tauri::Builder::default()
+        .manage(FsAccess::default())
         .plugin(tauri_plugin_dialog::init())
         .plugin(tauri_plugin_opener::init())
         .invoke_handler(tauri::generate_handler![
-            read_file, write_file, list_dir, start_ros_stream, run_colcon_build, run_colcon_build_streaming, start_gazebo_sim, stop_gazebo_sim, reset_gazebo_sim, ask_ai, initialize_ros_environment, publish_twist,
-            save_ai_settings, load_ai_settings, save_keybindings, load_keybindings, save_recent_projects, load_recent_projects, get_system_specs, list_templates, create_project_from_template, check_aiignore
+            read_file,
+            write_file,
+            list_dir,
+            pick_directory,
+            pick_save_path,
+            pick_keybindings_file,
+            import_world,
+            start_ros_stream,
+            run_colcon_build,
+            run_colcon_build_streaming,
+            start_gazebo_sim,
+            stop_gazebo_sim,
+            reset_gazebo_sim,
+            ask_ai,
+            initialize_ros_environment,
+            publish_twist,
+            save_ai_settings,
+            load_ai_settings,
+            delete_ai_credential,
+            has_ai_credential,
+            save_keybindings,
+            load_keybindings,
+            save_recent_projects,
+            load_recent_projects,
+            get_system_specs,
+            list_templates,
+            create_project_from_template,
+            check_aiignore
         ])
         .run(tauri::generate_context!())
         .expect("error while running tauri application");
@@ -165,11 +691,16 @@ pub mod rosengine {
 use rosengine::ros_engine_client::RosEngineClient;
 use rosengine::Empty;
 
-use tauri::{AppHandle, Emitter};
 use std::sync::atomic::{AtomicBool, Ordering};
+use tauri::{AppHandle, Emitter};
 
 static STREAM_RUNNING: AtomicBool = AtomicBool::new(false);
 
+fn stop_stream_for_auth_recovery(app: &AppHandle, message: &str) {
+    let _ = app.emit("ros-auth-error", message.to_string());
+    STREAM_RUNNING.store(false, Ordering::SeqCst);
+}
+
 #[tauri::command]
 async fn start_ros_stream(app: AppHandle) -> Result<(), String> {
     println!("start_ros_stream invoked");
@@ -180,52 +711,75 @@ async fn start_ros_stream(app: AppHandle) -> Result<(), String> {
 
     tauri::async_runtime::spawn(async move {
         loop {
+            let token = match read_grpc_auth_token(&app) {
+                Ok(token) => token,
+                Err(error) => {
+                    stop_stream_for_auth_recovery(&app, &error);
+                    return;
+                }
+            };
             let _ = app.emit("ros-connecting", ());
             println!("[ros-stream] attempting connection to localhost:50051");
             let client_result = tokio::time::timeout(
-    std::time::Duration::from_secs(5),
-    RosEngineClient::connect("http://localhost:50051"),
-).await;
-
-let mut client = match client_result {
-    Ok(Ok(c)) => {
-    println!("[ros-stream] connected successfully");
-    c
-}
-    Ok(Err(e)) => {
-        println!("[ros-stream] connection failed: {}", e);
-        let _ = app.emit("ros-error", format!("Connection failed: {}", e));
-        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
-        continue;
-    }
-    Err(_) => {
-        println!("[ros-stream] connection attempt timed out");
-        let _ = app.emit("ros-error", "Connection attempt timed out".to_string());
-        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
-        continue;
-    }
-};
-let mut stream = match tokio::time::timeout(
-    std::time::Duration::from_secs(5),
-    client.stream_events(Empty {}),
-)
-.await
-{
-    Ok(Ok(s)) => {
-        println!("[ros-stream] stream_events established, awaiting messages");
-        s.into_inner()
-    }
-    Ok(Err(e)) => {
-        let _ = app.emit("ros-error", format!("Stream failed: {}", e));
-        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
-        continue;
-    }
-    Err(_) => {
-        let _ = app.emit("ros-error", "stream_events timed out".to_string());
-        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
-        continue;
-    }
-};
+                std::time::Duration::from_secs(5),
+                RosEngineClient::connect("http://localhost:50051"),
+            )
+            .await;
+
+            let mut client = match client_result {
+                Ok(Ok(c)) => {
+                    println!("[ros-stream] connected successfully");
+                    c
+                }
+                Ok(Err(e)) => {
+                    println!("[ros-stream] connection failed: {}", e);
+                    let _ = app.emit("ros-error", format!("Connection failed: {}", e));
+                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
+                    continue;
+                }
+                Err(_) => {
+                    println!("[ros-stream] connection attempt timed out");
+                    let _ = app.emit("ros-error", "Connection attempt timed out".to_string());
+                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
+                    continue;
+                }
+            };
+            let request = match authenticated_request(Empty {}, &token) {
+                Ok(request) => request,
+                Err(error) => {
+                    let _ = app.emit("ros-error", error);
+                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
+                    continue;
+                }
+            };
+            let mut stream = match tokio::time::timeout(
+                std::time::Duration::from_secs(5),
+                client.stream_events(request),
+            )
+            .await
+            {
+                Ok(Ok(s)) => {
+                    println!("[ros-stream] stream_events established, awaiting messages");
+                    s.into_inner()
+                }
+                Ok(Err(e)) if e.code() == tonic::Code::Unauthenticated => {
+                    stop_stream_for_auth_recovery(
+                        &app,
+                        "ROS authentication was rejected. The session will be renewed.",
+                    );
+                    return;
+                }
+                Ok(Err(e)) => {
+                    let _ = app.emit("ros-error", format!("Stream failed: {}", e));
+                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
+                    continue;
+                }
+                Err(_) => {
+                    let _ = app.emit("ros-error", "stream_events timed out".to_string());
+                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
+                    continue;
+                }
+            };
 
             loop {
                 match stream.message().await {
@@ -233,33 +787,43 @@ let mut stream = match tokio::time::timeout(
                         println!("[ros-stream] received event: {:?}", evt);
                         let _ = app.emit("ros-event", format!("{:?}", evt));
 
-                        if let Some(rosengine::workspace_event::Event::OdometryUpdate(odom)) = &evt.event {
-                            let _ = app.emit("odometry-update", OdometryUpdate {
-                                node: odom.topic_name.clone(),
-                                x: odom.x,
-                                y: odom.y,
-                                z: odom.z,
-                                qx: odom.qx,
-                                qy: odom.qy,
-                                qz: odom.qz,
-                                qw: odom.qw,
-                            });
+                        if let Some(rosengine::workspace_event::Event::OdometryUpdate(odom)) =
+                            &evt.event
+                        {
+                            let _ = app.emit(
+                                "odometry-update",
+                                OdometryUpdate {
+                                    node: odom.topic_name.clone(),
+                                    x: odom.x,
+                                    y: odom.y,
+                                    z: odom.z,
+                                    qx: odom.qx,
+                                    qy: odom.qy,
+                                    qz: odom.qz,
+                                    qw: odom.qw,
+                                },
+                            );
                         }
 
                         if let Some(rosengine::workspace_event::Event::TfUpdate(tf)) = &evt.event {
-                            let _ = app.emit("tf-update", TfUpdate {
-                                parent_frame: tf.parent_frame.clone(),
-                                child_frame: tf.child_frame.clone(),
-                                x: tf.x,
-                                y: tf.y,
-                                z: tf.z,
-                                qx: tf.qx,
-                                qy: tf.qy,
-                                qz: tf.qz,
-                                qw: tf.qw,
-                            });
+                            let _ = app.emit(
+                                "tf-update",
+                                TfUpdate {
+                                    parent_frame: tf.parent_frame.clone(),
+                                    child_frame: tf.child_frame.clone(),
+                                    x: tf.x,
+                                    y: tf.y,
+                                    z: tf.z,
+                                    qx: tf.qx,
+                                    qy: tf.qy,
+                                    qz: tf.qz,
+                                    qw: tf.qw,
+                                },
+                            );
                         }
-                        if let Some(rosengine::workspace_event::Event::NodeSnapshot(snap)) = &evt.event {
+                        if let Some(rosengine::workspace_event::Event::NodeSnapshot(snap)) =
+                            &evt.event
+                        {
                             let _ = app.emit("node-snapshot", snap.node_names.clone());
                         }
                     }
@@ -267,6 +831,13 @@ let mut stream = match tokio::time::timeout(
                         let _ = app.emit("ros-error", "Stream closed by server".to_string());
                         break;
                     }
+                    Err(e) if e.code() == tonic::Code::Unauthenticated => {
+                        stop_stream_for_auth_recovery(
+                            &app,
+                            "ROS authentication expired. The session will be renewed.",
+                        );
+                        return;
+                    }
                     Err(e) => {
                         let _ = app.emit("ros-error", format!("Stream error: {}", e));
                         break;
@@ -287,25 +858,22 @@ let mut stream = match tokio::time::timeout(
 
 use rosengine::TwistCommand;
 
-const MAX_LINEAR_VELOCITY: f64 = 5.0;
-const MAX_ANGULAR_VELOCITY: f64 = 5.0;
+const MAX_LINEAR_VELOCITY: f64 = 0.5;
+const MAX_ANGULAR_VELOCITY: f64 = 1.0;
 
 fn is_valid_ros_topic(topic_name: &str) -> bool {
     topic_name.len() <= 200
         && topic_name.starts_with('/')
-        && topic_name
-            .split('/')
-            .skip(1)
-            .all(|segment| {
-                !segment.is_empty()
-                    && segment
-                        .chars()
-                        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
-                    && segment
-                        .chars()
-                        .next()
-                        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
-            })
+        && topic_name.split('/').skip(1).all(|segment| {
+            !segment.is_empty()
+                && segment
+                    .chars()
+                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
+                && segment
+                    .chars()
+                    .next()
+                    .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
+        })
 }
 
 fn is_safe_project_component(value: &str) -> bool {
@@ -314,7 +882,10 @@ fn is_safe_project_component(value: &str) -> bool {
         && value
             .chars()
             .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
-        && value.chars().next().is_some_and(|ch| ch.is_ascii_alphabetic())
+        && value
+            .chars()
+            .next()
+            .is_some_and(|ch| ch.is_ascii_alphabetic())
 }
 
 fn validate_velocity(value: f64, maximum: f64) -> Result<(), String> {
@@ -326,8 +897,22 @@ fn validate_velocity(value: f64, maximum: f64) -> Result<(), String> {
     Ok(())
 }
 
-#[tauri::command]
-async fn publish_twist(
+fn validate_velocity_vector(values: [f64; 3], maximum: f64, axis: &str) -> Result<(), String> {
+    for value in values {
+        validate_velocity(value, maximum)?;
+    }
+    let magnitude = values[0].hypot(values[1]).hypot(values[2]);
+    if magnitude > maximum {
+        return Err(format!(
+            "{axis} velocity magnitude must not exceed {maximum}"
+        ));
+    }
+    Ok(())
+}
+
+#[derive(Deserialize)]
+#[serde(rename_all = "camelCase")]
+struct TwistInput {
     topic_name: String,
     linear_x: Option<f64>,
     linear_y: Option<f64>,
@@ -335,35 +920,66 @@ async fn publish_twist(
     angular_x: Option<f64>,
     angular_y: Option<f64>,
     angular_z: Option<f64>,
-) -> Result<(), String> {
+}
+
+#[tauri::command]
+async fn publish_twist(app: AppHandle, command: TwistInput) -> Result<(), String> {
+    let TwistInput {
+        topic_name,
+        linear_x,
+        linear_y,
+        linear_z,
+        angular_x,
+        angular_y,
+        angular_z,
+    } = command;
     if !is_valid_ros_topic(&topic_name) {
         return Err("Invalid ROS topic name".to_string());
     }
-    for (value, limit) in [
-        (linear_x.unwrap_or(0.0), MAX_LINEAR_VELOCITY),
-        (linear_y.unwrap_or(0.0), MAX_LINEAR_VELOCITY),
-        (linear_z.unwrap_or(0.0), MAX_LINEAR_VELOCITY),
-        (angular_x.unwrap_or(0.0), MAX_ANGULAR_VELOCITY),
-        (angular_y.unwrap_or(0.0), MAX_ANGULAR_VELOCITY),
-        (angular_z.unwrap_or(0.0), MAX_ANGULAR_VELOCITY),
-    ] {
-        validate_velocity(value, limit)?;
-    }
+    validate_velocity_vector(
+        [
+            linear_x.unwrap_or(0.0),
+            linear_y.unwrap_or(0.0),
+            linear_z.unwrap_or(0.0),
+        ],
+        MAX_LINEAR_VELOCITY,
+        "Linear",
+    )?;
+    validate_velocity_vector(
+        [
+            angular_x.unwrap_or(0.0),
+            angular_y.unwrap_or(0.0),
+            angular_z.unwrap_or(0.0),
+        ],
+        MAX_ANGULAR_VELOCITY,
+        "Angular",
+    )?;
 
     let mut client = RosEngineClient::connect("http://localhost:50051")
         .await
         .map_err(|_| docker_lost_connection_message())?;
 
+    let token = read_grpc_auth_token(&app)?;
+    let timestamp_ns = std::time::SystemTime::now()
+        .duration_since(std::time::UNIX_EPOCH)
+        .map_err(|error| error.to_string())?
+        .as_nanos()
+        .try_into()
+        .map_err(|_| "Command timestamp is out of range".to_string())?;
     client
-        .publish_twist(TwistCommand {
-            topic_name,
-            linear_x: linear_x.unwrap_or(0.0),
-            linear_y: linear_y.unwrap_or(0.0),
-            linear_z: linear_z.unwrap_or(0.0),
-            angular_x: angular_x.unwrap_or(0.0),
-            angular_y: angular_y.unwrap_or(0.0),
-            angular_z: angular_z.unwrap_or(0.0),
-        })
+        .publish_twist(authenticated_request(
+            TwistCommand {
+                topic_name,
+                linear_x: linear_x.unwrap_or(0.0),
+                linear_y: linear_y.unwrap_or(0.0),
+                linear_z: linear_z.unwrap_or(0.0),
+                angular_x: angular_x.unwrap_or(0.0),
+                angular_y: angular_y.unwrap_or(0.0),
+                angular_z: angular_z.unwrap_or(0.0),
+                timestamp_ns,
+            },
+            &token,
+        )?)
         .await
         .map_err(|_| docker_lost_connection_message())?;
 
@@ -415,6 +1031,52 @@ fn ensure_docker_available() -> Result<(), String> {
     }
 }
 
+fn ensure_loopback_grpc_binding(container_id: &str) -> Result<(), String> {
+    let output = docker_command()
+        .args([
+            "inspect",
+            "--format",
+            "{{json .HostConfig.PortBindings}}",
+            container_id,
+        ])
+        .output()
+        .map_err(|error| format!("Could not verify ROS container port binding: {error}"))?;
+    if !output.status.success() {
+        return Err(format!(
+            "Could not verify ROS container port binding: {}",
+            String::from_utf8_lossy(&output.stderr)
+        ));
+    }
+
+    let bindings: serde_json::Value = serde_json::from_slice(&output.stdout)
+        .map_err(|error| format!("Could not parse ROS container port binding: {error}"))?;
+    let is_loopback_only = bindings
+        .get("50051/tcp")
+        .and_then(serde_json::Value::as_array)
+        .is_some_and(|entries| {
+            !entries.is_empty()
+                && entries.iter().all(|entry| {
+                    entry.get("HostIp").and_then(serde_json::Value::as_str) == Some("127.0.0.1")
+                })
+        });
+    if !is_loopback_only {
+        let _ = docker_command()
+            .args([
+                "exec",
+                container_id,
+                "bash",
+                "-c",
+                "pkill -f '[p]ython3 server.py'",
+            ])
+            .output();
+        return Err(
+            "This ROS container publishes gRPC beyond loopback. Its port binding cannot be changed in place; stop it and recreate the container with its existing workspace bind-mounted to preserve files."
+                .to_string(),
+        );
+    }
+    Ok(())
+}
+
 fn find_dev_container() -> Result<String, String> {
     if ensure_docker_available().is_err() {
         return Err(docker_lost_connection_message());
@@ -423,13 +1085,7 @@ fn find_dev_container() -> Result<String, String> {
     let workspace_path = get_default_workspace_dir()?;
     let filter = format!("label=devcontainer.local_folder={}", workspace_path);
     let output = docker_command()
-        .args([
-            "ps",
-            "--filter",
-            &filter,
-            "--format",
-            "{{.Names}}",
-        ])
+        .args(["ps", "--filter", &filter, "--format", "{{.Names}}"])
         .output()
         .map_err(|_| docker_lost_connection_message())?;
 
@@ -441,6 +1097,7 @@ fn find_dev_container() -> Result<String, String> {
     if name.is_empty() {
         Err("No running dev container found for this workspace".to_string())
     } else {
+        ensure_loopback_grpc_binding(&name)?;
         Ok(name)
     }
 }
@@ -474,7 +1131,7 @@ use std::io::{BufReader, Read};
 use std::process::Stdio;
 
 /// Event payload for streaming log output from Docker/build processes.
-/// Distinguishes between "append" (finalized line with \n) and "replace" 
+/// Distinguishes between "append" (finalized line with \n) and "replace"
 /// (in-progress update with \r) to properly render Docker BuildKit progress.
 #[derive(serde::Serialize, Clone)]
 struct LogEvent {
@@ -487,21 +1144,20 @@ struct LogEvent {
 /// before displaying in plain text UI.
 fn strip_ansi_codes(input: &str) -> String {
     let mut result = String::new();
-    let mut chars = input.chars().peekable();
-    
-    while let Some(ch) = chars.next() {
-        if ch == '\x1b' {
-            // Start of an ANSI escape sequence; consume until 'm' or other terminator
-            while let Some(c) = chars.next() {
-                if c.is_alphabetic() {
-                    break; // End of escape sequence
-                }
+    let mut skipping_escape = false;
+
+    for ch in input.chars() {
+        if skipping_escape {
+            if ch.is_alphabetic() {
+                skipping_escape = false;
             }
+        } else if ch == '\x1b' {
+            skipping_escape = true;
         } else {
             result.push(ch);
         }
     }
-    
+
     result
 }
 
@@ -513,12 +1169,14 @@ fn strip_ansi_codes(input: &str) -> String {
 struct StreamLineBuffer {
     /// Accumulated bytes not yet terminated by \n or \r.
     pending: Vec<u8>,
+    discarding_long_line: bool,
 }
 
 impl StreamLineBuffer {
     fn new() -> Self {
         StreamLineBuffer {
             pending: Vec::new(),
+            discarding_long_line: false,
         }
     }
 
@@ -526,6 +1184,15 @@ impl StreamLineBuffer {
     /// Any bytes that don't end with a delimiter are kept in the pending buffer
     /// for the next call, ensuring lines split across chunks are assembled correctly.
     fn process_chunk(&mut self, chunk: &[u8]) -> Vec<(String, String)> {
+        if self.discarding_long_line {
+            let Some(delimiter) = chunk.iter().position(|byte| matches!(byte, b'\n' | b'\r'))
+            else {
+                return Vec::new();
+            };
+            self.discarding_long_line = false;
+            return self.process_chunk(&chunk[delimiter + 1..]);
+        }
+
         let mut result = Vec::new();
 
         // Add chunk to pending buffer
@@ -559,12 +1226,26 @@ impl StreamLineBuffer {
             }
         }
 
+        if self.pending.len() > MAX_PENDING_LINE_BYTES {
+            let text = String::from_utf8_lossy(&self.pending[..MAX_PENDING_LINE_BYTES]);
+            result.push((
+                "append".to_string(),
+                format!("{} [line truncated]", strip_ansi_codes(&text)),
+            ));
+            self.pending.clear();
+            self.discarding_long_line = true;
+        }
+
         result
     }
 
     /// Flush any remaining bytes in the buffer as a final "append" event.
     /// Call this when EOF is reached to ensure no data is lost.
     fn flush(&mut self) -> Option<(String, String)> {
+        if self.discarding_long_line {
+            self.discarding_long_line = false;
+            return None;
+        }
         if !self.pending.is_empty() {
             let text = String::from_utf8_lossy(&self.pending).to_string();
             let text = strip_ansi_codes(&text);
@@ -577,6 +1258,8 @@ impl StreamLineBuffer {
     }
 }
 
+const MAX_PENDING_LINE_BYTES: usize = 64 * 1024;
+
 fn stream_process_output<R: Read + Send + 'static>(
     reader: R,
     app: AppHandle,
@@ -708,7 +1391,10 @@ fn detect_models_in_sdf(sdf_content: &str) -> Vec<String> {
 }
 
 #[tauri::command]
-fn start_gazebo_sim(world_path: Option<String>) -> Result<String, String> {
+fn start_gazebo_sim(
+    world_path: Option<String>,
+    access: tauri::State<'_, FsAccess>,
+) -> Result<String, String> {
     let container_name = find_dev_container()?;
 
     // Defensively kill any existing instances, and actively verify
@@ -717,13 +1403,15 @@ fn start_gazebo_sim(world_path: Option<String>) -> Result<String, String> {
     for attempt in 0..10 {
         let kill_result = docker_command()
             .args([
-                "exec", &container_name,
-                "bash", "-c",
+                "exec",
+                &container_name,
+                "bash",
+                "-c",
                 "pkill -9 -f '[g]z sim -s' ; pkill -9 -f '[/]parameter_bridge '",
             ])
             .output();
 
-        if let Err(_) = &kill_result {
+        if kill_result.is_err() {
             return Err(docker_lost_connection_message());
         }
 
@@ -731,14 +1419,20 @@ fn start_gazebo_sim(world_path: Option<String>) -> Result<String, String> {
 
         let check = docker_command()
             .args([
-                "exec", &container_name,
-                "bash", "-c",
+                "exec",
+                &container_name,
+                "bash",
+                "-c",
                 "pgrep -f '[g]z sim -s' ; pgrep -f '[/]parameter_bridge '",
             ])
             .output()
             .map_err(|_| docker_lost_connection_message())?;
 
-        eprintln!("DEBUG attempt {}: check.stdout = {:?}", attempt, String::from_utf8_lossy(&check.stdout));
+        eprintln!(
+            "DEBUG attempt {}: check.stdout = {:?}",
+            attempt,
+            String::from_utf8_lossy(&check.stdout)
+        );
 
         if check.stdout.is_empty() {
             break;
@@ -753,29 +1447,41 @@ fn start_gazebo_sim(world_path: Option<String>) -> Result<String, String> {
 
     // Determine which world file to use, and read its content (for model
     // auto-detection) either from our bundled copy or the user-selected one.
+    let workspace = PathBuf::from(get_default_workspace_dir()?)
+        .canonicalize()
+        .map_err(|error| format!("ROS workspace is unavailable: {error}"))?;
     let (container_world_path, sdf_content) = match &world_path {
         Some(host_path) => {
-            // Copy the custom world into the container's /workspace so
-            // Gazebo (running inside the container) can actually reach it.
-            let file_name = std::path::Path::new(host_path)
-                .file_name()
-                .ok_or("Invalid world file path")?
-                .to_string_lossy()
-                .to_string();
-            let content = std::fs::read_to_string(host_path).map_err(|e| e.to_string())?;
-            let dest = format!("{}/{}", get_default_workspace_dir()?, file_name);
-            std::fs::write(&dest, &content).map_err(|e| e.to_string())?;
-            (format!("/workspace/{}", file_name), content)
+            let host_path = access.resolve_existing(Path::new(host_path))?;
+            let worlds_dir = workspace
+                .join("worlds")
+                .canonicalize()
+                .map_err(|error| format!("Imported worlds are unavailable: {error}"))?;
+            if !host_path.starts_with(&worlds_dir) {
+                return Err(
+                    "Custom worlds must be imported into the managed worlds directory".to_string(),
+                );
+            }
+            let content = read_approved_file(&access, &host_path)?;
+            validate_sdf_file(&host_path, &content)?;
+            let relative = host_path
+                .strip_prefix(&workspace)
+                .map_err(|_| "World file is outside the ROS workspace".to_string())?;
+            let relative = relative.to_string_lossy().replace('\\', "/");
+            (format!("/workspace/{relative}"), content)
         }
         None => {
-            let dest = format!("{}/diff_drive.sdf", get_default_workspace_dir()?);
-            let content = std::fs::read_to_string(&dest).map_err(|e| e.to_string())?;
+            let bundled_path = workspace.join("diff_drive.sdf");
+            let content = read_approved_file(&access, &bundled_path)?;
             ("/workspace/diff_drive.sdf".to_string(), content)
         }
     };
 
     let models = detect_models_in_sdf(&sdf_content);
-    let robot_models: Vec<&String> = models.iter().filter(|m| m.as_str() != "ground_plane").collect();
+    let robot_models: Vec<&String> = models
+        .iter()
+        .filter(|m| m.as_str() != "ground_plane")
+        .collect();
 
     let gz_result = docker_command()
         .args([
@@ -806,13 +1512,16 @@ fn start_gazebo_sim(world_path: Option<String>) -> Result<String, String> {
     let mut bridge_topics = Vec::new();
     for model in &robot_models {
         bridge_topics.push(format!(
-            "/model/{}/cmd_vel@geometry_msgs/msg/Twist@gz.msgs.Twist", model
+            "/model/{}/cmd_vel@geometry_msgs/msg/Twist@gz.msgs.Twist",
+            model
         ));
         bridge_topics.push(format!(
-            "/model/{}/odometry@nav_msgs/msg/Odometry@gz.msgs.Odometry", model
+            "/model/{}/odometry@nav_msgs/msg/Odometry@gz.msgs.Odometry",
+            model
         ));
         bridge_topics.push(format!(
-            "/model/{}/tf@tf2_msgs/msg/TFMessage@gz.msgs.Pose_V", model
+            "/model/{}/tf@tf2_msgs/msg/TFMessage@gz.msgs.Pose_V",
+            model
         ));
     }
 
@@ -847,8 +1556,10 @@ fn stop_gazebo_sim() -> Result<String, String> {
 
     let result = docker_command()
         .args([
-            "exec", &container_name,
-            "bash", "-c",
+            "exec",
+            &container_name,
+            "bash",
+            "-c",
             "pkill -f 'gz sim' ; pkill -f parameter_bridge",
         ])
         .output()
@@ -863,8 +1574,10 @@ fn reset_gazebo_sim() -> Result<String, String> {
 
     let result = docker_command()
         .args([
-            "exec", &container_name,
-            "bash", "-c",
+            "exec",
+            &container_name,
+            "bash",
+            "-c",
             "gz service -s /world/default/control --reqtype gz.msgs.WorldControl \
              --reptype gz.msgs.Boolean --timeout 2000 --req 'reset: {all: true}'",
         ])
@@ -880,9 +1593,16 @@ fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<()
     copy_dir_recursive_impl(src, dst, 0)
 }
 
-fn copy_dir_recursive_impl(src: &std::path::Path, dst: &std::path::Path, depth: u32) -> Result<(), String> {
+fn copy_dir_recursive_impl(
+    src: &std::path::Path,
+    dst: &std::path::Path,
+    depth: u32,
+) -> Result<(), String> {
     if depth > 50 {
-        return Err(format!("copy_dir_recursive exceeded max depth (50) — possible cyclic path: {:?} -> {:?}", src, dst));
+        return Err(format!(
+            "copy_dir_recursive exceeded max depth (50) — possible cyclic path: {:?} -> {:?}",
+            src, dst
+        ));
     }
     std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
     for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
@@ -905,7 +1625,221 @@ fn copy_dir_recursive_impl(src: &std::path::Path, dst: &std::path::Path, depth:
     Ok(())
 }
 
-fn start_ros_engine_server(container_id: &str) -> Result<(), String> {
+const GRPC_AUTH_CONTAINER_PATH: &str = "/run/robotics-studio/grpc-auth.json";
+const GRPC_AUTH_LIFETIME_SECONDS: u64 = 24 * 60 * 60;
+const AUTH_PROTOCOL_MARKER: &str = "AUTH_PROTOCOL_VERSION = 1";
+
+#[derive(Serialize, Deserialize)]
+struct GrpcAuthCredentials {
+    token: String,
+    expires_at: u64,
+}
+
+fn create_grpc_auth_file(app: &AppHandle) -> Result<(PathBuf, String), String> {
+    let mut random = [0u8; 32];
+    getrandom::fill(&mut random)
+        .map_err(|error| format!("Could not generate gRPC credentials: {error}"))?;
+    let token = random
+        .iter()
+        .map(|byte| format!("{byte:02x}"))
+        .collect::<String>();
+    let expires_at = std::time::SystemTime::now()
+        .duration_since(std::time::UNIX_EPOCH)
+        .map_err(|error| error.to_string())?
+        .as_secs()
+        .saturating_add(GRPC_AUTH_LIFETIME_SECONDS);
+    let credentials = GrpcAuthCredentials {
+        token: token.clone(),
+        expires_at,
+    };
+
+    let directory = app
+        .path()
+        .app_config_dir()
+        .map_err(|error| error.to_string())?;
+    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
+    let path = directory.join("grpc-auth.json");
+    let timestamp = std::time::SystemTime::now()
+        .duration_since(std::time::UNIX_EPOCH)
+        .map_err(|error| error.to_string())?
+        .as_nanos();
+    let temporary = directory.join(format!(".grpc-auth-{}-{timestamp}.tmp", std::process::id()));
+    let serialized = serde_json::to_vec(&credentials).map_err(|error| error.to_string())?;
+    let mut options = fs::OpenOptions::new();
+    options.write(true).create_new(true);
+    #[cfg(unix)]
+    {
+        use std::os::unix::fs::OpenOptionsExt;
+        options.mode(0o600);
+    }
+    let mut file = options
+        .open(&temporary)
+        .map_err(|error| format!("Could not create gRPC credential file: {error}"))?;
+    if let Err(error) =
+        std::io::Write::write_all(&mut file, &serialized).and_then(|_| file.sync_all())
+    {
+        let _ = fs::remove_file(&temporary);
+        return Err(format!("Could not write gRPC credential file: {error}"));
+    }
+    #[cfg(windows)]
+    if path.exists() {
+        fs::remove_file(&path)
+            .map_err(|error| format!("Could not rotate gRPC credentials: {error}"))?;
+    }
+    fs::rename(&temporary, &path)
+        .map_err(|error| format!("Could not rotate gRPC credentials: {error}"))?;
+    #[cfg(unix)]
+    {
+        use std::os::unix::fs::PermissionsExt;
+        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
+            .map_err(|error| format!("Could not protect gRPC credentials: {error}"))?;
+    }
+    Ok((path, token))
+}
+
+fn read_grpc_auth_token(app: &AppHandle) -> Result<String, String> {
+    let path = app
+        .path()
+        .app_config_dir()
+        .map_err(|error| error.to_string())?
+        .join("grpc-auth.json");
+    let credentials: GrpcAuthCredentials = serde_json::from_slice(
+        &fs::read(path).map_err(|_| "ROS authentication is not initialized".to_string())?,
+    )
+    .map_err(|_| "ROS authentication credentials are invalid".to_string())?;
+    let now = std::time::SystemTime::now()
+        .duration_since(std::time::UNIX_EPOCH)
+        .map_err(|error| error.to_string())?
+        .as_secs();
+    if credentials.token.len() != 64 || now >= credentials.expires_at {
+        return Err(
+            "ROS authentication credentials have expired; restart the ROS environment".to_string(),
+        );
+    }
+    Ok(credentials.token)
+}
+
+fn install_grpc_auth_file(container_id: &str, host_path: &Path) -> Result<(), String> {
+    let directory = docker_command()
+        .args([
+            "exec",
+            container_id,
+            "bash",
+            "-c",
+            "mkdir -p /run/robotics-studio && chmod 700 /run/robotics-studio",
+        ])
+        .output()
+        .map_err(|error| format!("Could not prepare ROS authentication: {error}"))?;
+    if !directory.status.success() {
+        return Err(format!(
+            "Could not prepare ROS authentication: {}",
+            String::from_utf8_lossy(&directory.stderr)
+        ));
+    }
+    let destination = format!("{container_id}:{GRPC_AUTH_CONTAINER_PATH}");
+    let copied = docker_command()
+        .args(["cp", &host_path.to_string_lossy(), &destination])
+        .output()
+        .map_err(|error| format!("Could not transfer ROS authentication: {error}"))?;
+    if !copied.status.success() {
+        return Err(format!(
+            "Could not transfer ROS authentication: {}",
+            String::from_utf8_lossy(&copied.stderr)
+        ));
+    }
+    let protected = docker_command()
+        .args([
+            "exec",
+            container_id,
+            "chmod",
+            "600",
+            GRPC_AUTH_CONTAINER_PATH,
+        ])
+        .output()
+        .map_err(|error| format!("Could not protect ROS authentication: {error}"))?;
+    if !protected.status.success() {
+        return Err(format!(
+            "Could not protect ROS authentication: {}",
+            String::from_utf8_lossy(&protected.stderr)
+        ));
+    }
+    Ok(())
+}
+
+fn install_authenticated_server_files(
+    resource_dir: &Path,
+    workspace_dir: &Path,
+    backup_dir: &Path,
+) -> Result<(), String> {
+    fs::create_dir_all(backup_dir)
+        .map_err(|error| format!("Could not prepare server migration backup: {error}"))?;
+    for file_name in ["server.py", "server_core.py", "proto/ros_engine.proto"] {
+        let bundled_path = resource_dir.join(file_name);
+        let target_path = workspace_dir.join(file_name);
+        let bundled = fs::read(&bundled_path)
+            .map_err(|error| format!("Could not read bundled {file_name}: {error}"))?;
+        let installed = match fs::symlink_metadata(&target_path) {
+            Ok(metadata) if metadata.file_type().is_symlink() => {
+                return Err(format!("Workspace {file_name} must not be a symbolic link"));
+            }
+            Ok(_) => Some(
+                fs::read(&target_path)
+                    .map_err(|error| format!("Could not inspect workspace {file_name}: {error}"))?,
+            ),
+            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
+            Err(error) => return Err(format!("Could not inspect workspace {file_name}: {error}")),
+        };
+        if installed.as_ref().is_some_and(|contents| {
+            String::from_utf8_lossy(contents).contains(AUTH_PROTOCOL_MARKER)
+        }) {
+            continue;
+        }
+
+        if installed.is_some() {
+            let timestamp = std::time::SystemTime::now()
+                .duration_since(std::time::UNIX_EPOCH)
+                .map_err(|error| error.to_string())?
+                .as_nanos();
+            let backup_path = backup_dir.join(format!(
+                "{}.pre-auth-{}-{timestamp}",
+                file_name.replace('/', "_"),
+                std::process::id()
+            ));
+            fs::copy(&target_path, &backup_path)
+                .map_err(|error| format!("Could not preserve existing {file_name}: {error}"))?;
+        }
+
+        let timestamp = std::time::SystemTime::now()
+            .duration_since(std::time::UNIX_EPOCH)
+            .map_err(|error| error.to_string())?
+            .as_nanos();
+        let target_parent = target_path
+            .parent()
+            .ok_or_else(|| format!("Invalid workspace path for {file_name}"))?;
+        let target_name = target_path
+            .file_name()
+            .ok_or_else(|| format!("Invalid workspace filename for {file_name}"))?
+            .to_string_lossy();
+        let temporary = target_parent.join(format!(
+            ".{target_name}.auth-{}-{timestamp}.tmp",
+            std::process::id()
+        ));
+        fs::write(&temporary, bundled)
+            .map_err(|error| format!("Could not stage secure {file_name}: {error}"))?;
+        #[cfg(windows)]
+        if target_path.exists() {
+            fs::remove_file(&target_path)
+                .map_err(|error| format!("Could not update secure {file_name}: {error}"))?;
+        }
+        if let Err(error) = fs::rename(&temporary, &target_path) {
+            let _ = fs::remove_file(&temporary);
+            return Err(format!("Could not install secure {file_name}: {error}"));
+        }
+    }
+    Ok(())
+}
+
+fn start_ros_engine_server(container_id: &str, auth_file: &Path) -> Result<(), String> {
     let generate = docker_command()
         .args([
             "exec",
@@ -924,20 +1858,18 @@ fn start_ros_engine_server(container_id: &str) -> Result<(), String> {
         ));
     }
 
-    let running = docker_command()
+    install_grpc_auth_file(container_id, auth_file)?;
+
+    let _ = docker_command()
         .args([
             "exec",
             container_id,
             "bash",
             "-c",
-            "pgrep -f '[p]ython3 server.py'",
+            "pkill -f '[p]ython3 server.py'",
         ])
         .output()
-        .map_err(|error| format!("Failed to inspect ROS engine process: {error}"))?;
-
-    if running.status.success() && !running.stdout.is_empty() {
-        return Ok(());
-    }
+        .map_err(|error| format!("Could not rotate the ROS engine session: {error}"))?;
 
     let started = docker_command()
         .args([
@@ -946,7 +1878,7 @@ fn start_ros_engine_server(container_id: &str) -> Result<(), String> {
             container_id,
             "bash",
             "-c",
-            "source /opt/ros/jazzy/setup.bash && cd /workspace && exec python3 server.py > /tmp/server.log 2>&1",
+            "source /opt/ros/jazzy/setup.bash && cd /workspace && ROBOTICS_STUDIO_AUTH_FILE=/run/robotics-studio/grpc-auth.json exec python3 server.py > /tmp/server.log 2>&1",
         ])
         .output()
         .map_err(|error| format!("Failed to start ROS engine server: {error}"))?;
@@ -960,7 +1892,17 @@ fn start_ros_engine_server(container_id: &str) -> Result<(), String> {
     Ok(())
 }
 
-async fn wait_for_ros_engine(container_id: &str) -> Result<(), String> {
+fn authenticated_request<T>(message: T, token: &str) -> Result<tonic::Request<T>, String> {
+    let mut request = tonic::Request::new(message);
+    let token = tonic::metadata::MetadataValue::try_from(token)
+        .map_err(|error| format!("Could not encode ROS authentication metadata: {error}"))?;
+    request
+        .metadata_mut()
+        .insert("x-robotics-studio-token", token);
+    Ok(request)
+}
+
+async fn wait_for_ros_engine(container_id: &str, token: &str) -> Result<(), String> {
     for _ in 0..30 {
         if let Ok(Ok(mut client)) = tokio::time::timeout(
             std::time::Duration::from_secs(2),
@@ -970,7 +1912,7 @@ async fn wait_for_ros_engine(container_id: &str) -> Result<(), String> {
         {
             if tokio::time::timeout(
                 std::time::Duration::from_secs(2),
-                client.stream_events(Empty {}),
+                client.stream_events(authenticated_request(Empty {}, token)?),
             )
             .await
             .is_ok_and(|result| result.is_ok())
@@ -984,7 +1926,9 @@ async fn wait_for_ros_engine(container_id: &str) -> Result<(), String> {
     let logs = docker_command()
         .args(["exec", container_id, "cat", "/tmp/server.log"])
         .output()
-        .map_err(|error| format!("gRPC service did not become ready; could not read logs: {error}"))?;
+        .map_err(|error| {
+            format!("gRPC service did not become ready; could not read logs: {error}")
+        })?;
     Err(format!(
         "gRPC service did not become ready: {}",
         String::from_utf8_lossy(&logs.stdout)
@@ -992,7 +1936,10 @@ async fn wait_for_ros_engine(container_id: &str) -> Result<(), String> {
 }
 
 #[tauri::command]
-async fn initialize_ros_environment(app: AppHandle) -> Result<(), String> {
+async fn initialize_ros_environment(
+    app: AppHandle,
+    access: tauri::State<'_, FsAccess>,
+) -> Result<(), String> {
     if INITIALIZATION_RUNNING.swap(true, Ordering::SeqCst) {
         return Err("ROS environment initialization is already in progress".to_string());
     }
@@ -1002,18 +1949,34 @@ async fn initialize_ros_environment(app: AppHandle) -> Result<(), String> {
         let _ = app.emit("init-error", err.clone());
         return Err(err);
     }
+    let (auth_file, auth_token) = create_grpc_auth_file(&app)?;
 
     let resource_path = app
         .path()
-        .resolve("resources/devcontainer", tauri::path::BaseDirectory::Resource)
+        .resolve(
+            "resources/devcontainer",
+            tauri::path::BaseDirectory::Resource,
+        )
         .map_err(|e| e.to_string())?;
 
     let workspace_dir = get_default_workspace_dir()?;
-    let workspace_path = std::path::Path::new(&workspace_dir);
+    let workspace_path = Path::new(&workspace_dir);
+    fs::create_dir_all(workspace_path).map_err(|error| error.to_string())?;
+    let workspace_dir = access
+        .approve_directory(workspace_path)?
+        .to_string_lossy()
+        .into_owned();
+    let workspace_path = Path::new(&workspace_dir);
 
-    // Install missing bundled files without overwriting resources a user may
-    // have modified in the workspace.
+    // Install missing resources and preserve legacy server files before updating
+    // the authentication-critical ROS service scripts.
     copy_dir_recursive(&resource_path, workspace_path)?;
+    let backup_dir = app
+        .path()
+        .app_config_dir()
+        .map_err(|error| error.to_string())?
+        .join("server-migration-backups");
+    install_authenticated_server_files(&resource_path, workspace_path, &backup_dir)?;
 
     let filter = format!("label=devcontainer.local_folder={}", workspace_dir);
 
@@ -1054,6 +2017,7 @@ async fn initialize_ros_environment(app: AppHandle) -> Result<(), String> {
                 String::from_utf8_lossy(&started.stderr)
             ));
         }
+        ensure_loopback_grpc_binding(&container_name)?;
 
         let app = app.clone();
         let workspace_dir = workspace_dir.clone();
@@ -1066,8 +2030,8 @@ async fn initialize_ros_environment(app: AppHandle) -> Result<(), String> {
                     text: "Existing environment found. Checking ROS engine...".to_string(),
                 },
             );
-            let result = match start_ros_engine_server(&container_name) {
-                Ok(()) => wait_for_ros_engine(&container_name).await,
+            let result = match start_ros_engine_server(&container_name, &auth_file) {
+                Ok(()) => wait_for_ros_engine(&container_name, &auth_token).await,
                 Err(error) => Err(error),
             };
             match result {
@@ -1086,7 +2050,8 @@ async fn initialize_ros_environment(app: AppHandle) -> Result<(), String> {
         let _initialization_guard = initialization_guard;
         let event = LogEvent {
             kind: "append".to_string(),
-            text: "Building Docker image (this may take several minutes on first run)...".to_string(),
+            text: "Building Docker image (this may take several minutes on first run)..."
+                .to_string(),
         };
         let _ = app.emit("init-progress", event);
 
@@ -1128,7 +2093,10 @@ async fn initialize_ros_environment(app: AppHandle) -> Result<(), String> {
         };
 
         if !build_status.success() {
-            let _ = app.emit("init-error", "Docker build failed — see progress log above".to_string());
+            let _ = app.emit(
+                "init-error",
+                "Docker build failed — see progress log above".to_string(),
+            );
             return;
         }
 
@@ -1140,12 +2108,18 @@ async fn initialize_ros_environment(app: AppHandle) -> Result<(), String> {
 
         let run_output = docker_command()
             .args([
-                "run", "-d",
-                "--label", &format!("devcontainer.local_folder={}", workspace_dir),
-                "-p", "127.0.0.1:50051:50051",
-                "-v", &format!("{}:/workspace", workspace_dir),
+                "run",
+                "-d",
+                "--label",
+                &format!("devcontainer.local_folder={}", workspace_dir),
+                "-p",
+                "127.0.0.1:50051:50051",
+                "-v",
+                &format!("{}:/workspace", workspace_dir),
                 "robotics-studio-ros2",
-                "tail", "-f", "/dev/null",
+                "tail",
+                "-f",
+                "/dev/null",
             ])
             .output();
 
@@ -1158,11 +2132,20 @@ async fn initialize_ros_environment(app: AppHandle) -> Result<(), String> {
         };
 
         if !run_output.status.success() {
-            let _ = app.emit("init-error", String::from_utf8_lossy(&run_output.stderr).to_string());
+            let _ = app.emit(
+                "init-error",
+                String::from_utf8_lossy(&run_output.stderr).to_string(),
+            );
             return;
         }
 
-        let container_id = String::from_utf8_lossy(&run_output.stdout).trim().to_string();
+        let container_id = String::from_utf8_lossy(&run_output.stdout)
+            .trim()
+            .to_string();
+        if let Err(error) = ensure_loopback_grpc_binding(&container_id) {
+            let _ = app.emit("init-error", error.clone());
+            return;
+        }
 
         let event = LogEvent {
             kind: "append".to_string(),
@@ -1170,8 +2153,8 @@ async fn initialize_ros_environment(app: AppHandle) -> Result<(), String> {
         };
         let _ = app.emit("init-progress", event);
 
-        match start_ros_engine_server(&container_id) {
-            Ok(()) => match wait_for_ros_engine(&container_id).await {
+        match start_ros_engine_server(&container_id, &auth_file) {
+            Ok(()) => match wait_for_ros_engine(&container_id, &auth_token).await {
                 Ok(()) => {
                     let _ = app.emit("init-finished", workspace_dir.clone());
                 }
@@ -1191,33 +2174,158 @@ async fn initialize_ros_environment(app: AppHandle) -> Result<(), String> {
 #[derive(Serialize, Deserialize, Clone)]
 struct AIProviderSettings {
     provider: String, // "groq" | "anthropic" | "openai"
+    model: String,
+}
+
+#[derive(Deserialize)]
+struct SaveAIProviderSettings {
+    provider: String,
     api_key: String,
     model: String,
 }
 
+#[derive(Serialize)]
+struct PublicAIProviderSettings {
+    provider: String,
+    model: String,
+    has_api_key: bool,
+}
+
+trait CredentialProvider {
+    fn get(&self, provider: &str) -> Result<Option<String>, String>;
+    fn set(&self, provider: &str, secret: &str) -> Result<(), String>;
+    fn delete(&self, provider: &str) -> Result<(), String>;
+}
+
+struct SystemCredentialProvider;
+
+impl CredentialProvider for SystemCredentialProvider {
+    fn get(&self, provider: &str) -> Result<Option<String>, String> {
+        let entry = credential_entry(provider)?;
+        match entry.get_password() {
+            Ok(secret) => Ok(Some(secret)),
+            Err(keyring::Error::NoEntry) => Ok(None),
+            Err(error) => Err(format!("Secure credential storage is unavailable: {error}")),
+        }
+    }
+
+    fn set(&self, provider: &str, secret: &str) -> Result<(), String> {
+        credential_entry(provider)?
+            .set_password(secret)
+            .map_err(|error| format!("Could not store the AI credential securely: {error}"))
+    }
+
+    fn delete(&self, provider: &str) -> Result<(), String> {
+        match credential_entry(provider)?.delete_credential() {
+            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
+            Err(error) => Err(format!("Could not delete the AI credential: {error}")),
+        }
+    }
+}
+
+fn validate_ai_provider(provider: &str) -> Result<(), String> {
+    if matches!(provider, "groq" | "anthropic" | "openai") {
+        Ok(())
+    } else {
+        Err("Unsupported AI provider".to_string())
+    }
+}
+
+fn credential_entry(provider: &str) -> Result<keyring::Entry, String> {
+    validate_ai_provider(provider)?;
+    keyring::Entry::new("com.kartik.robotics-studio.ai", provider)
+        .map_err(|error| format!("Could not access secure credential storage: {error}"))
+}
+
 fn settings_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
     let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
     std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
     Ok(dir.join("ai_settings.json"))
 }
 
+fn write_ai_settings_file(path: &Path, settings: &AIProviderSettings) -> Result<(), String> {
+    let json = serde_json::to_vec_pretty(settings).map_err(|error| error.to_string())?;
+    let temporary = path.with_extension(format!(
+        "json.{}.{}.tmp",
+        std::process::id(),
+        std::time::SystemTime::now()
+            .duration_since(std::time::UNIX_EPOCH)
+            .map_err(|error| error.to_string())?
+            .as_nanos()
+    ));
+    fs::write(&temporary, json).map_err(|error| error.to_string())?;
+    #[cfg(windows)]
+    if path.exists() {
+        fs::remove_file(path).map_err(|error| error.to_string())?;
+    }
+    if let Err(error) = fs::rename(&temporary, path) {
+        let _ = fs::remove_file(&temporary);
+        return Err(error.to_string());
+    }
+    Ok(())
+}
+
 #[tauri::command]
-fn save_ai_settings(app: AppHandle, settings: AIProviderSettings) -> Result<(), String> {
+fn save_ai_settings(app: AppHandle, settings: SaveAIProviderSettings) -> Result<(), String> {
+    validate_ai_provider(&settings.provider)?;
+    let credentials = SystemCredentialProvider;
+    if !settings.api_key.trim().is_empty() {
+        credentials.set(&settings.provider, settings.api_key.trim())?;
+    }
+    let stored = AIProviderSettings {
+        provider: settings.provider,
+        model: settings.model,
+    };
     let path = settings_path(&app)?;
-    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
-    std::fs::write(path, json).map_err(|e| e.to_string())?;
-    Ok(())
+    write_ai_settings_file(&path, &stored)
 }
 
 #[tauri::command]
-fn load_ai_settings(app: AppHandle) -> Result<Option<AIProviderSettings>, String> {
+fn load_ai_settings(app: AppHandle) -> Result<Option<PublicAIProviderSettings>, String> {
     let path = settings_path(&app)?;
     if !path.exists() {
         return Ok(None);
     }
-    let json = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
-    let settings: AIProviderSettings = serde_json::from_str(&json).map_err(|e| e.to_string())?;
-    Ok(Some(settings))
+    let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
+    let mut value: serde_json::Value =
+        serde_json::from_str(&json).map_err(|error| error.to_string())?;
+    if let Some(api_key) = value
+        .get("api_key")
+        .and_then(serde_json::Value::as_str)
+        .filter(|secret| !secret.trim().is_empty())
+    {
+        let provider = value
+            .get("provider")
+            .and_then(serde_json::Value::as_str)
+            .ok_or_else(|| "Legacy AI settings have no provider".to_string())?;
+        SystemCredentialProvider.set(provider, api_key.trim())?;
+        value
+            .as_object_mut()
+            .ok_or_else(|| "Invalid AI settings format".to_string())?
+            .remove("api_key");
+        let migrated: AIProviderSettings =
+            serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
+        write_ai_settings_file(&path, &migrated)?;
+    }
+    let settings: AIProviderSettings =
+        serde_json::from_value(value).map_err(|error| error.to_string())?;
+    validate_ai_provider(&settings.provider)?;
+    let has_api_key = SystemCredentialProvider.get(&settings.provider)?.is_some();
+    Ok(Some(PublicAIProviderSettings {
+        provider: settings.provider,
+        model: settings.model,
+        has_api_key,
+    }))
+}
+
+#[tauri::command]
+fn delete_ai_credential(provider: String) -> Result<(), String> {
+    SystemCredentialProvider.delete(&provider)
+}
+
+#[tauri::command]
+fn has_ai_credential(provider: String) -> Result<bool, String> {
+    Ok(SystemCredentialProvider.get(&provider)?.is_some())
 }
 
 #[derive(Serialize, Deserialize, Clone)]
@@ -1265,23 +2373,41 @@ fn recent_projects_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
 }
 
 #[tauri::command]
-fn save_recent_projects(app: AppHandle, projects: Vec<RecentProject>) -> Result<(), String> {
+fn save_recent_projects(
+    app: AppHandle,
+    access: tauri::State<'_, FsAccess>,
+    projects: Vec<RecentProject>,
+) -> Result<(), String> {
     let path = recent_projects_path(&app)?;
     let normalized = normalize_recent_projects(projects);
+    for project in &normalized {
+        access.resolve_existing(Path::new(&project.path))?;
+    }
     let json = serde_json::to_string_pretty(&normalized).map_err(|e| e.to_string())?;
     std::fs::write(path, json).map_err(|e| e.to_string())?;
     Ok(())
 }
 
 #[tauri::command]
-fn load_recent_projects(app: AppHandle) -> Result<Vec<RecentProject>, String> {
+fn load_recent_projects(
+    app: AppHandle,
+    access: tauri::State<'_, FsAccess>,
+) -> Result<Vec<RecentProject>, String> {
     let path = recent_projects_path(&app)?;
     if !path.exists() {
         return Ok(vec![]);
     }
     let json = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
-    let projects: Vec<RecentProject> = serde_json::from_str(&json).unwrap_or_default();
-    Ok(normalize_recent_projects(projects))
+    let projects: Vec<RecentProject> = serde_json::from_str(&json).map_err(|e| e.to_string())?;
+    let projects = normalize_recent_projects(projects);
+    for project in &projects {
+        if let Ok(path) = Path::new(&project.path).canonicalize() {
+            if path.is_dir() {
+                access.approve_directory(&path)?;
+            }
+        }
+    }
+    Ok(projects)
 }
 
 #[derive(Serialize)]
@@ -1312,11 +2438,13 @@ fn get_system_specs() -> Result<SystemSpecs, String> {
 #[cfg(target_os = "macos")]
 fn get_total_ram_gb() -> Result<u64, String> {
     let output = std::process::Command::new("sysctl")
-        .args(&["-n", "hw.memsize"])
+        .args(["-n", "hw.memsize"])
         .output()
         .map_err(|e| e.to_string())?;
     let bytes_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
-    let bytes: u64 = bytes_str.parse().map_err(|_| "Failed to parse RAM size".to_string())?;
+    let bytes: u64 = bytes_str
+        .parse()
+        .map_err(|_| "Failed to parse RAM size".to_string())?;
     Ok(bytes / (1024 * 1024 * 1024))
 }
 
@@ -1329,7 +2457,9 @@ fn get_total_ram_gb() -> Result<u64, String> {
     let output_str = String::from_utf8_lossy(&output.stdout);
     let parts: Vec<&str> = output_str.split_whitespace().collect();
     if parts.len() >= 2 {
-        let kb: u64 = parts[1].parse().map_err(|_| "Failed to parse RAM size".to_string())?;
+        let kb: u64 = parts[1]
+            .parse()
+            .map_err(|_| "Failed to parse RAM size".to_string())?;
         Ok(kb / (1024 * 1024))
     } else {
         Err("Failed to parse /proc/meminfo".to_string())
@@ -1361,19 +2491,21 @@ fn get_cpu_count() -> u32 {
 fn get_gpu_info() -> Option<(String, Option<u64>)> {
     // Try to get GPU info from system_profiler on macOS
     let output = std::process::Command::new("system_profiler")
-        .args(&["SPDisplaysDataType"])
+        .args(["SPDisplaysDataType"])
         .output()
         .ok()?;
     let output_str = String::from_utf8_lossy(&output.stdout);
-    
+
     // Simple parsing: look for "Chip Model:" and "VRAM:"
     let mut gpu_name = None;
     let mut vram_gb = None;
-    
+
     for line in output_str.lines() {
         let trimmed = line.trim();
         if trimmed.starts_with("Chip Model:") {
-            gpu_name = trimmed.strip_prefix("Chip Model:").map(|s| s.trim().to_string());
+            gpu_name = trimmed
+                .strip_prefix("Chip Model:")
+                .map(|s| s.trim().to_string());
         }
         if trimmed.starts_with("VRAM") {
             // Try to extract VRAM amount (e.g., "10 GB")
@@ -1384,18 +2516,16 @@ fn get_gpu_info() -> Option<(String, Option<u64>)> {
             }
         }
     }
-    
+
     gpu_name.map(|name| (name, vram_gb))
 }
 
 #[cfg(target_os = "linux")]
 fn get_gpu_info() -> Option<(String, Option<u64>)> {
     // Try lspci on Linux
-    let output = std::process::Command::new("lspci")
-        .output()
-        .ok()?;
+    let output = std::process::Command::new("lspci").output().ok()?;
     let output_str = String::from_utf8_lossy(&output.stdout);
-    
+
     for line in output_str.lines() {
         if line.contains("VGA") || line.contains("3D") {
             // Extract GPU name from lspci output
@@ -1415,7 +2545,7 @@ fn get_gpu_info() -> Option<(String, Option<u64>)> {
         .output()
         .ok()?;
     let output_str = String::from_utf8_lossy(&output.stdout);
-    
+
     let lines: Vec<&str> = output_str.lines().collect();
     if lines.len() > 1 {
         let data_line = lines[1];
@@ -1495,12 +2625,13 @@ struct ClaudeResponse {
     content: Vec<ClaudeContentBlock>,
 }
 
-fn format_provider_http_error(provider: &str, model: &str, status: reqwest::StatusCode, body: &str) -> String {
-    let snippet = body
-        .trim()
-        .chars()
-        .take(200)
-        .collect::<String>();
+fn format_provider_http_error(
+    provider: &str,
+    model: &str,
+    status: reqwest::StatusCode,
+    body: &str,
+) -> String {
+    let snippet = body.trim().chars().take(200).collect::<String>();
     let snippet = if snippet.len() < body.trim().len() {
         format!("{}...", snippet)
     } else {
@@ -1509,12 +2640,15 @@ fn format_provider_http_error(provider: &str, model: &str, status: reqwest::Stat
 
     match status {
         reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN => {
-            format!("Invalid API key for {}. Check your key in Settings.", provider)
+            format!(
+                "Invalid API key for {}. Check your key in Settings.",
+                provider
+            )
         }
         reqwest::StatusCode::TOO_MANY_REQUESTS => {
             format!("Rate limited by {}. Wait a moment and try again.", provider)
         }
-        reqwest::StatusCode::NOT_FOUND => { 
+        reqwest::StatusCode::NOT_FOUND => {
             format!(
                 "Model '{}' not found for {}. It may have been deprecated — check available models in Settings.",
                 model, provider
@@ -1531,9 +2665,9 @@ fn format_provider_http_error(provider: &str, model: &str, status: reqwest::Stat
     }
 }
 
-#[tauri::command]
-async fn ask_ai(
-    app: AppHandle,
+#[derive(Deserialize)]
+#[serde(rename_all = "camelCase")]
+struct AskAiInput {
     user_message: String,
     open_file_content: Option<String>,
     open_file_path: Option<String>,
@@ -1542,9 +2676,30 @@ async fn ask_ai(
     recent_build_output: Vec<String>,
     tf_context: Option<String>,
     mode: Option<String>,
+}
+
+#[tauri::command]
+async fn ask_ai(
+    app: AppHandle,
+    access: tauri::State<'_, FsAccess>,
+    input: AskAiInput,
 ) -> Result<String, String> {
-    let settings = load_ai_settings(app)?
-        .ok_or_else(|| "No AI provider configured. Open Settings to add your API key.".to_string())?;
+    let AskAiInput {
+        user_message,
+        open_file_content,
+        open_file_path,
+        workspace_root,
+        recent_ros_events,
+        recent_build_output,
+        tf_context,
+        mode,
+    } = input;
+    let settings = load_ai_settings(app.clone())?.ok_or_else(|| {
+        "No AI provider configured. Open Settings to add your API key.".to_string()
+    })?;
+    let api_key = SystemCredentialProvider
+        .get(&settings.provider)?
+        .ok_or_else(|| "No API key stored for the selected AI provider".to_string())?;
 
     // --- Context assembly (unchanged, provider-agnostic) ---
     let mut context = String::new();
@@ -1555,13 +2710,13 @@ async fn ask_ai(
              before or after. Use rclpy, follow standard ROS 2 node conventions (a class \
              extending Node, a main() function with rclpy.init/spin/shutdown). Base it on \
              any workspace context below if relevant (e.g., existing topic names, package \
-             structure).\n\n"
+             structure).\n\n",
         ),
         Some("generate_launch") => String::from(
             "You are a ROS 2 launch file generator. The user wants a ROS 2 Python launch file. \
              Output ONLY a single Python code block using ```python fences — no explanation \
              before or after. Follow standard ROS 2 launch conventions (LaunchDescription, \
-             generate_launch_description()).\n\n"
+             generate_launch_description()).\n\n",
         ),
         Some("generate_urdf") => String::from(
             "You are a ROS 2 robot description generator. The user wants a URDF or xacro file \
@@ -1571,13 +2726,13 @@ async fn ask_ai(
              them with correct parent/child and joint type. If workspace context mentions \
              specific link/frame names (e.g., from TF data or existing topics), reuse those \
              names for consistency. Prefer plain URDF unless the user explicitly asks for \
-             xacro macros/parameters.\n\n"
+             xacro macros/parameters.\n\n",
         ),
         Some("explain_error") => String::from(
             "You are a ROS 2 / robotics debugging assistant. The user wants their most recent \
              build error or runtime error explained. Be concise and specific: name the likely \
              root cause, then give a concrete fix. Reference the actual error text from the \
-             context below rather than generic advice.\n\n"
+             context below rather than generic advice.\n\n",
         ),
         Some("explain_tf") => String::from(
             "You are a ROS 2 / robotics TF (transform tree) assistant. The user wants a specific \
@@ -1592,29 +2747,35 @@ async fn ask_ai(
              worth double-checking (e.g., an unexpectedly large offset, a non-normalized \
              quaternion, or a frame pair that doesn't follow common conventions). Be concise — \
              a few short paragraphs, not an exhaustive lecture. Do not invent details about the \
-             robot that aren't supported by the frame names or values given.\n\n"
+             robot that aren't supported by the frame names or values given.\n\n",
         ),
         _ => String::from(
             "You are an AI coding assistant. Answer the user's question directly and accurately. \
              Only reference ROS 2, robotics concepts, or the workspace context below if it is \
-             actually relevant to the question — do not force a connection that isn't there.\n\n"
+             actually relevant to the question — do not force a connection that isn't there.\n\n",
         ),
     };
 
-    if open_file_path.as_deref().map_or(false, |p| p.ends_with(".py"))
-        || recent_ros_events.iter().any(|e| e.contains("WorkspaceEvent")) {
+    if open_file_path
+        .as_deref()
+        .is_some_and(|p| p.ends_with(".py"))
+        || recent_ros_events
+            .iter()
+            .any(|e| e.contains("WorkspaceEvent"))
+    {
         system_context.push_str("This workspace appears to involve ROS 2 development.\n\n");
     }
 
     let file_is_ignored = match (&open_file_path, &workspace_root) {
-        (Some(path), Some(root)) => is_ignored_by_aiignore(root, path),
+        (Some(path), Some(root)) => {
+            is_ignored_by_aiignore(&access, Path::new(root), Path::new(path))?
+        }
         _ => false,
     };
 
     if file_is_ignored {
-        context.push_str(
-            "Currently open file: [path excluded — matches a pattern in .aiignore]\n\n",
-        );
+        context
+            .push_str("Currently open file: [path excluded — matches a pattern in .aiignore]\n\n");
     } else if let Some(path) = &open_file_path {
         context.push_str(&format!("Currently open file: {}\n\n", path));
     }
@@ -1658,69 +2819,111 @@ async fn ask_ai(
         "groq" => {
             let request_body = GroqRequest {
                 model: settings.model.clone(),
-                messages: vec![GroqMessage { role: "user".to_string(), content: full_prompt }],
+                messages: vec![GroqMessage {
+                    role: "user".to_string(),
+                    content: full_prompt,
+                }],
             };
             let response = client
                 .post("https://api.groq.com/openai/v1/chat/completions")
-                .header("Authorization", format!("Bearer {}", settings.api_key))
+                .header("Authorization", format!("Bearer {}", api_key))
                 .header("content-type", "application/json")
                 .json(&request_body)
-                .send().await.map_err(|e| e.to_string())?;
+                .send()
+                .await
+                .map_err(|e| e.to_string())?;
 
             if !response.status().is_success() {
                 let status = response.status();
                 let body = response.text().await.map_err(|e| e.to_string())?;
-                return Err(format_provider_http_error("groq", &settings.model, status, &body));
+                return Err(format_provider_http_error(
+                    "groq",
+                    &settings.model,
+                    status,
+                    &body,
+                ));
             }
 
             let parsed: GroqResponse = response.json().await.map_err(|e| e.to_string())?;
-            parsed.choices.into_iter().next().map(|c| c.message.content)
+            parsed
+                .choices
+                .into_iter()
+                .next()
+                .map(|c| c.message.content)
                 .ok_or_else(|| "No response from AI".to_string())
         }
         "anthropic" => {
             let request_body = ClaudeRequest {
                 model: settings.model.clone(),
                 max_tokens: 1024,
-                messages: vec![ClaudeMessage { role: "user".to_string(), content: full_prompt }],
+                messages: vec![ClaudeMessage {
+                    role: "user".to_string(),
+                    content: full_prompt,
+                }],
             };
             let response = client
                 .post("https://api.anthropic.com/v1/messages")
-                .header("x-api-key", settings.api_key)
+                .header("x-api-key", &api_key)
                 .header("anthropic-version", "2023-06-01")
                 .header("content-type", "application/json")
                 .json(&request_body)
-                .send().await.map_err(|e| e.to_string())?;
+                .send()
+                .await
+                .map_err(|e| e.to_string())?;
 
             if !response.status().is_success() {
                 let status = response.status();
                 let body = response.text().await.map_err(|e| e.to_string())?;
-                return Err(format_provider_http_error("anthropic", &settings.model, status, &body));
+                return Err(format_provider_http_error(
+                    "anthropic",
+                    &settings.model,
+                    status,
+                    &body,
+                ));
             }
 
             let parsed: ClaudeResponse = response.json().await.map_err(|e| e.to_string())?;
-            parsed.content.into_iter().next().and_then(|b| b.text)
+            parsed
+                .content
+                .into_iter()
+                .next()
+                .and_then(|b| b.text)
                 .ok_or_else(|| "No response from AI".to_string())
         }
         "openai" => {
             let request_body = GroqRequest {
                 model: settings.model.clone(),
-                messages: vec![GroqMessage { role: "user".to_string(), content: full_prompt }],
+                messages: vec![GroqMessage {
+                    role: "user".to_string(),
+                    content: full_prompt,
+                }],
             };
             let response = client
                 .post("https://api.openai.com/v1/chat/completions")
-                .header("Authorization", format!("Bearer {}", settings.api_key))
+                .header("Authorization", format!("Bearer {}", api_key))
                 .header("content-type", "application/json")
                 .json(&request_body)
-                .send().await.map_err(|e| e.to_string())?;
+                .send()
+                .await
+                .map_err(|e| e.to_string())?;
 
             if !response.status().is_success() {
                 let status = response.status();
                 let body = response.text().await.map_err(|e| e.to_string())?;
-                return Err(format_provider_http_error("openai", &settings.model, status, &body));
+                return Err(format_provider_http_error(
+                    "openai",
+                    &settings.model,
+                    status,
+                    &body,
+                ));
             }
 
             let parsed: GroqResponse = response.json().await.map_err(|e| e.to_string())?;
-            parsed.choices.into_iter().next().map(|c| c.message.content)
+            parsed
+                .choices
+                .into_iter()
+                .next()
+                .map(|c| c.message.content)
                 .ok_or_else(|| "No response from AI".to_string())
         }
         other => Err(format!("Unknown provider: {}", other)),
@@ -1728,8 +2931,12 @@ async fn ask_ai(
 }
 
 #[tauri::command]
-fn check_aiignore(workspace_root: String, file_path: String) -> bool {
-    is_ignored_by_aiignore(&workspace_root, &file_path)
+fn check_aiignore(
+    access: tauri::State<'_, FsAccess>,
+    workspace_root: String,
+    file_path: String,
+) -> Result<bool, String> {
+    is_ignored_by_aiignore(&access, Path::new(&workspace_root), Path::new(&file_path))
 }
 
 #[tauri::command]
@@ -1756,6 +2963,7 @@ fn list_templates(app: AppHandle) -> Result<Vec<String>, String> {
 #[tauri::command]
 fn create_project_from_template(
     app: AppHandle,
+    access: tauri::State<'_, FsAccess>,
     template_name: String,
     destination_dir: String,
     project_name: String,
@@ -1791,15 +2999,18 @@ fn create_project_from_template(
         return Err(format!("Template '{}' not found", template_name));
     }
 
-    let destination_root = std::path::Path::new(&destination_dir)
-        .canonicalize()
+    let destination_root = access
+        .resolve_existing(Path::new(&destination_dir))
         .map_err(|e| format!("Invalid project destination: {e}"))?;
     if !destination_root.is_dir() {
         return Err("Project destination must be a directory".to_string());
     }
     let dest = destination_root.join(&project_name);
     if dest.exists() {
-        return Err(format!("A folder named '{}' already exists there", project_name));
+        return Err(format!(
+            "A folder named '{}' already exists there",
+            project_name
+        ));
     }
 
     std::fs::create_dir(&dest).map_err(|e| format!("Could not create project directory: {e}"))?;
@@ -1827,6 +3038,62 @@ struct OdometryUpdate {
     qw: f64,
 }
 
+#[cfg(test)]
+mod credential_store_tests {
+    use super::*;
+    use std::collections::HashMap;
+
+    #[derive(Default)]
+    struct MockCredentialProvider {
+        entries: Mutex<HashMap<String, String>>,
+    }
+
+    impl CredentialProvider for MockCredentialProvider {
+        fn get(&self, provider: &str) -> Result<Option<String>, String> {
+            Ok(self
+                .entries
+                .lock()
+                .map_err(|_| "mock store lock failed".to_string())?
+                .get(provider)
+                .cloned())
+        }
+
+        fn set(&self, provider: &str, secret: &str) -> Result<(), String> {
+            self.entries
+                .lock()
+                .map_err(|_| "mock store lock failed".to_string())?
+                .insert(provider.to_string(), secret.to_string());
+            Ok(())
+        }
+
+        fn delete(&self, provider: &str) -> Result<(), String> {
+            self.entries
+                .lock()
+                .map_err(|_| "mock store lock failed".to_string())?
+                .remove(provider);
+            Ok(())
+        }
+    }
+
+    #[test]
+    fn mock_provider_supports_replacement_retrieval_and_deletion() {
+        let provider = MockCredentialProvider::default();
+        assert_eq!(provider.get("groq").unwrap(), None);
+        provider.set("groq", "old").unwrap();
+        assert_eq!(provider.get("groq").unwrap(), Some("old".to_string()));
+        provider.set("groq", "new").unwrap();
+        assert_eq!(provider.get("groq").unwrap(), Some("new".to_string()));
+        provider.delete("groq").unwrap();
+        assert_eq!(provider.get("groq").unwrap(), None);
+    }
+
+    #[test]
+    fn rejects_unknown_credential_providers() {
+        assert!(validate_ai_provider("groq").is_ok());
+        assert!(validate_ai_provider("unknown").is_err());
+    }
+}
+
 #[cfg(test)]
 mod recent_project_tests {
     use super::*;
@@ -1880,12 +3147,21 @@ mod validation_tests {
 
     #[test]
     fn rejects_non_finite_and_unbounded_velocity() {
-        assert!(validate_velocity(1.0, MAX_LINEAR_VELOCITY).is_ok());
+        assert!(validate_velocity(0.5, MAX_LINEAR_VELOCITY).is_ok());
+        assert!(validate_velocity(0.6, MAX_LINEAR_VELOCITY).is_err());
         assert!(validate_velocity(f64::NAN, MAX_LINEAR_VELOCITY).is_err());
         assert!(validate_velocity(f64::INFINITY, MAX_LINEAR_VELOCITY).is_err());
         assert!(validate_velocity(5.1, MAX_LINEAR_VELOCITY).is_err());
     }
 
+    #[test]
+    fn validates_combined_linear_and_angular_velocity_magnitudes() {
+        assert!(validate_velocity_vector([0.3, 0.3, 0.0], 0.5, "Linear").is_ok());
+        assert!(validate_velocity_vector([0.4, 0.4, 0.0], 0.5, "Linear").is_err());
+        assert!(validate_velocity_vector([0.8, 0.8, 0.0], 1.0, "Angular").is_err());
+        assert!(validate_velocity_vector([f64::NAN, 0.0, 0.0], 0.5, "Linear").is_err());
+    }
+
     #[test]
     fn validates_project_path_components() {
         assert!(is_safe_project_component("ros2_robot"));
@@ -1895,10 +3171,173 @@ mod validation_tests {
     }
 }
 
+#[cfg(test)]
+mod filesystem_security_tests {
+    use super::*;
+
+    struct TempDirectory(PathBuf);
+
+    impl TempDirectory {
+        fn new() -> Self {
+            static NEXT_DIRECTORY: std::sync::atomic::AtomicU64 =
+                std::sync::atomic::AtomicU64::new(0);
+            let unique = std::time::SystemTime::now()
+                .duration_since(std::time::UNIX_EPOCH)
+                .unwrap()
+                .as_nanos();
+            let path = std::env::temp_dir().join(format!(
+                "robotics-studio-test-{}-{unique}-{}",
+                std::process::id(),
+                NEXT_DIRECTORY.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
+            ));
+            fs::create_dir(&path).unwrap();
+            Self(path)
+        }
+    }
+
+    impl Drop for TempDirectory {
+        fn drop(&mut self) {
+            let _ = fs::remove_dir_all(&self.0);
+        }
+    }
+
+    #[test]
+    fn approved_root_allows_normal_reads_and_writes() {
+        let directory = TempDirectory::new();
+        let access = FsAccess::default();
+        access.approve_directory(&directory.0).unwrap();
+        let file = directory.0.join("robot.py");
+
+        write_approved_file(&access, &file, "print('ok')").unwrap();
+
+        assert_eq!(read_approved_file(&access, &file).unwrap(), "print('ok')");
+    }
+
+    #[test]
+    fn rejects_paths_outside_root_and_parent_traversal() {
+        let directory = TempDirectory::new();
+        let outside = TempDirectory::new();
+        let access = FsAccess::default();
+        access.approve_directory(&directory.0).unwrap();
+
+        assert!(access.resolve_existing(&outside.0).is_err());
+        assert!(access
+            .resolve_existing(&directory.0.join("../outside"))
+            .is_err());
+    }
+
+    #[cfg(unix)]
+    #[test]
+    fn rejects_symlink_escape_from_approved_root() {
+        use std::os::unix::fs::symlink;
+
+        let directory = TempDirectory::new();
+        let outside = TempDirectory::new();
+        let access = FsAccess::default();
+        access.approve_directory(&directory.0).unwrap();
+        symlink(&outside.0, directory.0.join("escape")).unwrap();
+
+        assert!(access
+            .resolve_existing(&directory.0.join("escape"))
+            .is_err());
+    }
+
+    #[test]
+    fn imports_worlds_to_unique_safe_paths_without_overwriting() {
+        let directory = TempDirectory::new();
+        let source = Path::new("../../Dockerfile.sdf");
+        let sdf = "<sdf version=\"1.9\"><world name=\"test\"/></sdf>";
+        let first = import_world_contents(&directory.0, source, sdf).unwrap();
+        let second = import_world_contents(&directory.0, source, sdf).unwrap();
+
+        assert_ne!(first, second);
+        assert!(first.starts_with(directory.0.canonicalize().unwrap().join("worlds")));
+        assert!(first
+            .file_name()
+            .unwrap()
+            .to_string_lossy()
+            .starts_with("Dockerfile-"));
+        assert_eq!(fs::read_to_string(first).unwrap(), sdf);
+        assert_eq!(fs::read_to_string(second).unwrap(), sdf);
+    }
+
+    #[test]
+    fn rejects_invalid_world_document_and_extension() {
+        let directory = TempDirectory::new();
+        assert!(
+            import_world_contents(&directory.0, Path::new("world.txt"), "<sdf></sdf>").is_err()
+        );
+        assert!(
+            import_world_contents(&directory.0, Path::new("world.sdf"), "<world></world>").is_err()
+        );
+    }
+
+    #[test]
+    fn validates_sdf_xml_root_and_supported_version() {
+        let path = Path::new("world.sdf");
+        assert!(validate_sdf_file(
+            path,
+            "<?xml version=\"1.0\"?><sdf version=\"1.10\"><world name=\"demo\"/></sdf>"
+        )
+        .is_ok());
+        assert!(validate_sdf_file(path, "<sdf version=\"1.9\"><world></sdf>").is_err());
+        assert!(validate_sdf_file(path, "<sdf version=\"1.11\"/>").is_err());
+        assert!(validate_sdf_file(path, "<sdf version=\"1.9\"/>").is_err());
+        assert!(validate_sdf_file(path, "<world version=\"1.9\"/>").is_err());
+        assert!(validate_sdf_file(path, "<!DOCTYPE sdf><sdf version=\"1.9\"/>").is_err());
+        assert!(validate_sdf_file(
+            path,
+            "<sdf version=\"1.9\"><world/></sdf><sdf version=\"1.9\"><world/></sdf>"
+        )
+        .is_err());
+    }
+
+    #[test]
+    fn accepts_namespaced_sdf_root() {
+        assert!(validate_sdf_file(
+            Path::new("world.sdf"),
+            "<sd:sdf xmlns:sd=\"urn:sdformat\" version=\"1.9\"><sd:world name=\"demo\"/></sd:sdf>"
+        )
+        .is_ok());
+    }
+
+    #[cfg(unix)]
+    #[test]
+    fn rejects_symlink_world_destination() {
+        use std::os::unix::fs::symlink;
+
+        let directory = TempDirectory::new();
+        let outside = TempDirectory::new();
+        symlink(&outside.0, directory.0.join("worlds")).unwrap();
+
+        assert!(
+            import_world_contents(&directory.0, Path::new("world.sdf"), "<sdf></sdf>").is_err()
+        );
+        assert_eq!(fs::read_dir(&outside.0).unwrap().count(), 0);
+    }
+}
+
 #[cfg(test)]
 mod stream_line_buffer_tests {
     use super::*;
 
+    #[test]
+    fn test_unterminated_line_is_truncated_and_bounded() {
+        let mut buffer = StreamLineBuffer::new();
+        let oversized = vec![b'x'; MAX_PENDING_LINE_BYTES + 4096];
+
+        let events = buffer.process_chunk(&oversized);
+
+        assert_eq!(events.len(), 1);
+        assert!(events[0].1.ends_with("[line truncated]"));
+        assert!(buffer.pending.len() <= MAX_PENDING_LINE_BYTES);
+        assert!(buffer.discarding_long_line);
+        assert!(buffer
+            .process_chunk(&vec![b'y'; MAX_PENDING_LINE_BYTES])
+            .is_empty());
+        assert_eq!(buffer.process_chunk(b"\nshort line\n")[0].1, "short line");
+    }
+
     #[test]
     fn test_single_chunk_complete_multibyte_utf8() {
         // Test case (a): Single read() with complete multi-byte UTF-8 character
@@ -1908,7 +3347,10 @@ mod stream_line_buffer_tests {
         let events = buf.process_chunk(chunk);
         assert_eq!(events.len(), 1);
         assert_eq!(events[0].0, "append");
-        assert!(events[0].1.contains("─"), "Multi-byte UTF-8 character should be decoded");
+        assert!(
+            events[0].1.contains("─"),
+            "Multi-byte UTF-8 character should be decoded"
+        );
     }
 
     #[test]
@@ -1917,7 +3359,7 @@ mod stream_line_buffer_tests {
         // The character '─' (U+2500) is E2 94 80 in UTF-8.
         // Split as: "Box: " + first 2 bytes of '─' in chunk 1, last byte + "\n" in chunk 2.
         let mut buf = StreamLineBuffer::new();
-        
+
         // First chunk: "Box: " (5 bytes ASCII) + E2 94 (first 2 bytes of '─')
         let chunk1 = b"Box: \xE2\x94";
         let events1 = buf.process_chunk(chunk1);
@@ -1929,8 +3371,7 @@ mod stream_line_buffer_tests {
         assert_eq!(events2.len(), 1, "Should emit one complete line");
         assert_eq!(events2[0].0, "append");
         assert_eq!(
-            events2[0].1,
-            "Box: ─ line",
+            events2[0].1, "Box: ─ line",
             "Split UTF-8 character should reassemble exactly, got '{}'",
             events2[0].1
         );
@@ -1949,7 +3390,10 @@ mod stream_line_buffer_tests {
         let events2 = buf.process_chunk(chunk2);
         assert_eq!(events2.len(), 1, "Should emit one complete line");
         assert_eq!(events2[0].0, "append");
-        assert_eq!(events2[0].1, "Downloading layer 3/8", "Should combine both chunks into one line");
+        assert_eq!(
+            events2[0].1, "Downloading layer 3/8",
+            "Should combine both chunks into one line"
+        );
     }
 
     #[test]
@@ -1959,9 +3403,13 @@ mod stream_line_buffer_tests {
         let mut buf = StreamLineBuffer::new();
         let chunk = b"Pulling fs layer\rDownloading 10%\rDownloading 45%\rDownloading 100%\n";
         let events = buf.process_chunk(chunk);
-        
+
         // Should produce 4 events: 3 "replace" and 1 final "append"
-        assert_eq!(events.len(), 4, "Should emit 4 events (3 replace + 1 append)");
+        assert_eq!(
+            events.len(),
+            4,
+            "Should emit 4 events (3 replace + 1 append)"
+        );
         assert_eq!(events[0].0, "replace");
         assert_eq!(events[0].1, "Pulling fs layer");
         assert_eq!(events[1].0, "replace");
@@ -1992,18 +3440,18 @@ mod stream_line_buffer_tests {
         // This test ensures we don't panic when a multi-byte UTF-8 sequence
         // happens to end exactly at the boundary of a chunk (the old code would panic here).
         let mut buf = StreamLineBuffer::new();
-        
+
         // First chunk ends with a complete multi-byte character (3-byte UTF-8)
         let chunk1 = "Test─".as_bytes(); // ends with complete ─
         let _events1 = buf.process_chunk(chunk1);
-        
+
         // Second chunk continues with more text
         let chunk2 = b"more\n";
         let events2 = buf.process_chunk(chunk2);
-        
+
         // Should not panic and should produce one event
         assert_eq!(events2.len(), 1);
         assert_eq!(events2[0].0, "append");
         assert!(events2[0].1.contains("Test─more"));
     }
-}
\ No newline at end of file
+}
diff --git a/src-tauri/tauri.conf.json b/src-tauri/tauri.conf.json
index 7dd9eb0..281e3db 100644
--- a/src-tauri/tauri.conf.json
+++ b/src-tauri/tauri.conf.json
@@ -1,7 +1,7 @@
 {
   "$schema": "https://schema.tauri.app/config/2",
   "productName": "robotics-studio",
-  "version": "0.2.0",
+  "version": "0.3.0",
   "identifier": "com.kartik.robotics-studio",
   "build": {
     "beforeDevCommand": "npm run dev",
@@ -19,7 +19,8 @@
       }
     ],
     "security": {
-      "csp": null
+      "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; worker-src 'self' blob:; img-src 'self' asset: http://asset.localhost data: blob:; font-src 'self' data:; connect-src ipc: http://ipc.localhost; object-src 'none'; base-uri 'self'; frame-ancestors 'none'",
+      "devCsp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; worker-src 'self' blob:; img-src 'self' asset: http://asset.localhost data: blob:; font-src 'self' data:; connect-src ipc: http://ipc.localhost http://localhost:1420 ws://localhost:1420; object-src 'none'; base-uri 'self'; frame-ancestors 'none'"
     }
   },
   "bundle": {
diff --git a/src/App.tsx b/src/App.tsx
index 81296ab..0abef8f 100644
--- a/src/App.tsx
+++ b/src/App.tsx
@@ -6,7 +6,6 @@ import { listen, emit } from "@tauri-apps/api/event";
 import TFTree from "./components/TFTree";
 import Settings from "./components/Settings";
 import NewProjectModal from "./components/NewProjectModal";
-import { open, save } from "@tauri-apps/plugin-dialog";
 import {
   KeyBinding,
   DEFAULT_KEYBINDINGS,
@@ -57,6 +56,21 @@ interface LogEvent {
 }
 
 const MAX_RECENT_PROJECTS = 8;
+const MAX_LOG_LINES = 1000;
+const MAX_LOG_CHARACTERS = 2_000_000;
+
+function trimLog(lines: string[]) {
+  const trimmed = lines.slice(-MAX_LOG_LINES);
+  let characters = trimmed.reduce((total, line) => total + line.length, 0);
+  while (characters > MAX_LOG_CHARACTERS && trimmed.length > 1) {
+    characters -= trimmed.shift()?.length ?? 0;
+  }
+  return trimmed;
+}
+
+function appendLog(lines: string[], line: string) {
+  return trimLog([...lines, line]);
+}
 
 function normalizeRecentProjects(projects: RecentProject[]) {
   const byPath = new Map<string, RecentProject>();
@@ -241,12 +255,11 @@ function App() {
   const [keymap, setKeymap] = useState<KeyBinding[]>(DEFAULT_KEYBINDINGS);
 
   const pickCustomWorld = async () => {
-    const selected = await open({
-      filters: [{ name: "SDF World", extensions: ["sdf"] }],
-      multiple: false,
-    });
-    if (typeof selected === "string") {
-      setWorldPath(selected);
+    try {
+      const selected = await invoke<string | null>("import_world");
+      if (selected) setWorldPath(selected);
+    } catch (error) {
+      console.error("World import failed:", error);
     }
   };
 
@@ -254,6 +267,8 @@ function App() {
   const [rosEvents, setRosEvents] = useState<string[]>([]);
   const [rosConnected, setRosConnected] = useState(false);
   const [initializing, setInitializing] = useState(false);
+  const rosWorkspaceReady = useRef(false);
+  const authRecoveryStarted = useRef(false);
   const [setupOutput, setSetupOutput] = useState<string[]>([]); // Environment setup logs
   const [currentSetupLine, setCurrentSetupLine] = useState<string>(""); // Current line for toast
 
@@ -394,8 +409,8 @@ function App() {
   }, []);
 
   const pickFolder = async () => {
-    const selected = await open({ directory: true, multiple: false });
-    if (typeof selected === "string") {
+    const selected = await invoke<string | null>("pick_directory");
+    if (selected) {
       await loadFolder(selected);
     }
   };
@@ -489,21 +504,16 @@ function App() {
   };
 
   const saveGeneratedCode = async (code: string, isXml: boolean) => {
-    const selected = await save({
-      defaultPath: currentPath ?? undefined,
-      filters: isXml
-        ? [{ name: "URDF/XML", extensions: ["urdf", "xacro", "xml"] }]
-        : [{ name: "Python", extensions: ["py"] }],
-    });
-    if (typeof selected === "string") {
+    const selected = await invoke<string | null>("pick_save_path", { isXml });
+    if (selected) {
       await invoke("write_file", { path: selected, contents: code });
     }
   };
 
   // ---------- ROS workspace selection ----------
   const pickRosWorkspace = async () => {
-    const selected = await open({ directory: true, multiple: false });
-    if (typeof selected === "string") {
+    const selected = await invoke<string | null>("pick_directory");
+    if (selected) {
       setRosWorkspacePath(selected);
     }
   };
@@ -518,7 +528,7 @@ function App() {
       await invoke("run_colcon_build_streaming");
     } catch (error) {
       setBuilding(false);
-      setBuildOutput((prev) => [...prev, `ERROR: ${String(error)}`]);
+      setBuildOutput((prev) => appendLog(prev, `ERROR: ${String(error)}`));
     }
   };
 
@@ -529,7 +539,7 @@ function App() {
     try {
       await invoke("initialize_ros_environment");
     } catch (err) {
-      setSetupOutput((prev) => [...prev, `❌ ${err}`]);
+      setSetupOutput((prev) => appendLog(prev, `❌ ${err}`));
       setCurrentSetupLine(`❌ ${err}`);
       setInitializing(false);
     }
@@ -574,7 +584,9 @@ function App() {
     angularZ: number,
   ) => {
     try {
-      await invoke("publish_twist", { topicName, linearX, angularZ });
+      await invoke("publish_twist", {
+        command: { topicName, linearX, angularZ },
+      });
     } catch (err) {
       console.error("Twist command failed:", err);
     }
@@ -594,14 +606,16 @@ function App() {
 
     try {
       const response = await invoke<string>("ask_ai", {
-        userMessage: question,
-        openFileContent: activeFile?.content || null,
-        openFilePath: activeFile?.path || null,
-        workspaceRoot: currentPath ?? null,
-        recentRosEvents: rosEvents,
-        recentBuildOutput: buildOutput,
-        tfContext: tfContext ?? null,
-        mode: mode ?? null,
+        input: {
+          userMessage: question,
+          openFileContent: activeFile?.content || null,
+          openFilePath: activeFile?.path || null,
+          workspaceRoot: currentPath ?? null,
+          recentRosEvents: rosEvents,
+          recentBuildOutput: buildOutput,
+          tfContext: tfContext ?? null,
+          mode: mode ?? null,
+        },
       });
       setChatHistory((prev) => [
         ...prev,
@@ -711,6 +725,7 @@ function App() {
     const unlistenEvent = listen<string>("ros-event", (event) => {
       setRosEvents((prev) => [...prev.slice(-49), event.payload]);
       setRosConnected(true);
+      authRecoveryStarted.current = false;
 
       // Extract any cmd_vel topics from TopicSnapshot events, so the
       // robot control dropdown reflects whatever's actually in the
@@ -745,6 +760,17 @@ function App() {
       setRosConnected(false);
     });
 
+    const unlistenAuthError = listen<string>("ros-auth-error", (event) => {
+      setRosConnected(false);
+      setRosEvents((prev) =>
+        [...prev.slice(-49), `Authentication: ${event.payload}`],
+      );
+      if (rosWorkspaceReady.current && !authRecoveryStarted.current) {
+        authRecoveryStarted.current = true;
+        void initializeRosEnvironment();
+      }
+    });
+
     const unlistenConnecting = listen<void>("ros-connecting", () => {
       setRosConnected(false);
     });
@@ -764,23 +790,26 @@ function App() {
         });
       } else {
         // Append a new line
-        setBuildOutput((prev) => [...prev, text]);
+        setBuildOutput((prev) => appendLog(prev, text));
       }
     });
     const unlistenBuildError = listen<string>("build-error", (event) => {
-      setBuildOutput((prev) => [...prev, `ERROR: ${event.payload}`]);
+      setBuildOutput((prev) => appendLog(prev, `ERROR: ${event.payload}`));
     });
     const unlistenBuildFinished = listen<boolean>("build-finished", (event) => {
       setBuilding(false);
-      setBuildOutput((prev) => [
-        ...prev,
-        event.payload ? "[✓ Build succeeded]" : "[✗ Build failed]",
-      ]);
+      setBuildOutput((prev) =>
+        appendLog(
+          prev,
+          event.payload ? "[✓ Build succeeded]" : "[✗ Build failed]",
+        ),
+      );
     });
 
     return () => {
       unlistenEvent.then((f) => f());
       unlistenError.then((f) => f());
+      unlistenAuthError.then((f) => f());
       unlistenNodeSnapshot.then((f) => f());
       unlistenConnecting.then((f) => f());
       unlistenBuildOutput.then((f) => f());
@@ -805,19 +834,23 @@ function App() {
         });
       } else {
         // Append a new line
-        setSetupOutput((prev) => [...prev.slice(-99), text]);
+        setSetupOutput((prev) => appendLog(prev, text));
       }
       // Always update the toast's current line
       setCurrentSetupLine(text);
     });
     const unlistenFinished = listen<string>("init-finished", (event) => {
+      rosWorkspaceReady.current = true;
       setRosWorkspacePath(event.payload);
-      setSetupOutput((prev) => [...prev, "✅ ROS environment ready"]);
+      setSetupOutput((prev) => appendLog(prev, "✅ ROS environment ready"));
       setCurrentSetupLine("✅ ROS environment ready");
       setInitializing(false);
+      void invoke("start_ros_stream").catch((error) => {
+        setRosEvents((prev) => [...prev.slice(-49), `ROS stream failed: ${error}`]);
+      });
     });
     const unlistenError = listen<string>("init-error", (event) => {
-      setSetupOutput((prev) => [...prev, `❌ ${event.payload}`]);
+      setSetupOutput((prev) => appendLog(prev, `❌ ${event.payload}`));
       setCurrentSetupLine(`❌ ${event.payload}`);
       setInitializing(false);
     });
diff --git a/src/components/NewProjectModal.tsx b/src/components/NewProjectModal.tsx
index 075a81e..66a6633 100644
--- a/src/components/NewProjectModal.tsx
+++ b/src/components/NewProjectModal.tsx
@@ -1,6 +1,5 @@
 import { useEffect, useState } from "react";
 import { invoke } from "@tauri-apps/api/core";
-import { open } from "@tauri-apps/plugin-dialog";
 import { revealItemInDir } from "@tauri-apps/plugin-opener";
 import { X } from "lucide-react";
 
@@ -29,8 +28,8 @@ export default function NewProjectModal({ onClose, onProjectCreated }: Props) {
   }, []);
 
   const pickDestination = async () => {
-    const selected = await open({ directory: true, multiple: false });
-    if (typeof selected === "string") setDestinationDir(selected);
+    const selected = await invoke<string | null>("pick_directory");
+    if (selected) setDestinationDir(selected);
   };
 
   const canCreate =
diff --git a/src/components/RobotView.tsx b/src/components/RobotView.tsx
index 3790283..50e76fa 100644
--- a/src/components/RobotView.tsx
+++ b/src/components/RobotView.tsx
@@ -278,6 +278,17 @@ export function RobotControls({
         </select>
       )}
 
+      <div
+        style={{
+          marginBottom: "var(--spacing-sm)",
+          fontSize: "var(--font-xs)",
+          opacity: 0.75,
+        }}
+      >
+        Speed magnitude limits: 0.5 m/s linear, 1 rad/s angular. Commands time
+        out after 0.5 s; this is not a safety-rated emergency stop.
+      </div>
+
       <div
         style={{
           display: "grid",
diff --git a/src/components/Settings.tsx b/src/components/Settings.tsx
index d3a2995..6fc5cf4 100644
--- a/src/components/Settings.tsx
+++ b/src/components/Settings.tsx
@@ -1,6 +1,5 @@
-import { useState, useEffect } from "react";
+import { useState, useEffect, useRef } from "react";
 import { invoke } from "@tauri-apps/api/core";
-import { open } from "@tauri-apps/plugin-dialog";
 import { X, Lightbulb } from "lucide-react";
 import {
   KeyBinding,
@@ -10,8 +9,8 @@ import {
 
 interface AIProviderSettings {
   provider: string;
-  api_key: string;
   model: string;
+  has_api_key: boolean;
 }
 
 interface SystemSpecs {
@@ -55,7 +54,11 @@ export default function Settings({
   const [provider, setProvider] = useState("groq");
   const [apiKey, setApiKey] = useState("");
   const [model, setModel] = useState(MODELS_BY_PROVIDER["groq"][0]);
+  const [hasApiKey, setHasApiKey] = useState(false);
+  const [aiError, setAiError] = useState<string | null>(null);
   const [aiSaved, setAiSaved] = useState(false);
+  const [aiSaving, setAiSaving] = useState(false);
+  const credentialLookupId = useRef(0);
 
   // Tab state
   const [activeTab, setActiveTab] = useState<"ai" | "shortcuts" | "system">(
@@ -82,10 +85,10 @@ export default function Settings({
     invoke<AIProviderSettings | null>("load_ai_settings").then((existing) => {
       if (existing) {
         setProvider(existing.provider);
-        setApiKey(existing.api_key);
         setModel(existing.model);
+        setHasApiKey(existing.has_api_key);
       }
-    });
+    }).catch((error) => setAiError(String(error)));
   }, []);
 
   // Load keybindings on mount
@@ -118,16 +121,53 @@ export default function Settings({
 
   // AI Provider tab handlers
   const handleProviderChange = (p: string) => {
+    const lookupId = ++credentialLookupId.current;
     setProvider(p);
     setModel(MODELS_BY_PROVIDER[p][0]);
+    setApiKey("");
+    setHasApiKey(false);
+    setAiError(null);
+    setAiSaved(false);
+    invoke<boolean>("has_ai_credential", { provider: p })
+      .then((exists) => {
+        if (lookupId === credentialLookupId.current) setHasApiKey(exists);
+      })
+      .catch((error) => {
+        if (lookupId === credentialLookupId.current) setAiError(String(error));
+      });
   };
 
   const handleSaveAi = async () => {
-    await invoke("save_ai_settings", {
-      settings: { provider, api_key: apiKey, model },
-    });
-    setAiSaved(true);
-    setTimeout(() => setAiSaved(false), 2000);
+    if (aiSaving) return;
+    const selectedProvider = provider;
+    const selectedApiKey = apiKey;
+    const hadSavedCredential = hasApiKey;
+    setAiSaving(true);
+    setAiError(null);
+    try {
+      await invoke("save_ai_settings", {
+        settings: { provider: selectedProvider, api_key: selectedApiKey, model },
+      });
+      setHasApiKey(hadSavedCredential || selectedApiKey.trim().length > 0);
+      setApiKey("");
+      setAiSaved(true);
+      setTimeout(() => setAiSaved(false), 2000);
+    } catch (error) {
+      setAiError(String(error));
+    } finally {
+      setAiSaving(false);
+    }
+  };
+
+  const handleDeleteAiCredential = async () => {
+    try {
+      await invoke("delete_ai_credential", { provider });
+      setHasApiKey(false);
+      setApiKey("");
+      setAiError(null);
+    } catch (error) {
+      setAiError(String(error));
+    }
   };
 
   // Keyboard shortcuts handlers
@@ -182,14 +222,8 @@ export default function Settings({
 
   const handleImportVsCode = async () => {
     try {
-      const path = await open({
-        filters: [{ name: "JSON", extensions: ["json"] }],
-        multiple: false,
-      });
-
-      if (!path) return;
-
-      const content = await invoke<string>("read_file", { path });
+      const content = await invoke<string | null>("pick_keybindings_file");
+      if (!content) return;
       const vsCodeBindings: VsCodeKeybinding[] = JSON.parse(content);
 
       if (!Array.isArray(vsCodeBindings)) {
@@ -373,6 +407,7 @@ export default function Settings({
               <select
                 value={provider}
                 onChange={(e) => handleProviderChange(e.target.value)}
+                disabled={aiSaving}
                 style={{
                   width: "100%",
                   padding: "var(--spacing-sm)",
@@ -400,6 +435,7 @@ export default function Settings({
               <select
                 value={model}
                 onChange={(e) => setModel(e.target.value)}
+                disabled={aiSaving}
                 style={{
                   width: "100%",
                   padding: "var(--spacing-sm)",
@@ -430,7 +466,10 @@ export default function Settings({
                 type="password"
                 value={apiKey}
                 onChange={(e) => setApiKey(e.target.value)}
-                placeholder="sk-..."
+                disabled={aiSaving}
+                placeholder={
+                  hasApiKey ? "Leave blank to keep the saved key" : "Enter API key"
+                }
                 style={{
                   width: "100%",
                   padding: "var(--spacing-sm)",
@@ -440,6 +479,28 @@ export default function Settings({
                   border: "1px solid #3c3c3c",
                 }}
               />
+              <div
+                style={{
+                  fontSize: "var(--font-xs)",
+                  marginBottom: "var(--spacing-md)",
+                  opacity: 0.75,
+                }}
+              >
+                {hasApiKey
+                  ? "A key is stored in the operating-system credential store."
+                  : "No key is stored for this provider."}
+              </div>
+              {aiError && (
+                <div
+                  style={{
+                    color: "#e08080",
+                    fontSize: "var(--font-xs)",
+                    marginBottom: "var(--spacing-sm)",
+                  }}
+                >
+                  {aiError}
+                </div>
+              )}
 
               <div
                 style={{
@@ -459,8 +520,20 @@ export default function Settings({
                     Saved
                   </span>
                 )}
-                <button className="ide-btn primary" onClick={handleSaveAi}>
-                  Save
+                {hasApiKey && (
+                  <button
+                    className="ide-btn"
+                    onClick={handleDeleteAiCredential}
+                  >
+                    Delete Key
+                  </button>
+                )}
+                <button
+                  className="ide-btn primary"
+                  onClick={handleSaveAi}
+                  disabled={aiSaving}
+                >
+                  {aiSaving ? "Saving..." : "Save"}
                 </button>
               </div>
             </div>

``````````
