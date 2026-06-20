//! Stage a release directory with the binary, plugins, web assets and platform
//! assets ready to be fed into a format builder.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use colored::Colorize;
use walkdir::WalkDir;

use super::config::ResolvedConfig;
use super::format::platform_from_target;

/// What was staged and where it lives on disk.
pub struct Staged {
    pub root: PathBuf,
    pub binary: PathBuf,
    pub plugins_dir: PathBuf,
    pub web_dir: PathBuf,
    pub assets_dir: PathBuf,
}

/// Stage a release for the given platform. The staging directory is created at
/// `target/packaging/<platform>/stage` and follows a stable layout:
///
/// ```text
/// stage/
///   sd-core[.exe]                  <- the core binary
///   plugins/
///     libplugin_*.so|.dll|.dylib  <- plugin shared libraries
///   web/                           <- web frontend assets
///   assets/                        <- platform-specific assets (.desktop, icons, …)
///   README.md, LICENSE             <- top-level docs
/// ```
pub fn stage_release(
    workspace_root: &Path,
    cfg: &ResolvedConfig,
    platform: &str,
    source_target: Option<&str>,
) -> Result<Staged> {
    let stage_dir = workspace_root
        .join("target")
        .join("packaging")
        .join(platform)
        .join("stage");
    if stage_dir.exists() {
        std::fs::remove_dir_all(&stage_dir)
            .with_context(|| format!("removing stale stage dir {}", stage_dir.display()))?;
    }
    std::fs::create_dir_all(&stage_dir)
        .with_context(|| format!("creating stage dir {}", stage_dir.display()))?;

    let plugins_dir = stage_dir.join("plugins");
    let web_dir = stage_dir.join("web");
    let assets_dir = stage_dir.join("assets");
    std::fs::create_dir_all(&plugins_dir)?;
    std::fs::create_dir_all(&web_dir)?;
    std::fs::create_dir_all(&assets_dir)?;

    // 1) Locate prebuilt artifacts in `target/<triple>/release`
    let target_triple = source_target
        .map(str::to_string)
        .or_else(|| crate::packaging::format::platform_default_target(platform).map(str::to_string))
        .context("could not determine target triple for platform {platform}")?;

    let target_dir = if target_triple == current_host_target()? {
        workspace_root.join("target/release")
    } else {
        workspace_root
            .join("target")
            .join(&target_triple)
            .join("release")
    };

    if !target_dir.exists() {
        anyhow::bail!(
            "target directory {} does not exist; build the project for {} first (cargo build --release --target {}{})",
            target_dir.display(),
            platform,
            target_triple,
            if let Some(p) = platform_from_target(&target_triple) {
                format!(" ; expected platform id `{p}`")
            } else {
                String::new()
            }
        );
    }

    // 2) Copy sd-core binary
    let (core_name, core_ext) = core_binary_for(platform);
    let core_filename = format!("{core_name}{core_ext}");
    let core_src = target_dir.join(&core_filename);
    if !core_src.exists() {
        anyhow::bail!(
            "core binary {} not found; run `cargo build --release --target {} -p sd-core`",
            core_src.display(),
            target_triple
        );
    }
    let core_dst = stage_dir.join(&core_filename);
    std::fs::copy(&core_src, &core_dst)
        .with_context(|| format!("copying core binary from {}", core_src.display()))?;

    // 3) Copy plugin .so / .dll / .dylib from the same target dir
    let lib_ext = lib_extension_for(platform);
    let lib_prefix = lib_prefix_for(platform);
    let mut plugin_count = 0usize;
    for entry in WalkDir::new(&target_dir)
        .max_depth(1)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let fname = match path.file_name().and_then(|s| s.to_str()) {
            Some(s) => s,
            None => continue,
        };
        if !fname.starts_with(lib_prefix) {
            continue;
        }
        if !fname.ends_with(&lib_ext) {
            continue;
        }
        // Skip the sd-core binary (which doesn't start with `libplugin_` on any
        // platform, but defensively re-check by name).
        if fname == format!("sd-core{lib_ext}") || fname == "sd-core.exe" {
            continue;
        }
        let dst = plugins_dir.join(fname);
        std::fs::copy(path, &dst).with_context(|| format!("copying plugin {}", fname))?;
        plugin_count += 1;
    }
    if plugin_count == 0 {
        eprintln!(
            "  {} no plugins copied (none found in {})",
            "warning:".yellow(),
            target_dir.display()
        );
    } else {
        println!(
            "  {} {} plugin(s) staged",
            "ok".green(),
            plugin_count.to_string().cyan()
        );
    }

    // 4) Copy web/dist
    let web_src = workspace_root.join("web").join("dist");
    if web_src.exists() {
        copy_dir_recursive(&web_src, &web_dir)
            .with_context(|| format!("copying web assets from {}", web_src.display()))?;
        println!("  {} web/ staged", "ok".green());
    } else {
        eprintln!(
            "  {} web/dist not found; skipping web assets",
            "warning:".yellow()
        );
    }

    // 5) Copy platform assets (.desktop, icon, etc.) and top-level docs
    copy_assets(workspace_root, cfg, platform, &assets_dir)?;
    copy_top_level_docs(workspace_root, &stage_dir)?;

    Ok(Staged {
        root: stage_dir,
        binary: core_dst,
        plugins_dir,
        web_dir,
        assets_dir,
    })
}

fn copy_assets(
    workspace_root: &Path,
    cfg: &ResolvedConfig,
    platform: &str,
    assets_dir: &Path,
) -> Result<()> {
    let candidates: Vec<Option<&str>> = match platform {
        p if p.starts_with("linux") => vec![
            cfg.linux.desktop_file.as_deref(),
            cfg.linux.icon_file.as_deref(),
        ],
        p if p.starts_with("macos") => vec![cfg.macos.icon.as_deref()],
        p if p.starts_with("windows") => vec![None],
        _ => vec![],
    };

    for candidate in candidates.into_iter().flatten() {
        let src = workspace_root.join(candidate);
        if !src.exists() {
            eprintln!(
                "  {} asset {} not found; skipping",
                "warning:".yellow(),
                src.display()
            );
            continue;
        }
        let dst = assets_dir.join(src.file_name().unwrap());
        std::fs::copy(&src, &dst).with_context(|| format!("copying asset {}", src.display()))?;
    }
    Ok(())
}

fn copy_top_level_docs(workspace_root: &Path, stage_dir: &Path) -> Result<()> {
    for name in ["README.md", "LICENSE", "CHANGELOG.md"] {
        let src = workspace_root.join(name);
        if src.exists() {
            std::fs::copy(&src, stage_dir.join(name))?;
        }
    }
    Ok(())
}

fn core_binary_for(platform: &str) -> (&'static str, &'static str) {
    if platform.starts_with("windows") {
        ("sd-core", ".exe")
    } else {
        ("sd-core", "")
    }
}

fn lib_extension_for(platform: &str) -> &'static str {
    if platform.starts_with("windows") {
        ".dll"
    } else if platform.starts_with("macos") {
        ".dylib"
    } else {
        ".so"
    }
}

/// On Linux/macOS Rust produces `lib<name>.{so,dylib}`; on Windows the
/// `lib` prefix is omitted and the file is `<name>.dll`.
fn lib_prefix_for(platform: &str) -> &'static str {
    if platform.starts_with("windows") {
        "plugin_"
    } else {
        "libplugin_"
    }
}

fn current_host_target() -> Result<String> {
    let output = Command::new("rustc")
        .args(["-Vv"])
        .output()
        .context("running rustc -Vv")?;
    let stdout = String::from_utf8(output.stdout)?;
    for line in stdout.lines() {
        if let Some(triple) = line.strip_prefix("host:") {
            return Ok(triple.trim().to_string());
        }
    }
    anyhow::bail!("could not determine host rustc target triple")
}

pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in WalkDir::new(src).into_iter().filter_map(Result::ok) {
        let src_path = entry.path();
        let rel = src_path.strip_prefix(src).unwrap();
        let dst_path = dst.join(rel);
        if src_path.is_dir() {
            std::fs::create_dir_all(&dst_path)?;
        } else {
            if let Some(parent) = dst_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(src_path, &dst_path)?;
        }
    }
    Ok(())
}
