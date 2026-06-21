//! `packaging.toml` schema and config resolution.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};

/// Raw TOML schema for `packaging.toml` at workspace root.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PackagingConfig {
    #[serde(default)]
    pub app: AppConfig,

    #[serde(default)]
    pub linux: LinuxConfig,

    #[serde(default)]
    pub macos: MacosConfig,

    #[serde(default)]
    pub windows: WindowsConfig,

    #[serde(default)]
    pub formats: FormatsConfig,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AppConfig {
    pub name: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub long_description: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub homepage: String,
    #[serde(default)]
    pub maintainer_name: String,
    #[serde(default)]
    pub maintainer_email: String,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LinuxConfig {
    #[serde(default = "default_linux_binary")]
    pub binary_name: String,
    #[serde(default = "default_linux_install_path")]
    pub install_path: String,
    #[serde(default = "default_linux_symlinks")]
    pub symlinks: Vec<String>,
    #[serde(default)]
    pub desktop_file: Option<String>,
    #[serde(default)]
    pub icon_file: Option<String>,

    #[serde(default)]
    pub deb: DebConfig,

    #[serde(default)]
    pub rpm: RpmConfig,

    #[serde(default)]
    pub appimage: AppImageConfig,
}

fn default_linux_binary() -> String {
    "sd-core".into()
}
fn default_linux_install_path() -> String {
    "/opt/streamdeck-core".into()
}
fn default_linux_symlinks() -> Vec<String> {
    vec!["/usr/bin/sd-core".into()]
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DebConfig {
    #[serde(default = "default_deb_section")]
    pub section: String,
    #[serde(default = "default_deb_priority")]
    pub priority: String,
    #[serde(default = "default_deb_depends")]
    pub depends: Vec<String>,
    #[serde(default)]
    pub recommends: Vec<String>,
    #[serde(default)]
    pub suggests: Vec<String>,
    #[serde(default)]
    pub conflicts: Vec<String>,
}

fn default_deb_section() -> String {
    "utils".into()
}
fn default_deb_priority() -> String {
    "optional".into()
}
fn default_deb_depends() -> Vec<String> {
    vec!["libc6 (>= 2.31)".into()]
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RpmConfig {
    #[serde(default = "default_rpm_group")]
    pub group: String,
    #[serde(default = "default_rpm_depends")]
    pub depends: Vec<String>,
    #[serde(default)]
    pub summary: String,
}

fn default_rpm_group() -> String {
    "Applications/System".into()
}
fn default_rpm_depends() -> Vec<String> {
    vec!["glibc >= 2.31".into()]
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AppImageConfig {
    #[serde(default)]
    pub update_string: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MacosConfig {
    #[serde(default = "default_macos_binary")]
    pub binary_name: String,
    #[serde(default = "default_macos_app_name")]
    pub app_name: String,
    #[serde(default = "default_macos_bundle_id")]
    pub bundle_id: String,
    #[serde(default)]
    pub icon: Option<String>,

    #[serde(default)]
    pub dmg: DmgConfig,

    #[serde(default)]
    pub pkg: PkgConfig,
}

fn default_macos_binary() -> String {
    "sd-core".into()
}
fn default_macos_app_name() -> String {
    "StreamDeck Core".into()
}
fn default_macos_bundle_id() -> String {
    "com.streamdeck.core".into()
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DmgConfig {
    #[serde(default = "default_dmg_volume")]
    pub volume_name: String,
    #[serde(default)]
    pub background: Option<String>,
}

fn default_dmg_volume() -> String {
    "StreamDeck Core".into()
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PkgConfig {
    #[serde(default = "default_pkg_identifier")]
    pub identifier: String,
    #[serde(default = "default_pkg_install_location")]
    pub install_location: String,
    #[serde(default)]
    pub version: String,
}

fn default_pkg_identifier() -> String {
    "com.streamdeck.core.pkg".into()
}
fn default_pkg_install_location() -> String {
    "/Applications".into()
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct WindowsConfig {
    #[serde(default = "default_windows_binary")]
    pub binary_name: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub publisher: String,

    #[serde(default)]
    pub msi: MsiConfig,

    #[serde(default)]
    pub nsis: NsisConfig,
}

fn default_windows_binary() -> String {
    "sd-core.exe".into()
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MsiConfig {
    #[serde(default)]
    pub upgrade_code: String,
    #[serde(default = "default_msi_manufacturer")]
    pub manufacturer: String,
    #[serde(default)]
    pub install_dir: String,
}

fn default_msi_manufacturer() -> String {
    "StreamDeck Core".into()
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct NsisConfig {
    #[serde(default = "default_nsis_publisher")]
    pub publisher: String,
    #[serde(default)]
    pub install_dir: String,
}

fn default_nsis_publisher() -> String {
    "StreamDeck Core".into()
}

/// Maps platform id (e.g. `linux-x64`) to a list of format ids to build by default.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct FormatsConfig {
    #[serde(default = "default_formats_linux")]
    pub linux: Vec<String>,
    #[serde(default = "default_formats_windows")]
    pub windows: Vec<String>,
    #[serde(default = "default_formats_macos")]
    pub macos: Vec<String>,
}

fn default_formats_linux() -> Vec<String> {
    vec!["tar.gz".into(), "deb".into(), "rpm".into()]
}
fn default_formats_windows() -> Vec<String> {
    vec!["zip".into(), "msi".into()]
}
fn default_formats_macos() -> Vec<String> {
    vec!["tar.gz".into(), "dmg".into()]
}

/// Resolved config with all defaults filled in and version/platform substituted.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    pub app: ResolvedApp,
    pub linux: ResolvedLinux,
    pub macos: ResolvedMacos,
    pub windows: ResolvedWindows,
    pub formats: ResolvedFormats,
    pub platform: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ResolvedApp {
    pub name: String,
    pub display_name: String,
    pub version: String,
    pub description: String,
    pub long_description: String,
    pub license: String,
    pub homepage: String,
    pub maintainer: String,
    pub categories: Vec<String>,
    pub keywords: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ResolvedLinux {
    pub binary_name: String,
    pub install_path: String,
    pub symlinks: Vec<String>,
    pub desktop_file: Option<String>,
    pub icon_file: Option<String>,
    pub deb: DebConfig,
    pub rpm: RpmConfig,
    pub appimage: AppImageConfig,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ResolvedMacos {
    pub binary_name: String,
    pub app_name: String,
    pub bundle_id: String,
    pub icon: Option<String>,
    pub dmg: DmgConfig,
    pub pkg: PkgConfig,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ResolvedWindows {
    pub binary_name: String,
    pub display_name: String,
    pub publisher: String,
    pub msi: MsiConfig,
    pub nsis: NsisConfig,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ResolvedFormats {
    pub by_platform: BTreeMap<String, Vec<String>>,
}

/// Load `packaging.toml` from the workspace root, returning defaults if absent.
pub fn load(workspace_root: &Path) -> Result<PackagingConfig> {
    let path = workspace_root.join("packaging.toml");
    if !path.exists() {
        return Ok(PackagingConfig {
            app: AppConfig {
                name: "streamdeck-core".into(),
                display_name: "StreamDeck Core".into(),
                description: "Plugin-based StreamDeck control system".into(),
                long_description:
                    "A plugin-based StreamDeck control system with web UI, built in Rust.".into(),
                license: "MIT".into(),
                maintainer_name: "StreamDeck Team".into(),
                maintainer_email: "[email protected]".into(),
                ..Default::default()
            },
            linux: LinuxConfig {
                deb: DebConfig {
                    depends: default_deb_depends(),
                    ..Default::default()
                },
                rpm: RpmConfig {
                    depends: default_rpm_depends(),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        });
    }

    let raw =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
}

/// Resolve a [`PackagingConfig`] for a given `version` and `platform`.
pub fn resolve(cfg: PackagingConfig, version: &str, platform: &str) -> Result<ResolvedConfig> {
    if cfg.app.name.is_empty() {
        anyhow::bail!("packaging.toml [app].name is required");
    }

    let app = ResolvedApp {
        name: cfg.app.name.clone(),
        display_name: if cfg.app.display_name.is_empty() {
            cfg.app.name.clone()
        } else {
            cfg.app.display_name
        },
        version: if cfg.app.version.is_empty() {
            version.to_string()
        } else {
            cfg.app.version
        },
        description: cfg.app.description,
        long_description: cfg.app.long_description,
        license: cfg.app.license,
        homepage: cfg.app.homepage,
        maintainer: format!(
            "{} <{}>",
            if cfg.app.maintainer_name.is_empty() {
                "Maintainer"
            } else {
                cfg.app.maintainer_name.as_str()
            },
            if cfg.app.maintainer_email.is_empty() {
                "[email protected]"
            } else {
                cfg.app.maintainer_email.as_str()
            }
        ),
        categories: cfg.app.categories,
        keywords: cfg.app.keywords,
    };

    let linux = ResolvedLinux {
        binary_name: cfg.linux.binary_name,
        install_path: cfg.linux.install_path,
        symlinks: cfg.linux.symlinks,
        desktop_file: cfg.linux.desktop_file,
        icon_file: cfg.linux.icon_file,
        deb: cfg.linux.deb,
        rpm: cfg.linux.rpm,
        appimage: cfg.linux.appimage,
    };

    let macos = ResolvedMacos {
        binary_name: cfg.macos.binary_name,
        app_name: cfg.macos.app_name,
        bundle_id: cfg.macos.bundle_id,
        icon: cfg.macos.icon,
        dmg: cfg.macos.dmg,
        pkg: cfg.macos.pkg,
    };

    let windows = ResolvedWindows {
        binary_name: cfg.windows.binary_name,
        display_name: if cfg.windows.display_name.is_empty() {
            app.display_name.clone()
        } else {
            cfg.windows.display_name
        },
        publisher: if cfg.windows.publisher.is_empty() {
            app.maintainer.clone()
        } else {
            cfg.windows.publisher
        },
        msi: cfg.windows.msi,
        nsis: cfg.windows.nsis,
    };

    // Warn if the user is still shipping with the documented placeholder
    // upgrade code. This is a strong signal that they forgot to generate
    // a real GUID for their product, which means Windows cannot
    // distinguish major upgrades of this MSI from any other product and
    // reinstalls will collide.
    const PLACEHOLDER_UPGRADE_CODE: &str = "{12345678-1234-1234-1234-123456789012}";
    if windows.msi.upgrade_code.is_empty()
        || windows.msi.upgrade_code == PLACEHOLDER_UPGRADE_CODE
    {
        eprintln!(
            "{} [windows.msi].upgrade_code is unset or still the placeholder {PLACEHOLDER_UPGRADE_CODE}. \
             Generate a stable GUID (e.g. `uuidgen` / PowerShell `[guid]::NewGuid()`) and set it in \
             packaging.toml, otherwise Windows will treat every reinstall as a new product.",
            "warning:".yellow(),
        );
    }

    let mut by_platform: BTreeMap<String, Vec<String>> = BTreeMap::new();
    by_platform.insert("linux-x64".into(), cfg.formats.linux.clone());
    by_platform.insert("linux-arm64".into(), cfg.formats.linux.clone());
    by_platform.insert("windows-x64".into(), cfg.formats.windows.clone());
    by_platform.insert("windows-arm64".into(), cfg.formats.windows.clone());
    by_platform.insert("macos-x64".into(), cfg.formats.macos.clone());
    by_platform.insert("macos-arm64".into(), cfg.formats.macos.clone());

    Ok(ResolvedConfig {
        app,
        linux,
        macos,
        windows,
        formats: ResolvedFormats { by_platform },
        platform: platform.to_string(),
    })
}
