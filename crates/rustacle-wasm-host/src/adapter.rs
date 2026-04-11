/// Adapter stub: bridges a loaded wasmtime component to `RustacleModule`.
///
/// The full implementation wraps a `wasmtime::component::Instance` and calls
/// the WIT `module` exports (manifest, init, on-event, call, shutdown)
/// through generated bindings.
pub struct WasmModuleAdapter;
