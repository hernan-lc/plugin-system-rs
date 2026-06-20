//! Optional code-signing and GPG-signing hooks.
//!
//! Enabled when the corresponding environment variables are set:
//!   - `SD_SIGN_WINDOWS_PFX` (path) + `SD_SIGN_WINDOWS_PASSWORD`
//!   - `SD_SIGN_MACOS_IDENTITY` (signing identity, e.g. "Developer ID Application: ...")
//!   - `SD_SIGN_GPG_KEY_ID` (key id for `dpkg-sig` and `rpm --addsign`)
//!
//! This is intentionally a thin wrapper around the platform tools. If a tool
//! isn't available, signing is silently skipped and a warning is emitted so
//! local dev builds aren't blocked.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;
use colored::Colorize;

/// Try to sign the produced artifacts in `output_root/platform/` using whatever
/// signing tools are available in the environment. Returns the list of files
/// that were successfully signed (or an empty list if signing was skipped).
pub fn sign_artifacts(platform: &str, output_root: &Path) -> Result<Vec<PathBuf>> {
    let mut signed = Vec::new();
    match platform {
        p if p.starts_with("windows") => {
            signed.extend(sign_windows(output_root)?);
        }
        p if p.starts_with("macos") => {
            signed.extend(sign_macos(output_root)?);
        }
        p if p.starts_with("linux") => {
            signed.extend(sign_linux_debs(output_root)?);
            signed.extend(sign_linux_rpms(output_root)?);
        }
        _ => {}
    }
    Ok(signed)
}

fn sign_windows(output_root: &Path) -> Result<Vec<PathBuf>> {
    let pfx_path = match std::env::var("SD_SIGN_WINDOWS_PFX").ok() {
        Some(p) => p,
        None => return Ok(Vec::new()),
    };
    let password = std::env::var("SD_SIGN_WINDOWS_PASSWORD").unwrap_or_default();
    let timestamp = std::env::var("SD_SIGN_WINDOWS_TIMESTAMP")
        .unwrap_or_else(|_| "http://timestamp.digicert.com".into());

    let mut signed = Vec::new();
    for entry in walkdir::WalkDir::new(output_root)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if ext != "exe" && ext != "dll" {
            continue;
        }
        // Skip files that aren't the core / plugins (e.g. uninstall.exe inside NSIS is fine to sign)
        let fname = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if !fname.starts_with("sd-core") && !fname.starts_with("plugin_") {
            continue;
        }
        let status = Command::new("signtool")
            .args([
                "sign", "/fd", "SHA256", "/tr", &timestamp, "/td", "SHA256", "/f", &pfx_path, "/p",
                &password,
            ])
            .arg(path)
            .status();
        match status {
            Ok(s) if s.success() => {
                println!("    {} signed {}", "✓".green(), path.display());
                signed.push(path.to_path_buf());
            }
            Ok(_) => eprintln!(
                "    {} signtool exited non-zero for {}",
                "warn:".yellow(),
                path.display()
            ),
            Err(e) => eprintln!(
                "    {} signtool not available ({e}); skipping signing",
                "warn:".yellow()
            ),
        }
    }
    Ok(signed)
}

fn sign_macos(output_root: &Path) -> Result<Vec<PathBuf>> {
    let identity = match std::env::var("SD_SIGN_MACOS_IDENTITY").ok() {
        Some(s) => s,
        None => return Ok(Vec::new()),
    };
    let mut signed = Vec::new();
    for entry in walkdir::WalkDir::new(output_root)
        .max_depth(3)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let fname = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        // Only sign the core binary and plugin dylibs, not the .app bundle
        if fname != "sd-core" && !fname.starts_with("libplugin_") {
            continue;
        }
        let status = Command::new("codesign")
            .args([
                "--force",
                "--options",
                "runtime",
                "--timestamp",
                "--sign",
                &identity,
            ])
            .arg(path)
            .status();
        match status {
            Ok(s) if s.success() => {
                println!("    {} signed {}", "✓".green(), path.display());
                signed.push(path.to_path_buf());
            }
            Ok(_) => eprintln!(
                "    {} codesign exited non-zero for {}",
                "warn:".yellow(),
                path.display()
            ),
            Err(e) => eprintln!(
                "    {} codesign not available ({e}); skipping signing",
                "warn:".yellow()
            ),
        }
    }
    Ok(signed)
}

fn sign_linux_debs(output_root: &Path) -> Result<Vec<PathBuf>> {
    let key_id = match std::env::var("SD_SIGN_GPG_KEY_ID").ok() {
        Some(k) => k,
        None => return Ok(Vec::new()),
    };
    let mut signed = Vec::new();
    for entry in walkdir::WalkDir::new(output_root)
        .max_depth(1)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("deb") {
            continue;
        }
        let status = Command::new("dpkg-sig")
            .args(["--sign", "builder", "-k", &key_id])
            .arg(path)
            .status();
        match status {
            Ok(s) if s.success() => {
                println!("    {} signed {}", "✓".green(), path.display());
                signed.push(path.to_path_buf());
            }
            Ok(_) => eprintln!(
                "    {} dpkg-sig exited non-zero for {}",
                "warn:".yellow(),
                path.display()
            ),
            Err(_) => {
                // dpkg-sig not installed; just warn
                eprintln!(
                    "    {} dpkg-sig not installed; skipping .deb signing",
                    "warn:".yellow()
                );
                return Ok(signed);
            }
        }
    }
    Ok(signed)
}

fn sign_linux_rpms(output_root: &Path) -> Result<Vec<PathBuf>> {
    let key_id = match std::env::var("SD_SIGN_GPG_KEY_ID").ok() {
        Some(k) => k,
        None => return Ok(Vec::new()),
    };
    let mut signed = Vec::new();
    let mut rpms: Vec<PathBuf> = Vec::new();
    for entry in walkdir::WalkDir::new(output_root)
        .max_depth(1)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("rpm") {
            rpms.push(path.to_path_buf());
        }
    }
    if rpms.is_empty() {
        return Ok(signed);
    }
    let mut cmd = Command::new("rpm");
    cmd.arg("--addsign");
    for rpm in &rpms {
        cmd.arg(rpm);
    }
    // rpm's --addsign with a specific key needs additional config; we pass it
    // via RPM macros.
    let status = Command::new("rpm")
        .arg("--define")
        .arg(format!("%_gpg_name {key_id}"))
        .arg("--addsign")
        .args(&rpms)
        .status();
    match status {
        Ok(s) if s.success() => {
            for rpm in &rpms {
                println!("    {} signed {}", "✓".green(), rpm.display());
            }
            signed.extend(rpms);
        }
        Ok(_) => eprintln!("    {} rpm --addsign exited non-zero", "warn:".yellow()),
        Err(e) => eprintln!(
            "    {} rpm not available ({e}); skipping .rpm signing",
            "warn:".yellow()
        ),
    }
    let _ = cmd;
    Ok(signed)
}
