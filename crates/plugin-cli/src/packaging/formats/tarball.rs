//! `.tar.gz` and `.zip` archives — the universal baseline.

use std::fs::File;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use flate2::write::GzEncoder;
use flate2::Compression;
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;

use crate::packaging::config::ResolvedConfig;
use crate::packaging::format::Format;
use crate::packaging::stage::Staged;

use super::artifact_name;

pub fn build_tar_gz(
    cfg: &ResolvedConfig,
    staged: &Staged,
    output_root: &Path,
    platform: &str,
) -> Result<Vec<PathBuf>> {
    let artifact = output_root.join(artifact_name(cfg, platform, &Format::TarGz));
    let file =
        File::create(&artifact).with_context(|| format!("creating {}", artifact.display()))?;
    let gz = GzEncoder::new(file, Compression::default());
    let mut tar = tar::Builder::new(gz);

    let prefix = format!("{}-{}/", cfg.app.name, cfg.app.version);
    append_dir_recursive(&mut tar, &staged.root, &prefix)?;

    tar.into_inner()
        .context("finalizing tar")?
        .finish()
        .context("finalizing gzip stream")?;

    Ok(vec![artifact])
}

pub fn build_zip(
    cfg: &ResolvedConfig,
    staged: &Staged,
    output_root: &Path,
    platform: &str,
) -> Result<Vec<PathBuf>> {
    let artifact = output_root.join(artifact_name(cfg, platform, &Format::Zip));
    let file =
        File::create(&artifact).with_context(|| format!("creating {}", artifact.display()))?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    let prefix = format!("{}-{}/", cfg.app.name, cfg.app.version);
    for entry in WalkDir::new(&staged.root)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        let rel = path
            .strip_prefix(&staged.root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let zip_path = format!("{prefix}{rel}");

        if path.is_dir() {
            zip.add_directory(zip_path, opts)?;
        } else {
            zip.start_file(zip_path, opts)?;
            let mut f = File::open(path)?;
            std::io::copy(&mut f, &mut zip)?;
        }
    }
    zip.finish()?;

    Ok(vec![artifact])
}

fn append_dir_recursive<W: std::io::Write>(
    tar: &mut tar::Builder<W>,
    root: &Path,
    prefix: &str,
) -> Result<()> {
    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        let rel = path
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let archive_path = if rel.is_empty() {
            prefix.trim_end_matches('/').to_string()
        } else {
            format!("{prefix}{rel}")
        };
        let metadata = entry.metadata()?;
        if path.is_dir() {
            let mut header = tar::Header::new_ustar();
            header.set_entry_type(tar::EntryType::Directory);
            header.set_path(&archive_path)?;
            header.set_size(0);
            header.set_mode(0o755);
            header.set_cksum();
            tar.append(&header, std::io::empty())
                .with_context(|| format!("adding dir {} to tar", archive_path))?;
        } else {
            let mut f = File::open(path).with_context(|| format!("opening {}", path.display()))?;
            tar.append_file(&archive_path, &mut f)
                .with_context(|| format!("adding file {} to tar", archive_path))?;
            let _ = metadata; // silence unused warning
        }
    }
    Ok(())
}
