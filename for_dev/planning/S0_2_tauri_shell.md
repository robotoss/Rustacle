# S0.2 — Tauri v2 Shell

## Goal
Turn `rustacle-app` into a Tauri v2 application. `cargo run -p rustacle-app` opens a native window with a placeholder HTML page.

## Context
Tauri v2 is the project's desktop shell. It handles the window, webview, IPC bridge, bundler, and updater. At this stage — only a window with a placeholder; IPC comes in S1.

## Docs to read
- `for_dev/tech_stack_2026.md` §1 — Tauri v2, Vite.
- `for_dev/project_structure.md` §`rustacle-app` — files: `main.rs`, `setup.rs`, `tauri.conf.json`.
- `for_dev/cross_platform.md` §9 — native menu on macOS.

## Reference code
- `refs/acc/acc-app/src-tauri/src/main.rs` — minimal Tauri entrypoint.
- `refs/acc/acc-app/src-tauri/tauri.conf.json` — Tauri config.
- Internet: [Tauri v2 Quick Start](https://v2.tauri.app/start/create-project/), [tauri::Builder docs](https://docs.rs/tauri/latest/tauri/struct.Builder.html).

## Deliverables

### `crates/rustacle-app/`
```
crates/rustacle-app/
├── Cargo.toml          # deps: tauri (v2), tauri-build
├── tauri.conf.json     # app name, window config, identifier
├── build.rs            # tauri_build::build()
├── icons/              # placeholder icon (Tauri default)
└── src/
    ├── main.rs         # tauri::Builder::default().run()
    └── setup.rs        # on_setup hook (empty, for S0_3)
```

### `ui/` (minimal placeholder)
```
ui/
├── index.html          # <div id="app">Rustacle loading...</div>
├── package.json        # name: rustacle-ui, scripts: { dev, build }
├── vite.config.ts      # minimal Vite config
└── src/
    └── main.ts         # document.getElementById("app").innerHTML = "Rustacle v0.0.1"
```

### `tauri.conf.json` key fields
```json
{
  "productName": "Rustacle",
  "identifier": "dev.rustacle.app",
  "build": {
    "devUrl": "http://localhost:1420",
    "frontendDist": "../ui/dist"
  },
  "app": {
    "windows": [{
      "title": "Rustacle",
      "width": 1200,
      "height": 800,
      "resizable": true
    }]
  }
}
```

## Checklist
- [x] `Cargo.toml` for `rustacle-app` adds `tauri` v2 as a dependency.
- [x] `build.rs` calls `tauri_build::build()`.
- [x] `tauri.conf.json` created with correct identifier and window config.
- [x] `ui/index.html` exists with placeholder content.
- [x] `ui/package.json` + `vite.config.ts` are configured.
- [x] `npm install` (or `pnpm install`) in `ui/` passes.
- [x] `cargo run -p rustacle-app` opens a window on all three OSes.
- [x] Window shows placeholder text.
- [x] Window is resizable, with correct title "Rustacle".
- [ ] macOS: native menu (File, Edit, Window, Help) works. *(Not verified — no macOS available)*
- [x] Windows: window has standard title bar.
- [x] Closing the window terminates the process.

## Acceptance criteria
```bash
cd ui && npm install && npm run build  # exit 0, creates ui/dist/
cargo run -p rustacle-app  # opens a window showing "Rustacle v0.0.1"
```

## Anti-patterns
- Do NOT choose the UI framework here (Solid/React is decided in S0_5).
- Do NOT add IPC commands — that is S1.
- Do NOT configure the updater/bundler — that is S8.
- Do NOT create a complex HTML structure — placeholder only.
- Do NOT add Tailwind yet — that is S0_5.
