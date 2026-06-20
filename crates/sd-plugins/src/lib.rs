use plugin_system::{serde_json, PluginManager};
use sd_actions::ActionRegistry;
use sd_events::EventBus;
use sd_types::ActionId;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
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
            let file_name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            let derived_name = derive_plugin_name(&file_name);
            let metadata_name = PluginManager::metadata_from_path(&path)
                .ok()
                .map(|metadata| metadata.name);
            let enabled_by_file_name = state.is_enabled(&derived_name);
            let enabled_by_metadata = metadata_name
                .as_ref()
                .map(|name| state.is_enabled(name))
                .unwrap_or(true);
            if !enabled_by_file_name && !enabled_by_metadata {
                continue;
            }

            let mut manager = self.plugin_manager.write().await;
            let loaded_name = if manager.is_loaded(&derived_name) {
                Some(derived_name.clone())
            } else if let Some(name) = &metadata_name {
                manager.is_loaded(name).then(|| name.clone())
            } else {
                None
            };
            if let Some(name) = loaded_name {
                loaded.push(name);
                continue;
            }
            let actual_name = manager.load_plugin(&path)?;
            loaded.push(actual_name);
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
                let file_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                let derived_name = derive_plugin_name(&file_name);
                let metadata = manager
                    .plugin_metadata(&derived_name)
                    .or_else(|| PluginManager::metadata_from_path(&path).ok());
                let plugin_name = metadata
                    .as_ref()
                    .map(|metadata| metadata.name.clone())
                    .unwrap_or_else(|| derived_name.clone());
                let loaded = manager.is_loaded(&plugin_name) || manager.is_loaded(&derived_name);
                let enabled = state.is_enabled(&plugin_name) || state.is_enabled(&derived_name);
                let path = manager
                    .plugin_path(&plugin_name)
                    .or_else(|| manager.plugin_path(&derived_name))
                    .unwrap_or(path);
                statuses.push(PluginStatus {
                    name: plugin_name,
                    path: path.display().to_string(),
                    loaded,
                    enabled,
                    version: metadata
                        .map(|metadata| metadata.version)
                        .unwrap_or_default(),
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

    pub async fn set_plugin_enabled(
        &self,
        name: String,
        enabled: bool,
    ) -> PluginResult<PluginStatus> {
        let mut state = PluginState::load()?;
        self.set_plugin_enabled_with_state(name, enabled, &mut state)
            .await
            .inspect(|_status| {
                let _ = state.save();
            })
    }

    async fn set_plugin_enabled_with_state(
        &self,
        name: String,
        enabled: bool,
        state: &mut PluginState,
    ) -> PluginResult<PluginStatus> {
        let mut manager = self.plugin_manager.write().await;
        let path = manager
            .plugin_path(&name)
            .unwrap_or_else(|| self.find_plugin_path(&name));
        let metadata_name = PluginManager::metadata_from_path(&path)
            .ok()
            .map(|metadata| metadata.name);
        let target_name = metadata_name.unwrap_or_else(|| name.clone());
        let already_loaded = manager.is_loaded(&target_name) || manager.is_loaded(&name);
        if enabled {
            state.set_enabled(target_name.clone(), true);
            if !already_loaded {
                let path = manager
                    .plugin_path(&target_name)
                    .unwrap_or_else(|| self.find_plugin_path(&target_name));
                if !path.exists() {
                    return Err(PluginResultError::NotFound(target_name.clone()));
                }
                manager.load_plugin(&path)?;
            }
        } else {
            if already_loaded {
                manager
                    .unload_plugin(&target_name)
                    .or_else(|_| manager.unload_plugin(&name))?;
            }
            state.set_enabled(target_name.clone(), false);
        }

        let status = self.plugin_status_from_manager(&manager, &target_name)?;
        Ok(status)
    }

    pub async fn reload_plugins(&self) -> Result<(), String> {
        let mut manager = self.plugin_manager.write().await;
        for name in manager.plugin_names() {
            manager.reload_plugin(&name).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub async fn refresh_plugins_from_dir(&self) -> PluginResult<Vec<String>> {
        let state = PluginState::load()?;
        self.load_enabled_plugins_from_dir_with_state(&state).await
    }

    pub async fn install_plugin_file(
        &self,
        bytes: &[u8],
        filename: &str,
        enabled: bool,
    ) -> PluginResult<PluginStatus> {
        validate_uploaded_filename(filename)?;

        let dir = self.plugin_dir_path();
        fs::create_dir_all(&dir)?;

        let temp_path = self.temp_plugin_path(filename);
        fs::write(&temp_path, bytes)?;

        let metadata = PluginManager::metadata_from_path(&temp_path)
            .map_err(|e| PluginResultError::PluginLoad(e.to_string()))?;
        validate_plugin_name(&metadata.name)?;

        let final_path = self.plugin_file_path(&metadata.name);
        if final_path.exists() {
            let _ = fs::remove_file(&temp_path);
            return Err(PluginResultError::PluginExists(metadata.name));
        }

        fs::rename(&temp_path, &final_path)?;

        let mut state = PluginState::load()?;
        state.set_enabled(metadata.name.clone(), enabled);
        state.save()?;

        let mut manager = self.plugin_manager.write().await;
        if enabled {
            manager.load_plugin(&final_path)?;
        }
        self.plugin_status_from_manager(&manager, &metadata.name)
    }

    pub async fn update_plugin_file(
        &self,
        plugin_name: &str,
        bytes: &[u8],
        filename: &str,
        enabled: Option<bool>,
    ) -> PluginResult<PluginStatus> {
        validate_uploaded_filename(filename)?;

        let mut manager = self.plugin_manager.write().await;
        let mut state = PluginState::load()?;
        let existing_path = manager
            .plugin_path(plugin_name)
            .or_else(|| Some(self.find_plugin_path(plugin_name)));
        let existing_path = match existing_path {
            Some(path) if path.exists() => path,
            _ => return Err(PluginResultError::NotFound(plugin_name.to_string())),
        };

        let target_name = PluginManager::metadata_from_path(&existing_path)
            .map(|metadata| metadata.name)
            .unwrap_or(plugin_name.to_string());
        validate_plugin_name(&target_name)?;

        let was_loaded = manager.is_loaded(&target_name) || manager.is_loaded(plugin_name);
        let was_enabled = state.is_enabled(&target_name) || state.is_enabled(plugin_name);
        if was_loaded {
            manager
                .unload_plugin(&target_name)
                .or_else(|_| manager.unload_plugin(plugin_name))?;
        }

        let dir = self.plugin_dir_path();
        fs::create_dir_all(&dir)?;

        let temp_path = self.temp_plugin_path(filename);
        fs::write(&temp_path, bytes)?;

        let metadata = PluginManager::metadata_from_path(&temp_path)
            .map_err(|e| PluginResultError::PluginLoad(e.to_string()))?;
        validate_plugin_name(&metadata.name)?;

        let final_path = self.plugin_file_path(&metadata.name);
        if final_path.exists() && final_path != existing_path {
            let _ = fs::remove_file(&temp_path);
            return Err(PluginResultError::PluginExists(metadata.name));
        }

        let backup_path = self.backup_plugin_path(&existing_path);
        let keep_enabled = enabled.unwrap_or(was_enabled);
        if existing_path.exists() {
            if let Err(e) = fs::rename(&existing_path, &backup_path) {
                if was_loaded {
                    let _ = manager.load_plugin(&existing_path);
                }
                let _ = fs::remove_file(&temp_path);
                return Err(PluginResultError::Io(e.to_string()));
            }
        }

        if let Err(e) = fs::rename(&temp_path, &final_path) {
            if backup_path.exists() {
                let _ = fs::rename(&backup_path, &existing_path);
            }
            if was_loaded {
                let _ = manager.load_plugin(&existing_path);
            }
            return Err(PluginResultError::Io(e.to_string()));
        }

        state.set_enabled(target_name.clone(), false);
        state.set_enabled(metadata.name.clone(), keep_enabled);
        if let Err(e) = state.save() {
            let _ = fs::remove_file(&final_path);
            if backup_path.exists() {
                let _ = fs::rename(&backup_path, &existing_path);
            }
            if was_loaded {
                let _ = manager.load_plugin(&existing_path);
            }
            return Err(e);
        }

        let load_result = if keep_enabled {
            manager.load_plugin(&final_path)
        } else {
            Ok(String::new())
        };

        match load_result {
            Ok(_) => {
                let _ = fs::remove_file(&backup_path);
                self.plugin_status_from_manager(&manager, &metadata.name)
            }
            Err(e) => {
                let _ = fs::remove_file(&final_path);
                if backup_path.exists() {
                    let _ = fs::rename(&backup_path, &existing_path);
                }
                state.set_enabled(target_name.clone(), was_enabled);
                let _ = state.save();
                if was_loaded {
                    let _ = manager.load_plugin(&existing_path);
                }
                Err(PluginResultError::PluginLoad(e.to_string()))
            }
        }
    }

    pub fn plugin_dir(&self) -> &str {
        &self.plugin_dir
    }

    fn plugin_dir_path(&self) -> PathBuf {
        PathBuf::from(&self.plugin_dir)
    }

    fn plugin_file_path(&self, name: &str) -> PathBuf {
        self.plugin_dir_path()
            .join(format!("{}.{}", plugin_file_stem(name), plugin_extension()))
    }

    fn temp_plugin_path(&self, filename: &str) -> PathBuf {
        let stem = Path::new(filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("plugin");
        self.plugin_dir_path().join(format!(
            ".{}.{}.tmp.{}",
            stem,
            unique_suffix(),
            plugin_extension()
        ))
    }

    fn backup_plugin_path(&self, path: &Path) -> PathBuf {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("plugin");
        path.with_file_name(format!(
            "{}.{}.bak.{}",
            stem,
            unique_suffix(),
            plugin_extension()
        ))
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
                    if PluginManager::metadata_from_path(&path)
                        .map(|metadata| metadata.name == name)
                        .unwrap_or(false)
                    {
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
        let metadata = manager.plugin_metadata(name).or_else(|| {
            if !path.exists() {
                return None;
            }
            PluginManager::metadata_from_path(&path).ok()
        });
        Ok(PluginStatus {
            name: name.to_string(),
            path: path.display().to_string(),
            loaded: manager.is_loaded(name),
            enabled: state.is_enabled(name),
            version: metadata
                .map(|metadata| metadata.version)
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

fn plugin_file_stem(name: &str) -> String {
    let normalized = name.replace('-', "_");
    if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
        format!("libplugin_{}", normalized)
    } else {
        format!("plugin_{}", normalized)
    }
}

fn validate_uploaded_filename(filename: &str) -> PluginResult<()> {
    if filename.is_empty()
        || filename.contains('/')
        || filename.contains('\\')
        || filename.contains(':')
    {
        return Err(PluginResultError::InvalidInput(
            "Invalid plugin filename".to_string(),
        ));
    }

    let extension = Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default();
    if extension != plugin_extension() {
        return Err(PluginResultError::InvalidInput(format!(
            "Plugin file must use .{} on this platform",
            plugin_extension()
        )));
    }

    if Path::new(filename)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .is_empty()
    {
        return Err(PluginResultError::InvalidInput(
            "Plugin filename is missing a file stem".to_string(),
        ));
    }

    Ok(())
}

fn validate_plugin_name(name: &str) -> PluginResult<()> {
    if name.is_empty() || name.contains('/') || name.contains('\\') || name.contains(':') {
        return Err(PluginResultError::InvalidInput(
            "Invalid plugin metadata name".to_string(),
        ));
    }

    Ok(())
}

fn unique_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{}.{}", std::process::id(), nanos)
}

#[derive(Debug)]
pub enum PluginResultError {
    Io(String),
    Json(String),
    NotFound(String),
    PluginLoad(String),
    InvalidInput(String),
    PluginExists(String),
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
            PluginResultError::InvalidInput(err) => write!(f, "Invalid plugin input: {err}"),
            PluginResultError::PluginExists(name) => write!(f, "Plugin '{name}' already exists"),
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
