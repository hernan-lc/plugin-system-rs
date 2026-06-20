//! Emit a minimal SPDX 2.3 SBOM for a release, listing each produced artifact.

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;
use colored::Colorize;
use serde::Serialize;

use super::config::ResolvedConfig;
use super::sha::sha256_file;

#[allow(non_snake_case)]
#[derive(Debug, Serialize)]
struct SpdxDoc {
    spdxVersion: &'static str,
    dataLicense: &'static str,
    SPDXID: &'static str,
    name: String,
    documentNamespace: String,
    creationInfo: CreationInfo,
    packages: Vec<Package>,
    files: Vec<FileEntry>,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize)]
struct CreationInfo {
    created: String,
    creators: Vec<String>,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize)]
struct Package {
    SPDXID: String,
    name: String,
    versionInfo: String,
    downloadLocation: String,
    licenseConcluded: String,
    licenseDeclared: String,
    copyrightText: String,
    filesAnalyzed: bool,
    supplier: String,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize)]
struct FileEntry {
    SPDXID: String,
    fileName: String,
    checksum: Checksum,
    licenseConcluded: String,
    copyrightText: String,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize)]
struct Checksum {
    algorithm: &'static str,
    checksumValue: String,
}

pub fn write_sbom(
    artifacts: &[PathBuf],
    cfg: &ResolvedConfig,
    output_root: &Path,
    platform: &str,
) -> Result<()> {
    let sbom_path = output_root.join("sbom.spdx.json");
    let mut files = Vec::with_capacity(artifacts.len());
    for (counter, artifact) in (1usize..).zip(artifacts.iter()) {
        let rel = artifact
            .strip_prefix(output_root)
            .unwrap_or(artifact)
            .to_string_lossy()
            .replace('\\', "/");
        let digest = sha256_file(artifact)?;
        files.push(FileEntry {
            SPDXID: format!("SPDXRef-File-{counter}"),
            fileName: rel,
            checksum: Checksum {
                algorithm: "SHA256",
                checksumValue: digest,
            },
            licenseConcluded: "NOASSERTION".into(),
            copyrightText: "NOASSERTION".into(),
        });
    }

    let package_id = "SPDXRef-Package-Release".to_string();
    let document_namespace = format!(
        "https://github.com/streamdeck/core/spdx/{}/{}-{}",
        cfg.app.name, cfg.app.version, platform
    );

    let doc = SpdxDoc {
        spdxVersion: "SPDX-2.3",
        dataLicense: "CC0-1.0",
        SPDXID: "SPDXRef-DOCUMENT",
        name: format!("{}-{}-{}", cfg.app.name, cfg.app.version, platform),
        documentNamespace: document_namespace,
        creationInfo: CreationInfo {
            created: Utc::now().to_rfc3339(),
            creators: vec![format!(
                "Tool: sd-plugins-cli-{}",
                env!("CARGO_PKG_VERSION")
            )],
        },
        packages: vec![Package {
            SPDXID: package_id,
            name: cfg.app.name.clone(),
            versionInfo: cfg.app.version.clone(),
            downloadLocation: if cfg.app.homepage.is_empty() {
                "NOASSERTION".into()
            } else {
                cfg.app.homepage.clone()
            },
            licenseConcluded: if cfg.app.license.is_empty() {
                "NOASSERTION".into()
            } else {
                cfg.app.license.clone()
            },
            licenseDeclared: if cfg.app.license.is_empty() {
                "NOASSERTION".into()
            } else {
                cfg.app.license.clone()
            },
            copyrightText: "NOASSERTION".into(),
            filesAnalyzed: false,
            supplier: cfg.app.maintainer.clone(),
        }],
        files,
    };

    let mut file = File::create(&sbom_path)
        .with_context(|| format!("creating SBOM at {}", sbom_path.display()))?;
    let json = serde_json::to_string_pretty(&doc)?;
    file.write_all(json.as_bytes())?;
    println!(
        "    {} {}",
        "Wrote".green(),
        sbom_path.display().to_string().bold()
    );
    Ok(())
}
