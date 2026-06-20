//! Emit SHA256 sidecar files for every produced artifact.

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use colored::Colorize;
use sha2::{Digest, Sha256};

pub fn sha256_file(path: &Path) -> Result<String> {
    let bytes =
        std::fs::read(path).with_context(|| format!("reading {} for hashing", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = hasher.finalize();
    Ok(hex::encode(digest))
}

pub fn write_sha256_sidecar(
    artifacts: &[PathBuf],
    output_root: &Path,
    platform: &str,
) -> Result<()> {
    let sidecar_path = output_root.join("checksums-sha256.txt");
    let mut file = File::create(&sidecar_path)
        .with_context(|| format!("creating SHA256 sidecar at {}", sidecar_path.display()))?;
    writeln!(file, "# SHA256 checksums for platform: {platform}")?;
    for artifact in artifacts {
        let rel = artifact
            .strip_prefix(output_root)
            .unwrap_or(artifact)
            .to_string_lossy()
            .replace('\\', "/");
        let digest = sha256_file(artifact)?;
        writeln!(file, "{digest}  {rel}")?;
    }
    println!(
        "    {} {}",
        "Wrote".green(),
        sidecar_path.display().to_string().bold()
    );
    Ok(())
}
