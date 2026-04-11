#!/usr/bin/env bash
set -euo pipefail

WIT_DIR="crates/rustacle-plugin-wit/wit"

echo "Checking WIT contract in $WIT_DIR..."

if command -v wasm-tools &>/dev/null; then
    wasm-tools component wit "$WIT_DIR"
    echo "WIT contract is valid."
else
    echo "wasm-tools not installed. Install with: cargo install wasm-tools"
    echo "Skipping WIT validation."
fi
