//! RPM package (`.rpm`) builder.
//!
//! Generates a `.spec` file describing the install layout, then invokes
//! `rpmbuild` to produce the binary RPM. If `rpmbuild` is not available, the
//! function returns an error with a hint to install it (or to use the
//! containerized `rpmbuild` image in CI).
//!
//! This keeps us from reimplementing the RPM header + cpio encoding, which is
//! notoriously fiddly. CI runners on `ubuntu-latest` and most Linux distros
//! have `rpm` and `rpm-build` available via `apt` / `dnf`.

use std::fs;
use std::io::Write;
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
    let artifact = output_root.join(artifact_name(cfg, platform, &Format::Rpm));
    let staging_root = staged.root.parent().context("staged root has no parent")?;
    let work = staging_root.join(format!(".rpm-build-{platform}"));
    if work.exists() {
        fs::remove_dir_all(&work).ok();
    }

    let rpmbuild_root = work.join("rpmbuild");
    let sources = rpmbuild_root.join("SOURCES");
    let specs = rpmbuild_root.join("SPECS");
    let build_root = rpmbuild_root.join("BUILD");
    fs::create_dir_all(&sources)?;
    fs::create_dir_all(&specs)?;
    fs::create_dir_all(&build_root)?;

    // 1) Stage install tree under a tarball
    let stage_archive = sources.join(format!("{}-{}.tar.gz", cfg.app.name, cfg.app.version));
    build_payload_tarball(staged, &stage_archive, cfg)?;

    // 2) Write the .spec file
    let spec = build_spec(cfg, platform, &stage_archive)?;
    let spec_path = specs.join(format!("{}.spec", cfg.app.name));
    let mut f = fs::File::create(&spec_path)?;
    f.write_all(spec.as_bytes())?;

    // 3) Run rpmbuild
    let status = Command::new("rpmbuild")
        .args([
            "-bb",
            "--define",
            &format!("_topdir {}", rpmbuild_root.display()),
            "--define",
            &format!("_tmppath {}", build_root.display()),
            spec_path.to_str().context("spec path is not valid UTF-8")?,
        ])
        .status();

    match status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            anyhow::bail!("rpmbuild exited with status {s}");
        }
        Err(e) => {
            anyhow::bail!(
                "rpmbuild not available ({e}); install with `apt install rpm` or `dnf install rpm-build` and retry"
            );
        }
    }

    // 4) Locate the produced .rpm and move it to the artifact path
    let rpms_dir = rpmbuild_root.join("RPMS");
    let produced = find_rpm(&rpms_dir, &cfg.app.name).context("locating produced RPM")?;
    fs::copy(&produced, &artifact)
        .with_context(|| format!("copying RPM to {}", artifact.display()))?;

    fs::remove_dir_all(&work).ok();

    Ok(vec![artifact])
}

fn find_rpm(root: &Path, name: &str) -> Option<PathBuf> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .find(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "rpm")
                .unwrap_or(false)
                && e.path()
                    .file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| s.starts_with(name))
                    .unwrap_or(false)
        })
        .map(|e| e.into_path())
}

fn build_payload_tarball(staged: &Staged, out: &Path, cfg: &ResolvedConfig) -> Result<()> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use tar::Builder;
    use walkdir::WalkDir;

    let file = fs::File::create(out)?;
    let gz = GzEncoder::new(file, Compression::default());
    let mut tar = Builder::new(gz);
    let prefix = format!("{}-{}", cfg.app.name, cfg.app.version);
    for entry in WalkDir::new(&staged.root)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        let rel = path.strip_prefix(&staged.root).unwrap();
        if rel.as_os_str().is_empty() {
            continue;
        }
        let archive_path = format!("{prefix}/{}", rel.to_string_lossy().replace('\\', "/"));
        if path.is_dir() {
            tar.append_dir(&archive_path, path)?;
        } else {
            tar.append_path_with_name(path, &archive_path)?;
        }
    }
    tar.into_inner()?.finish()?;
    Ok(())
}

fn build_spec(cfg: &ResolvedConfig, platform: &str, source: &Path) -> Result<String> {
    let rpm = &cfg.linux.rpm;
    let arch = match platform {
        "linux-x64" => "x86_64",
        "linux-arm64" => "aarch64",
        other => anyhow::bail!("rpm: unsupported platform {other}"),
    };
    let summary = if rpm.summary.is_empty() {
        cfg.app.description.clone()
    } else {
        rpm.summary.clone()
    };
    let source_basename = source
        .file_name()
        .and_then(|s| s.to_str())
        .context("source basename")?;

    let depends = if rpm.depends.is_empty() {
        "glibc >= 2.31".to_string()
    } else {
        rpm.depends.join(", ")
    };

    let install_path = cfg.linux.install_path.trim_end_matches('/');

    let mut s = String::new();
    s.push_str(&format!("Name: {}\n", cfg.app.name));
    s.push_str(&format!("Version: {}\n", cfg.app.version));
    s.push_str("Release: 1%{?dist}\n");
    s.push_str(&format!("Summary: {}\n", summary));
    s.push_str(&format!("License: {}\n", cfg.app.license));
    s.push_str(&format!("URL: {}\n", cfg.app.homepage));
    s.push_str("Source0: %{name}-%{version}.tar.gz\n");
    s.push_str(&format!("BuildArch: {arch}\n"));
    s.push_str(&format!("Group: {}\n", rpm.group));
    s.push_str(&format!("Requires: {depends}\n"));
    s.push_str(&format!("Packager: {}\n", cfg.app.maintainer));
    s.push_str("AutoReqProv: no\n");
    s.push_str("\n%description\n");
    s.push_str(&cfg.app.long_description);
    s.push_str("\n\n%global __os_install_post %{nil}\n");
    s.push_str("%prep\n");
    s.push_str(&format!(
        "%setup -q -n {0}-{1}\n",
        cfg.app.name, cfg.app.version
    ));
    s.push_str("\n%build\n");
    s.push_str("# Source archive already contains the staged payload\n");
    s.push_str("echo \"Source archive: ");
    s.push_str(source_basename);
    s.push_str("\"\n");
    s.push_str("\n%install\n");
    s.push_str(&format!("mkdir -p %{{buildroot}}{install_path}\n"));
    s.push_str(&format!("cp -a sd-core %{{buildroot}}{install_path}/\n"));
    s.push_str(&format!("cp -a plugins %{{buildroot}}{install_path}/\n"));
    s.push_str(&format!("cp -a web %{{buildroot}}{install_path}/\n"));
    for link in &cfg.linux.symlinks {
        let link = link.trim_start_matches('/');
        s.push_str(&format!(
            "mkdir -p %{{buildroot}}/{dir}\n",
            dir = std::path::Path::new(link)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default()
        ));
        s.push_str(&format!(
            "ln -s {install_path}/sd-core %{{buildroot}}/{link}\n"
        ));
    }
    s.push_str("\n%files\n");
    s.push_str(&format!("%attr(0755, root, root) {install_path}/sd-core\n"));
    s.push_str(&format!("{install_path}/plugins\n"));
    s.push_str(&format!("{install_path}/web\n"));
    for link in &cfg.linux.symlinks {
        s.push_str(&format!("{link}\n"));
    }
    s.push_str("%doc README.md CHANGELOG.md\n");
    s.push_str("\n%changelog\n");
    s.push_str(&format!(
        "* {date} {maintainer} - {version}-1\n- Automated build by sd-plugins-cli\n",
        date = chrono::Utc::now().format("%a %b %d %Y"),
        maintainer = cfg.app.maintainer,
        version = cfg.app.version,
    ));
    Ok(s)
}
