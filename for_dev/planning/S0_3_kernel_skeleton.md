# S0.3 — Kernel Skeleton

## Goal
`rustacle-kernel` gets a real `Kernel` struct with `start()` and `stop()`, `AppState`, and structured logging via `tracing`. The kernel logs its lifecycle on application startup.

## Context
The kernel is the micro-kernel core. At this stage it knows nothing about plugins — only lifecycle and tracing setup. This is the foundation for everything else.

## Docs to read
- `for_dev/architecture.md` §1, §5, §6 — system shape, event bus concept, state model.
- `for_dev/project_structure.md` §`rustacle-kernel` — files: `kernel.rs`, `state.rs`, `lifecycle.rs`, `bus/`, `errors.rs`.
- `for_dev/knowledge_base.md` §1.2 (task ownership), §3.1 (logging rules).
- `for_dev/observability.md` §2 (tracing rules, levels, span taxonomy).
- `for_dev/glossary.md` — AppState, Kernel, Event Bus.

## Reference code
- `refs/claw-code/rust/` — Rust project structure for an agent system.
- Internet: [`tracing` crate docs](https://docs.rs/tracing/latest/tracing/), [`tokio::sync` docs](https://docs.rs/tokio/latest/tokio/sync/index.html), [`tokio_util::sync::CancellationToken`](https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html).

## Deliverables

### `crates/rustacle-kernel/Cargo.toml` — add deps:
```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
tokio-util = "0.7"  # CancellationToken
thiserror = "2"
```

### `crates/rustacle-kernel/src/`
```rust
// kernel.rs
pub struct Kernel {
    pub shutdown: CancellationToken,
    tasks: JoinSet<()>,
}

impl Kernel {
    pub fn new() -> Self { ... }
    pub async fn start(&mut self) -> Result<(), KernelError> {
        tracing::info!("kernel starting");
        // Future: discover + load plugins here
        tracing::info!("kernel started");
        Ok(())
    }
    pub async fn stop(&mut self) -> Result<(), KernelError> {
        tracing::info!("kernel stopping");
        self.shutdown.cancel();
        while self.tasks.join_next().await.is_some() {}
        tracing::info!("kernel stopped");
        Ok(())
    }
}
```

```rust
// state.rs
pub struct AppState {
    pub kernel: Arc<Kernel>,
    // Future: settings, bus, llm, permission
}
```

```rust
// lifecycle.rs
pub fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("RUSTACLE_LOG")
                .unwrap_or_else(|_| EnvFilter::new("info"))
        )
        .with_target(true)
        .with_span_events(FmtSpan::CLOSE)
        .init();
}
```

```rust
// errors.rs
#[derive(thiserror::Error, Debug)]
pub enum KernelError {
    #[error("lifecycle: {0}")]
    Lifecycle(String),
    #[error("internal: {0}")]
    Internal(String),
}
```

```rust
// bus/mod.rs — stub
pub struct Bus;
impl Bus { pub fn new() -> Self { Bus } }
```

### Integration in `rustacle-app/src/main.rs`
```rust
fn main() {
    rustacle_kernel::lifecycle::init_tracing();
    
    tauri::Builder::default()
        .setup(|app| {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                let mut kernel = Kernel::new();
                kernel.start().await?;
                // store AppState in tauri managed state
                app.manage(AppState { kernel: Arc::new(kernel) });
                Ok(())
            })
        })
        .run(tauri::generate_context!())
        .expect("error while running rustacle");
}
```

## Checklist
- [ ] `Kernel::new()` creates an instance with `CancellationToken` and `JoinSet`.
- [ ] `Kernel::start()` logs `info` "kernel starting" and "kernel started".
- [ ] `Kernel::stop()` calls `shutdown.cancel()`, awaits JoinSet.
- [ ] `AppState` is created and managed via Tauri.
- [ ] `init_tracing()` configures `tracing-subscriber` with env filter.
- [ ] `RUSTACLE_LOG=debug cargo run -p rustacle-app` shows debug-level output.
- [ ] `KernelError` is defined with `thiserror`.
- [ ] `Bus` stub exists (empty).
- [ ] All spans use structured fields (no string interpolation).
- [ ] `cargo test -p rustacle-kernel` — at least 1 test: `kernel_start_stop`.

## Acceptance criteria
```bash
cargo run -p rustacle-app 2>&1 | grep "kernel started"  # prints info line
cargo test -p rustacle-kernel  # kernel_start_stop passes
```

## Anti-patterns
- Do NOT implement the Bus with real logic — stub only.
- Do NOT add the plugin registry — that is S2.
- Do NOT use `println!` — use `tracing` only.
- Do NOT use `lazy_static` for mutable data.
- Do NOT make `Kernel` `Clone` — it is `Arc`-wrapped in `AppState`.
