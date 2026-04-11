# S2.4 — Permission Broker

## Goal
Implement the `PermissionBroker` in `rustacle-kernel` with the ask-grant-cache-invalidate flow, enforcing capability checks for every plugin operation.

## Context
Every capability use — filesystem reads, network calls, PTY access, secrets, LLM providers — passes through the broker. It maintains a thread-safe cache of grants (using `DashMap`), asks the user via a `tokio::sync::oneshot` channel when a capability is not yet granted, and invalidates cached grants when settings change. The implementation in architecture.md section 4.7 is the reference. This crate depends on S2.3 (plugin API types for `Capability`).

## Docs to read
- `for_dev/architecture.md` section 4.7 — full Rust implementation of the Permission Broker, including `check()`, `invalidate()`, `CapabilityKey`, and `PermissionAsk`.
- `for_dev/security.md` — security model, principle of least privilege, grant scoping.
- `for_dev/project_structure.md` — `rustacle-kernel/permission/` module layout.

## Reference code
- `for_dev/architecture.md` section 4.7 has the complete Rust implementation to follow.
- Internet: [`dashmap` docs](https://docs.rs/dashmap), [`tokio::sync::oneshot`](https://docs.rs/tokio/latest/tokio/sync/oneshot/).

## Deliverables
```
crates/rustacle-kernel/src/permission/
├── mod.rs    # PermissionBroker struct, check(), invalidate(), new()
├── key.rs    # CapabilityKey canonicalization, Fs prefix matching, Net host pattern matching, Secret exact match
└── ask.rs    # PermissionAsk struct (capability, plugin_id, oneshot sender), Grant enum, PermissionDecision
```

## Checklist
- [x] `PermissionBroker::check()` returns a cached `Grant` immediately if present
- [x] `PermissionBroker::check()` sends a `PermissionAsk` via channel when no cached grant exists
- [x] `PermissionAsk` includes the capability, plugin ID, and a `oneshot::Sender<PermissionDecision>` for the UI to respond
- [x] `PermissionBroker::invalidate()` removes a specific cache entry by plugin + capability
- [x] `PermissionDecision` enum has `Allow`, `AllowSession`, `Deny` variants
- [x] Fs capability key uses canonicalized path with prefix matching (granting `/home/user/project` covers `/home/user/project/src/main.rs`)
- [x] Net capability key uses host pattern matching (`*.openai.com` matches `api.openai.com`)
- [x] Secret capability key uses exact key match
- [x] All cache hits are logged at `tracing::trace` level
- [x] Thread-safe via `DashMap` — no `Mutex` on the hot path
- [x] Denials are NOT cached permanently — user can retry
- [ ] `Grant` has an optional TTL or session scope *(AllowSession variant exists; TTL deferred)*

## Acceptance criteria
```bash
# Compiles
cargo check -p rustacle-kernel

# Unit tests pass
cargo test -p rustacle-kernel -- permission

# No warnings
cargo clippy -p rustacle-kernel -- -D warnings
```

## Anti-patterns
- Do NOT bypass the broker for any capability check — no "trusted plugin" shortcut.
- Do NOT cache denials permanently — the user must be able to retry after changing their mind.
- Do NOT log secret values or capability keys containing sensitive data.
- Do NOT use `std::sync::Mutex` — use `DashMap` for lock-free concurrent access.
- Do NOT make the broker depend on UI crates — it communicates via channels only.
