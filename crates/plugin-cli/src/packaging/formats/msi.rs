//! Windows MSI (WiX Toolset) builder.
//!
//! Produces a WiX 3.x source file (`.wxs`) and invokes `candle` + `light` to
//! produce a signed-capable `.msi`.
//!
//! Required tooling: WiX 3.11+ (`candle.exe`, `light.exe`). The CI
//! `windows-latest` runner installs WiX via Chocolatey.

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
    let artifact = output_root.join(artifact_name(cfg, platform, &Format::Msi));
    let staging_root = staged.root.parent().context("staged root has no parent")?;
    let work = staging_root.join(format!(".msi-build-{platform}"));
    if work.exists() {
        fs::remove_dir_all(&work).ok();
    }
    fs::create_dir_all(&work)?;

    let wxs = work.join("installer.wxs");
    let mut f = fs::File::create(&wxs)?;
    f.write_all(build_wxs(cfg, platform, staged).as_bytes())?;

    let obj_dir = work.join("obj");
    fs::create_dir_all(&obj_dir)?;
    let candle = Command::new("candle")
        .args([
            "-arch",
            "x64",
            "-out",
            obj_dir.join("installer.wixobj").to_str().unwrap(),
            wxs.to_str().unwrap(),
        ])
        .status();
    match candle {
        Ok(s) if s.success() => {}
        Ok(s) => anyhow::bail!("candle exited with status {s}"),
        Err(e) => anyhow::bail!(
            "candle (WiX) not available ({e}); install with `choco install wixtoolset` and retry"
        ),
    }

    let light = Command::new("light")
        .args([
            "-ext",
            "WixUIExtension",
            "-out",
            artifact.to_str().unwrap(),
            obj_dir.join("installer.wixobj").to_str().unwrap(),
        ])
        .status();
    match light {
        Ok(s) if s.success() => {}
        Ok(s) => anyhow::bail!("light exited with status {s}"),
        Err(e) => anyhow::bail!("light (WiX) not available ({e})"),
    }

    fs::remove_dir_all(&work).ok();

    Ok(vec![artifact])
}

fn build_wxs(cfg: &ResolvedConfig, platform: &str, staged: &Staged) -> String {
    let manufacturer = &cfg.windows.msi.manufacturer;
    let product_id = "*";
    let upgrade_code = if cfg.windows.msi.upgrade_code.is_empty() {
        "{12345678-1234-1234-1234-123456789012}".to_string()
    } else {
        cfg.windows.msi.upgrade_code.clone()
    };
    let display_name = sanitize_wix_id(&cfg.app.display_name);
    let _ = platform;

    let mut component_lines: Vec<String> = Vec::new();
    let mut id_counter = 0u32;
    for entry in walkdir::WalkDir::new(&staged.root)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("file");
        let file_id = format!("F{id_counter}");
        let component_id = format!("C{id_counter}");
        let source = path.to_string_lossy().replace('\\', "\\\\");
        component_lines.push(format!(
            "            <Component Id=\"{cid}\" Guid=\"*\" Win64=\"yes\"><File Id=\"{fid}\" Name=\"{name}\" Source=\"{source}\" KeyPath=\"yes\" /></Component>",
            cid = component_id,
            fid = file_id,
            name = xml_escape(name),
            source = source,
        ));
        id_counter += 1;
    }

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Wix xmlns="http://schemas.microsoft.com/wix/2006/wi">
  <Product Name="{name}" Version="{version}" Manufacturer="{manufacturer}"
           Id="{product_id}" UpgradeCode="{upgrade_code}" Language="1033">
    <Package InstallerVersion="500" Compressed="yes" InstallScope="perMachine" Platform="x64"/>
    <MajorUpgrade DowngradeErrorMessage="A newer version is already installed."/>
    <MediaTemplate EmbedCab="yes"/>
    <Feature Id="ProductFeature" Title="{name}" Level="1">
      <ComponentGroupRef Id="ProductComponents"/>
    </Feature>
    <Directory Id="TARGETDIR" Name="SourceDir">
      <Directory Id="ProgramFiles64Folder">
        <Directory Id="INSTALLDIR" Name="{display_name}">
          <ComponentGroup Id="ProductComponents">
{components}
          </ComponentGroup>
        </Directory>
      </Directory>
    </Directory>
    <UIRef Id="WixUI_Minimal"/>
  </Product>
</Wix>
"#,
        name = xml_escape(&cfg.app.display_name),
        version = cfg.app.version,
        manufacturer = xml_escape(manufacturer),
        product_id = product_id,
        upgrade_code = upgrade_code,
        display_name = xml_escape(&display_name),
        components = component_lines.join("\n"),
    )
}

fn sanitize_wix_id(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
