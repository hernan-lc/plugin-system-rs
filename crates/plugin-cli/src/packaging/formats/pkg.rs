//! macOS `.pkg` installer builder (shell-out to `pkgbuild`).
//!
//! Requires macOS with command-line developer tools installed. On other
//! platforms this builder is a no-op that returns an explanatory error so
//! the pipeline keeps going.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

use crate::packaging::config::ResolvedConfig;
use crate::packaging::format::Format;
use crate::packaging::stage::Staged;

use super::artifact_name;

pub fn build(
    cfg: &ResolvedConfig,
    staged: &Staged,
    output_root: &Path,
    platform: &str,
) -> Result<Vec<PathBuf>> {
    let artifact = output_root.join(artifact_name(cfg, platform, &Format::Pkg));
    let staging_root = staged.root.parent().context("staged root has no parent")?;
    let work = staging_root.join(format!(".pkg-build-{platform}"));
    if work.exists() {
        fs::remove_dir_all(&work).ok();
    }
    fs::create_dir_all(&work)?;

    let payload_root = work.join("payload");
    fs::create_dir_all(&payload_root)?;
    let install_dir = payload_root
        .join(cfg.macos.pkg.install_location.trim_start_matches('/'))
        .join(&cfg.app.display_name);
    fs::create_dir_all(&install_dir)?;
    let bin_name = staged.binary.file_name().context("core binary name")?;
    fs::copy(&staged.binary, install_dir.join(bin_name))?;
    set_executable(&install_dir.join(bin_name))?;
    copy_dir(&staged.plugins_dir, &install_dir.join("plugins"))?;
    copy_dir(&staged.web_dir, &install_dir.join("web"))?;

    let component_pkg = work.join("component.pkg");
    let status = Command::new("pkgbuild")
        .args([
            "--root",
            payload_root.to_str().unwrap(),
            "--identifier",
            &cfg.macos.pkg.identifier,
            "--version",
            &cfg.app.version,
            "--install-location",
            &cfg.macos.pkg.install_location,
            component_pkg.to_str().unwrap(),
        ])
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => anyhow::bail!("pkgbuild exited with status {s}"),
        Err(e) => anyhow::bail!(
            "pkgbuild not available ({e}); install Xcode command-line tools and retry"
        ),
    }

    fs::copy(&component_pkg, &artifact).with_context(|| {
        format!(
            "copying {} to {}",
            component_pkg.display(),
            artifact.display()
        )
    })?;
    fs::remove_dir_all(&work).ok();
    Ok(vec![artifact])
}

fn set_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms)?;
    }
    let _ = path;
    Ok(())
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in walkdir::WalkDir::new(src)
        .into_iter()
        .filter_map(Result::ok)
    {
        let rel = entry.path().strip_prefix(src).unwrap();
        let target = dst.join(rel);
        if entry.path().is_dir() {
            fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}
