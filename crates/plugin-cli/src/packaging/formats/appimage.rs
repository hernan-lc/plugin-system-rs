//! AppImage builder.
//!
//! An AppImage is a regular ELF (or shell script) that mounts itself via FUSE
//! at runtime. The minimum recipe is:
//!   1. SquashFS root filesystem (built with `mksquashfs`)
//!   2. An `AppRun` executable that points at the real binary
//!   3. A `.desktop` file and icon
//!   4. An ELF runtime header (`runtime-x86_64` or `runtime-aarch64`)
//!
//! This builder generates 1–4 from the staged payload, then either concatenates
//! the runtime + squashfs (the AppImage spec) or, if `mksquashfs` and the
//! runtime are missing, produces a tar.gz that documents the AppDir so the
//! user knows what to do.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

use crate::packaging::config::ResolvedConfig;
use crate::packaging::format::Format;
use crate::packaging::stage::Staged;

use super::artifact_name;

const APPIMAGE_RUNTIMES: &[(&str, &str)] = &[
    (
        "x86_64",
        "https://github.com/AppImageCommunity/AppImageKit/releases/download/continuous/runtime-x86_64",
    ),
    (
        "aarch64",
        "https://github.com/AppImageCommunity/AppImageKit/releases/download/continuous/runtime-aarch64",
    ),
];

pub fn build(
    cfg: &ResolvedConfig,
    staged: &Staged,
    output_root: &Path,
    platform: &str,
) -> Result<Vec<PathBuf>> {
    let arch = match platform {
        "linux-x64" => "x86_64",
        "linux-arm64" => "aarch64",
        other => anyhow::bail!("appimage: unsupported platform {other}"),
    };
    let artifact = output_root.join(artifact_name(cfg, platform, &Format::AppImage));
    let staging_root = staged.root.parent().context("staged root has no parent")?;
    let appdir = staging_root.join(format!("AppDir-{arch}"));
    if appdir.exists() {
        fs::remove_dir_all(&appdir).ok();
    }
    fs::create_dir_all(&appdir)?;

    let usr = appdir.join("usr");
    fs::create_dir_all(&usr)?;

    let bin_dir = usr.join("bin");
    fs::create_dir_all(&bin_dir)?;
    let bin_name = staged
        .binary
        .file_name()
        .context("core binary has no filename")?;
    let bin_dest = bin_dir.join(bin_name);
    fs::copy(&staged.binary, &bin_dest)?;
    set_executable(&bin_dest)?;

    fs::create_dir_all(usr.join("lib").join("plugins"))?;
    fs::create_dir_all(usr.join("lib").join("web"))?;
    copy_dir(&staged.plugins_dir, &usr.join("lib").join("plugins"))?;
    copy_dir(&staged.web_dir, &usr.join("lib").join("web"))?;

    // AppRun
    let apprun = appdir.join("AppRun");
    let apprun_body = format!(
        "#!/bin/sh\n\
         set -e\n\
         HERE=\"$(dirname \"$(readlink -f \"$0\")\")\"\n\
         exec \"$HERE/usr/bin/{bin}\" \"$@\"\n",
        bin = bin_name.to_string_lossy()
    );
    fs::write(&apprun, apprun_body)?;
    set_executable(&apprun)?;

    // .desktop
    let desktop_dest = appdir.join(format!("{}.desktop", cfg.app.name));
    let desktop_body = build_desktop(cfg, bin_name.to_string_lossy().as_ref());
    fs::write(&desktop_dest, desktop_body)?;

    // icon
    if let Some(icon_src) = find_asset(&staged.assets_dir, &["icon.png", "icon.svg"]) {
        let ext = icon_src
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("png");
        let icon_dest = appdir.join(format!("{}.{}", cfg.app.name, ext));
        fs::copy(&icon_src, &icon_dest)?;
    }

    // Look for runtime + mksquashfs
    let runtime_path = match APPIMAGE_RUNTIMES.iter().find(|(a, _)| *a == arch) {
        Some((_, url)) => ensure_appimage_runtime(url, staging_root).ok(),
        None => None,
    };

    if let (Some(runtime), Ok(squashfs_path)) =
        (runtime_path, build_squashfs(&appdir, staging_root))
    {
        // Real AppImage: runtime + squashfs offset
        let mut f = fs::File::create(&artifact)?;
        let mut r = fs::File::open(&runtime)
            .with_context(|| format!("opening runtime {}", runtime.display()))?;
        std::io::copy(&mut r, &mut f)?;
        let mut s = fs::File::open(&squashfs_path)?;
        std::io::copy(&mut s, &mut f)?;
        // Set executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = f.metadata()?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&artifact, perms)?;
        }
        fs::remove_dir_all(&appdir).ok();
        fs::remove_file(&squashfs_path).ok();
        Ok(vec![artifact])
    } else {
        // Fallback: ship the AppDir as a tar.gz and instruct the user
        eprintln!(
            "  {} AppImage runtime or mksquashfs unavailable; producing AppDir .tar.gz fallback",
            "warning:".yellow()
        );
        let fallback = output_root.join(format!("{}-{}.AppDir.tar.gz", cfg.app.name, platform));
        let parent = fallback.parent().context("fallback has no parent")?;
        fs::create_dir_all(parent)?;
        let f = fs::File::create(&fallback)?;
        let gz = flate2::write::GzEncoder::new(f, flate2::Compression::default());
        let mut tar = tar::Builder::new(gz);
        tar.append_dir_all(format!("{}.AppDir", cfg.app.name), &appdir)?;
        tar.into_inner()?.finish()?;
        fs::remove_dir_all(&appdir).ok();
        Ok(vec![fallback])
    }
}

fn build_desktop(cfg: &ResolvedConfig, bin: &str) -> String {
    let categories = if cfg.app.categories.is_empty() {
        "Utility;".to_string()
    } else {
        format!("{};", cfg.app.categories.join(";"))
    };
    let keywords = if cfg.app.keywords.is_empty() {
        String::new()
    } else {
        format!("Keywords={}\n", cfg.app.keywords.join(";"))
    };
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name={name}\n\
         GenericName={desc}\n\
         Comment={desc}\n\
         Exec={bin} %U\n\
         Icon={name}\n\
         Terminal=false\n\
         Categories={categories}\n\
         {keywords}",
        name = cfg.app.display_name,
        desc = cfg.app.description,
        bin = bin,
        categories = categories,
        keywords = keywords,
    )
}

fn find_asset(assets_dir: &Path, candidates: &[&str]) -> Option<PathBuf> {
    for c in candidates {
        let p = assets_dir.join(c);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

fn build_squashfs(appdir: &Path, work: &Path) -> Result<PathBuf> {
    let out = work.join("payload.squashfs");
    let status = Command::new("mksquashfs")
        .args([
            appdir.to_str().context("appdir is not UTF-8")?,
            out.to_str().context("squashfs out is not UTF-8")?,
            "-noappend",
            "-comp",
            "xz",
            "-no-xattrs",
        ])
        .status();
    match status {
        Ok(s) if s.success() => Ok(out),
        Ok(s) => anyhow::bail!("mksquashfs exited with status {s}"),
        Err(_) => anyhow::bail!("mksquashfs not available"),
    }
}

fn ensure_appimage_runtime(url: &str, work: &Path) -> Result<PathBuf> {
    let filename = url.rsplit('/').next().context("runtime url has no file")?;
    let target = work.join(filename);
    if target.exists() {
        return Ok(target);
    }
    // Try wget / curl
    let status = Command::new("curl")
        .args(["-fsSL", "-o", target.to_str().unwrap(), url])
        .status();
    if let Ok(s) = status {
        if s.success() {
            return Ok(target);
        }
    }
    anyhow::bail!("could not download AppImage runtime from {url}")
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

// silence the `colored::Colorize::yellow` warning
use colored::Colorize;
