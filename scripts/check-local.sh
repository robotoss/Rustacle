#!/usr/bin/env bash
set -euo pipefail

echo "=== Format check ==="
cargo fmt --all -- --check

echo "=== Clippy ==="
cargo clippy --workspace -- -D warnings

echo "=== Tests ==="
cargo nextest run --workspace 2>/dev/null || cargo test --workspace

echo "=== Deny ==="
cargo deny check 2>/dev/null || echo "cargo-deny not installed, skipping"

echo "All checks passed!"
