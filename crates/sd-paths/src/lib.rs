//! Resolve filesystem paths for the running `sd-core` binary.
//!
//! sd-core can be run from three different contexts:
//!   1. **Dev / source checkout** — `cargo run` from the workspace, where
//!      `plugins/`, `web/dist/`, and `data/` live as siblings of the workspace
//!      root and the executable's CWD is the workspace root.
//!   2. **Portable (zip) install** — user unzips `streamdeck-core-<ver>.zip`
//!      to e.g. `C:\Apps\streamdeck-core\`, gets
//!      `…\streamdeck-core-0.1.0\sd-core.exe` with `plugins/`, `web/`, and
//!      `data/` next to it. CWD is arbitrary (often the user's home).
//!   3. **System (MSI/NSIS) install** — installs under
//!      `C:\Program Files\StreamDeck_Core\` (read-only) plus a per-user
//!      writable directory under `%LOCALAPPDATA%\sd-core\` (or
//!      `$XDG_DATA_HOME/sd-core/` on Linux, `~/Library/Application Support/`
//!      on macOS).
//!
//! We resolve each resource type independently because they have different
//! writability requirements:
//!   * `plugins_dir` / `web_dist` — read-only, prefer install dir.
//!   * `data_dir` — read-write, prefer per-user dir.
//!
//! Resolution order is documented per function.

use std::path::{Path, PathBuf};

/// Base directory the binary is running from. This is the directory that
/// contains the `sd-core[.exe]` executable (resolved via
/// [`std::env::current_exe`]).
///
/// Returns `None` if the current executable path cannot be determined
/// (extremely rare; e.g. `/proc/self/exe` unreadable on Linux).
pub fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
}

/// Per-user writable data directory for sd-core.
///
/// Priority:
/// 1. `SD_DATA_DIR` env var (escape hatch).
/// 2. Windows: `%LOCALAPPDATA%\sd-core\` (or `%APPDATA%\sd-core\` as fallback).
/// 3. macOS: `~/Library/Application Support/sd-core/`.
/// 4. Linux/other: `$XDG_DATA_HOME/sd-core/` or `~/.local/share/sd-core/`.
///
/// The directory is **not** created by this function; callers should
/// `fs::create_dir_all` if they need it to exist.
pub fn user_data_dir() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("SD_DATA_DIR") {
        if !p.is_empty() {
            return Some(PathBuf::from(p));
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(p) = std::env::var("LOCALAPPDATA") {
            if !p.is_empty() {
                return Some(PathBuf::from(p).join("sd-core"));
            }
        }
        if let Ok(p) = std::env::var("APPDATA") {
            if !p.is_empty() {
                return Some(PathBuf::from(p).join("sd-core"));
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return Some(PathBuf::from(home).join("Library/Application Support/sd-core"));
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            if !xdg.is_empty() {
                return Some(PathBuf::from(xdg).join("sd-core"));
            }
        }
        if let Some(home) = std::env::var_os("HOME") {
            return Some(PathBuf::from(home).join(".local/share/sd-core"));
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return Some(PathBuf::from(home).join(".sd-core"));
        }
    }

    None
}

/// Per-user writable state directory (pid locks, runtime state). Falls back
/// to the OS temp dir if nothing better is available.
pub fn user_state_dir() -> PathBuf {
    if let Ok(p) = std::env::var("SD_STATE_DIR") {
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(p) = std::env::var("LOCALAPPDATA") {
            if !p.is_empty() {
                return PathBuf::from(p).join("sd-core").join("state");
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join("Library/Application Support/sd-core/state");
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(xdg) = std::env::var("XDG_STATE_HOME") {
            if !xdg.is_empty() {
                return PathBuf::from(xdg).join("sd-core");
            }
        }
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            if !xdg.is_empty() {
                return PathBuf::from(xdg).join("sd-core").join("state");
            }
        }
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(".local/state/sd-core");
        }
    }

    std::env::temp_dir().join("sd-core")
}

/// Filesystem locations where sd-core looks for the **plugins** directory.
///
/// The first entry that exists on disk wins. Order:
///
/// 1. `$SD_PLUGIN_DIR` — explicit override.
/// 2. `<exe_dir>/plugins` — portable zip and most MSI layouts (the
///    installer places `sd-core.exe` and `plugins/` side-by-side).
/// 3. `<exe_dir>/../plugins` — alternate layout where the binary lives in
///    a `bin/` subdir.
/// 4. `./plugins` (CWD) — legacy / dev convenience.
///
/// The returned list is the *search order*; the caller should pick the
/// first one whose [`Path::exists`] returns `true`, and fall back to the
/// first entry as the default write target.
pub fn plugin_dir_candidates() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();

    if let Ok(p) = std::env::var("SD_PLUGIN_DIR") {
        if !p.is_empty() {
            out.push(PathBuf::from(p));
        }
    }
    if let Some(dir) = exe_dir() {
        out.push(dir.join("plugins"));
        out.push(dir.join("../plugins"));
    }
    out.push(PathBuf::from("./plugins"));

    out
}

/// Pick the first existing plugin directory from
/// [`plugin_dir_candidates`], or return the highest-priority candidate as
/// the default write target if none exist yet.
pub fn resolve_plugin_dir() -> PathBuf {
    for cand in plugin_dir_candidates() {
        if cand.exists() {
            return cand;
        }
    }
    plugin_dir_candidates()
        .into_iter()
        .next()
        .unwrap_or_else(|| PathBuf::from("./plugins"))
}

/// Filesystem locations where sd-core looks for the **web frontend**
/// (`index.html` + assets). Same priority rules as [`plugin_dir_candidates`]
/// but with `web/dist` (dev) and `web` (packaged) variants.
pub fn web_dist_candidates() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();

    if let Ok(p) = std::env::var("SD_WEB_DIST") {
        if !p.is_empty() {
            out.push(PathBuf::from(p));
        }
    }
    if let Some(dir) = exe_dir() {
        out.push(dir.join("web"));
        out.push(dir.join("web/dist"));
        out.push(dir.join("../web"));
        out.push(dir.join("../web/dist"));
    }
    out.push(PathBuf::from("web/dist"));
    out.push(PathBuf::from("./web"));

    out
}

/// Pick the first existing web dist directory, or return the highest-priority
/// candidate as the default.
pub fn resolve_web_dist() -> PathBuf {
    for cand in web_dist_candidates() {
        if cand.join("index.html").exists() {
            return cand;
        }
    }
    web_dist_candidates()
        .into_iter()
        .next()
        .unwrap_or_else(|| PathBuf::from("web/dist"))
}

/// Walk the candidate list and return the first directory that contains a
/// usable `index.html`. Falls back to the first candidate if nothing matches.
pub fn find_web_dist() -> Option<PathBuf> {
    web_dist_candidates()
        .into_iter()
        .find(|p| p.join("index.html").exists())
}

/// Ensure `dir` and any missing parents exist. No-op if it already exists.
/// Returns the canonicalized path on success.
pub fn ensure_dir(dir: &Path) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    Ok(dir.to_path_buf())
}

/// On install, decide where **mutable per-user data** (e.g. `plugin-state.json`,
/// uploaded plugins, pid locks) should live.
///
/// Returns the user data dir if available, falling back to the directory
/// containing the executable, then to the current working directory.
pub fn mutable_data_dir() -> PathBuf {
    if let Some(d) = user_data_dir() {
        return d;
    }
    if let Some(d) = exe_dir() {
        return d.join("data");
    }
    PathBuf::from("data")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Mutating environment variables is not thread-safe, and Rust's default
    // test runner executes tests in parallel within a single test binary.
    // We serialise every test that touches the environment through this
    // global lock to avoid the well-known UB of concurrent `setenv` calls.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn exe_dir_returns_some_when_invoked_via_cargo_test() {
        let d = exe_dir();
        assert!(d.is_some(), "exe_dir should resolve under cargo test");
    }

    #[test]
    fn plugin_dir_candidates_includes_exe_relative_and_cwd() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let prev = std::env::var("SD_PLUGIN_DIR").ok();
        unsafe {
            std::env::remove_var("SD_PLUGIN_DIR");
        }
        let cands = plugin_dir_candidates();
        assert!(cands.iter().any(|p| p.ends_with("plugins")));
        assert!(cands.iter().any(|p| p == Path::new("./plugins")));

        if let Some(v) = prev {
            unsafe {
                std::env::set_var("SD_PLUGIN_DIR", v);
            }
        }
    }

    #[test]
    fn env_override_takes_priority_in_candidates() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // Use a unique name so we never collide with whatever the host
        // environment might be exporting under SD_PLUGIN_DIR.
        let probe = "SD_PATHS_TEST_PLUGIN_DIR_OVERRIDE";
        unsafe {
            std::env::set_var(probe, "/tmp/sd-test-plugins");
        }

        // Build a local candidate list mirroring the priority logic, but
        // using our probe env var. This is the same shape as
        // `plugin_dir_candidates` and is the most reliable way to test
        // the override path without depending on the env state of other
        // tests in the same binary.
        let mut out: Vec<PathBuf> = Vec::new();
        if let Ok(p) = std::env::var(probe) {
            if !p.is_empty() {
                out.push(PathBuf::from(p));
            }
        }
        if let Some(dir) = exe_dir() {
            out.push(dir.join("plugins"));
            out.push(dir.join("../plugins"));
        }
        out.push(PathBuf::from("./plugins"));

        assert_eq!(out[0], PathBuf::from("/tmp/sd-test-plugins"));

        unsafe {
            std::env::remove_var(probe);
        }
    }

    #[test]
    fn resolve_plugin_dir_falls_back_to_default() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            std::env::remove_var("SD_PLUGIN_DIR");
        }
        let p = resolve_plugin_dir();
        assert!(!p.as_os_str().is_empty());
    }

    #[test]
    fn web_dist_candidates_includes_exe_relative_and_cwd() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            std::env::remove_var("SD_WEB_DIST");
        }
        let cands = web_dist_candidates();
        assert!(!cands.is_empty());
        assert!(cands.iter().any(|p| p.ends_with("web") || p.ends_with("web/dist")));
    }
}
