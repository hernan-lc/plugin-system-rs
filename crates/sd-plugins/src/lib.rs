use plugin_system::{PluginManager, serde_json};
use sd_actions::ActionRegistry;
use sd_events::EventBus;
use sd_types::ActionId;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

fn plugin_state_path() -> PathBuf {
    PathBuf::from("data").join("plugin-state.json")
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginStatus {
    pub name: String,
    pub path: String,
    pub loaded: bool,
    pub enabled: bool,
    pub version: String,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PluginState {
    pub disabled: HashSet<String>,
}

impl PluginState {
    pub fn load() -> PluginResult<Self> {
        let path = plugin_state_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let data = fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&data)?)
    }

    pub fn save(&self) -> PluginResult<()> {
        let path = plugin_state_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self)?;
        fs::write(&path, data)?;
        Ok(())
    }

    pub fn is_enabled(&self, name: &str) -> bool {
        !self.disabled.contains(name)
    }

    pub fn set_enabled(&mut self, name: String, enabled: bool) {
        if enabled {
            self.disabled.remove(&name);
        } else {
            self.disabled.insert(name);
        }
    }
}

pub struct SdPluginManager {
    plugin_manager: Arc<RwLock<PluginManager>>,
    action_registry: Arc<RwLock<ActionRegistry>>,
    events: Arc<EventBus>,
    plugin_actions: Arc<RwLock<HashMap<String, Vec<ActionId>>>>,
    plugin_dir: String,
}

impl SdPluginManager {
    pub fn new(events: Arc<EventBus>, action_registry: Arc<RwLock<ActionRegistry>>) -> Self {
        Self {
            plugin_manager: Arc::new(RwLock::new(PluginManager::new())),
            action_registry,
            events,
            plugin_actions: Arc::new(RwLock::new(HashMap::new())),
            plugin_dir: "./plugins".to_string(),
        }
    }

    pub fn with_plugin_dir(mut self, plugin_dir: impl Into<String>) -> Self {
        self.plugin_dir = plugin_dir.into();
        self
    }

    pub async fn load_enabled_plugins_from_dir(&self) -> Result<Vec<String>, String> {
        let state = PluginState::load().map_err(|e| e.to_string())?;
        self.load_enabled_plugins_from_dir_with_state(&state)
            .await
            .map_err(|e| e.to_string())
    }

    async fn load_enabled_plugins_from_dir_with_state(
        &self,
        state: &PluginState,
    ) -> PluginResult<Vec<String>> {
        let dir = Path::new(&self.plugin_dir);
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let expected_ext = plugin_extension();
        let mut loaded = Vec::new();
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if !path.is_file() {
                continue;
            }
            if let Some(ext) = path.extension() {
                if ext != expected_ext {
                    continue;
                }
            }
            let file_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
            let name = derive_plugin_name(file_name);
            if !state.is_enabled(&name) {
                continue;
            }
            let mut manager = self.plugin_manager.write().await;
            if manager.is_loaded(&name) {
                loaded.push(name);
                continue;
            }
            manager.load_plugin(&path).map(|_| loaded.push(name))?;
        }
        Ok(loaded)
    }

    pub async fn list_plugin_statuses(&self) -> PluginResult<Vec<PluginStatus>> {
        let manager = self.plugin_manager.read().await;
        let state = PluginState::load()?;
        let dir = Path::new(&self.plugin_dir);
        let mut statuses = Vec::new();
        if dir.exists() {
            let expected_ext = plugin_extension();
            for entry in fs::read_dir(dir)? {
                let path = entry?.path();
                if !path.is_file() {
                    continue;
                }
                if let Some(ext) = path.extension() {
                    if ext != expected_ext {
                        continue;
                    }
                }
                let file_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
                let name = derive_plugin_name(file_name);
                let loaded = manager.is_loaded(&name);
                let enabled = state.is_enabled(&name);
                let metadata = manager.plugin_metadata(&name);
                let version = metadata.map(|m| m.version).unwrap_or_default();
                statuses.push(PluginStatus {
                    name,
                    path: path.display().to_string(),
                    loaded,
                    enabled,
                    version,
                });
            }
        }
        statuses.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(statuses)
    }

    pub async fn list_plugins(&self) -> Vec<String> {
        self.list_plugin_statuses()
            .await
            .unwrap_or_default()
            .into_iter()
            .filter(|p| p.loaded)
            .map(|p| p.name)
            .collect()
    }

    pub async fn set_plugin_enabled(&self, name: String, enabled: bool) -> PluginResult<PluginStatus> {
        let mut state = PluginState::load()?;
        self.set_plugin_enabled_with_state(name, enabled, &mut state)
            .await
            .map(|status| {
                let _ = state.save();
                status
            })
    }

    async fn set_plugin_enabled_with_state(
        &self,
        name: String,
        enabled: bool,
        state: &mut PluginState,
    ) -> PluginResult<PluginStatus> {
        let mut manager = self.plugin_manager.write().await;
        let already_loaded = manager.is_loaded(&name);
        if enabled {
            state.set_enabled(name.clone(), true);
            if !already_loaded {
                let path = manager
                    .plugin_path(&name)
                    .unwrap_or_else(|| self.find_plugin_path(&name));
                if !path.exists() {
                    return Err(PluginResultError::NotFound(name.clone()));
                }
                manager.load_plugin(&path)?;
            }
        } else {
            if already_loaded {
                manager.unload_plugin(&name)?;
            }
            state.set_enabled(name.clone(), false);
        }

        let status = self.plugin_status_from_manager(&manager, &name)?;
        Ok(status)
    }

    pub async fn reload_plugins(&self) -> Result<(), String> {
        let mut manager = self.plugin_manager.write().await;
        for name in manager.plugin_names() {
            manager
                .reload_plugin(&name)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn plugin_manager(&self) -> Arc<RwLock<PluginManager>> {
        self.plugin_manager.clone()
    }

    pub fn events(&self) -> &Arc<EventBus> {
        &self.events
    }

    pub fn action_registry(&self) -> &Arc<RwLock<ActionRegistry>> {
        &self.action_registry
    }

    pub async fn plugin_actions(&self) -> HashMap<String, Vec<ActionId>> {
        self.plugin_actions.read().await.clone()
    }

    fn find_plugin_path(&self, name: &str) -> PathBuf {
        let dir = Path::new(&self.plugin_dir);
        let expected_ext = plugin_extension();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                if let Some(ext) = path.extension() {
                    if ext != expected_ext {
                        continue;
                    }
                }
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let derived = derive_plugin_name(stem);
                    if derived == name {
                        return path;
                    }
                }
            }
        }
        PathBuf::new()
    }

    fn plugin_status_from_manager(
        &self,
        manager: &PluginManager,
        name: &str,
    ) -> PluginResult<PluginStatus> {
        let state = PluginState::load()?;
        let path = manager
            .plugin_path(name)
            .unwrap_or_else(|| self.find_plugin_path(name));
        Ok(PluginStatus {
            name: name.to_string(),
            path: path.display().to_string(),
            loaded: manager.is_loaded(name),
            enabled: state.is_enabled(name),
            version: manager
                .plugin_metadata(name)
                .map(|m| m.version)
                .unwrap_or_default(),
        })
    }
}

fn plugin_extension() -> &'static str {
    if cfg!(target_os = "linux") {
        "so"
    } else if cfg!(target_os = "macos") {
        "dylib"
    } else if cfg!(target_os = "windows") {
        "dll"
    } else {
        "so"
    }
}

fn derive_plugin_name(stem: &str) -> String {
    let name = if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
        stem.strip_prefix("lib").unwrap_or(stem)
    } else {
        stem
    };
    let name = name.strip_prefix("plugin_").unwrap_or(name);
    let name = name.strip_prefix("plugin-").unwrap_or(name);
    if name.is_empty() {
        stem.to_string()
    } else {
        name.replace('-', "_")
    }
}

#[derive(Debug)]
pub enum PluginResultError {
    Io(String),
    Json(String),
    NotFound(String),
    PluginLoad(String),
}

impl std::fmt::Display for PluginResultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginResultError::Io(err) => write!(f, "I/O error: {err}"),
            PluginResultError::Json(err) => write!(f, "JSON error: {err}"),
            PluginResultError::NotFound(name) => {
                write!(f, "Plugin '{name}' not found in directory scan")
            }
            PluginResultError::PluginLoad(err) => write!(f, "Plugin load failed: {err}"),
        }
    }
}

impl std::error::Error for PluginResultError {}

impl From<plugin_system::PluginError> for PluginResultError {
    fn from(value: plugin_system::PluginError) -> Self {
        PluginResultError::PluginLoad(value.to_string())
    }
}

impl From<std::io::Error> for PluginResultError {
    fn from(value: std::io::Error) -> Self {
        PluginResultError::Io(value.to_string())
    }
}

impl From<serde_json::Error> for PluginResultError {
    fn from(value: serde_json::Error) -> Self {
        PluginResultError::Json(value.to_string())
    }
}

pub type PluginResult<T> = Result<T, PluginResultError>;
