pub mod cabi;
pub mod context;
pub mod error;
pub mod handler;
pub mod loader;
pub mod macros;
pub mod manager;
pub mod manifest;
pub mod platform;
pub mod plugin_info;
pub mod registry;
pub mod traits;

pub use context::PluginContext;
pub use error::{PluginError, Result};
pub use handler::{
    new_shared_command_registry, CommandHandler, CommandRegistry, PluginCommandHandler,
    SharedCommandRegistry,
};
#[cfg(feature = "url-loader")]
pub use loader::UrlLoader;
pub use loader::{FileLoader, MultiLoader, PluginLoader};
pub use manager::PluginManager;
pub use platform::{
    copy_cargo_plugin, copy_plugin, library_extension, library_filename, library_path,
};
pub use plugin_info::{PluginInfo, PluginResult};
pub use registry::{new_shared_registry, PluginRegistry, SharedRegistry};
pub use serde_json;
pub use traits::{command_to_json, CommandResult, Plugin, PluginDependency, PluginMetadata};

pub use cabi::{is_cabi_manifest, CAbiManifest, CAbiPlugin};

pub use plugin_macros::{command, define_plugin, plugin_export, plugin_interface};
