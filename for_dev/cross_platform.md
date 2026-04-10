# Cross-Platform Reality — Windows, Linux, macOS

> The things that differ between OSes and how Rustacle handles each. Read before you write anything that touches the filesystem, a process, a path, a signal, or a keyring.

---

## 1. Target matrix

| OS | Minimum version | Bundle | PTY backend | Keyring backend |
|---|---|---|---|---|
| Windows | 10 1809 (ConPTY required) | `.msi` | ConPTY via `portable-pty` | Windows Credential Manager |
| macOS | 12 (Monterey) | signed + notarized `.dmg` | `openpty` via `portable-pty` | macOS Keychain |
| Linux (glibc) | Ubuntu 22.04+, Fedora 38+ | `.AppImage` + `.deb` + `.rpm` | `openpty` | Secret Service (libsecret) |
| Linux (musl) | stretch goal | static `.AppImage` | `openpty` | Secret Service, with fallback to an encrypted file prompt |

CI runs all three OSes on every PR. No "works on my machine" — if a PR breaks the matrix, it does not merge.

---

## 2. Paths

### 2.1 Canonicalization

Path canonicalization differs subtly between OSes (drive letters, `\\?\` prefix on Windows, case insensitivity on macOS by default). Rustacle uses one helper:

```rust
// crates/rustacle-kernel/src/fs/canonical.rs
pub fn canonicalize_strict(p: &Path) -> Result<PathBuf, FsError> {
    let canon = dunce::canonicalize(p)?;        // on Windows strips \\?\ when safe
    #[cfg(target_os = "macos")]
    { /* case-fold comparison for scope checks, but keep original for display */ }
    Ok(canon)
}
```

**Rule**: every FS scope check calls `canonicalize_strict`, compares via `is_path_prefix_with_boundary` (see `knowledge_base.md` §4.2), and keeps both a canonical and a display form.

### 2.2 Separators

- Internal: always `/` (the `Path` / `PathBuf` abstraction handles it).
- Display: OS-native in the UI (`\` on Windows), via `path.display()` and a small display helper.
- Never manually concatenate path strings — always `.join()`.

### 2.3 App data dirs

| OS | Path | Set by |
|---|---|---|
| Windows | `%APPDATA%\Rustacle\` | `dirs::config_dir()` |
| macOS | `~/Library/Application Support/Rustacle/` | `dirs::config_dir()` |
| Linux | `$XDG_CONFIG_HOME/rustacle/` (fallback `~/.config/rustacle/`) | `dirs::config_dir()` |

Subdirs (all OSes): `db/`, `blobs/`, `logs/`, `plugins/`, `cache/`.

---

## 3. Shells & PTY

### 3.1 Default shell selection

| OS | Default | Override |
|---|---|---|
| Windows | `pwsh.exe` if present, else `powershell.exe`, else `cmd.exe` | Settings UI per tab |
| macOS | `$SHELL` env (usually `/bin/zsh`) | Settings UI |
| Linux | `$SHELL` env (usually `/bin/bash`) | Settings UI |

### 3.2 PTY backends

`portable-pty` handles the abstraction:

- **Windows**: ConPTY (`CreatePseudoConsole`). Legacy WinPTY is disabled — we require Windows 10 1809+. Resize via `ResizePseudoConsole`; close via `ClosePseudoConsole`.
- **Linux / macOS**: `openpty(3)` + `forkpty`-style. Resize via `ioctl(TIOCSWINSZ)`. Signals via `kill(pid, SIG)`.

### 3.3 Signals and cancellation

- Linux / macOS: `SIGTERM` grace 3 s → `SIGKILL`.
- Windows: `GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT)` → grace 3 s → `TerminateProcess`. ConPTY swallows `Ctrl+C` differently; we use `CTRL_BREAK_EVENT` because it propagates reliably to the console process group.

### 3.4 Line endings

Terminal output is passed through as bytes; UI (XTerm.js) handles the wire form. Files written by `fs_write` preserve the existing file's line endings on overwrite; new files default to `\n` (even on Windows) unless the user toggles "native line endings" in Settings (default off, to keep git histories clean).

---

## 4. Processes & commands

### 4.1 Spawn

Always via `plugins/terminal`; never from the agent plugin. See [`security.md` §3](./security.md).

### 4.2 Environment

Whitelist of env vars inherited by spawned shells:

```
PATH  HOME  TERM  LANG  LC_*  USER  SHELL  PWD  HOSTNAME
```

Plus per-OS:
- Windows: `SYSTEMROOT`, `USERPROFILE`, `APPDATA`, `LOCALAPPDATA`, `TEMP`, `TMP`, `COMSPEC`.
- macOS: `TMPDIR`.
- Linux: `XDG_*`.

Explicitly stripped: anything matching `*_TOKEN`, `*_KEY`, `*_SECRET`, `*_PASSWORD`, `*_API*`, `AWS_*`, `GOOGLE_APPLICATION_*`, `OPENAI_*`, `ANTHROPIC_*`, or present in the user's keyring (by env-var-name field, if any).

### 4.3 Command resolution

The `bash` tool does **not** shell-interpolate user input. Arguments pass through the shell as-is (the shell itself is the quoting engine). The Rust validator refuses input containing null bytes and enforces length caps.

---

## 5. Filesystem watches

Live project-doc injection, git status badges, and file-change events go through a single watcher in `rustacle-kernel/src/fs/watcher.rs`:

- **Linux**: `inotify` via the `notify` crate.
- **macOS**: `FSEvents` via `notify`.
- **Windows**: `ReadDirectoryChangesW` via `notify`.

Watchers are registered per user-granted scope; a revoked scope unsubscribes its watcher immediately.

---

## 6. Keyring / secret storage

| OS | Backend | Fallback |
|---|---|---|
| Windows | Credential Manager (`wincred`) via `keyring` crate | none — Credential Manager is always available on supported versions |
| macOS | Keychain via `keyring` crate | none |
| Linux | Secret Service (libsecret) via `keyring` crate | if unavailable: show an in-app dialog with install instructions **and** offer an encrypted file fallback (AES-GCM, key derived from OS user id + a required UI passphrase). The fallback is opt-in and warned about. |

Every access is audited in the `secret_audit` table.

---

## 7. Networking

### 7.1 Providers

All providers use `reqwest` with rustls. TLS roots:

- Windows: `rustls-native-certs` reads the system store.
- macOS: same.
- Linux: same + fallback to `webpki-roots` if system store missing or unparseable.

### 7.2 Loopback for local models

`http://localhost:*` is allowed without a `Net` capability for the built-in local provider auto-discovery (Ollama default port 11434, LM Studio 1234, …). This is the **only** loopback exception; all other network destinations require a `Net { hosts }` capability.

### 7.3 IPv6

Providers and loopback probes try IPv6 first on macOS and Linux (standard `reqwest` behavior), IPv4 first on Windows (due to common misconfiguration).

---

## 8. Fonts & rendering

- **Windows**: default UI font is `Segoe UI Variable`, terminal font is `Cascadia Code`.
- **macOS**: UI `SF Pro`, terminal `SF Mono`.
- **Linux**: UI `Inter` (bundled) fallback to system sans; terminal `JetBrains Mono` (bundled).
- Fonts are bundled for Linux because distros vary wildly. Windows/macOS ship with the above by default.

---

## 9. Window management & menus

- **Native menu bar** on macOS (required by HIG). Implemented via `tauri::Menu`.
- **Window menu inside the UI** on Windows and Linux (cross-platform).
- Title bar customization on Windows 11 and macOS 13+ matches the UI theme; on older Windows we use the default title bar.

---

## 10. Packaging & distribution

| OS | Format | Signing | Updater |
|---|---|---|---|
| Windows | `.msi` (WiX) + `.exe` (NSIS, optional) | Authenticode via SignTool; certificate in CI secrets | Tauri updater, signed manifest |
| macOS | `.dmg` + `.app` inside | `codesign` + `notarytool` | Tauri updater, signed manifest |
| Linux | `.AppImage` + `.deb` + `.rpm` | `gpg` sig of the artifact | Tauri updater for AppImage; distro package managers for deb/rpm |

CI pipeline (GitHub Actions) produces all targets per tag. Reproducible builds are a sprint-8 stretch goal.

---

## 11. Known platform gotchas (tracked list)

- **Windows long paths**: enabled by manifest (`longPathAware`). We still accept short paths and normalize internally.
- **Windows antivirus**: some AV products flag unsigned dev builds; production builds are Authenticode-signed.
- **macOS Gatekeeper**: requires notarization; CI runs `xcrun notarytool`.
- **Linux Wayland vs X11**: Tauri webview uses the native WebKit on Linux — we require WebKitGTK 2.40+. X11 sessions work; Wayland sessions work; XWayland works.
- **Linux clipboard**: clipboard access is Wayland-restricted; Tauri's clipboard API handles both backends.
- **Windows case-insensitive FS**: scope checks are case-insensitive on Windows, case-sensitive on Linux, case-insensitive-but-preserving on macOS. Handled in `canonicalize_strict` + scope compare.

---

## 12. Per-OS test jobs

CI runs this matrix per PR:

```yaml
jobs:
  windows: { os: windows-2022 }
  macos:   { os: macos-14 }
  linux:   { os: ubuntu-22.04 }
```

Each runs: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo nextest run --workspace`, `cargo deny check`, `bindings-regen-diff` (fails if `bindings.ts` is stale), plus a smoke e2e via `tauri-driver`.

Platform-specific tests gated on `cfg(target_os = "...")` for things like keyring and PTY.

---
*Related: [README](./README.md) · [security](./security.md) · [architecture](./architecture.md) · [tech_stack_2026](./tech_stack_2026.md) · [mcp_and_models](./mcp_and_models.md)*
