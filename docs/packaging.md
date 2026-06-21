# Packaging

StreamDeck Core ships as a single Rust CLI tool, `sd-plugins`, that builds the
core binary + plugins, stages a release tree, then produces installers in
every common format.

## Quick start

```bash
# Build the CLI once
cargo build --release -p sd-plugins-cli

# Build everything for the current host (web + core + plugins)
./target/release/sd-plugins build --release --with-web --with-core

# Package for the current host using formats from packaging.toml
./target/release/sd-plugins pkg --version 0.1.0
```

Artifacts land in `releases/<version>/<platform>/` along with a
`checksums-sha256.txt` sidecar and a `sbom.spdx.json` SPDX 2.3 SBOM.

## Targets

`sd-plugins pkg` accepts one of six platform ids:

| ID | Default target triple | Default formats |
|----|----------------------|-----------------|
| `linux-x64` | `x86_64-unknown-linux-gnu` | `tar.gz`, `deb`, `rpm` |
| `linux-arm64` | `aarch64-unknown-linux-gnu` | `tar.gz`, `deb` |
| `windows-x64` | `x86_64-pc-windows-msvc` | `zip`, `msi` |
| `windows-arm64` | `aarch64-pc-windows-msvc` | `zip`, `msi` |
| `macos-x64` | `x86_64-apple-darwin` | `tar.gz`, `dmg` |
| `macos-arm64` | `aarch64-apple-darwin` | `tar.gz`, `dmg` |

Override the default formats:

```bash
./target/release/sd-plugins pkg --version 0.1.0 --platform linux-x64 --formats deb,rpm,appimage
```

Override the platform too:

```bash
./target/release/sd-plugins pkg --version 0.1.0 --platform windows-x64 --formats zip,msi,nsis
```

## All-platforms mode

When the workspace already contains prebuilt artifacts in
`target/<triple>/release/`, you can produce a release for every supported
platform in one go:

```bash
./target/release/sd-plugins pkg --version 0.1.0 --all-platforms --formats tar.gz
```

`sd-plugins` will skip any platform whose target directory doesn't exist and
print a clear error.

## Auto-build mode

Pass `--build` to have `sd-plugins` invoke `cargo build --release --target
<triple> -p sd-core` and the plugin workspace build before packaging:

```bash
./target/release/sd-plugins pkg --version 0.1.0 --platform linux-arm64 --build --formats deb
```

## Configuration: `packaging.toml`

The `packaging.toml` file at the workspace root controls the metadata baked
into each installer (name, version, maintainer, license, dependencies, etc.)
and the per-platform default format list. See the file at the repo root for
the full schema; here is a minimal example:

```toml
[app]
name = "streamdeck-core"
version = "0.1.0"
description = "Plugin-based StreamDeck control system"
license = "MIT"
maintainer_name = "StreamDeck Team"
maintainer_email = "[email protected]"

[linux]
install_path = "/opt/streamdeck-core"
symlinks = ["/usr/bin/sd-core"]

[linux.deb]
depends = ["libc6 (>= 2.31)"]

[formats]
linux   = ["tar.gz", "deb", "rpm"]
windows = ["zip", "msi"]
macos   = ["tar.gz", "dmg"]
```

If `packaging.toml` is missing, sensible defaults are used.

## Format reference

| Format | Tooling required | Notes |
|--------|------------------|-------|
| `tar.gz` | none | Pure-Rust (`flate2` + `tar`) |
| `zip` | none | Pure-Rust (`zip` crate) |
| `deb` | none on build host | Pure-Rust `ar` + `tar` writer. Output is a valid Debian package |
| `rpm` | `rpmbuild` | Generates a `.spec` and shells out |
| `appimage` | `mksquashfs` + downloaded runtime | Falls back to `AppDir.tar.gz` if either is missing |
| `msi` | WiX 3.x (`candle`, `light`) | `choco install wixtoolset` on Windows |
| `nsis` | NSIS 3.x (`makensis`) | `choco install nsis` on Windows |
| `dmg` | `hdiutil` (macOS only) | Falls back to `.app.tar.gz` elsewhere |
| `pkg` | `pkgbuild` (macOS only) | Requires Xcode command-line tools |

## Code signing

Signing is **opt-in** via environment variables:

| Env var | Format | Tool |
|---------|--------|------|
| `SD_SIGN_WINDOWS_PFX` + `SD_SIGN_WINDOWS_PASSWORD` | `.exe`, `.dll` | `signtool` |
| `SD_SIGN_WINDOWS_TIMESTAMP` (optional) | signing timestamp URL, default `http://timestamp.digicert.com` | |
| `SD_SIGN_MACOS_IDENTITY` | `sd-core` binary + `libplugin_*.dylib` | `codesign --options runtime` |
| `SD_SIGN_GPG_KEY_ID` | `.deb` (via `dpkg-sig`) and `.rpm` (via `rpm --addsign`) | |

When set, `sd-plugins pkg` signs files after the format build but before
emitting the SHA256 sidecar. CI uses GitHub Actions secrets for the same
variables.

## CI

`.github/workflows/release.yml` runs the full matrix on every `v*` tag push.
It:

1. Builds `sd-plugins-cli` once on Ubuntu
2. For each (platform, target) pair:
   - Installs the platform's packaging tools
   - Builds web + core + plugins for the target triple
   - Runs `sd-plugins pkg` with the right `--platform` and `--formats`
3. Signs Windows / macOS / Linux artifacts when the corresponding secrets
   are present
4. Uploads the artifacts and creates a GitHub Release

To produce a local release matching CI exactly:

```bash
make build-plugins-release
make package VERSION=0.1.0
```

To package a single platform locally:

```bash
make package-platform PLATFORM=linux-x64 VERSION=0.1.0
```

## Runtime path resolution

When `sd-core` starts it must locate three things:

* The **plugins** directory (`plugin_*.dll` / `libplugin_*.so` / `*.dylib`).
* The **web frontend** (`index.html` and assets).
* A writable directory for **per-user state** (PID lock file, plugin-state
  json, uploaded plugins).

The lookup is centralised in the `sd-paths` crate so the MSI, the portable
zip, and a dev `cargo run` all behave the same way.

### Plugin and web lookup order

1. Explicit env-var override: `SD_PLUGIN_DIR` / `SD_WEB_DIST`.
2. The directory containing the running executable — `…/sd-core.exe` →
   `…/plugins/`, `…/web/`. This is the layout used by both the MSI
   installer and the portable zip.
3. `…/bin/` and `…/share/` siblings — for installers that split binaries
   from assets.
4. The current working directory — the legacy dev layout.

The first candidate that exists on disk is used. If none exist, the
highest-priority candidate is created when the binary writes to it (e.g.
uploading a new plugin).

### Per-user state

* Windows: `%LOCALAPPDATA%\sd-core\` (with `%APPDATA%\sd-core\` as
  fallback). Subdirectories: `state\` (pid lock, runtime state) and the
  data root (plugin-state.json, uploaded plugins).
* macOS: `~/Library/Application Support/sd-core/`.
* Linux: `$XDG_STATE_HOME/sd-core/` then
  `$XDG_DATA_HOME/sd-core/` then `~/.local/state/sd-core/`.

Override with `SD_DATA_DIR` / `SD_STATE_DIR` / `SD_CORE_PID_LOCK` env vars.

### Self-describing archives

Every archive (zip, tar.gz, deb, rpm, msi, …) contains a top-level
`platform.txt` next to the executable, e.g.:

```
platform=windows-x64
target_triple=x86_64-pc-windows-msvc
```

This is what the GitHub release smoke-test step uses to confirm the
artifact was packaged for the intended architecture before publishing.

### "This app can't run on your PC"

If Windows shows this dialog when launching `sd-core.exe`, you almost
certainly downloaded the **arm64** build on an **x64** system (or vice
versa). Check `platform.txt` inside the zip, and grab the matching arch
from the GitHub release page.
