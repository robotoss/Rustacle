# S1.3 — UI Round-Trip (Ping/Pong IPC Proof)

## Goal
Create a UI "Ping" button that calls a typed Tauri command and displays the response, proving end-to-end type-safe IPC works from UI to Rust and back.

## Context
This is the first real IPC round-trip. It validates that specta-generated types work across the full path: UI component imports from `bindings.ts`, calls a Tauri command, Rust handles it, and the response renders in the UI. The `ping` command lives in `rustacle-ipc`, is registered in `rustacle-app`, and is called from a UI component using the generated `bindings.ts`.

## Docs to read
- `for_dev/architecture.md` section 3.2 — example command flow (request/response lifecycle).
- `for_dev/ui_ux_manifesto.md` section 1 — minimal UI principles.
- `for_dev/project_structure.md` section `ui/` — component directory conventions.

## Reference code
- `refs/acc/acc-app/` — Tauri command registration patterns (how commands are added to the app builder).
- Internet:
  - Tauri v2 command invocation from frontend (`@tauri-apps/api`)
  - Tauri v2 `tauri::command` macro usage
  - `tauri-specta` command registration with the Tauri Builder

## Deliverables

### Rust side
```
crates/rustacle-ipc/src/commands/
└── system.rs               # new module: ping() -> PingResponse, version() -> String

crates/rustacle-app/src/
└── main.rs                 # register ping and version commands with tauri-specta Builder
```

#### `ping` command
- Input: none
- Output: `PingResponse { message: String, timestamp: u64 }` (epoch millis)
- Returns `"pong"` and current timestamp

#### `version` command
- Input: none
- Output: `String`
- Returns the value from `env!("CARGO_PKG_VERSION")`

### UI side
```
ui/src/
├── components/
│   └── common/
│       └── PingButton.tsx  # button that calls ping(), displays result
└── App.tsx                 # updated to include PingButton
```

#### `PingButton` component
- Renders a button labeled "Ping"
- On click, calls `ping()` imported from `bindings.ts`
- Displays the returned message and formatted timestamp below the button
- Shows a loading state while the command is in flight
- Handles and displays errors using `RustacleError` type from bindings

### Event stub
- Wire a `log_subscribe` event stub in `rustacle-ipc/src/events/` (type only, no handler logic yet)

### Updated bindings
- `ui/bindings.ts` regenerated to include `ping`, `version`, `PingResponse`, and `log_subscribe` event type

## Checklist
- [ ] Clicking Ping button shows "pong" and a human-readable timestamp
- [ ] `version` command returns the app version string from `Cargo.toml`
- [ ] All types are imported from `bindings.ts`, never hand-written
- [ ] Error cases render using `RustacleError` type (test by temporarily returning an error)
- [ ] Browser console shows no TypeScript type errors
- [ ] `cargo clippy -p rustacle-ipc -p rustacle-app -- -D warnings` passes
- [ ] `cd ui && npx tsc --noEmit` passes with no type errors
- [ ] Works on Windows, macOS, and Linux

## Acceptance criteria
```bash
# Rust compiles
cargo check -p rustacle-ipc -p rustacle-app

# Bindings include new commands
grep "ping" ui/bindings.ts && echo "PASS: ping in bindings"
grep "version" ui/bindings.ts && echo "PASS: version in bindings"
grep "PingResponse" ui/bindings.ts && echo "PASS: PingResponse in bindings"

# UI compiles
cd ui && npm run build

# TypeScript type check
cd ui && npx tsc --noEmit

# App launches (manual verification: click Ping, see "pong" + timestamp)
cargo run -p rustacle-app
```

## Anti-patterns
- Do NOT add complex UI beyond a button and text display — this is a proof of concept.
- Do NOT bypass `bindings.ts` by hand-writing invoke calls or types.
- Do NOT add state management libraries (Redux, Zustand, etc.) — use local component state only.
- Do NOT implement real business logic in the ping handler.
- Do NOT add routing or navigation — single page with the ping button is sufficient.
- Do NOT create additional Tauri windows or system tray functionality.
