# hello-js — JavaScript WASM Plugin

A demo plugin written in **JavaScript** (not Rust) that compiles to the same WASM Component format and implements the same `rustacle:plugin/module` WIT interface.

This proves the WIT contract is **language-neutral**: any language that can target the WebAssembly Component Model can be a Rustacle plugin.

## Commands

| Command | Input | Output |
|---------|-------|--------|
| `greet` | `{ "name": "Alice" }` | `Hello from JavaScript, Alice!` |
| `ping` | `{}` | `pong from JS plugin` |
| `info` | `{}` | Runtime info and supported commands |

## Build

```bash
# Prerequisites
npm install -g @bytecodealliance/jco @bytecodealliance/componentize-js @bytecodealliance/preview2-shim

# Build
jco componentize plugins/hello-js/plugin.js \
  --wit crates/rustacle-plugin-wit/wit/ \
  --world-name plugin \
  --out plugins/hello-js/hello-js.wasm
```

## How it works

1. `plugin.js` exports a `module` object implementing the WIT `module` interface
2. `jco componentize` compiles JS + SpiderMonkey engine into a WASM Component
3. The resulting `.wasm` file has the same component type as Rust plugins
4. The host loads it with the same wasmtime linker — no special handling needed
