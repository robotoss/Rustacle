pub mod bus;
pub mod demo_plugin;
pub mod errors;
pub mod kernel;
pub mod lifecycle;
pub mod permission;
pub mod registry;
pub mod state;

pub use kernel::Kernel;
pub use registry::PluginRegistry;
pub use state::AppState;
