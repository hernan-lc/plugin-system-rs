# StreamDeck Core

A plugin-based StreamDeck control system with web UI, built in Rust. Control OBS, system volume, keyboard shortcuts, and more from a browser-based dashboard.

## Download

Pre-built releases are available on the [Releases page](https://github.com/yourusername/streamdeck-core/releases):

| Platform | Architecture | Format |
|----------|--------------|--------|
| Linux | x86_64 | `.tar.gz` |
| Linux | ARM64 | `.tar.gz` |
| Windows | x86_64 | `.zip` |
| Windows | ARM64 | `.zip` |
| macOS | x86_64 (Intel) | `.tar.gz` |
| macOS | ARM64 (Apple Silicon) | `.tar.gz` |

### Quick Install

```bash
# Download and extract
tar xzf streamdeck-linux-x64.tar.gz
cd streamdeck-linux-x64

# Run
./sd-core
```

The server starts on `http://localhost:3000`. Open your browser or scan the QR code from your phone.

## Features

### System Tray
- **System tray icon** with context menu (multiplatform: Linux/Windows/macOS)
- **Open in Browser** - Launch the web UI in your default browser
- **Exit** - Clean shutdown from tray

### QR Code (Web UI)
- **QR button** in the navigation bar
- **Scan to connect** - Shows QR code for mobile access
- **Copy URL** - One-click copy of the dashboard URL
- Works on any device with a browser

### Plugins
- **System Monitor** - CPU, memory, load, uptime monitoring
- **Volume Control** - Master volume + per-app volume (Linux/Windows/macOS)
- **Key Simulator** - Keyboard hotkey simulation and recording
- **Timer** - Countdown timers with start/stop/pause
- **OBS Control** - Full OBS Studio integration via WebSocket (stream, record, scenes, inputs, transitions, virtual cam, replay buffer)

### Widgets
- **System Monitor** - 3 variants (minimal/compact/detailed)
- **Clock** - 3 variants (simple/digital/detailed)
- **Volume Master** - Master volume slider with mute
- **Volume Apps** - Per-app volume control
- **OBS Control** - Stream/record/virtual cam toggles with stats
- **OBS Scenes** - Scene switcher with transitions and source visibility
- **OBS Inputs** - Per-input volume and mute controls
- **Send Hotkey** - Trigger keyboard shortcuts
- **Open URL** - Open URLs in default browser
- **Type Text** - Type text strings

### Built-in Actions
- **HotkeyAction** - Send keyboard shortcuts
- **TextAction** - Type text
- **OpenUrlAction** - Open URLs in browser

### Web UI
- Virtual StreamDeck with 15 buttons
- Profile management
- Plugin browser
- Real-time event feed via WebSocket
- Mobile/tablet responsive design
- Widget wizard with live preview

## Architecture

```
streamdeck/
├── crates/
│   ├── plugin-system/      Core plugin framework (libloading-based)
│   ├── plugin-macros/      Proc macros for plugin exports
│   ├── sd-types/           Shared types (ActionId, ProfileId, etc.)
│   ├── sd-events/          Event bus for inter-plugin communication
│   ├── sd-actions/         Action trait + built-in actions
│   ├── sd-profiles/        Profile management (in-memory)
│   ├── sd-devices/         Device abstraction (virtual devices)
│   ├── sd-api/             axum HTTP + WebSocket server
│   ├── sd-plugins/         Plugin manager integration
│   ├── sd-core/            Main binary
│   └── plugin-cli/         Build CLI tool (sd-plugins)
├── plugins/
│   ├── plugin-timer/       Timer/countdown plugin
│   ├── plugin-system-monitor/  System resource monitoring
│   ├── plugin-key-simulator/  Key simulation plugin
│   ├── plugin-volume-master/  Multiplatform volume control
│   └── plugin-obs/         OBS Studio WebSocket control
└── web/                    Preact web UI
```

## Quick Start

### 1. Install system dependencies (Linux only)

The system tray requires GTK and libappindicator:

```bash
# Arch Linux / Manjaro
pacman -S gtk3 xdotool libappindicator-gtk3

# Debian / Ubuntu
sudo apt install libgtk-3-dev libxdo-dev libappindicator3-dev

# Windows / macOS: No extra dependencies needed
```

### 2. Build everything (recommended)

```bash
# Build CLI tool, plugins, web frontend, and core binary
cargo build --release -p sd-plugins-cli
./target/release/sd-plugins build --release --with-web --with-core
```

### 3. Run

```bash
cargo run
```

Or use the pre-built binary:

```bash
./target/release/sd-core
```

The server starts on `http://localhost:3000`. A system tray icon will appear in your system tray. Right-click it for options:
- **Open in Browser** - Launch the web UI
- **Exit** - Shutdown the server

### Development Mode

For plugin development with auto-rebuild and auto-restart:

```bash
# Build CLI tool first
cargo build -p sd-plugins-cli

# Watch plugins + auto-restart sd-core
./target/debug/sd-plugins dev -- cargo run --bin sd-core

# Or with release mode
./target/debug/sd-plugins dev -r -- cargo run --release --bin sd-core
```

Or use Make targets:

```bash
make dev CMD="cargo run --bin sd-core"
make dev-release CMD="cargo run --release --bin sd-core"
```

The `dev` command:
1. Builds all plugins once
2. Runs your command (e.g. `cargo run --bin sd-core`)
3. Watches `plugins/*/src/`, `crates/plugin-system/src/`, `crates/plugin-macros/src/` for changes
4. On change: rebuilds affected plugins and restarts your command
5. Press Ctrl+C to stop

For frontend development with hot reload:

```bash
cd web
npm run dev
```

The web UI starts on `http://localhost:5173` and proxies API requests to the backend.

## OBS Setup

1. Open OBS Studio
2. Go to **Tools > WebSocket Server Settings**
3. Enable the WebSocket server
4. Set a password (recommended)
5. Note the port (default: 4455)
6. In the web UI, add an **OBS Control** widget
7. Configure the widget with your OBS host/port/password
8. Click "Connect" in the widget

## API Endpoints

### Core
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/devices` | List connected devices |
| POST | `/api/devices/:id/press/:index` | Simulate button press |
| GET | `/api/profiles` | List profiles |
| POST | `/api/profiles` | Create profile |
| GET | `/api/profiles/:id` | Get profile |
| DELETE | `/api/profiles/:id` | Delete profile |
| GET | `/api/actions` | List available actions |
| POST | `/api/actions` | Execute action |
| POST | `/api/actions/open-url` | Open URL in browser |
| GET | `/api/plugins` | List loaded plugins |
| POST | `/api/plugins/reload` | Reload all plugins |
| GET | `/api/plugins/:name` | Get plugin data |
| GET | `/api/system-stats` | Get system stats |
| GET | `/api/local-ip` | Get local network IP |
| GET | `/api/dashboard` | Get dashboard layout |
| PUT | `/api/dashboard` | Save dashboard layout |
| WS | `/ws` | WebSocket for real-time events |

### Volume
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/volume` | Get master volume state + apps |
| PUT | `/api/volume/master` | Set master volume |
| PUT | `/api/volume/mute` | Set master mute |
| GET | `/api/volume/apps` | List per-app volumes |
| PUT | `/api/volume/app/volume` | Set app volume |
| PUT | `/api/volume/app/mute` | Set app mute |

### OBS
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/obs/status` | Get OBS connection + stream/record state |
| POST | `/api/obs/connect` | Connect to OBS |
| POST | `/api/obs/disconnect` | Disconnect from OBS |
| POST | `/api/obs/stream/start` | Start streaming |
| POST | `/api/obs/stream/stop` | Stop streaming |
| POST | `/api/obs/record/start` | Start recording |
| POST | `/api/obs/record/stop` | Stop recording |
| POST | `/api/obs/record/pause` | Toggle record pause |
| GET | `/api/obs/scenes` | List scenes |
| POST | `/api/obs/scenes/current` | Switch scene |
| GET | `/api/obs/inputs` | List inputs |
| PUT | `/api/obs/inputs/volume` | Set input volume |
| PUT | `/api/obs/inputs/mute` | Set input mute |
| POST | `/api/obs/virtualcam/toggle` | Toggle virtual camera |
| POST | `/api/obs/replay/save` | Save replay buffer |
| GET | `/api/obs/transitions` | List transitions |
| POST | `/api/obs/transitions/current` | Set transition |
| GET | `/api/obs/scene-items` | List scene items |
| PUT | `/api/obs/scene-item/enabled` | Toggle source visibility |
| GET | `/api/obs/studio-mode` | Get studio mode state |
| POST | `/api/obs/studio-mode` | Set studio mode |

### Hotkey
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/hotkey/send` | Send hotkey combination |
| POST | `/api/hotkey/record` | Record hotkey (3s timeout) |
| POST | `/api/hotkey/record/reset` | Reset hotkey recording |

## Creating a Plugin

```rust
use plugin_system::{Plugin, PluginMetadata};

pub struct MyPlugin;

#[plugin_system::plugin_export]
impl Plugin for MyPlugin {
    fn metadata(&self) -> PluginMetadata {
        plugin_system::plugin_metadata! {
            name: "my-plugin",
            version: "0.1.0",
            authors: ["You"],
            dependencies: []
        }
    }

    fn on_load(&mut self, _ctx: &plugin_system::PluginContext) {
        log::info!("MyPlugin loaded");
    }

    fn on_unload(&mut self) {
        log::info!("MyPlugin unloading");
    }

    fn plugin_type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn interface_ids(&self) -> Vec<&'static str> {
        vec!["MyInterface"]
    }

    fn interface_data(&self) -> Option<serde_json::Value> {
        None
    }

    fn handle_command(&mut self, method: &str, args: serde_json::Value) -> Option<serde_json::Value> {
        match method {
            "my_command" => Some(serde_json::json!({"ok": true})),
            _ => None,
        }
    }
}
```

## Tech Stack

- **Backend**: Rust, tokio, axum
- **Plugin System**: libloading, custom proc macros
- **Frontend**: Preact, TypeScript, Vite
- **Communication**: REST + WebSocket
- **OBS Integration**: obws (WebSocket 5.x)

## Platform Support

| Plugin | Linux | Windows | macOS |
|--------|-------|---------|-------|
| plugin-timer | ✓ | ✓ | ✓ |
| plugin-system-monitor | ✓ | ✗ | ✗ |
| plugin-key-simulator | ✓ | ✓ | ✓ |
| plugin-volume-master | ✓ | ✓ | ✓ |
| plugin-volume-master (per-app) | ✓ | ✓ | ✗ |
| plugin-obs | ✓ | ✓ | ✓ |

## FAQ

### Plugin doesn't load?

**Q: I built the plugin but it doesn't appear in the plugin list.**

A: Make sure you copied the `.so` file to the `plugins/` directory:
```bash
cp target/debug/libplugin_obs.so plugins/
```
Then restart the server or call `POST /api/plugins/reload`.

### OBS connection fails?

**Q: The OBS widget shows "Disconnected" even after clicking Connect.**

A: Check these:
1. OBS WebSocket server is enabled (Tools > WebSocket Server Settings)
2. The port matches (default: 4455)
3. If you set a password, make sure it's correct in the widget settings
4. OBS is running
5. No firewall blocking the connection

### Per-app volume not working on macOS?

**Q: The App Volume widget shows "Not supported".**

A: macOS doesn't expose per-app volume control through public APIs. The volume plugin only supports master volume on macOS. Per-app volume is available on Linux (via PulseAudio/PipeWire) and Windows (via CoreAudio).

### Port conflicts?

**Q: Port 3000 is already in use.**

A: Set the `PORT` environment variable:
```bash
PORT=3001 cargo run
```

### WebSocket not connecting?

**Q: The web UI shows "WebSocket disconnected".**

A: 
1. Make sure the backend is running on port 3000
2. Check browser console for errors
3. In development mode, ensure the Vite proxy is configured correctly
4. Try accessing `ws://localhost:3000/ws` directly

### How to build for other platforms?

**Q: Can I cross-compile for Windows from Linux?**

A: Yes, using `cargo-zigbuild`:
```bash
# Install cargo-zigbuild
cargo install cargo-zigbuild

# Build for Windows x64
cargo zigbuild --release --target x86_64-pc-windows-gnu -p plugin-obs

# Build for Windows ARM64
cargo zigbuild --release --target aarch64-pc-windows-gnullvm -p plugin-obs
```

### How to add custom widgets?

**Q: I want to create a custom widget for my plugin.**

A: 
1. Create `web/src/components/MyWidget.tsx`
2. Add widget type to `web/src/lib/types.ts`
3. Add widget catalog entry to `web/src/components/widgetHelpers.ts`
4. Register in `web/src/components/WidgetContent.tsx`
5. Add CSS styles to `web/src/styles/widgets.css`
6. Add wizard config in `web/src/components/WidgetWizard.tsx`

See `docs/system-plugins.md` for detailed instructions.

### OBS widgets show "OBS plugin not available"?

**Q: The OBS widgets show an error even though OBS is connected.**

A: The OBS plugin needs to be loaded. Check:
1. `libplugin_obs.so` exists in `plugins/` directory
2. The plugin appears in `GET /api/plugins` response
3. Restart the server after copying the plugin

### Hotkey recording doesn't work?

**Q: Clicking "Record" doesn't capture my keypress.**

A: 
1. Make sure you're pressing the keys within 3 seconds
2. Some keys (like Print Screen) may not be capturable
3. On Linux, you may need to grant input permissions
4. Try using the key picker instead of recording

### Volume slider doesn't update in real-time?

**Q: The volume slider shows old values.**

A: The widget polls every 2 seconds by default. You can change this in the widget settings (Config > Refresh Interval). Lower values increase CPU usage.

### Tray icon doesn't appear?

**Q: I don't see a tray icon after starting the server.**

A: On Linux, make sure you have the required packages:
- `libappindicator-gtk3` or `libayatana-appindicator`
- `gtk3`
- `xdotool`

On some desktop environments (like Wayland), you may need a tray indicator extension.

### How to access from mobile?

**Q: How do I control StreamDeck from my phone?**

A: 
1. Start the server on your computer
2. Click the **QR** button in the web UI navigation bar
3. Scan the QR code with your phone camera
4. Or manually enter `http://<your-computer-ip>:3000` in your phone's browser

Make sure your phone and computer are on the same WiFi network. The QR code shows the local network IP automatically.

## Building from Source

### Prerequisites

**Rust** (1.70+):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Node.js** (18+):
```bash
# Via nvm
nvm install 20
```

**Platform dependencies**:

| Platform | Command |
|----------|---------|
| Arch Linux | `pacman -S gtk3 xdotool libappindicator-gtk3` |
| Debian/Ubuntu | `apt install libgtk-3-dev libxdo-dev libappindicator3-dev` |
| Windows | None required |
| macOS | None required |

### Build

```bash
# Clone
git clone https://github.com/yourusername/streamdeck-core.git
cd streamdeck-core

# Build everything (CLI + plugins + web + core)
cargo build --release -p sd-plugins-cli
./target/release/sd-plugins build --release --with-web --with-core

# Or build manually step by step:
cd web && npm ci && npm run build && cd ..
cargo build --release
```

### Plugin Build CLI

The `sd-plugins` CLI tool auto-discovers and builds all plugins:

```bash
# Build CLI tool
cargo build -p sd-plugins-cli

# Build all plugins
./target/debug/sd-plugins build

# Build in release mode with web frontend and core binary
./target/debug/sd-plugins build --release --with-web --with-core

# Build specific plugin
./target/debug/sd-plugins build -p plugin-obs

# Watch plugins + auto-rebuild + run command
./target/debug/sd-plugins dev -- cargo run --bin sd-core

# List discovered plugins
./target/debug/sd-plugins list

# Validate plugin configurations
./target/debug/sd-plugins check

# Package for release — produces .tar.gz + .deb + .rpm on Linux
./target/debug/sd-plugins pkg --version 1.0.0

# Package for one platform with specific formats
./target/debug/sd-plugins pkg --version 1.0.0 --platform windows-x64 --formats zip,msi,nsis

# Package for every platform whose artifacts exist
./target/debug/sd-plugins pkg --version 1.0.0 --all-platforms

# Auto-build then package (one shot)
./target/debug/sd-plugins pkg --version 1.0.0 --platform linux-arm64 --build --formats deb

# Clean build artifacts
./target/debug/sd-plugins clean
```

Or use Make targets:
```bash
make build-plugins          # Build all plugins
make build-plugins-release  # Build release mode
make dev CMD="cargo run --bin sd-core"  # Watch + rebuild + run
make plugins-list           # List plugins
make plugins-check          # Validate configs
make plugins-clean          # Clean artifacts
make package VERSION=1.0.0                            # Package for host
make package-all VERSION=1.0.0                        # Package for every platform
make package-platform PLATFORM=linux-x64 VERSION=1.0.0
make package-formats VERSION=1.0.0 FORMATS="deb,rpm"   # Override formats
```

#### Supported installer formats

| Format | Platform | Tooling |
|--------|----------|---------|
| `.tar.gz` | all | none (pure Rust) |
| `.zip` | all | none (pure Rust) |
| `.deb` | Linux | none (pure Rust) |
| `.rpm` | Linux | `rpmbuild` |
| `.AppImage` | Linux | `mksquashfs` |
| `.msi` | Windows | WiX 3.x |
| `.nsis` (`.exe`) | Windows | NSIS 3.x |
| `.dmg` | macOS | `hdiutil` |
| `.pkg` | macOS | `pkgbuild` |

Each release also emits:

- `checksums-sha256.txt` — SHA256 of every artifact
- `sbom.spdx.json` — SPDX 2.3 SBOM with file checksums

See [`docs/packaging.md`](docs/packaging.md) for the full reference.

Or use Make targets:

```bash
make build-plugins          # Build all plugins
make build-plugins-release  # Build release mode
make dev CMD="cargo run --bin sd-core"  # Watch + rebuild + run
make plugins-list           # List plugins
make plugins-check          # Validate configs
make plugins-clean          # Clean artifacts
```

### Create Release

To build all platforms locally:

```bash
# Tag the release
git tag v1.0.0

# Run release script (builds for current platform + cross-compiles where possible)
./scripts/release.sh v1.0.0

# Check releases/
ls releases/v1.0.0/
```

### GitHub Actions

Releases are automatically built and published when you push a tag:

```bash
git tag v1.0.0
git push origin v1.0.0
```

This triggers the `.github/workflows/release.yml` workflow which:
1. Builds for all 6 platforms (Linux/Windows/macOS × x64/ARM64)
2. Packages each with web UI + plugins
3. Creates a GitHub Release with all artifacts

## License

MIT
