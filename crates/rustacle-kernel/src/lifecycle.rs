use tracing_subscriber::{EnvFilter, fmt::format::FmtSpan};

/// Initialize the tracing subscriber with env-filter support.
///
/// Set `RUSTACLE_LOG=debug` (or `trace`, `warn`, etc.) to control log level.
pub fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("RUSTACLE_LOG").unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(true)
        .with_span_events(FmtSpan::CLOSE)
        .init();
}
