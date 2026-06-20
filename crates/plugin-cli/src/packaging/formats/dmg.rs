//! macOS `.dmg` builder.
//!
//! Creates a read-only compressed disk image containing the staged payload
//! using `hdiutil` (the macOS built-in tool). If `hdiutil` is not available
//! (e.g. running on Linux), the builder falls back to a `.tar.gz` of the
//! `.app` bundle directory so the artifact is still useful.
//!
//! Required tooling: macOS only. On Linux/Windows CI this builder produces
//! the tar.gz fallback (callers should arrange for macOS-hosted packaging for
//! the real `.dmg`).

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
    let staging_root = staged.root.parent().context("staged root has no parent")?;
    let work = staging_root.join(format!(".dmg-build-{platform}"));
    if work.exists() {
        fs::remove_dir_all(&work).ok();
    }
    fs::create_dir_all(&work)?;

    let app_bundle = work.join(format!("{}.app", cfg.app.display_name));
    let contents = app_bundle.join("Contents");
    let macos_dir = contents.join("MacOS");
    let resources_dir = contents.join("Resources");
    fs::create_dir_all(&macos_dir)?;
    fs::create_dir_all(&resources_dir)?;

    // Info.plist
    let info_plist = contents.join("Info.plist");
    fs::write(&info_plist, build_info_plist(cfg, platform))?;

    // Binary
    let bin_name = &cfg.macos.binary_name;
    let bin_dest = macos_dir.join(bin_name);
    fs::copy(&staged.binary, &bin_dest)?;
    set_executable(&bin_dest)?;

    // Plugins and web alongside the binary
    copy_dir(&staged.plugins_dir, &macos_dir.join("plugins"))?;
    copy_dir(&staged.web_dir, &macos_dir.join("web"))?;

    // Icon
    if let Ok(icon_src) = staged.assets_dir.join("icon.icns").canonicalize() {
        if icon_src.exists() {
            fs::copy(&icon_src, resources_dir.join("AppIcon.icns"))?;
        }
    }

    let artifact = output_root.join(artifact_name(cfg, platform, &Format::Dmg));

    // Try hdiutil
    let dmg_status = Command::new("hdiutil")
        .args([
            "create",
            "-ov",
            "-volname",
            &cfg.macos.dmg.volume_name,
            "-fs",
            "HFS+",
            "-srcfolder",
            app_bundle.to_str().unwrap(),
            artifact.to_str().unwrap(),
        ])
        .status();

    match dmg_status {
        Ok(s) if s.success() => {
            fs::remove_dir_all(&work).ok();
            return Ok(vec![artifact]);
        }
        Ok(s) => {
            eprintln!(
                "  {} hdiutil exited with status {s}; falling back to .tar.gz",
                "warning:".yellow()
            );
        }
        Err(e) => {
            eprintln!(
                "  {} hdiutil not available ({e}); falling back to .tar.gz (run on macOS for a real .dmg)",
                "warning:".yellow()
            );
        }
    }

    // Fallback: tar.gz the .app bundle
    let fallback = output_root.join(format!("{}-{}.app.tar.gz", cfg.app.name, platform));
    let file = fs::File::create(&fallback)?;
    let gz = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut tar = tar::Builder::new(gz);
    tar.append_dir_all(format!("{}.app", cfg.app.display_name), &app_bundle)?;
    tar.into_inner()?.finish()?;

    fs::remove_dir_all(&work).ok();
    Ok(vec![fallback])
}

fn build_info_plist(cfg: &ResolvedConfig, platform: &str) -> String {
    let min_macos = if platform == "macos-arm64" {
        "11.0"
    } else {
        "10.15"
    };
    let _ = min_macos;
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>{name}</string>
    <key>CFBundleDisplayName</key>
    <string>{display}</string>
    <key>CFBundleIdentifier</key>
    <string>{bundle_id}</string>
    <key>CFBundleVersion</key>
    <string>{version}</string>
    <key>CFBundleShortVersionString</key>
    <string>{version}</string>
    <key>CFBundleExecutable</key>
    <string>{exe}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.15</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSHumanReadableCopyright</key>
    <string>{license}</string>
</dict>
</plist>
"#,
        name = cfg.app.name,
        display = cfg.app.display_name,
        bundle_id = cfg.macos.bundle_id,
        version = cfg.app.version,
        exe = cfg.macos.binary_name,
        license = cfg.app.license,
    )
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

use colored::Colorize;
