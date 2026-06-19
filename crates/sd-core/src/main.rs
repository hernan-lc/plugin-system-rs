use sd_actions::{ActionRegistry, HotkeyAction, OpenUrlAction, TextAction};
use sd_api::{create_router, load_dashboard_config, AppState};
use sd_devices::{DeviceManager, VirtualDevice};
use sd_events::EventBus;
use sd_plugins::SdPluginManager;
use sd_profiles::ProfileManager;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

mod tray;

#[derive(Debug, Error)]
enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("sd-core is already running with PID {pid}. Lock file: {path}")]
    AlreadyRunning { pid: u32, path: PathBuf },
}

type Result<T, E = Error> = std::result::Result<T, E>;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let pid_lock = acquire_pid_lock()?;
    tracing::info!(
        pid = std::process::id(),
        path = %pid_lock.path().display(),
        "Acquired pid lock"
    );

    println!("sd-core starting up...");

    let shutdown = Arc::new(AtomicBool::new(false));

    let shutdown_clone = shutdown.clone();
    ctrlc_handler(shutdown_clone, pid_lock.path().to_path_buf());

    let events = Arc::new(EventBus::new());

    let action_registry = Arc::new(RwLock::new(ActionRegistry::new()));
    {
        let mut registry = action_registry.write().await;
        registry.register(Arc::new(HotkeyAction::new("ctrl+c")));
        registry.register(Arc::new(TextAction::new("Hello, World!")));
        registry.register(Arc::new(OpenUrlAction::new("https://example.com")));
    }

    let profile_manager = Arc::new(ProfileManager::new(events.clone()));

    let default_profile_id = profile_manager.create_profile("Default").await;
    println!("Created default profile: {:?}", default_profile_id);

    let device_manager = Arc::new(DeviceManager::new(events.clone()));

    let virtual_device = Arc::new(VirtualDevice::new("virtual-1", 15, events.clone()));
    device_manager.add_device(virtual_device).await;
    println!("Added virtual device with 15 buttons");

    let plugin_dir = std::env::var("SD_PLUGIN_DIR")
        .unwrap_or_else(|_| "./plugins".to_string());

    let plugin_dir = std::env::var("SD_PLUGIN_DIR")
        .unwrap_or_else(|_| "./plugins".to_string());

    let plugin_manager = Arc::new(
        SdPluginManager::new(events.clone(), action_registry.clone())
            .with_plugin_dir(plugin_dir.clone()),
    );

    match plugin_manager.load_enabled_plugins_from_dir().await {
        Ok(loaded) => {
            if loaded.is_empty() {
                println!("No plugins found in {}", plugin_dir);
            } else {
                println!("Loaded {} plugins: {:?}", loaded.len(), loaded);
            }
        }
        Err(e) => {
            println!("Warning: Failed to load plugins: {}", e);
        }
    }

    let dashboard_config = Arc::new(RwLock::new(load_dashboard_config()));
    let state = AppState {
        events: events.clone(),
        action_registry,
        profile_manager,
        device_manager,
        plugin_manager,
        dashboard_config,
    };

    let events_clone = events.clone();
    tokio::spawn(async move {
        events_clone.run().await;
    });

    let bind_addr = bind_addr();
    let local_ip = local_ip_address::local_ip()
        .map(|ip| ip.to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string());

    let app = create_router(state);
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    let actual_addr = listener.local_addr()?;
    let port = actual_addr.port();

    let local_http_url = format!("http://127.0.0.1:{port}");
    let network_http_url = format!("http://{local_ip}:{port}");
    let local_ws_url = format!("ws://127.0.0.1:{port}/ws");
    let network_ws_url = format!("ws://{local_ip}:{port}/ws");

    println!("\nStarting HTTP server on http://{actual_addr}");
    println!("Local access: {local_http_url}");
    println!("Network access: {network_http_url}");
    println!("Local WebSocket endpoint: {local_ws_url}");
    println!("Network WebSocket endpoint: {network_ws_url}");
    println!("Local API docs: {local_http_url}/api");
    println!("Network API docs: {network_http_url}/api");
    println!("\nSystem tray icon active. Right-click for options.");
    println!("Press Ctrl+C to stop\n");

    let shutdown_tray = shutdown.clone();
    tray::spawn_tray(shutdown_tray, pid_lock.path().to_path_buf(), port);

    axum::serve(listener, app).await?;

    Ok(())
}

fn bind_addr() -> SocketAddr {
    std::env::var("SD_CORE_BIND_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:0".to_string())
        .parse()
        .expect("SD_CORE_BIND_ADDR must be a valid SocketAddr like 0.0.0.0:0")
}

fn ctrlc_handler(shutdown: Arc<AtomicBool>, pid_lock_path: PathBuf) {
    let _ = ctrlc::set_handler(move || {
        tracing::info!("Shutting down...");
        shutdown.store(true, Ordering::Relaxed);
        remove_pid_lock(&pid_lock_path);
        std::process::exit(0);
    });
}

struct PidLock {
    path: PathBuf,
    pid: u32,
}

impl PidLock {
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for PidLock {
    fn drop(&mut self) {
        if let Ok(contents) = fs::read_to_string(&self.path) {
            if contents.trim() == self.pid.to_string() {
                remove_pid_lock(&self.path);
            }
        }
    }
}

fn acquire_pid_lock() -> Result<PidLock> {
    let path = pid_lock_path();
    let pid = std::process::id();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    if let Some(existing_pid) = read_pid_lock(&path) {
        if existing_pid == pid {
            return Err(Error::AlreadyRunning {
                pid: existing_pid,
                path: path.clone(),
            });
        }

        if pid_is_running(existing_pid) {
            return Err(Error::AlreadyRunning {
                pid: existing_pid,
                path,
            });
        }

        tracing::warn!(
            pid = existing_pid,
            path = %path.display(),
            "Removing stale pid lock"
        );
        let _ = fs::remove_file(&path);
    }

    match OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(mut file) => {
            writeln!(file, "{}", pid)?;
            Ok(PidLock { path, pid })
        }
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
            if let Some(existing_pid) = read_pid_lock(&path) {
                if pid_is_running(existing_pid) {
                    return Err(Error::AlreadyRunning {
                        pid: existing_pid,
                        path,
                    });
                }

                let _ = fs::remove_file(&path);
                return acquire_pid_lock();
            }

            Err(Error::AlreadyRunning { pid: 0, path })
        }
        Err(err) => Err(err.into()),
    }
}

fn read_pid_lock(path: &Path) -> Option<u32> {
    fs::read_to_string(path)
        .ok()
        .and_then(|contents| contents.trim().parse::<u32>().ok())
}

fn pid_lock_path() -> PathBuf {
    if let Ok(path) = std::env::var("SD_CORE_PID_LOCK") {
        return PathBuf::from(path);
    }

    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        return PathBuf::from(runtime_dir)
            .join("sd-core")
            .join("sd-core.pid.lock");
    }

    let uid = std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|contents| {
            contents
                .lines()
                .find(|line| line.starts_with("Uid:"))
                .and_then(|line| line.split_whitespace().nth(1).map(str::to_string))
        })
        .unwrap_or_else(|| "unknown".to_string());

    std::env::temp_dir()
        .join(format!("sd-core-{uid}"))
        .join("sd-core.pid.lock")
}

fn pid_is_running(pid: u32) -> bool {
    #[cfg(target_os = "linux")]
    {
        Path::new("/proc").join(pid.to_string()).exists()
    }

    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
        use windows_sys::Win32::System::Threading::OpenProcess;

        unsafe {
            let handle: HANDLE = OpenProcess(0x1000, 0, pid);
            if handle == 0 {
                return false;
            }
            let _ = CloseHandle(handle);
            true
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = pid;
        true
    }
}

fn remove_pid_lock(path: &Path) {
    if let Err(err) = fs::remove_file(path) {
        if err.kind() != io::ErrorKind::NotFound {
            tracing::warn!(path = %path.display(), error = %err, "Failed to remove pid lock");
        }
    }
}
