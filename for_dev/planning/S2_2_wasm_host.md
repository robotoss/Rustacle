# S2.2 — WASM Host (wasmtime Integration)

## Goal
Implement `rustacle-wasm-host` — the wasmtime integration crate that loads, links, and runs WASM plugin components, enforcing fuel and memory limits.

## Context
This crate is the runtime bridge between the kernel and WASM plugins. It creates wasmtime Stores with resource limits (fuel 10M instructions, 64 MiB memory), links host imports defined in the WIT contract, verifies Ed25519 signatures on `.wasm` files, and wraps loaded components behind the `RustacleModule` trait from `rustacle-plugin-api`. Each plugin gets its own Store — never shared. This depends on S2.1 (WIT contract) and S2.3 (plugin API trait).

## Docs to read
- `for_dev/architecture.md` section 4.1 — WASM-first design, component model rationale.
- `for_dev/project_structure.md` — `rustacle-wasm-host` crate layout and file responsibilities.
- `for_dev/tech_stack_2026.md` section 2 — sandbox hardening stack, wasmtime version.
- `for_dev/security.md` — plugin isolation requirements, signature verification, TOCTOU concerns.

## Reference code
- Internet: [wasmtime component model Rust API](https://docs.wasmtime.dev/api/wasmtime/component/), [wit-bindgen host generation](https://github.com/bytecodealliance/wit-bindgen), [wasmtime fuel metering](https://docs.wasmtime.dev/api/wasmtime/struct.Config.html#method.consume_fuel).

## Deliverables
```
crates/rustacle-wasm-host/src/
├── lib.rs               # public API re-exports
├── host.rs              # Store setup: fuel 10M default, memory 64 MiB limit
├── linker.rs            # host imports linked from WIT (publish, fs-read, log, etc.)
├── loader.rs            # .wasm file loader + Ed25519 signature verification
├── adapter.rs           # impl RustacleModule for WasmtimeInstance
├── llm_bridge.rs        # llm-stream / llm-poll stubs (no-op until LLM sprint)
└── state_migration.rs   # export_state / import_state with Transient/Serialized/ExternalStore policies
```

## Checklist
- [ ] A test `.wasm` component can be loaded and its `init` export called
- [ ] Fuel limit (10M default) enforces instruction budgets; exceeding fuel returns a Trap error
- [ ] Memory limit (64 MiB) is enforced via wasmtime `StoreLimits`
- [ ] Unsigned `.wasm` files are refused with a clear error message
- [ ] Signed `.wasm` files with valid Ed25519 signatures load successfully
- [ ] Host imports (`publish`, `fs-read`, `log`) are linked and callable from guest
- [ ] `RustacleModule` trait is implemented for loaded WASM components
- [ ] Each plugin gets its own `Store` — no sharing
- [ ] State migration policies (Transient, Serialized, ExternalStore) are handled in `export_state`/`import_state`
- [ ] `llm-stream` and `llm-poll` stubs return "not implemented" gracefully
- [ ] Unit tests cover load, fuel exhaustion, memory limit, and signature rejection

## Acceptance criteria
```bash
# Workspace compiles
cargo check --workspace

# Unit tests pass
cargo test -p rustacle-wasm-host

# A signed test .wasm loads; unsigned is rejected
cargo test -p rustacle-wasm-host -- signature
```

## Anti-patterns
- Do NOT give plugins ambient filesystem or network access — all I/O goes through host imports and the permission broker.
- Do NOT skip signature verification, even in debug builds (use a dev key instead).
- Do NOT use a single Store for multiple plugins — each plugin must be isolated.
- Do NOT block the async runtime in host import implementations — use `async` throughout.
- Do NOT allow unbounded memory growth — always configure `StoreLimits`.
