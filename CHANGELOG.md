# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- System tray icon with multiplatform support (Linux/Windows/macOS)
- QR code in web UI for mobile access
- `/api/local-ip` endpoint for network IP discovery
- OBS plugin with full WebSocket 5.x integration
- Volume master plugin with per-app control (Linux/Windows)
- Widget system with 10+ widget types
- **`sd-plugins pkg` cross-platform packaging pipeline**
  - Supports `.tar.gz`, `.zip`, `.deb` (pure Rust), `.rpm`, `.AppImage`,
    `.msi`, `.nsis`, `.dmg`, `.pkg`
  - Configured via `packaging.toml` at the repo root
  - Emits `checksums-sha256.txt` and `sbom.spdx.json` (SPDX 2.3) next to every
    artifact
  - Opt-in code signing for Windows (`signtool`), macOS (`codesign`) and
    Linux (GPG via `dpkg-sig` / `rpm --addsign`) via env vars
- `make package`, `make package-all`, `make package-platform`,
  `make package-formats` Makefile targets
- `docs/packaging.md` user guide

### Changed
- Simplified tray menu to: Status, Open in Browser, Exit
- QR code now uses real local IP from API instead of `window.location.origin`
- CI release workflow rewritten around `sd-plugins pkg`; one job per platform
  replacing the previous per-OS bash/powershell copy-paste
- Plugin staging dir moved from `releases/<v>/<p>/stage/` to
  `target/packaging/<p>/stage/` so it doesn't pollute the release output

### Fixed
- Linux event loop creation on background thread
- Menu event handling with `ControlFlow::Poll`
- QR code generation overflow panic
- `Command::new("npm")` no longer fails on Windows when the Node install
  directory isn't on `PATH` (uses `PATHEXT` resolution)

## [0.1.0] - 2026-06-11

### Added
- Initial release
- Plugin system with libloading
- Web UI with Preact + TypeScript
- System monitor, timer, key simulator plugins
- Virtual StreamDeck device
- Profile management
- WebSocket real-time events
