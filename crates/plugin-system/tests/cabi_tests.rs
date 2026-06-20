//! Integration tests for the C-ABI ("c-flat") plugin loader.
//!
//! These tests build a small C-ABI compatible shared library from inline Rust
//! (compiled on the fly via `cc` would be ideal, but to keep the dependency
//! surface small we instead build it as a separate test crate that exposes
//! the right C symbols). For a self-contained test, we use a small
//! hand-written Rust file that implements the C ABI and produce a `.so`/`.dll`
//! via cargo, then load it through `PluginManager`.

use std::path::PathBuf;

use plugin_system::PluginManager;

/// Locates the test fixture C-ABI plugin built by `tests/cabi-fixture/`.
fn fixture_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("cabi-fixture");
    path.push("target");
    for entry in walkdir(&path) {
        let p = entry.as_path();
        let fname = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if cfg!(target_os = "windows") {
            if fname.starts_with("cabi_fixture") && fname.ends_with(".dll") {
                return p.to_path_buf();
            }
        } else if cfg!(target_os = "macos") {
            if fname.starts_with("libcabi_fixture") && fname.ends_with(".dylib") {
                return p.to_path_buf();
            }
        } else if fname.starts_with("libcabi_fixture") && fname.ends_with(".so") {
            return p.to_path_buf();
        }
    }
    panic!("C-ABI test fixture not built; run `cargo build -p cabi-fixture` first");
}

/// Minimal recursive directory walker (avoids adding a `walkdir` dependency
/// to the test crate).
fn walkdir(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    fn visit(p: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        if let Ok(rd) = std::fs::read_dir(p) {
            for entry in rd.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    visit(&path, out);
                } else {
                    out.push(path);
                }
            }
        }
    }
    visit(root, &mut out);
    out
}

#[test]
fn cabi_manifest_helper_detects_abi_field() {
    let v: serde_json::Value = serde_json::json!({
        "name": "demo",
        "version": "0.1.0",
        "authors": ["Test"],
        "dependencies": [],
        "abi": "c-flat"
    });
    assert!(plugin_system::is_cabi_manifest(&v));
    let v2: serde_json::Value = serde_json::json!({
        "name": "demo",
        "version": "0.1.0",
        "authors": ["Test"],
        "dependencies": []
    });
    assert!(!plugin_system::is_cabi_manifest(&v2));
}

#[test]
fn cabi_manifest_parses_with_interfaces() {
    let v: serde_json::Value = serde_json::json!({
        "name": "demo",
        "version": "1.2.3",
        "authors": ["Alice"],
        "dependencies": [],
        "abi": "c-flat",
        "interfaces": ["Timer", "Counter"]
    });
    let m: plugin_system::CAbiManifest = serde_json::from_value(v).unwrap();
    assert_eq!(m.base.name, "demo");
    assert_eq!(m.abi.as_deref(), Some("c-flat"));
    assert_eq!(m.interfaces, vec!["Timer", "Counter"]);
}

#[test]
fn cabi_plugin_load_and_handle_command() {
    // Skip if the fixture hasn't been built (the test is opt-in).
    let lib = match std::env::var("SD_CABI_FIXTURE") {
        Ok(p) => PathBuf::from(p),
        Err(_) => {
            eprintln!("skipping: set SD_CABI_FIXTURE=/path/to/libcabi_fixture.so to enable");
            return;
        }
    };
    assert!(
        lib.exists(),
        "SD_CABI_FIXTURE points at a non-existent file: {}",
        lib.display()
    );

    // Ensure the sidecar `*.manifest.json` is present next to the DLL so the
    // host can auto-detect the C-ABI mode. We copy it from the tracked
    // location in the fixture source tree.
    let manifest_dst = lib.with_extension("manifest.json");
    if !manifest_dst.exists() {
        let mut src = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        src.push("tests/cabi-fixture/cabi_fixture.manifest.json");
        if src.exists() {
            std::fs::copy(&src, &manifest_dst).expect("copying sidecar manifest");
        } else {
            panic!(
                "sidecar manifest not found at {}; ensure tests/cabi-fixture/cabi_fixture.manifest.json is tracked",
                src.display()
            );
        }
    }

    let mut mgr = PluginManager::new();
    let name = mgr
        .load_plugin(&lib)
        .expect("loading C-ABI plugin should succeed");
    assert_eq!(name, "cabi-fixture");

    let metadata = mgr.plugin_metadata(&name).unwrap();
    assert_eq!(metadata.version, "1.2.3");

    // Call the plugin's `echo` command
    let response = mgr
        .with_plugin_mut(&name, |p| {
            p.handle_command("echo", serde_json::json!({"message": "hello"}))
        })
        .unwrap();
    assert_eq!(response, Some(serde_json::json!({"echoed": "hello"})));

    // Tear down
    mgr.unload_plugin(&name).unwrap();
}

#[test]
fn _ensure_fixture_path_compiles() {
    // This is a no-op; it just ensures `fixture_path` is referenced so
    // dead-code analysis doesn't complain when the test is filtered out.
    let _ = fixture_path;
}
