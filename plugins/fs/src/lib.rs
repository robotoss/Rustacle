#[allow(warnings)]
mod bindings;

use bindings::exports::rustacle::plugin::module::Guest;
use bindings::rustacle::plugin::host;
use bindings::rustacle::plugin::types::{
    Capability, FsMode, FsScope, ModuleError, ModuleManifest, PaletteEntry, PanelDesc,
    UiContributions,
};

mod commands;

struct FsPlugin;

impl Guest for FsPlugin {
    fn manifest() -> ModuleManifest {
        ModuleManifest {
            id: "rustacle.fs".to_string(),
            version: "0.1.0".to_string(),
            capabilities: vec![Capability::Fs(FsScope {
                paths: vec![],
                mode: FsMode::Read,
            })],
            subscriptions: vec![],
            ui_contributions: UiContributions {
                panels: vec![PanelDesc {
                    id: "file-tree".to_string(),
                    title: "Files".to_string(),
                    icon: "folder".to_string(),
                }],
                palette_entries: vec![PaletteEntry {
                    id: "fs.open".to_string(),
                    label: "Open File".to_string(),
                    shortcut: String::new(),
                }],
                settings_schema: String::new(),
            },
        }
    }

    fn init() -> Result<(), ModuleError> {
        host::log("info", "fs plugin initialized", &[]);
        Ok(())
    }

    fn on_event(_topic: String, _payload: Vec<u8>) -> Result<(), ModuleError> {
        Ok(())
    }

    fn shutdown() -> Result<(), ModuleError> {
        host::log("info", "fs plugin shutting down", &[]);
        Ok(())
    }

    fn call(command: String, payload: Vec<u8>) -> Result<Vec<u8>, ModuleError> {
        match command.as_str() {
            "read_file" => commands::read_file(&payload),
            "list_dir" => commands::list_dir(&payload),
            "stat" => commands::stat(&payload),
            _ => Err(ModuleError::InvalidInput(format!(
                "unknown command: {command}"
            ))),
        }
    }
}

bindings::export!(FsPlugin with_types_in bindings);
