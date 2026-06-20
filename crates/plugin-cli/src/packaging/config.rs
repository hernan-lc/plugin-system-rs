//! Re-exports for the resolved config types loaded from `packaging.toml`.

#[allow(unused_imports)]
pub use super::manifest::{
    load, resolve, AppImageConfig, DebConfig, DmgConfig, MsiConfig, NsisConfig, PackagingConfig,
    PkgConfig, ResolvedApp, ResolvedConfig, ResolvedFormats, ResolvedLinux, ResolvedMacos,
    ResolvedWindows, RpmConfig,
};
