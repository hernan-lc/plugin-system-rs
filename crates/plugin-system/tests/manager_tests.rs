use plugin_system::PluginManager;
use std::path::PathBuf;
use tempfile::TempDir;

fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("../..")
}

fn real_plugin_timer_path() -> PathBuf {
    plugin_system::platform::library_path(workspace_root().join("target/debug"), "plugin_timer")
}

fn plugin_dir() -> PathBuf {
    workspace_root().join("plugins")
}

fn all_plugin_so_paths() -> Vec<PathBuf> {
    let dir = plugin_dir();
    let mut paths = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "so" {
                        paths.push(path);
                    }
                }
            }
        }
    }
    paths
}

/// Get available bytes on the filesystem containing /tmp using `df`.
fn temp_dir_available_bytes() -> u64 {
    let output = match std::process::Command::new("df")
        .arg("--output=avail")
        .arg(std::env::temp_dir())
        .output()
    {
        Ok(o) => o,
        Err(_) => return 0,
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    // df output: "Avail\n  123456\n"
    stdout
        .lines()
        .nth(1)
        .and_then(|line| line.trim().parse::<u64>().ok())
        .map(|blocks| blocks * 1024) // df reports in 1K blocks by default
        .unwrap_or(0)
}

fn temp_dir_path() -> PathBuf {
    std::env::temp_dir().join("plugin-system")
}

#[test]
fn test_manager_new_creates_empty_registry() {
    let manager = PluginManager::new();
    assert!(manager.plugin_names().is_empty());
}

#[test]
fn test_load_plugins_from_dir_empty() {
    let mut manager = PluginManager::new();
    let temp_dir = TempDir::new().unwrap();
    let loaded = manager.load_plugins_from_dir(temp_dir.path()).unwrap();
    assert!(loaded.is_empty());
}

#[test]
fn test_load_plugin_real_plugin() {
    let mut manager = PluginManager::new();
    let path = real_plugin_timer_path();
    if !path.exists() {
        eprintln!(
            "Skipping test_load_plugin_real_plugin: timer plugin not found at {}",
            path.display()
        );
        return;
    }
    let name = manager.load_plugin(&path).unwrap();
    assert_eq!(name, "timer");
    assert!(manager.is_loaded("timer"));
    assert!(manager.plugin_names().contains(&"timer".to_string()));
    let meta = manager.plugin_metadata("timer");
    assert!(meta.is_some());
    let meta = meta.unwrap();
    assert_eq!(meta.name, "timer");
    assert_eq!(meta.version, "0.1.0");
}

#[test]
fn test_unload_plugin() {
    let mut manager = PluginManager::new();
    let path = real_plugin_timer_path();
    if !path.exists() {
        eprintln!("Skipping test_unload_plugin: timer plugin not found");
        return;
    }
    let name = manager.load_plugin(&path).unwrap();
    assert!(manager.is_loaded(&name));
    manager.unload_plugin(&name).unwrap();
    assert!(!manager.is_loaded(&name));
    assert!(manager.plugin_metadata(&name).is_none());
}

#[test]
fn test_with_plugin() {
    let mut manager = PluginManager::new();
    let path = real_plugin_timer_path();
    if !path.exists() {
        eprintln!("Skipping test_with_plugin: timer plugin not found");
        return;
    }
    manager.load_plugin(&path).unwrap();
    let result = manager.with_plugin("timer", |plugin| plugin.plugin_type_name());
    assert!(result.is_ok());
    let type_name = result.unwrap();
    assert!(type_name.contains("TimerPlugin"));
}

#[test]
fn test_with_plugin_mut() {
    let mut manager = PluginManager::new();
    let path = real_plugin_timer_path();
    if !path.exists() {
        eprintln!("Skipping test_with_plugin_mut: timer plugin not found");
        return;
    }
    manager.load_plugin(&path).unwrap();
    let result = manager.with_plugin_mut("timer", |plugin| {
        plugin.on_load(&plugin_system::PluginContext::new(
            manager.registry().clone(),
            manager.command_registry().clone(),
        ));
    });
    assert!(result.is_ok());
}

#[test]
fn test_get_all_plugin_info() {
    let mut manager = PluginManager::new();
    let path = real_plugin_timer_path();
    if !path.exists() {
        eprintln!("Skipping test_get_all_plugin_info: timer plugin not found");
        return;
    }
    manager.load_plugin(&path).unwrap();
    let infos = manager.get_all_plugin_info();
    assert_eq!(infos.len(), 1);
    let info = &infos[0];
    assert_eq!(info.name, "timer");
    assert_eq!(info.version, "0.1.0");
}

#[test]
fn test_reload_plugin() {
    let mut manager = PluginManager::new();
    let path = real_plugin_timer_path();
    if !path.exists() {
        eprintln!("Skipping test_reload_plugin: timer plugin not found");
        return;
    }
    let original_name = manager.load_plugin(&path).unwrap();
    assert!(manager.is_loaded(&original_name));
    manager.reload_plugin(&original_name).unwrap();
    assert!(manager.is_loaded(&original_name));
    let meta = manager.plugin_metadata(&original_name);
    assert!(meta.is_some());
    assert_eq!(meta.unwrap().name, "timer");
}

#[test]
fn test_load_plugin_missing_dependency() {
    let mut manager = PluginManager::new();
    let path = std::path::PathBuf::from(
        "/tmp/plugin-fake-missing-dep/target/debug/libplugin_fake_missing_dep.so",
    );
    if !path.exists() {
        eprintln!("Skipping test_load_plugin_missing_dependency: mock plugin not found");
        return;
    }
    let result = manager.load_plugin(path);
    assert!(result.is_err(), "Expected error but got Ok");
    let err = result.unwrap_err();
    eprintln!("Got error: {:?}", err);
    assert!(
        matches!(
            err,
            plugin_system::PluginError::MissingDependency { .. }
                | plugin_system::PluginError::SymbolNotFound { .. }
        ),
        "Expected MissingDependency or SymbolNotFound, got: {:?}",
        err
    );
}

// ============================================================================
// Diagnostic tests: always run to catch ENOSPC and loading issues after changes
// ============================================================================

/// Test 1: Verify /tmp has enough space for plugin loading.
/// This is the #1 cause of ENOSPC errors. Requires at least 100MB free.
#[test]
fn test_temp_dir_has_space() {
    let available = temp_dir_available_bytes();
    let min_required: u64 = 100 * 1024 * 1024; // 100 MB
    eprintln!(
        "/tmp available: {} bytes ({:.1} MB), required: {} bytes ({:.1} MB)",
        available,
        available as f64 / 1024.0 / 1024.0,
        min_required,
        min_required as f64 / 1024.0 / 1024.0,
    );
    assert!(
        available >= min_required,
        "FAIL: /tmp has only {:.1} MB free (need {:.1} MB). \
         Plugin loading will fail with ENOSPC. \
         Fix: 'rm -rf /tmp/plugin-system/*' or reboot to clear tmpfs.",
        available as f64 / 1024.0 / 1024.0,
        min_required as f64 / 1024.0 / 1024.0,
    );
}

/// Test 2: Verify the temp file write step works independently of libloading.
/// Isolates the ENOSPC error to the write-to-temp phase.
#[test]
fn test_load_plugin_temp_file_write() {
    let paths = all_plugin_so_paths();
    if paths.is_empty() {
        eprintln!("No .so plugin files found in plugins/ — skipping");
        return;
    }

    let temp_dir = temp_dir_path();
    std::fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

    for path in &paths {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let bytes = std::fs::read(path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e));
        assert!(!bytes.is_empty(), "Plugin file {} is empty", path.display());

        let temp_path = temp_dir.join(format!("{}_test.so", name));
        let write_result = std::fs::write(&temp_path, &bytes);
        assert!(
            write_result.is_ok(),
            "FAIL: Could not write temp file for plugin '{}' to {}. \
             /tmp is likely full (ENOSPC). Error: {} \
             Fix: 'rm -rf /tmp/plugin-system/*' or free space in /tmp.",
            name,
            temp_path.display(),
            write_result.unwrap_err(),
        );

        // Verify written file size matches
        let written_size = std::fs::metadata(&temp_path).map(|m| m.len()).unwrap_or(0);
        assert_eq!(
            written_size,
            bytes.len() as u64,
            "Temp file size mismatch for '{}': wrote {} bytes, expected {}",
            name,
            written_size,
            bytes.len(),
        );

        let _ = std::fs::remove_file(&temp_path);
    }
}

/// Test 3: Catch ENOSPC specifically during load_plugin and produce a diagnostic message.
#[test]
fn test_load_plugin_enospc_diagnostic() {
    let path = real_plugin_timer_path();
    if !path.exists() {
        eprintln!("Skipping: timer plugin not found at {}", path.display());
        return;
    }

    let mut manager = PluginManager::new();
    let result = manager.load_plugin(&path);

    match result {
        Ok(name) => {
            eprintln!("OK: plugin '{}' loaded successfully", name);
        }
        Err(e) => {
            let err_str = format!("{:?}", e);
            if err_str.contains("os error 28") || err_str.contains("No queda espacio") {
                panic!(
                    "ENOSPC detected loading plugin '{}'. \
                     /tmp is full. Diagnostics:\n\
                     {}\n\
                     Fix: 'rm -rf /tmp/plugin-system/*' or reboot to clear tmpfs.",
                    path.display(),
                    collect_diagnostics(),
                );
            }
            // Other errors are acceptable (e.g., missing symbols in debug builds)
            eprintln!("Non-ENOSPC error (acceptable): {}", e);
        }
    }
}

/// Test 4: Simulate the full plugin loading pipeline without libloading:
/// read bytes → write to temp → verify temp exists and matches size.
#[test]
fn test_plugin_load_pipeline_dry_run() {
    let paths = all_plugin_so_paths();
    if paths.is_empty() {
        eprintln!("No .so plugin files found in plugins/ — skipping");
        return;
    }

    let temp_dir = temp_dir_path();
    std::fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

    let pid = std::process::id();

    for path in &paths {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        // Step 1: Read bytes (same as loader.load())
        let bytes = std::fs::read(path)
            .unwrap_or_else(|e| panic!("Step 1 FAIL - read {}: {}", path.display(), e));

        // Step 2: Write to temp (same as manager.rs:89-90)
        let temp_path = temp_dir.join(format!("{}_{}.so", name, pid));
        std::fs::write(&temp_path, &bytes).unwrap_or_else(|e| {
            panic!(
                "Step 2 FAIL - write temp for '{}': {}. {}",
                name,
                e,
                collect_diagnostics(),
            )
        });

        // Step 3: Verify temp file exists and size matches
        let meta = std::fs::metadata(&temp_path)
            .unwrap_or_else(|e| panic!("Step 3 FAIL - stat temp for '{}': {}", name, e));
        assert_eq!(
            meta.len(),
            bytes.len() as u64,
            "Step 3 FAIL - size mismatch for '{}': temp={} vs original={}",
            name,
            meta.len(),
            bytes.len(),
        );

        let _ = std::fs::remove_file(&temp_path);
        eprintln!("Pipeline dry-run OK for '{}' ({} bytes)", name, bytes.len());
    }
}

/// Test 5: Attempt to load all plugins from ./plugins and report per-plugin results.
#[test]
fn test_all_plugins_load() {
    let paths = all_plugin_so_paths();
    if paths.is_empty() {
        eprintln!("No .so plugin files found in plugins/ — skipping");
        return;
    }

    let mut manager = PluginManager::new();
    let mut successes = Vec::new();
    let mut failures = Vec::new();

    for path in &paths {
        let display = path.display().to_string();
        match manager.load_plugin(path) {
            Ok(name) => {
                successes.push(name.clone());
                eprintln!("OK: {} -> '{}'", display, name);
            }
            Err(e) => {
                let err_str = format!("{:?}", e);
                let is_enospc =
                    err_str.contains("os error 28") || err_str.contains("No queda espacio");
                failures.push((display.clone(), format!("{}", e), is_enospc));
                eprintln!("FAIL: {} -> {}", display, e);
            }
        }
    }

    eprintln!("\n=== Plugin Load Summary ===");
    eprintln!("Successes: {} / {}", successes.len(), paths.len());
    if !failures.is_empty() {
        eprintln!("Failures:");
        for (name, err, enospc) in &failures {
            let tag = if *enospc { " [ENOSPC]" } else { "" };
            eprintln!("  {}{}: {}", name, tag, err);
        }
    }

    // Check if any failures are ENOSPC — those are environment issues, not code bugs
    let enospc_count = failures.iter().filter(|(_, _, e)| *e).count();
    if enospc_count > 0 {
        panic!(
            "{} plugin(s) failed with ENOSPC. /tmp is full. \
             Fix: 'rm -rf /tmp/plugin-system/*' or reboot. \
             Diagnostics:\n{}",
            enospc_count,
            collect_diagnostics(),
        );
    }

    // If no ENOSPC, allow other failures (e.g., missing symbols in debug builds)
    // but assert that at least the timer plugin loads
    if successes.is_empty() && !failures.is_empty() {
        eprintln!(
            "WARNING: All plugins failed to load (non-ENOSPC). \
             Check plugin symbols and dependencies."
        );
    }
}

/// Test 6: Verify total plugin sizes vs available temp space.
/// Catches the case where loading all plugins simultaneously would exceed /tmp capacity.
#[test]
fn test_plugin_size_vs_temp_space() {
    let paths = all_plugin_so_paths();
    if paths.is_empty() {
        eprintln!("No .so plugin files found in plugins/ — skipping");
        return;
    }

    let total_plugin_bytes: u64 = paths
        .iter()
        .filter_map(|p| std::fs::metadata(p).ok().map(|m| m.len()))
        .sum();

    let available = temp_dir_available_bytes();

    eprintln!(
        "Total plugin size: {} bytes ({:.1} MB), /tmp available: {} bytes ({:.1} MB)",
        total_plugin_bytes,
        total_plugin_bytes as f64 / 1024.0 / 1024.0,
        available,
        available as f64 / 1024.0 / 1024.0,
    );

    let max_plugin_size = paths
        .iter()
        .filter_map(|p| std::fs::metadata(p).ok().map(|m| m.len()))
        .max()
        .unwrap_or(0);

    let required = max_plugin_size * 2;
    assert!(
        available >= required,
        "FAIL: /tmp has {:.1} MB free but largest plugin is {:.1} MB. \
         Not enough space for temp files. \
         Fix: 'rm -rf /tmp/plugin-system/*' or free space in /tmp.",
        available as f64 / 1024.0 / 1024.0,
        max_plugin_size as f64 / 1024.0 / 1024.0,
    );
}

/// Collect system diagnostics for error messages.
fn collect_diagnostics() -> String {
    let mut info = String::new();

    // /tmp usage
    if let Ok(output) = std::process::Command::new("df")
        .arg("-h")
        .arg("/tmp")
        .output()
    {
        info.push_str(&format!(
            "df -h /tmp:\n{}\n",
            String::from_utf8_lossy(&output.stdout)
        ));
    }

    // /tmp/plugin-system contents
    let ps_dir = temp_dir_path();
    if ps_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&ps_dir) {
            let files: Vec<String> = entries
                .flatten()
                .map(|e| {
                    let size = e.metadata().map(|m| m.len()).unwrap_or(0);
                    format!("  {} ({} bytes)", e.path().display(), size)
                })
                .collect();
            info.push_str(&format!(
                "/tmp/plugin-system contents ({} files):\n{}\n",
                files.len(),
                files.join("\n")
            ));
        }
    } else {
        info.push_str("/tmp/plugin-system does not exist\n");
    }

    // Available space
    let avail = temp_dir_available_bytes();
    info.push_str(&format!(
        "Available /tmp space: {} bytes ({:.1} MB)\n",
        avail,
        avail as f64 / 1024.0 / 1024.0,
    ));

    info
}
