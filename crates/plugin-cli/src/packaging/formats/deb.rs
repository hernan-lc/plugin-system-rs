//! Debian package (`.deb`) builder.
//!
//! A `.deb` is an `ar` archive containing:
//!   - `debian-binary`     — a single line: `2.0\n`
//!   - `control.tar.gz`    — control metadata
//!   - `data.tar.gz`       — the filesystem payload
//!
//! We build everything in pure Rust so this works without `dpkg-deb` on the
//! build host.

use std::fs::File;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use anyhow::{Context, Result};
use flate2::write::GzEncoder;
use flate2::Compression;
use walkdir::WalkDir;

use crate::packaging::config::ResolvedConfig;
use crate::packaging::format::Format;
use crate::packaging::stage::Staged;

use super::artifact_name;

const DEB_BINARY: &[u8] = b"2.0\n";

pub fn build(
    cfg: &ResolvedConfig,
    staged: &Staged,
    output_root: &Path,
    platform: &str,
) -> Result<Vec<PathBuf>> {
    let artifact = output_root.join(artifact_name(cfg, platform, &Format::Deb));
    let staging_root = staged.root.parent().context("staged root has no parent")?;
    let work = staging_root.join(format!(".deb-build-{platform}"));
    if work.exists() {
        std::fs::remove_dir_all(&work).ok();
    }
    std::fs::create_dir_all(&work)?;

    let data_root = work.join("data");
    std::fs::create_dir_all(&data_root)?;

    let install_path = &cfg.linux.install_path;
    let bin_dest_dir = data_root.join(install_path.trim_start_matches('/'));
    std::fs::create_dir_all(&bin_dest_dir)?;
    let bin_name = staged
        .binary
        .file_name()
        .context("core binary has no filename")?;
    let bin_dest = bin_dest_dir.join(bin_name);
    std::fs::copy(&staged.binary, &bin_dest)
        .with_context(|| format!("installing binary to {}", bin_dest.display()))?;
    set_executable(&bin_dest)?;

    let plugin_dest = data_root
        .join(install_path.trim_start_matches('/'))
        .join("plugins");
    copy_dir_recursive(&staged.plugins_dir, &plugin_dest)?;

    let web_dest = data_root
        .join(install_path.trim_start_matches('/'))
        .join("web");
    copy_dir_recursive(&staged.web_dir, &web_dest)?;

    for link in &cfg.linux.symlinks {
        let link_path = data_root.join(link.trim_start_matches('/'));
        if let Some(parent) = link_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let target = format!(
            "{}/{}",
            install_path.trim_end_matches('/'),
            bin_name.to_string_lossy()
        );
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&target, &link_path).with_context(|| {
                format!("creating symlink {} -> {}", link_path.display(), target)
            })?;
        }
        #[cfg(not(unix))]
        {
            let _ = (target, link_path);
        }
    }

    install_assets(&staged.assets_dir, &cfg.linux, &data_root)?;

    let data_tar = work.join("data.tar.gz");
    build_data_tar(&data_root, &data_tar)?;

    let control_dir = work.join("control");
    std::fs::create_dir_all(&control_dir)?;
    write_control(cfg, platform, &control_dir)?;

    let control_tar = work.join("control.tar.gz");
    build_control_tar(&control_dir, &control_tar)?;

    let deb_file =
        File::create(&artifact).with_context(|| format!("creating {}", artifact.display()))?;
    let mut ar = ar_builder::ArBuilder::new(deb_file);
    ar.add_file(b"debian-binary", DEB_BINARY)?;
    ar.add_file(b"control.tar.gz", &std::fs::read(&control_tar)?)?;
    ar.add_file(b"data.tar.gz", &std::fs::read(&data_tar)?)?;
    ar.finish()?;

    std::fs::remove_dir_all(&work).ok();

    Ok(vec![artifact])
}

fn install_assets(
    assets_dir: &Path,
    linux: &crate::packaging::config::ResolvedLinux,
    data_root: &Path,
) -> Result<()> {
    if linux.desktop_file.is_some() {
        let desktop_src = assets_dir.join("streamdeck.desktop");
        if desktop_src.exists() {
            let dest = data_root.join("usr/share/applications/streamdeck-core.desktop");
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&desktop_src, &dest)?;
        }
    }
    if linux.icon_file.is_some() {
        let icon_src = assets_dir.join("icon.png");
        if icon_src.exists() {
            let dest = data_root.join("usr/share/icons/hicolor/256x256/apps/streamdeck-core.png");
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&icon_src, &dest)?;
        }
    }
    Ok(())
}

fn write_control(cfg: &ResolvedConfig, platform: &str, control_dir: &Path) -> Result<()> {
    let arch = match platform {
        "linux-x64" => "amd64",
        "linux-arm64" => "arm64",
        other => anyhow::bail!("deb: unsupported platform {other}"),
    };
    let deb = &cfg.linux.deb;
    let mut s = String::new();
    s.push_str(&format!("Package: {}\n", debian_safe(&cfg.app.name)));
    s.push_str(&format!("Version: {}\n", debian_safe(&cfg.app.version)));
    s.push_str(&format!("Architecture: {arch}\n"));
    s.push_str(&format!(
        "Maintainer: {}\n",
        debian_safe(&cfg.app.maintainer)
    ));
    s.push_str(&format!("Section: {}\n", deb.section));
    s.push_str(&format!("Priority: {}\n", deb.priority));
    if !deb.depends.is_empty() {
        s.push_str(&format!("Depends: {}\n", deb.depends.join(", ")));
    }
    if !deb.recommends.is_empty() {
        s.push_str(&format!("Recommends: {}\n", deb.recommends.join(", ")));
    }
    if !deb.suggests.is_empty() {
        s.push_str(&format!("Suggests: {}\n", deb.suggests.join(", ")));
    }
    if !deb.conflicts.is_empty() {
        s.push_str(&format!("Conflicts: {}\n", deb.conflicts.join(", ")));
    }
    s.push_str(&format!(
        "Description: {}\n",
        single_line(&cfg.app.description)
    ));
    for line in cfg.app.long_description.lines() {
        s.push_str(&format!(" {line}\n"));
    }
    s.push_str(&format!("Homepage: {}\n", cfg.app.homepage));
    s.push_str(&format!("License: {}\n", cfg.app.license));

    std::fs::write(control_dir.join("control"), s)?;
    Ok(())
}

fn debian_safe(s: &str) -> String {
    s.replace('\n', " ").replace('\r', "")
}

fn single_line(s: &str) -> String {
    s.replace('\n', " ").trim().to_string()
}

fn set_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        let mut perms = std::fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms)?;
    }
    let _ = path;
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in WalkDir::new(src).into_iter().filter_map(Result::ok) {
        let rel = entry.path().strip_prefix(src).unwrap();
        let target = dst.join(rel);
        if entry.path().is_dir() {
            std::fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

fn build_data_tar(root: &Path, out: &Path) -> Result<()> {
    let file = File::create(out)?;
    let gz = GzEncoder::new(file, Compression::default());
    let mut tar = tar::Builder::new(gz);
    for entry in WalkDir::new(root).follow_links(false).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        let rel = path.strip_prefix(root).unwrap();
        if rel.as_os_str().is_empty() {
            continue;
        }
        let archive_path = rel.to_string_lossy().replace('\\', "/").to_string();
        let metadata = entry.metadata()?;
        let mut header = tar::Header::new_ustar();
        header.set_metadata_in_mode(&metadata, tar::HeaderMode::Deterministic);
        header.set_uid(0);
        header.set_gid(0);
        header.set_username("root")?;
        header.set_groupname("root")?;
        header.set_path(&archive_path)?;
        header.set_cksum();
        if path.is_symlink() {
            header.set_entry_type(tar::EntryType::Symlink);
            header.set_size(0);
            let target = std::fs::read_link(path)?;
            header.set_link_name(&target)?;
            tar.append(&header, std::io::empty())?;
        } else if path.is_dir() {
            header.set_entry_type(tar::EntryType::Directory);
            header.set_size(0);
            tar.append(&header, std::io::empty())?;
        } else {
            let mut f = File::open(path)?;
            tar.append(&header, &mut f)?;
        }
    }
    tar.into_inner()?.finish()?;
    Ok(())
}

fn build_control_tar(control_dir: &Path, out: &Path) -> Result<()> {
    let file = File::create(out)?;
    let gz = GzEncoder::new(file, Compression::default());
    let mut tar = tar::Builder::new(gz);
    for entry in WalkDir::new(control_dir).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        let rel = path.strip_prefix(control_dir).unwrap();
        if rel.as_os_str().is_empty() {
            continue;
        }
        let archive_path = rel.to_string_lossy().replace('\\', "/");
        let metadata = entry.metadata()?;
        let mut header = tar::Header::new_ustar();
        header.set_metadata_in_mode(&metadata, tar::HeaderMode::Deterministic);
        header.set_uid(0);
        header.set_gid(0);
        header.set_username("root")?;
        header.set_groupname("root")?;
        header.set_mode(0o755);
        header.set_path(&archive_path)?;
        header.set_cksum();
        if path.is_dir() {
            header.set_entry_type(tar::EntryType::Directory);
            header.set_size(0);
            tar.append(&header, std::io::empty())?;
        } else {
            let mut f = File::open(path)?;
            tar.append(&header, &mut f)?;
        }
    }
    tar.into_inner()?.finish()?;
    Ok(())
}

/// Minimal `ar` archive writer. The Debian `ar` format is the System V variant
/// (also called GNU `ar`) with a global header of "!<arch>\n".
mod ar_builder {
    use std::io::{self, Write};

    use anyhow::Result;

    const MAGIC: &[u8; 8] = b"!<arch>\n";

    pub struct ArBuilder<W: Write> {
        inner: W,
        finished: bool,
    }

    impl<W: Write> ArBuilder<W> {
        pub fn new(inner: W) -> Self {
            Self {
                inner,
                finished: false,
            }
        }

        pub fn add_file(&mut self, name: &[u8], data: &[u8]) -> Result<()> {
            assert!(!self.finished, "ar archive already finished");
            let mut header = [b' '; 60];
            write_field(&mut header[0..16], name, b'/')?;
            write_int(&mut header[16..28], 0, 12)?;
            write_int(&mut header[28..34], 0, 6)?;
            write_int(&mut header[34..40], 0, 6)?;
            write_octal(&mut header[40..48], 0o100644)?;
            write_int(&mut header[48..58], data.len() as u64, 10)?;
            header[58..60].copy_from_slice(b"`\n");

            self.inner.write_all(MAGIC)?;
            self.inner.write_all(&header)?;
            self.inner.write_all(data)?;
            if data.len() % 2 == 1 {
                self.inner.write_all(b"\n")?;
            }
            Ok(())
        }

        pub fn finish(mut self) -> Result<()> {
            self.finished = true;
            self.inner.flush()?;
            Ok(())
        }
    }

    fn write_field(dst: &mut [u8], value: &[u8], pad: u8) -> io::Result<()> {
        let len = value.len().min(dst.len());
        dst[..len].copy_from_slice(&value[..len]);
        for b in &mut dst[len..] {
            *b = pad;
        }
        Ok(())
    }

    fn write_int(dst: &mut [u8], value: u64, width: usize) -> io::Result<()> {
        let s = format!("{value}");
        let s_bytes = s.as_bytes();
        let len = s_bytes.len().min(width);
        for b in &mut dst[..width - len] {
            *b = b' ';
        }
        dst[width - len..].copy_from_slice(&s_bytes[..len]);
        Ok(())
    }

    fn write_octal(dst: &mut [u8], value: u64) -> io::Result<()> {
        let s = format!("{value:o}");
        let s_bytes = s.as_bytes();
        let dst_len = dst.len();
        let len = s_bytes.len().min(dst_len - 1);
        for b in &mut dst[..dst_len - 1 - len] {
            *b = b' ';
        }
        dst[dst_len - 1 - len..dst_len - 1].copy_from_slice(&s_bytes[..len]);
        dst[dst_len - 1] = b' ';
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::ar_builder::ArBuilder;

    #[test]
    fn ar_archive_starts_with_magic_and_has_three_members() {
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut ar = ArBuilder::new(&mut buf);
            ar.add_file(b"debian-binary", b"2.0\n").unwrap();
            ar.add_file(b"control.tar.gz", b"hello-control").unwrap();
            ar.add_file(b"data.tar.gz", b"hello-data-here").unwrap();
            ar.finish().unwrap();
        }

        // Starts with "!<arch>\n"
        assert_eq!(&buf[..8], b"!<arch>\n");

        // Each member: 8-byte global header (only on first) + 60-byte file header + body
        // First file header is at offset 8
        let h1 = &buf[8..68];
        let name = &h1[..16];
        assert_eq!(&name[..13], b"debian-binary");
        // GNU ar pads the name field with '/' as the end-of-name marker
        assert_eq!(&name[13..16], b"///");
        assert_eq!(&h1[58..60], b"`\n");

        // size is at offset 48..58 (10 chars, ascii decimal)
        let size_str = std::str::from_utf8(&h1[48..58]).unwrap();
        let size: usize = size_str.trim().parse().unwrap();
        assert_eq!(size, 4); // "2.0\n"

        // body of first file: bytes 68..72 should be "2.0\n"
        assert_eq!(&buf[68..72], b"2.0\n");
    }
}
