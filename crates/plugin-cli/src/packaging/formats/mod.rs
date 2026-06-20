//! Format-specific artifact builders.
//!
//! Every builder returns a list of produced artifact paths (typically one, but
//! NSIS can produce both 32/64-bit, etc.).

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};

use super::config::ResolvedConfig;
use super::format::Format;
use super::stage::Staged;

pub mod appimage;
pub mod deb;
pub mod dmg;
pub mod msi;
pub mod nsis;
pub mod pkg;
pub mod rpm;
pub mod tarball;

pub fn build(
    fmt: &Format,
    cfg: &ResolvedConfig,
    staged: &Staged,
    output_root: &Path,
    platform: &str,
) -> Result<Vec<PathBuf>> {
    if let Some(supported) = fmt.supported_on() {
        if !supported.contains(&platform) {
            return Err(anyhow!(
                "format {} is not supported on platform {} (supports {:?})",
                fmt.id(),
                platform,
                supported
            ));
        }
    }

    match fmt {
        Format::TarGz => tarball::build_tar_gz(cfg, staged, output_root, platform),
        Format::Zip => tarball::build_zip(cfg, staged, output_root, platform),
        Format::Deb => deb::build(cfg, staged, output_root, platform),
        Format::Rpm => rpm::build(cfg, staged, output_root, platform),
        Format::AppImage => appimage::build(cfg, staged, output_root, platform),
        Format::Msi => msi::build(cfg, staged, output_root, platform),
        Format::Nsis => nsis::build(cfg, staged, output_root, platform),
        Format::Dmg => dmg::build(cfg, staged, output_root, platform),
        Format::Pkg => pkg::build(cfg, staged, output_root, platform),
    }
}

/// Return the canonical artifact filename for a (platform, format) pair
/// (without the `releases/<version>/<platform>/` prefix).
pub fn artifact_name(cfg: &ResolvedConfig, platform: &str, fmt: &Format) -> String {
    format!("{}-{}.{}", cfg.app.name, platform, fmt.extension())
}
