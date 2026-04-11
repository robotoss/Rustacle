#!/usr/bin/env bash
set -euo pipefail

# Generate TypeScript bindings from Rust types via tauri-specta.
# Run this after modifying IPC types in rustacle-ipc.

cargo run -p rustacle-app --bin export_bindings

echo "Bindings regenerated at ui/bindings.ts"
