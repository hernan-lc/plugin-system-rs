//! Supported packaging output formats.

use std::str::FromStr;

use anyhow::{anyhow, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    TarGz,
    Zip,
    Deb,
    Rpm,
    AppImage,
    Msi,
    Nsis,
    Dmg,
    Pkg,
}

impl Format {
    /// Stable lowercase id used on the CLI and in config files.
    pub fn id(self) -> &'static str {
        match self {
            Format::TarGz => "tar.gz",
            Format::Zip => "zip",
            Format::Deb => "deb",
            Format::Rpm => "rpm",
            Format::AppImage => "appimage",
            Format::Msi => "msi",
            Format::Nsis => "nsis",
            Format::Dmg => "dmg",
            Format::Pkg => "pkg",
        }
    }

    /// Human-readable label for log output.
    pub fn label(self) -> &'static str {
        match self {
            Format::TarGz => ".tar.gz archive",
            Format::Zip => ".zip archive",
            Format::Deb => "Debian package (.deb)",
            Format::Rpm => "RPM package (.rpm)",
            Format::AppImage => "AppImage",
            Format::Msi => "Windows Installer (.msi)",
            Format::Nsis => "NSIS installer (.exe)",
            Format::Dmg => "macOS disk image (.dmg)",
            Format::Pkg => "macOS installer (.pkg)",
        }
    }

    /// File extension (without the dot) used when writing the artifact.
    pub fn extension(self) -> &'static str {
        match self {
            Format::TarGz => "tar.gz",
            Format::Zip => "zip",
            Format::Deb => "deb",
            Format::Rpm => "rpm",
            Format::AppImage => "AppImage",
            Format::Msi => "msi",
            Format::Nsis => "exe",
            Format::Dmg => "dmg",
            Format::Pkg => "pkg",
        }
    }

    /// Platforms this format is valid for. `None` means "all platforms".
    pub fn supported_on(self) -> Option<&'static [&'static str]> {
        match self {
            Format::TarGz | Format::Zip => None,
            Format::Deb | Format::Rpm | Format::AppImage => Some(&["linux-x64", "linux-arm64"]),
            Format::Msi | Format::Nsis => Some(&["windows-x64", "windows-arm64"]),
            Format::Dmg | Format::Pkg => Some(&["macos-x64", "macos-arm64"]),
        }
    }
}

impl FromStr for Format {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "tar.gz" | "tgz" => Ok(Format::TarGz),
            "zip" => Ok(Format::Zip),
            "deb" | "debian" => Ok(Format::Deb),
            "rpm" => Ok(Format::Rpm),
            "appimage" => Ok(Format::AppImage),
            "msi" | "wix" => Ok(Format::Msi),
            "nsis" | "exe" => Ok(Format::Nsis),
            "dmg" => Ok(Format::Dmg),
            "pkg" => Ok(Format::Pkg),
            other => Err(anyhow!("unknown packaging format: {other}")),
        }
    }
}

/// Parse a comma-separated list like `deb,rpm,appimage`.
pub fn parse_format_list(raw: &str) -> Result<Vec<Format>> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(Format::from_str)
        .collect()
}

/// Canonical list of platforms we ship for.
pub const PLATFORMS: &[&str] = &[
    "linux-x64",
    "linux-arm64",
    "windows-x64",
    "windows-arm64",
    "macos-x64",
    "macos-arm64",
];

pub fn is_valid_platform(platform: &str) -> bool {
    PLATFORMS.contains(&platform)
}

/// Map a `rustc` target triple to a canonical packaging platform id.
pub fn platform_from_target(triple: &str) -> Option<&'static str> {
    if triple.contains("linux") && triple.contains("x86_64") {
        Some("linux-x64")
    } else if triple.contains("linux") && triple.contains("aarch64") {
        Some("linux-arm64")
    } else if triple.contains("windows") && triple.contains("x86_64") {
        Some("windows-x64")
    } else if triple.contains("windows") && triple.contains("aarch64") {
        Some("windows-arm64")
    } else if triple.contains("apple") && triple.contains("x86_64") {
        Some("macos-x64")
    } else if triple.contains("apple") && triple.contains("aarch64") {
        Some("macos-arm64")
    } else {
        None
    }
}

/// Default rustc target triple for a canonical packaging platform id.
pub fn platform_default_target(platform: &str) -> Option<&'static str> {
    match platform {
        "linux-x64" => Some("x86_64-unknown-linux-gnu"),
        "linux-arm64" => Some("aarch64-unknown-linux-gnu"),
        "windows-x64" => Some("x86_64-pc-windows-msvc"),
        "windows-arm64" => Some("aarch64-pc-windows-msvc"),
        "macos-x64" => Some("x86_64-apple-darwin"),
        "macos-arm64" => Some("aarch64-apple-darwin"),
        _ => None,
    }
}
