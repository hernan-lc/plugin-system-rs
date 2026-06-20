//! NSIS installer builder for Windows.
//!
//! Generates a `.nsi` script and invokes `makensis` to produce a single-file
//! installer executable.
//!
//! Required tooling: NSIS 3.x (`makensis.exe`). Install with
//! `choco install nsis` on Windows.

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
    let artifact = output_root.join(artifact_name(cfg, platform, &Format::Nsis));
    let staging_root = staged.root.parent().context("staged root has no parent")?;
    let work = staging_root.join(format!(".nsis-build-{platform}"));
    if work.exists() {
        fs::remove_dir_all(&work).ok();
    }
    fs::create_dir_all(&work)?;

    let nsi = work.join("installer.nsi");
    let mut f = fs::File::create(&nsi)?;
    f.write_all(build_nsi(cfg, platform, staged).as_bytes())?;

    let status = Command::new("makensis")
        .arg(format!("/DOUTPUT={}", artifact.to_string_lossy()))
        .arg("/V2")
        .arg(nsi.to_str().unwrap())
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => anyhow::bail!("makensis exited with status {s}"),
        Err(e) => anyhow::bail!(
            "makensis (NSIS) not available ({e}); install with `choco install nsis` and retry"
        ),
    }

    fs::remove_dir_all(&work).ok();
    Ok(vec![artifact])
}

fn build_nsi(cfg: &ResolvedConfig, platform: &str, staged: &Staged) -> String {
    let install_dir = if cfg.windows.nsis.install_dir.is_empty() {
        r#"$PROGRAMFILES64\${APP_NAME}"#.to_string()
    } else {
        cfg.windows.nsis.install_dir.clone()
    };
    let publisher = &cfg.windows.nsis.publisher;
    let _ = platform;

    let staged_path = staged.root.to_string_lossy().replace('\\', "\\\\");
    let exe_name = staged
        .binary
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("sd-core.exe");

    format!(
        r#"!include "MUI2.nsh"
!include "FileFunc.nsh"

!define APP_NAME "{name}"
!define APP_DISPLAY_NAME "{display}"
!define APP_VERSION "{version}"
!define APP_PUBLISHER "{publisher}"
!define APP_EXE "{exe_name}"

Name "${{APP_DISPLAY_NAME}} ${{APP_VERSION}}"
OutFile "${{OUTPUT}}"
InstallDir "{install_dir}"
InstallDirRegKey HKLM "Software\${{APP_NAME}}" "InstallDir"
RequestExecutionLevel admin
ShowInstDetails show
ShowUninstDetails show

!macro MUI_PAGE_INSTALLDIR
  !insertmacro MUI_PAGE_INSTALLDIR
!macroend

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_WELCOME
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

!insertmacro MUI_LANGUAGE "English"

Section "Install"
  SetOutPath "$INSTDIR"
  File /r "{staged}\*.*"
  WriteRegStr HKLM "Software\${{APP_NAME}}" "InstallDir" "$INSTDIR"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${{APP_NAME}}" "DisplayName" "${{APP_DISPLAY_NAME}}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${{APP_NAME}}" "DisplayVersion" "${{APP_VERSION}}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${{APP_NAME}}" "Publisher" "${{APP_PUBLISHER}}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${{APP_NAME}}" "UninstallString" "$\"$INSTDIR\uninstall.exe$\""
  WriteUninstaller "$INSTDIR\uninstall.exe"
  CreateDirectory "$SMPROGRAMS\${{APP_DISPLAY_NAME}}"
  CreateShortCut "$SMPROGRAMS\${{APP_DISPLAY_NAME}}\${{APP_DISPLAY_NAME}}.lnk" "$INSTDIR\${{APP_EXE}}"
SectionEnd

Section "Uninstall"
  RMDir /r "$INSTDIR"
  RMDir /r "$SMPROGRAMS\${{APP_DISPLAY_NAME}}"
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${{APP_NAME}}"
  DeleteRegKey HKLM "Software\${{APP_NAME}}"
SectionEnd
"#,
        name = cfg.app.name,
        display = cfg.app.display_name,
        version = cfg.app.version,
        install_dir = install_dir,
        publisher = publisher,
        staged = staged_path,
        exe_name = exe_name,
    )
}
