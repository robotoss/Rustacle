/// Configuration for the WASM host runtime.
#[derive(Debug, Clone)]
pub struct WasmHostConfig {
    /// Maximum fuel (instruction budget) per plugin instance.
    pub fuel_limit: u64,

    /// Maximum memory in bytes per plugin instance.
    pub memory_limit: usize,
}

impl Default for WasmHostConfig {
    fn default() -> Self {
        Self {
            fuel_limit: 10_000_000,         // 10M instructions
            memory_limit: 64 * 1024 * 1024, // 64 MiB
        }
    }
}
