#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

step() {
  echo
  echo "==> $1"
}

run_step() {
  step "$1"
  shift
  "$@"
}

if ! command -v docker >/dev/null 2>&1; then
  echo "ERROR: Docker is required for the release gate, but it is not installed or not on PATH." >&2
  exit 1
fi

if ! docker info >/dev/null 2>&1; then
  echo "ERROR: Docker is required for the release gate, but the daemon is not reachable." >&2
  exit 1
fi

run_step "Install frontend dependencies" npm ci
run_step "Frontend dependency audit" npm audit --audit-level=high
run_step "Frontend TypeScript build" npm run build
run_step "Frontend tests" npm run test:frontend -- --run

run_step "Rust format check" cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
run_step "Rust test suite" cargo test --manifest-path src-tauri/Cargo.toml --locked
run_step "Rust clippy" cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features --locked -- -D warnings

if ! cargo audit --version >/dev/null 2>&1; then
  cargo install cargo-audit --version 0.23.0 --locked
fi
run_step "Rust dependency audit" cargo audit --file src-tauri/Cargo.lock

run_step "Python unit tests" python -m unittest discover -s src-tauri/resources/devcontainer -p 'test_*.py' -v
run_step "Python compile check" python -m py_compile src-tauri/resources/devcontainer/server.py src-tauri/resources/devcontainer/server_core.py src-tauri/resources/devcontainer/test_server_core.py

step "Docker integration script syntax check"
bash -n scripts/docker-integration.sh

step "Docker integration smoke test"
bash scripts/docker-integration.sh

echo
echo "Release readiness checks completed successfully."
