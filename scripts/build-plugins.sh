#!/usr/bin/env bash
set -euo pipefail

# Build all WASM plugins.
# Requires: cargo-component, wasm32-wasip1 target, jco (for JS plugins)

PLUGINS_DIR="plugins"

# Rust plugins (cargo-component)
RUST_PLUGINS=(fs chat agent memory skills)

for plugin in "${RUST_PLUGINS[@]}"; do
    echo "=== Building Rust plugin: $plugin ==="
    (cd "$PLUGINS_DIR/$plugin" && cargo component build)
done

# JavaScript plugins (jco componentize)
if command -v jco &>/dev/null; then
    echo "=== Building JS plugin: hello-js ==="
    jco componentize "$PLUGINS_DIR/hello-js/plugin.js" \
        --wit crates/rustacle-plugin-wit/wit/ \
        --world-name plugin \
        --out "$PLUGINS_DIR/hello-js/hello-js.wasm"
else
    echo "SKIP: jco not installed. Install with: npm install -g @bytecodealliance/jco @bytecodealliance/componentize-js @bytecodealliance/preview2-shim"
fi

echo "All plugins built."
