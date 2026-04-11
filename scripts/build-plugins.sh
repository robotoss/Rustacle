#!/usr/bin/env bash
set -euo pipefail

# Build all WASM plugins using cargo-component.
# Requires: cargo-component, wasm32-wasip1 target

PLUGINS_DIR="plugins"
WASM_PLUGINS=(fs chat agent memory skills)

for plugin in "${WASM_PLUGINS[@]}"; do
    echo "=== Building $plugin ==="
    (cd "$PLUGINS_DIR/$plugin" && cargo component build)
done

echo "All WASM plugins built."
