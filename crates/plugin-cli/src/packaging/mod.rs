//! Cross-platform packaging for StreamDeck Core releases.
//!
//! Supported output formats: `tar.gz`, `zip`, `deb`, `rpm`, `appimage`, `msi`, `nsis`, `dmg`, `pkg`.
//!
//! Pipeline:
//!   1. Load [`manifest::PackagingConfig`] from `packaging.toml` at workspace root
//!   2. For each (platform, format) pair, call [`stage::stage_release`] to assemble
//!      a staging directory with the binary, plugins, web assets, and platform assets
//!   3. Call the format-specific builder from [`formats`] to produce the artifact
//!   4. Emit SHA256 + SPDX SBOM next to each artifact

pub mod config;
pub mod format;
pub mod manifest;
pub mod sbom;
pub mod sha;
pub mod sign;
pub mod stage;

pub mod formats;

use std::path::Path;

use anyhow::{Context, Result};
use colored::Colorize;

use crate::packaging::config::ResolvedConfig;
use crate::packaging::format::Format;
use crate::packaging::manifest::PackagingConfig;
use crate::packaging::stage::stage_release;

/// Run the full packaging pipeline for a single (platform, formats) invocation.
pub fn package_release(
    workspace_root: &Path,
    version: &str,
    output_root: &Path,
    platform: &str,
    formats: &[Format],
    source_target: Option<&str>,
) -> Result<Vec<std::path::PathBuf>> {
    println!(
        "{}",
        format!("=== Packaging {platform} ({version}) ===")
            .cyan()
            .bold()
    );
    println!("Formats: {}", format_list_human(formats));
    println!();

    let raw: PackagingConfig = config::load(workspace_root)
        .with_context(|| format!("loading packaging.toml in {}", workspace_root.display()))?;
    let resolved: ResolvedConfig = config::resolve(raw, version, platform)?;

    let staged = stage_release(workspace_root, &resolved, platform, source_target)?;

    let mut produced: Vec<std::path::PathBuf> = Vec::new();
    for fmt in formats {
        println!("  {} {}", "Building".yellow(), fmt.label());
        match formats::build(fmt, &resolved, &staged, output_root, platform) {
            Ok(artifacts) => {
                for artifact in artifacts {
                    println!(
                        "    {} {}",
                        "Created".green(),
                        artifact.display().to_string().bold()
                    );
                    produced.push(artifact);
                }
            }
            Err(e) => {
                eprintln!("    {} {}: {e:#}", "Failed".red(), fmt.label());
                return Err(e);
            }
        }
    }

    // Optional signing pass (only if env vars are set)
    let signed = sign::sign_artifacts(platform, output_root)?;
    if !signed.is_empty() {
        println!(
            "    {} {} file(s) signed",
            "✓".green(),
            signed.len().to_string().cyan()
        );
    }

    // Hash + SBOM for every produced file
    if !produced.is_empty() {
        println!();
        println!("  {} hashes + SBOM", "Emitting".yellow());
        sha::write_sha256_sidecar(&produced, output_root, platform)?;
        sbom::write_sbom(&produced, &resolved, output_root, platform)?;
    }

    Ok(produced)
}

fn format_list_human(formats: &[Format]) -> String {
    formats
        .iter()
        .map(|f| f.id())
        .collect::<Vec<_>>()
        .join(", ")
}
