# S2.5 — Filesystem Plugin (First Real WASM Plugin)

## Goal
Implement `plugins/fs` as the first real WASM plugin component with `read_file`, `list_dir`, `stat`, `search`, and `selected_files`, proving the entire plugin pipeline end-to-end.

## Context
This plugin validates the full stack: WIT contract (S2.1) to wasm-host loading (S2.2) to permission broker checks (S2.4) to real filesystem functionality. The fs plugin is capability-scoped — it can only access paths the user has explicitly granted via the permission broker. It is compiled as a WASM component using `cargo-component` and must be Ed25519-signed before the host will load it.

## Docs to read
- `for_dev/architecture.md` section 4.2 — WIT contract, the `module` interface this plugin exports.
- `for_dev/project_structure.md` — `plugins/fs` layout.
- `for_dev/tools_catalog.md` section 1 — `fs_read` tool behavior and parameters.
- `for_dev/security.md` — TOCTOU race conditions, symlink escape prevention, path canonicalization.

## Reference code
- Internet: [cargo-component guest crate setup](https://github.com/bytecodealliance/cargo-component), [wit-bindgen guest generation](https://github.com/bytecodealliance/wit-bindgen).
- `refs/cc-src/tools/FileReadTool/` — behavior patterns for file reading (line ranges, encoding handling).

## Deliverables
```
plugins/fs/
├── Cargo.toml          # crate-type = ["cdylib"], cargo-component metadata
├── src/
│   ├── lib.rs          # wit-bindgen export macro, init/shutdown/handle_command dispatch
│   ├── commands.rs     # read_file, list_dir, stat, search implementations
│   ├── selection.rs    # selected_files set management, publishes fs.selected event
│   └── scopes.rs      # client-side scope checks before calling host fs-read

scripts/
└── sign-plugin.sh      # Ed25519 signing script for .wasm artifacts

keys/
└── trusted_plugin_keys.toml   # dev signing key for local development
```

## Checklist
- [ ] `cargo component build -p fs` produces a valid `.wasm` component
- [ ] Plugin loads from a signed `.wasm` file at startup
- [ ] Unsigned `.wasm` is refused with a visible, descriptive error
- [ ] `read_file` works within a granted path scope
- [ ] `read_file` outside the granted scope returns `Denied` error
- [ ] `read_file` supports line range parameters (start_line, end_line)
- [ ] `list_dir` returns directory entries with names and types
- [ ] `stat` returns file metadata (size, modified time, type)
- [ ] `search` performs content search within scoped directories (no shell-out)
- [ ] `selected_files` maintains a set and publishes `fs.selected` events via host `publish`
- [ ] Path canonicalization prevents symlink escape attacks (verified with proptest)
- [ ] Permission cache invalidates when the user edits grant settings
- [ ] Plugin handles UTF-8 and binary files gracefully (binary returns error or base64)
- [ ] Fuel budget is respected — large directory traversals don't hang the host

## Acceptance criteria
```bash
# Build the WASM component
cargo component build -p fs

# Sign it
bash scripts/sign-plugin.sh target/wasm32-wasip1/debug/fs.wasm

# Run integration tests (loads plugin, checks permission flow)
cargo test -p rustacle-wasm-host -- fs_plugin

# Proptest for symlink escape
cargo test -p fs -- symlink_escape --features proptest
```

## Anti-patterns
- Do NOT skip signature verification, even during development — use the dev key from `keys/trusted_plugin_keys.toml`.
- Do NOT allow path traversal via symlinks — always canonicalize and re-check scope after resolution.
- Do NOT implement search as a shell-out to `grep` or `rg` — use Rust's `std` or `walkdir` within the WASM guest.
- Do NOT read entire large files into memory — support streaming or chunked reads.
- Do NOT bypass the permission broker for any filesystem access.
