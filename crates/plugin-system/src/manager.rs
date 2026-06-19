use std::collections::HashMap;
use std::ffi::CStr;
use std::path::{Path, PathBuf};

use crate::context::PluginContext;
use crate::error::{PluginError, Result};
use crate::handler::{new_shared_command_registry, SharedCommandRegistry};
use crate::loader::{FileLoader, PluginLoader};
use crate::registry::{new_shared_registry, SharedRegistry};
use crate::traits::{Plugin, PluginMetadata};

type PluginCreateFn = unsafe extern "C" fn() -> *mut ();
type PluginDestroyFn = unsafe extern "C" fn(*mut ());
type PluginMetadataJsonFn = unsafe extern "C" fn() -> *mut std::ffi::c_char;
type PluginFreeStringFn = unsafe extern "C" fn(*mut std::ffi::c_char);

/// Derive a symbol prefix from the library filename.
/// e.g. "libplugin_volume_master.so" -> "volume_master"
/// The macro generates symbols like "plugin_{prefix}_create", so we need just the suffix.
fn prefix_from_path(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;

    let name = if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
        stem.strip_prefix("lib").unwrap_or(stem)
    } else {
        stem
    };

    // Strip "plugin_" prefix since the macro already adds it
    let name = name.strip_prefix("plugin_").unwrap_or(name);
    // Also strip "plugin-" prefix (with dash) for crate names like "plugin-volume-master"
    let name = name.strip_prefix("plugin-").unwrap_or(name);

    if name.is_empty() {
        return None;
    }

    Some(name.replace('-', "_"))
}

struct LoadedPlugin {
    _lib: libloading::Library,
    path: PathBuf,
    metadata: PluginMetadata,
    temp_path: Option<PathBuf>,
}

pub struct PluginManager {
    registry: SharedRegistry,
    command_registry: SharedCommandRegistry,
    loaded: HashMap<String, LoadedPlugin>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            registry: new_shared_registry(),
            command_registry: new_shared_command_registry(),
            loaded: HashMap::new(),
        }
    }

    pub fn registry(&self) -> SharedRegistry {
        self.registry.clone()
    }

    pub fn command_registry(&self) -> SharedCommandRegistry {
        self.command_registry.clone()
    }

    pub fn load_plugin_from_loader(
        &mut self,
        loader: &dyn PluginLoader,
        name: &str,
    ) -> Result<String> {
        log::info!("Loading plugin '{}' from {}", name, loader.source());

        let bytes = loader.load().map_err(|e| PluginError::PluginLoad {
            name: name.to_string(),
            reason: e.to_string(),
        })?;

        let ext = if cfg!(target_os = "linux") {
            "so"
        } else if cfg!(target_os = "macos") {
            "dylib"
        } else if cfg!(target_os = "windows") {
            "dll"
        } else {
            "so"
        };

        let temp_dir = std::env::temp_dir().join("plugin-system");
        std::fs::create_dir_all(&temp_dir)?;

        let temp_path = temp_dir.join(format!("{}_{}.{}", name, std::process::id(), ext));
        if let Err(e) = std::fs::write(&temp_path, &bytes) {
            let _ = std::fs::remove_file(&temp_path);
            return Err(e.into());
        }

        log::info!(
            "Wrote {} bytes to temp file: {}",
            bytes.len(),
            temp_path.display()
        );

        let source = loader.source();
        let original_path = Path::new(&source);
        let actual_name = match self.load_plugin_with_prefix(&temp_path, Some(original_path)) {
            Ok(name) => name,
            Err(e) => {
                let _ = std::fs::remove_file(&temp_path);
                return Err(e);
            }
        };
        if let Some(loaded) = self.loaded.get_mut(&actual_name) {
            if loaded.temp_path.is_none() {
                loaded.temp_path = Some(temp_path.clone());
            }
        }
        Ok(actual_name)
    }

    fn read_registry<'a>(
        &'a self,
        guard: std::sync::LockResult<
            std::sync::RwLockReadGuard<'a, crate::registry::PluginRegistry>,
        >,
        lock_name: &str,
    ) -> Result<std::sync::RwLockReadGuard<'a, crate::registry::PluginRegistry>> {
        match guard {
            Ok(reg) => Ok(reg),
            Err(poisoned) => {
                let reg = poisoned.into_inner();
                log::error!("{} poisoned; recovering with current state", lock_name);
                Ok(reg)
            }
        }
    }

    fn write_registry<'a>(
        &'a self,
        guard: std::sync::LockResult<
            std::sync::RwLockWriteGuard<'a, crate::registry::PluginRegistry>,
        >,
        lock_name: &str,
    ) -> Result<std::sync::RwLockWriteGuard<'a, crate::registry::PluginRegistry>> {
        match guard {
            Ok(reg) => Ok(reg),
            Err(poisoned) => {
                let reg = poisoned.into_inner();
                log::error!("{} poisoned; recovering with current state", lock_name);
                Ok(reg)
            }
        }
    }

    fn write_plugin<'a>(
        &'a self,
        guard: std::sync::LockResult<
            std::sync::RwLockWriteGuard<'a, Box<dyn crate::traits::Plugin>>,
        >,
        plugin_name: &str,
    ) -> Result<std::sync::RwLockWriteGuard<'a, Box<dyn crate::traits::Plugin>>> {
        match guard {
            Ok(plugin) => Ok(plugin),
            Err(poisoned) => {
                let plugin = poisoned.into_inner();
                log::error!(
                    "Plugin '{}' lock poisoned; recovering with current state",
                    plugin_name
                );
                Ok(plugin)
            }
        }
    }

    fn read_plugin<'a>(
        &'a self,
        guard: std::sync::LockResult<
            std::sync::RwLockReadGuard<'a, Box<dyn crate::traits::Plugin>>,
        >,
        plugin_name: &str,
    ) -> Result<std::sync::RwLockReadGuard<'a, Box<dyn crate::traits::Plugin>>> {
        match guard {
            Ok(plugin) => Ok(plugin),
            Err(poisoned) => {
                let plugin = poisoned.into_inner();
                log::error!(
                    "Plugin '{}' lock poisoned; recovering with current state",
                    plugin_name
                );
                Ok(plugin)
            }
        }
    }

    fn remove_temp_path(&self, name: &str) {
        if let Some(temp_path) = self
            .loaded
            .get(name)
            .and_then(|loaded| loaded.temp_path.clone())
        {
            let _ = std::fs::remove_file(&temp_path);
            log::debug!("Removed temp plugin file: {}", temp_path.display());
        }
    }

    fn cleanup_stale_temp_files(&self) {
        let temp_dir = std::env::temp_dir().join("plugin-system");
        if !temp_dir.exists() {
            return;
        }
        let my_pid = std::process::id();
        let mut removed = 0u32;
        if let Ok(entries) = std::fs::read_dir(&temp_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                // Only remove files that belong to a different PID
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    // Format: {name}_{pid}.so — extract the PID suffix
                    if let Some(last_underscore) = stem.rfind('_') {
                        let pid_str = &stem[last_underscore + 1..];
                        if let Ok(file_pid) = pid_str.parse::<u32>() {
                            if file_pid != my_pid {
                                let _ = std::fs::remove_file(&path);
                                removed += 1;
                            }
                        }
                    }
                }
            }
        }
        if removed > 0 {
            log::info!("Cleaned up {} stale temp plugin files", removed);
        }
    }

    fn load_metadata(
        lib: &libloading::Library,
        path: &Path,
        prefix: Option<&str>,
    ) -> Result<PluginMetadata> {
        if let Some(manifest) = crate::manifest::load_manifest(path).map_err(PluginError::Io)? {
            return Ok(manifest.into());
        }

        // Try prefixed symbols first, then fallback to unprefixed
        let (meta_sym, free_sym) = match prefix {
            Some(p) => (
                format!("plugin_{}_metadata_json", p),
                format!("plugin_{}_free_string", p),
            ),
            None => (
                "plugin_metadata_json".to_string(),
                "plugin_free_string".to_string(),
            ),
        };

        let metadata_json_fn: PluginMetadataJsonFn = unsafe {
            let sym = lib.get(meta_sym.as_bytes());
            let sym = match sym {
                Ok(s) => s,
                Err(_) => {
                    // Fallback to unprefixed
                    let fallback = match prefix {
                        Some(_) => "plugin_metadata_json",
                        None => {
                            return Err(PluginError::SymbolNotFound { symbol: meta_sym });
                        }
                    };
                    lib.get(fallback.as_bytes())
                        .map_err(|_| PluginError::SymbolNotFound {
                            symbol: meta_sym.clone(),
                        })?
                }
            };
            *sym
        };

        let free_string_fn: PluginFreeStringFn = unsafe {
            let sym = lib.get(free_sym.as_bytes());
            let sym = match sym {
                Ok(s) => s,
                Err(_) => {
                    let fallback = match prefix {
                        Some(_) => "plugin_free_string",
                        None => {
                            return Err(PluginError::SymbolNotFound { symbol: free_sym });
                        }
                    };
                    lib.get(fallback.as_bytes())
                        .map_err(|_| PluginError::SymbolNotFound {
                            symbol: free_sym.clone(),
                        })?
                }
            };
            *sym
        };

        let ptr = unsafe { metadata_json_fn() };
        if ptr.is_null() {
            return Err(PluginError::PluginLoad {
                name: "metadata".to_string(),
                reason: "plugin_metadata_json returned null".to_string(),
            });
        }

        let json = unsafe { CStr::from_ptr(ptr) }
            .to_str()
            .map_err(|e| PluginError::PluginLoad {
                name: "metadata".to_string(),
                reason: e.to_string(),
            })?
            .to_string();

        unsafe { free_string_fn(ptr) };

        serde_json::from_str(&json).map_err(|e| PluginError::PluginLoad {
            name: "metadata".to_string(),
            reason: e.to_string(),
        })
    }

    pub fn load_plugin(&mut self, path: impl AsRef<Path>) -> Result<String> {
        self.load_plugin_with_prefix(path, None)
    }

    fn load_plugin_with_prefix(
        &mut self,
        path: impl AsRef<Path>,
        original_path: Option<&Path>,
    ) -> Result<String> {
        let path = path.as_ref().to_path_buf();
        let path_display = path.display().to_string();

        log::info!("Loading plugin from {}", path_display);

        let lib = unsafe {
            libloading::Library::new(&path).map_err(|e| PluginError::LibraryLoad {
                path: path.clone(),
                reason: e.to_string(),
            })?
        };

        // Derive prefix from original path (not temp path which has PID suffix)
        let prefix_source = original_path.unwrap_or(&path);
        let prefix = prefix_from_path(prefix_source);

        // Try create symbol: prefixed first, then fallback to unprefixed
        let create: PluginCreateFn = unsafe {
            let sym_name = match &prefix {
                Some(p) => format!("plugin_{}_create", p),
                None => "plugin_create".to_string(),
            };
            let sym = lib.get(sym_name.as_bytes());
            let sym = match sym {
                Ok(s) => s,
                Err(_) => {
                    let fallback = match &prefix {
                        Some(_) => "plugin_create",
                        None => {
                            return Err(PluginError::SymbolNotFound { symbol: sym_name });
                        }
                    };
                    lib.get(fallback.as_bytes())
                        .map_err(|_| PluginError::SymbolNotFound {
                            symbol: sym_name.clone(),
                        })?
                }
            };
            *sym
        };

        let _destroy: PluginDestroyFn = unsafe {
            let sym_name = match &prefix {
                Some(p) => format!("plugin_{}_destroy", p),
                None => "plugin_destroy".to_string(),
            };
            let sym = lib.get(sym_name.as_bytes());
            let sym = match sym {
                Ok(s) => s,
                Err(_) => {
                    let fallback = match &prefix {
                        Some(_) => "plugin_destroy",
                        None => {
                            return Err(PluginError::SymbolNotFound { symbol: sym_name });
                        }
                    };
                    lib.get(fallback.as_bytes())
                        .map_err(|_| PluginError::SymbolNotFound {
                            symbol: sym_name.clone(),
                        })?
                }
            };
            *sym
        };

        let metadata = Self::load_metadata(&lib, &path, prefix.as_deref())?;
        let name = metadata.name.clone();

        log::info!("Plugin metadata: {} v{}", name, metadata.version);

        {
            let registry = self.read_registry(self.registry.read(), "PluginRegistry")?;
            for dep in &metadata.dependencies {
                if !registry.contains(dep.name.as_str()) {
                    return Err(PluginError::MissingDependency {
                        plugin: name.clone(),
                        dependency: dep.name.clone(),
                    });
                }
            }
        }

        let found_version = self
            .loaded
            .get(&name)
            .map(|p| p.metadata.version.clone())
            .unwrap_or_default();
        if !found_version.is_empty() {
            for dep in &metadata.dependencies {
                let req = match semver::VersionReq::parse(&dep.version_req) {
                    Ok(r) => r,
                    Err(e) => {
                        return Err(PluginError::InvalidSemverRequirement {
                            name: name.clone(),
                            reason: format!("{}: {}", dep.name, e),
                        });
                    }
                };
                let found = match semver::Version::parse(&found_version) {
                    Ok(v) => v,
                    Err(e) => {
                        return Err(PluginError::InvalidSemverRequirement {
                            name: name.clone(),
                            reason: format!("{}: {}", found_version, e),
                        });
                    }
                };
                if !req.matches(&found) {
                    return Err(PluginError::VersionIncompatible {
                        name: name.clone(),
                        required: dep.version_req.clone(),
                        found: found_version.clone(),
                    });
                }
            }
        }

        let raw_instance = unsafe { create() };
        let boxed: Box<Box<dyn Plugin>> =
            unsafe { Box::from_raw(raw_instance as *mut Box<dyn Plugin>) };
        let instance: Box<dyn Plugin> = *boxed;

        if self.loaded.contains_key(&name) {
            self.unload_plugin(&name)?;
        }

        {
            let mut registry = self.write_registry(self.registry.write(), "PluginRegistry")?;
            registry.register(instance);
        }

        let loaded_plugin = LoadedPlugin {
            _lib: lib,
            path: path.clone(),
            metadata,
            temp_path: None,
        };

        self.loaded.insert(name.clone(), loaded_plugin);

        {
            let ctx = PluginContext::new(self.registry.clone(), self.command_registry.clone());
            let registry = self.read_registry(self.registry.read(), "PluginRegistry")?;
            if let Some(plugin_arc) = registry.get_by_name(&name) {
                let mut plugin = self.write_plugin(plugin_arc.write(), &name)?;
                plugin.on_load(&ctx);
            }
        }

        log::info!("Plugin '{}' loaded successfully", name);
        Ok(name)
    }

    pub fn load_plugins_from_dir(&mut self, dir: impl AsRef<Path>) -> Result<Vec<String>> {
        let dir = dir.as_ref();
        log::info!("Scanning for plugins in {}", dir.display());

        self.cleanup_stale_temp_files();

        let mut loaded = Vec::new();

        if !dir.exists() {
            log::warn!("Plugin directory {} does not exist", dir.display());
            return Ok(loaded);
        }

        let expected_ext = if cfg!(target_os = "linux") {
            "so"
        } else if cfg!(target_os = "macos") {
            "dylib"
        } else if cfg!(target_os = "windows") {
            "dll"
        } else {
            "so"
        };

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == expected_ext {
                        let loader = FileLoader::new(&path);
                        let name = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown")
                            .to_string();

                        match self.load_plugin_from_loader(&loader, &name) {
                            Ok(name) => loaded.push(name),
                            Err(e) => {
                                log::error!("Failed to load {}: {}", path.display(), e);
                            }
                        }
                    }
                }
            }
        }

        Ok(loaded)
    }

    pub fn load_plugins_from_loaders(
        &mut self,
        loaders: &[(String, Box<dyn PluginLoader>)],
    ) -> Result<Vec<String>> {
        let mut loaded = Vec::new();

        for (name, loader) in loaders {
            match self.load_plugin_from_loader(loader.as_ref(), name) {
                Ok(name) => loaded.push(name),
                Err(e) => {
                    log::error!("Failed to load plugin '{}': {}", name, e);
                }
            }
        }

        Ok(loaded)
    }

    pub fn unload_plugin(&mut self, name: &str) -> Result<()> {
        log::info!("Unloading plugin '{}'", name);

        {
            let registry = self.read_registry(self.registry.read(), "PluginRegistry")?;
            if let Some(plugin_arc) = registry.get_by_name(name) {
                let mut plugin = self.write_plugin(plugin_arc.write(), name)?;
                plugin.on_unload();
            }
        }

        {
            let mut registry = self.write_registry(self.registry.write(), "PluginRegistry")?;
            registry
                .unregister(name)
                .ok_or_else(|| PluginError::PluginNotFound {
                    name: name.to_string(),
                })?;
        }

        self.loaded
            .remove(name)
            .ok_or_else(|| PluginError::PluginNotFound {
                name: name.to_string(),
            })?;

        self.remove_temp_path(name);

        log::info!("Plugin '{}' unloaded", name);
        Ok(())
    }

    pub fn reload_plugin(&mut self, name: &str) -> Result<()> {
        let path = self
            .loaded
            .get(name)
            .map(|p| p.path.clone())
            .ok_or_else(|| PluginError::PluginNotFound {
                name: name.to_string(),
            })?;

        log::info!("Reloading plugin '{}' from {}", name, path.display());

        self.unload_plugin(name)?;
        self.load_plugin(path)?;

        Ok(())
    }

    pub fn plugin_names(&self) -> Vec<String> {
        let registry = self.registry.read().ok();
        registry.map(|reg| reg.plugin_names()).unwrap_or_default()
    }

    pub fn is_loaded(&self, name: &str) -> bool {
        let registry = self.registry.read().ok();
        registry.map(|reg| reg.contains(name)).unwrap_or(false)
    }

    pub fn plugin_path(&self, name: &str) -> Option<PathBuf> {
        self.loaded.get(name).map(|p| p.path.clone())
    }

    pub fn plugin_metadata(&self, name: &str) -> Option<PluginMetadata> {
        self.loaded.get(name).map(|p| p.metadata.clone())
    }

    pub fn metadata_from_path(path: impl AsRef<Path>) -> Result<PluginMetadata> {
        let path = path.as_ref();
        let lib = unsafe {
            libloading::Library::new(path).map_err(|e| PluginError::LibraryLoad {
                path: path.to_path_buf(),
                reason: e.to_string(),
            })?
        };

        Self::load_metadata(&lib, path, prefix_from_path(path).as_deref())
    }

    pub fn plugin_metadata_from_path(&self, path: impl AsRef<Path>) -> Result<PluginMetadata> {
        Self::metadata_from_path(path)
    }

    pub fn with_plugin<R>(&self, name: &str, f: impl FnOnce(&dyn Plugin) -> R) -> Result<R> {
        let registry = self.read_registry(self.registry.read(), "PluginRegistry")?;
        let plugin_arc = registry
            .get_by_name(name)
            .ok_or_else(|| PluginError::PluginNotFound {
                name: name.to_string(),
            })?;
        let guard = self.read_plugin(plugin_arc.read(), name)?;
        let plugin_ref: &dyn Plugin = &**guard;
        Ok(f(plugin_ref))
    }

    pub fn with_plugin_mut<R>(
        &self,
        name: &str,
        f: impl FnOnce(&mut dyn Plugin) -> R,
    ) -> Result<R> {
        let registry = self.read_registry(self.registry.read(), "PluginRegistry")?;
        let plugin_arc = registry
            .get_by_name(name)
            .ok_or_else(|| PluginError::PluginNotFound {
                name: name.to_string(),
            })?;
        let mut guard = self.write_plugin(plugin_arc.write(), name)?;
        let plugin_ref: &mut dyn Plugin = &mut **guard;
        Ok(f(plugin_ref))
    }

    pub fn get_plugin_arc(
        &self,
        name: &str,
    ) -> Result<std::sync::Arc<std::sync::RwLock<Box<dyn Plugin>>>> {
        let registry = self.read_registry(self.registry.read(), "PluginRegistry")?;
        registry
            .get_by_name(name)
            .ok_or_else(|| PluginError::PluginNotFound {
                name: name.to_string(),
            })
    }

    pub fn get_plugin_info(&self, name: &str) -> Result<crate::plugin_info::PluginInfo> {
        let registry = self.read_registry(self.registry.read(), "PluginRegistry")?;
        let plugin_arc = registry
            .get_by_name(name)
            .ok_or_else(|| PluginError::PluginNotFound {
                name: name.to_string(),
            })?;
        let guard = self.read_plugin(plugin_arc.read(), name)?;
        let plugin_ref: &dyn Plugin = &**guard;
        let meta = plugin_ref.metadata();
        let dep_names = meta.dependencies_names();
        Ok(crate::plugin_info::PluginInfo {
            name: meta.name,
            version: meta.version,
            authors: meta.authors,
            dependencies: dep_names,
            public_methods: Vec::new(),
        })
    }

    pub fn call_plugin_result(
        &self,
        name: &str,
        f: impl FnOnce(&dyn Plugin) -> crate::plugin_info::PluginResult,
    ) -> Result<crate::plugin_info::PluginResult> {
        self.with_plugin(name, f)
    }

    pub fn get_all_plugin_info(&self) -> Vec<crate::plugin_info::PluginInfo> {
        let registry = match self.registry.read() {
            Ok(r) => r,
            Err(poisoned) => {
                log::error!("PluginRegistry poisoned while listing plugins");
                poisoned.into_inner()
            }
        };
        let mut infos = Vec::new();
        for (plugin_name, plugin_arc) in registry.iter_plugins() {
            let guard = match plugin_arc.read() {
                Ok(p) => p,
                Err(poisoned) => {
                    log::error!("Plugin '{}' lock poisoned while listing", plugin_name);
                    poisoned.into_inner()
                }
            };
            let plugin_ref: &dyn Plugin = &**guard;
            let meta = plugin_ref.metadata();
            let dep_names = meta.dependencies_names();
            infos.push(crate::plugin_info::PluginInfo {
                name: meta.name,
                version: meta.version,
                authors: meta.authors,
                dependencies: dep_names,
                public_methods: Vec::new(),
            });
        }
        infos
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}
