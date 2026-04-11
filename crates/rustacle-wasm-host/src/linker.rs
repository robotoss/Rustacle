/// Host import linker stub.
///
/// In the full implementation, this module links WIT host functions
/// (publish, fs-read, fs-write, net-fetch, secret-get, llm-stream, llm-poll, log)
/// into the wasmtime `Linker<HostState>`.
///
/// Actual linking requires generated bindings from `wit-bindgen` and
/// a `HostState` struct carrying references to the permission broker,
/// event bus, and LLM router.
pub struct HostLinker;

impl HostLinker {
    /// Create a new host linker (stub).
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for HostLinker {
    fn default() -> Self {
        Self::new()
    }
}
