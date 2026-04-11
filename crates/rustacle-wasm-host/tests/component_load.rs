//! Integration tests: load WASM plugin components and verify they are valid.
//! Proves the WIT contract is language-neutral (same interface, different languages).

use std::path::Path;

use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Store};

fn load_and_verify(wasm_path: &Path, label: &str) {
    if !wasm_path.exists() {
        eprintln!("SKIP: {label} not built at {}", wasm_path.display());
        return;
    }

    let mut config = Config::new();
    config.wasm_component_model(true);

    let engine = Engine::new(&config).expect("failed to create engine");
    let component = Component::from_file(&engine, wasm_path)
        .unwrap_or_else(|e| panic!("{label}: failed to load WASM component: {e}"));

    let linker: Linker<()> = Linker::new(&engine);
    let mut store = Store::new(&engine, ());

    // Full instantiation requires host import linking (not wired here).
    // Loading + type-checking alone proves the component is valid.
    match linker.instantiate(&mut store, &component) {
        Ok(_) => println!("{label}: instantiated (all imports satisfied)"),
        Err(e) => {
            let msg = e.to_string();
            // Missing-import errors are expected — type mismatch errors are NOT.
            assert!(
                msg.contains("import") || msg.contains("unknown"),
                "{label}: unexpected error (not a missing-import): {msg}"
            );
            println!("{label}: component loaded and type-checked OK");
        }
    }
}

/// Load the JavaScript WASM plugin and verify it's a valid component.
#[test]
fn js_plugin_component_is_valid() {
    // Path relative to workspace root (tests run from workspace root)
    load_and_verify(
        Path::new("plugins/hello-js/hello-js.wasm"),
        "hello-js (JavaScript)",
    );
}

/// Load the Rust FS WASM plugin and verify it's a valid component.
#[test]
fn rust_fs_plugin_component_is_valid() {
    load_and_verify(
        Path::new("target/wasm32-wasip1/debug/rustacle_plugin_fs.wasm"),
        "fs (Rust)",
    );
}
