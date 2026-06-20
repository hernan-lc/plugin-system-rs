use anyhow::{Context, Result};
use cargo_metadata::MetadataCommand;
use clap::{Parser, Subcommand};
use colored::Colorize;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

mod packaging;

use crate::packaging::format::{parse_format_list, Format};

/// Run a command, transparently resolving `.cmd` / `.bat` shims on Windows
/// (so `npm`, `npx`, etc. work even when `C:\Program Files\nodejs` isn't on
/// the current `PATH`).
fn run_cmd(program: &str, args: &[&str], cwd: Option<&Path>) -> Result<std::process::Output> {
    let resolved = resolve_program(program);
    let mut cmd = Command::new(&resolved);
    cmd.args(args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let output = cmd.output().with_context(|| {
        format!(
            "failed to spawn `{}` (resolved to `{}`)",
            program,
            resolved.display()
        )
    })?;
    Ok(output)
}

fn resolve_program(program: &str) -> std::path::PathBuf {
    #[cfg(target_os = "windows")]
    {
        // First, try the program as-is (handles absolute paths and programs
        // already on PATH with an extension like `.exe`).
        let path = Path::new(program);
        if path.is_absolute() || program.contains(std::path::MAIN_SEPARATOR) {
            return path.to_path_buf();
        }
        // Otherwise, look up via `where.exe` and try each extension in PATHEXT.
        if let Some(found) = which_ext(program) {
            return found;
        }
    }
    PathBuf::from(program)
}

#[cfg(target_os = "windows")]
fn which_ext(program: &str) -> Option<PathBuf> {
    let pathext = std::env::var_os("PATHEXT").unwrap_or_default();
    let exts: Vec<String> = pathext
        .to_string_lossy()
        .split(';')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    // If `program` already has one of the extensions, just return it.
    let program_upper = program.to_ascii_uppercase();
    if exts
        .iter()
        .any(|e| program_upper.ends_with(&e.to_ascii_uppercase()))
    {
        return Some(PathBuf::from(program));
    }
    // Look up each `<program><ext>` in PATH.
    for ext in &exts {
        if let Ok(found) = which::which(format!("{program}{ext}")) {
            return Some(found);
        }
    }
    None
}

#[derive(Parser)]
#[command(name = "sd-plugins", about = "StreamDeck Plugin Build CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build all or specific plugins
    Build {
        /// Build in release mode
        #[arg(short, long)]
        release: bool,

        /// Build specific plugin(s)
        #[arg(short, long)]
        package: Vec<String>,

        /// Target triple for cross-compilation
        #[arg(short, long)]
        target: Option<String>,

        /// Also build the web frontend
        #[arg(long)]
        with_web: bool,

        /// Also build the sd-core binary
        #[arg(long)]
        with_core: bool,
    },

    /// List all discovered plugins
    List,

    /// Clean build artifacts
    Clean,

    /// Package plugins for distribution
    Package {
        /// Version string
        #[arg(short, long)]
        version: String,

        /// Output directory
        #[arg(short, long, default_value = "releases")]
        output: String,

        /// Target platform id (linux-x64, linux-arm64, windows-x64, windows-arm64,
        /// macos-x64, macos-arm64). Defaults to the host platform.
        #[arg(short, long)]
        platform: Option<String>,

        /// Comma-separated list of formats to produce (tar.gz, zip, deb, rpm,
        /// appimage, msi, nsis, dmg, pkg). Defaults to the formats configured
        /// in `packaging.toml` for the selected platform.
        #[arg(short, long, value_delimiter = ',')]
        formats: Option<Vec<String>>,

        /// Build every platform defined in the matrix using the artifacts that
        /// already exist in `target/<triple>/release/`. Useful in CI.
        #[arg(long)]
        all_platforms: bool,

        /// Build the core + plugins for the requested target triple before
        /// packaging. Implies `cargo build --release --target <triple>` for the
        /// host-only case and for the current host.
        #[arg(long)]
        build: bool,
    },

    /// Validate plugin configurations
    Check,

    /// Watch plugins and auto-rebuild, then run a command
    Dev {
        /// Build in release mode
        #[arg(short, long)]
        release: bool,

        /// Command to run after building plugins
        #[arg(required = true, last = true)]
        command: Vec<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build {
            release,
            package,
            target,
            with_web,
            with_core,
        } => cmd_build(release, package, target, with_web, with_core),
        Commands::List => cmd_list(),
        Commands::Clean => cmd_clean(),
        Commands::Package {
            version,
            output,
            platform,
            formats,
            all_platforms,
            build,
        } => cmd_package(&version, &output, platform, formats, all_platforms, build),
        Commands::Check => cmd_check(),
        Commands::Dev { release, command } => cmd_dev(release, command),
    }
}

fn find_workspace_root() -> Result<PathBuf> {
    let metadata = MetadataCommand::new()
        .exec()
        .context("Failed to read Cargo workspace metadata")?;

    Ok(metadata.workspace_root.into_std_path_buf())
}

fn discover_plugins(workspace_root: &Path) -> Result<Vec<PluginInfo>> {
    let metadata = MetadataCommand::new()
        .current_dir(workspace_root)
        .exec()
        .context("Failed to read Cargo workspace metadata")?;

    let mut plugins = Vec::new();

    for package in &metadata.packages {
        let manifest_str = package.manifest_path.to_string();
        if manifest_str.contains("/plugins/plugin-") || manifest_str.contains("\\plugins\\plugin-")
        {
            let is_cdylib = package
                .targets
                .iter()
                .any(|t| t.kind.iter().any(|k| k == "cdylib" || k == "lib"));

            if is_cdylib {
                let dir_name = package
                    .manifest_path
                    .parent()
                    .unwrap()
                    .file_name()
                    .unwrap()
                    .to_string();

                let lib_name = package.name.replace('-', "_");

                plugins.push(PluginInfo {
                    name: package.name.clone(),
                    dir_name,
                    lib_name,
                    version: package.version.to_string(),
                    manifest_path: package.manifest_path.clone().into_std_path_buf(),
                });
            }
        }
    }

    plugins.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(plugins)
}

fn get_plugin_lib_filename(lib_name: &str, target: &str) -> String {
    if target.contains("windows") {
        format!("{}.dll", lib_name)
    } else if target.contains("apple") || target.contains("darwin") {
        format!("lib{}.dylib", lib_name)
    } else {
        format!("lib{}.so", lib_name)
    }
}

fn get_host_target() -> Result<String> {
    let output = Command::new("rustc")
        .args(["-Vv"])
        .output()
        .context("Failed to run rustc")?;

    let stdout = String::from_utf8(output.stdout)?;
    for line in stdout.lines() {
        if let Some(triple) = line.strip_prefix("host:") {
            return Ok(triple.trim().to_string());
        }
    }

    let host = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        ("windows", "aarch64") => "aarch64-pc-windows-msvc",
        _ => anyhow::bail!(
            "Unsupported host platform: {}-{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ),
    };

    Ok(host.to_string())
}

fn cmd_build(
    release: bool,
    packages: Vec<String>,
    target: Option<String>,
    with_web: bool,
    with_core: bool,
) -> Result<()> {
    let workspace_root = find_workspace_root()?;
    let plugins = discover_plugins(&workspace_root)?;

    let target_triple = target.unwrap_or_else(|| get_host_target().unwrap_or_default());
    let profile_flag = if release { "--release" } else { "" };

    println!("{}", "=== StreamDeck Plugin Builder ===".cyan().bold());
    println!("Target: {}", target_triple.yellow());
    println!("Mode: {}", if release { "release" } else { "debug" });
    println!();

    // Build web frontend if requested
    if with_web {
        println!("{}", "Building web frontend...".yellow());
        let web_dir = workspace_root.join("web");
        if web_dir.exists() {
            let status = run_cmd("npm", &["ci"], Some(&web_dir))?.status;
            if !status.success() {
                anyhow::bail!("npm ci failed");
            }

            let status = run_cmd("npm", &["run", "build"], Some(&web_dir))?.status;
            if !status.success() {
                anyhow::bail!("npm build failed");
            }
            println!("  {}", "Web frontend built".green());
        }
        println!();
    }

    // Build core binary if requested
    if with_core {
        println!("{}", "Building sd-core binary...".yellow());
        let mut args = vec!["build"];
        if !profile_flag.is_empty() {
            args.push(profile_flag);
        }
        if target_triple != get_host_target().unwrap_or_default() {
            args.push("--target");
            args.push(&target_triple);
        }
        args.push("-p");
        args.push("sd-core");

        let status = Command::new("cargo")
            .args(&args)
            .current_dir(&workspace_root)
            .status()
            .context("Failed to build sd-core")?;

        if !status.success() {
            anyhow::bail!("Failed to build sd-core");
        }
        println!("  {}", "sd-core built".green());
        println!();
    }

    // Filter plugins if specific packages requested
    let plugins_to_build: Vec<&PluginInfo> = if packages.is_empty() {
        plugins.iter().collect()
    } else {
        plugins
            .iter()
            .filter(|p| packages.contains(&p.name))
            .collect()
    };

    if plugins_to_build.is_empty() {
        println!("{}", "No plugins found to build".yellow());
        return Ok(());
    }

    println!("Building {} plugin(s):", plugins_to_build.len());
    for plugin in &plugins_to_build {
        println!("  - {} ({})", plugin.name.cyan(), plugin.version);
    }
    println!();

    let mut built = 0;
    let mut failed = 0;

    for plugin in &plugins_to_build {
        print!("Building {}... ", plugin.name.cyan());

        let mut args = vec!["build"];
        if !profile_flag.is_empty() {
            args.push(profile_flag);
        }
        if target_triple != get_host_target().unwrap_or_default() {
            args.push("--target");
            args.push(&target_triple);
        }
        args.push("--lib");
        args.push("-p");
        args.push(&plugin.name);

        let status = Command::new("cargo")
            .args(&args)
            .current_dir(&workspace_root)
            .status()
            .context(format!("Failed to build {}", plugin.name))?;

        if status.success() {
            println!("{}", "OK".green());
            built += 1;

            // Copy plugin to plugins/ directory
            let lib_filename = get_plugin_lib_filename(&plugin.lib_name, &target_triple);
            let profile = if release { "release" } else { "debug" };
            let src_dir = if target_triple == get_host_target().unwrap_or_default() {
                workspace_root.join(format!("target/{}", profile))
            } else {
                workspace_root.join(format!("target/{}/{}", target_triple, profile))
            };
            let src = src_dir.join(&lib_filename);
            let dst = workspace_root.join("plugins").join(&lib_filename);

            if src.exists() {
                std::fs::copy(&src, &dst)
                    .context(format!("Failed to copy {} to plugins/", lib_filename))?;
                println!("    -> {}", dst.display());
            }
        } else {
            println!("{}", "FAILED".red());
            failed += 1;
        }
    }

    println!();
    println!(
        "Result: {} built, {} failed",
        built.to_string().green(),
        if failed > 0 {
            failed.to_string().red()
        } else {
            "0".normal()
        }
    );

    if failed > 0 {
        anyhow::bail!("{} plugin(s) failed to build", failed);
    }

    Ok(())
}

fn cmd_list() -> Result<()> {
    let workspace_root = find_workspace_root()?;
    let plugins = discover_plugins(&workspace_root)?;

    println!("{}", "=== Discovered Plugins ===".cyan().bold());
    println!();

    if plugins.is_empty() {
        println!("No plugins found in plugins/ directory");
        return Ok(());
    }

    for plugin in &plugins {
        println!("  {} ({})", plugin.name.cyan().bold(), plugin.version);
        println!("    Directory: {}", plugin.dir_name);
        println!("    Library:   {}", plugin.lib_name);
        println!("    Manifest:  {}", plugin.manifest_path.display());
        println!();
    }

    println!("Total: {} plugin(s)", plugins.len());

    Ok(())
}

fn cmd_clean() -> Result<()> {
    let workspace_root = find_workspace_root()?;

    println!("{}", "Cleaning build artifacts...".yellow());

    // Clean target directory
    let status = Command::new("cargo")
        .args(["clean"])
        .current_dir(&workspace_root)
        .status()
        .context("Failed to run cargo clean")?;

    if status.success() {
        println!("  {}", "target/ cleaned".green());
    }

    // Remove plugin binaries from plugins/
    let plugins_dir = workspace_root.join("plugins");
    if plugins_dir.exists() {
        for entry in std::fs::read_dir(&plugins_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy();
                if ext_str == "so" || ext_str == "dylib" || ext_str == "dll" {
                    std::fs::remove_file(&path)?;
                    println!("  Removed {}", path.display());
                }
            }
        }
    }

    println!("{}", "Clean complete".green());

    Ok(())
}

fn cmd_package(
    version: &str,
    output_dir: &str,
    platform: Option<String>,
    formats: Option<Vec<String>>,
    all_platforms: bool,
    build: bool,
) -> Result<()> {
    use crate::packaging::format::{is_valid_platform, platform_from_target, PLATFORMS};
    use crate::packaging::package_release;

    let workspace_root = find_workspace_root()?;
    let host_target = get_host_target()?;

    println!("{}", "=== StreamDeck Packaging ===".cyan().bold());
    println!("Version: {}", version.yellow());
    println!("Host target: {}", host_target.yellow());
    println!();

    let output_root = workspace_root.join(output_dir).join(version);

    // Determine the list of (platform, source_target, formats) to process
    let targets: Vec<(String, Option<String>, Vec<Format>)> = if all_platforms {
        PLATFORMS
            .iter()
            .map(|p| {
                let triple =
                    crate::packaging::format::platform_default_target(p).map(str::to_string);
                let fmts = default_formats_for_platform(p, &formats);
                (p.to_string(), triple, fmts)
            })
            .collect()
    } else {
        let chosen = platform
            .or_else(|| platform_from_target(&host_target).map(str::to_string))
            .context("could not determine platform; pass --platform")?;
        if !is_valid_platform(&chosen) {
            anyhow::bail!("unknown platform `{chosen}`; expected one of {PLATFORMS:?}");
        }
        let triple = crate::packaging::format::platform_default_target(&chosen).map(str::to_string);
        let fmts = default_formats_for_platform(&chosen, &formats);
        vec![(chosen, triple, fmts)]
    };

    // Optionally build first
    if build {
        for (plat, triple_opt, _) in &targets {
            let triple = match triple_opt {
                Some(t) => t.clone(),
                None => host_target.clone(),
            };
            build_for_target(&workspace_root, &triple)?;
            let _ = plat;
        }
        println!();
    }

    let mut total = 0usize;
    for (plat, triple_opt, fmts) in targets {
        if fmts.is_empty() {
            eprintln!(
                "  {} no formats configured for platform `{}`",
                "skip:".yellow(),
                plat
            );
            continue;
        }
        let platform_dir = output_root.join(&plat);
        std::fs::create_dir_all(&platform_dir)?;
        match package_release(
            &workspace_root,
            version,
            &platform_dir,
            &plat,
            &fmts,
            triple_opt.as_deref(),
        ) {
            Ok(artifacts) => {
                total += artifacts.len();
            }
            Err(e) => {
                eprintln!("  {} platform `{}` failed: {e:#}", "error:".red(), plat);
                return Err(e);
            }
        }
        println!();
    }

    println!(
        "{} {} artifact(s) produced under {}",
        "Done.".green().bold(),
        total.to_string().cyan(),
        output_root.display()
    );
    Ok(())
}

fn default_formats_for_platform(platform: &str, explicit: &Option<Vec<String>>) -> Vec<Format> {
    if let Some(list) = explicit {
        return list
            .iter()
            .filter_map(|s| s.parse::<Format>().ok())
            .collect();
    }
    let cfg = match crate::packaging::config::load(&find_workspace_root().unwrap_or_default()) {
        Ok(c) => c,
        Err(_) => return vec![Format::TarGz],
    };
    let list = match platform {
        p if p.starts_with("linux") => &cfg.formats.linux,
        p if p.starts_with("windows") => &cfg.formats.windows,
        p if p.starts_with("macos") => &cfg.formats.macos,
        _ => return vec![Format::TarGz],
    };
    list.iter()
        .filter_map(|s| parse_format_list(s).ok())
        .flatten()
        .collect()
}

fn build_for_target(workspace_root: &Path, triple: &str) -> Result<()> {
    println!("  {} building for {}...", "build:".yellow(), triple.cyan());
    let status = Command::new("cargo")
        .args(["build", "--release", "--target", triple, "-p", "sd-core"])
        .current_dir(workspace_root)
        .status()
        .with_context(|| format!("building sd-core for {triple}"))?;
    if !status.success() {
        anyhow::bail!("cargo build for {triple} failed");
    }
    let status = Command::new("cargo")
        .args(["build", "--release", "--target", triple])
        .current_dir(workspace_root)
        .status()
        .with_context(|| format!("building plugins for {triple}"))?;
    if !status.success() {
        anyhow::bail!("cargo build (workspace) for {triple} failed");
    }
    Ok(())
}

fn cmd_check() -> Result<()> {
    let workspace_root = find_workspace_root()?;
    let plugins = discover_plugins(&workspace_root)?;

    println!("{}", "=== Checking Plugins ===".cyan().bold());
    println!();

    let mut errors = 0;

    for plugin in &plugins {
        print!("{}... ", plugin.name.cyan());

        // Check Cargo.toml exists
        if !plugin.manifest_path.exists() {
            println!("{} Cargo.toml not found", "ERROR".red());
            errors += 1;
            continue;
        }

        // Check src/ directory exists
        let src_dir = plugin.manifest_path.parent().unwrap().join("src");
        if !src_dir.exists() {
            println!("{} src/ directory not found", "ERROR".red());
            errors += 1;
            continue;
        }

        // Check for lib.rs or main.rs
        let has_entry = src_dir.join("lib.rs").exists() || src_dir.join("main.rs").exists();
        if !has_entry {
            println!("{} no lib.rs or main.rs found", "ERROR".red());
            errors += 1;
            continue;
        }

        println!("{}", "OK".green());
    }

    println!();
    if errors == 0 {
        println!(
            "{} All {} plugin(s) passed validation",
            "✓".green().bold(),
            plugins.len()
        );
    } else {
        println!(
            "{} {} plugin(s) failed validation",
            "✗".red().bold(),
            errors
        );
        anyhow::bail!("Validation failed");
    }

    Ok(())
}

fn cmd_dev(release: bool, command: Vec<String>) -> Result<()> {
    let workspace_root = find_workspace_root()?;
    let target_triple = get_host_target()?;

    println!("{}", "=== StreamDeck Dev Mode ===".cyan().bold());
    println!("Watching plugins for changes...");
    println!("Command: {}", command.join(" ").yellow());
    println!();

    // Initial build of all plugins
    println!("{}", "Building plugins...".yellow());
    build_all_plugins(&workspace_root, &target_triple, release)?;
    println!();

    // Spawn the user's command
    println!("{}", "Starting application...".green());
    let mut child = spawn_command(&command)?;

    // Set up file watcher
    let (tx, rx) = mpsc::channel();
    let mut watcher: RecommendedWatcher =
        Watcher::new(tx, notify::Config::default()).context("Failed to create file watcher")?;

    let watch_dirs = get_watch_dirs(&workspace_root);
    for dir in &watch_dirs {
        if dir.exists() {
            watcher
                .watch(dir.as_path(), RecursiveMode::Recursive)
                .context(format!("Failed to watch {}", dir.display()))?;
            println!("  Watching: {}", dir.display().to_string().dimmed());
        }
    }
    println!();
    println!("{}", "Press Ctrl+C to stop".dimmed());
    println!();

    // Debounce loop
    let mut last_build = std::time::Instant::now();
    let debounce = Duration::from_millis(500);

    loop {
        match rx.recv() {
            Ok(Ok(event)) => {
                if !is_relevant_event(&event) {
                    continue;
                }

                // Debounce: skip if we just built
                if last_build.elapsed() < debounce {
                    continue;
                }

                let paths = get_changed_paths(&event);
                if paths.is_empty() {
                    continue;
                }

                println!("\n{}", "Changes detected, rebuilding...".yellow().bold());
                for p in &paths {
                    println!("  Changed: {}", p.display().to_string().dimmed());
                }

                // Determine affected plugins
                let affected = determine_affected_plugins(&paths);
                if affected.is_empty() {
                    println!("  {}", "No plugin changes, skipping rebuild".dimmed());
                    continue;
                }

                // Rebuild affected plugins
                match build_plugins(&workspace_root, &target_triple, release, &affected) {
                    Ok(()) => {
                        last_build = std::time::Instant::now();

                        // Kill old process and respawn
                        println!("{}", "Restarting application...".green());
                        child.kill().ok();
                        child.wait().ok();
                        child = spawn_command(&command)?;
                    }
                    Err(e) => {
                        println!("{} {}", "Build failed:".red(), e);
                        println!("  {}", "Waiting for more changes...".dimmed());
                    }
                }
            }
            Ok(Err(e)) => {
                println!("{} {}", "Watch error:".red(), e);
            }
            Err(e) => {
                println!("{} {}", "Channel error:".red(), e);
                break;
            }
        }
    }

    child.kill().ok();
    child.wait().ok();
    Ok(())
}

fn get_watch_dirs(workspace_root: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // Watch plugin source directories
    let plugins_dir = workspace_root.join("plugins");
    if plugins_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&plugins_dir) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    let src_dir = entry.path().join("src");
                    if src_dir.exists() {
                        dirs.push(src_dir);
                    }
                }
            }
        }
    }

    // Watch shared plugin crates
    let system_src = workspace_root.join("crates/plugin-system/src");
    if system_src.exists() {
        dirs.push(system_src);
    }

    let macros_src = workspace_root.join("crates/plugin-macros/src");
    if macros_src.exists() {
        dirs.push(macros_src);
    }

    dirs
}

fn is_relevant_event(event: &Event) -> bool {
    matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}

fn get_changed_paths(event: &Event) -> Vec<PathBuf> {
    event
        .paths
        .iter()
        .filter(|p| {
            // Only care about .rs and .toml files
            p.extension()
                .map(|ext| ext == "rs" || ext == "toml")
                .unwrap_or(false)
        })
        .cloned()
        .collect()
}

fn determine_affected_plugins(paths: &[PathBuf]) -> Vec<String> {
    let mut affected = Vec::new();
    let mut rebuild_all = false;

    for path in paths {
        let path_str = path.to_string_lossy();

        // Changes in shared crates affect ALL plugins
        if path_str.contains("crates/plugin-system/") || path_str.contains("crates/plugin-macros/")
        {
            rebuild_all = true;
            break;
        }

        // Changes in a specific plugin
        if let Some(plugin_name) = extract_plugin_name(&path_str) {
            if !affected.contains(&plugin_name) {
                affected.push(plugin_name);
            }
        }
    }

    if rebuild_all {
        return vec!["__all__".to_string()];
    }

    affected
}

fn extract_plugin_name(path: &str) -> Option<String> {
    // Extract plugin name from path like ".../plugins/plugin-volume-master/src/..."
    if let Some(start) = path.find("/plugins/plugin-") {
        let rest = &path[start + "/plugins/plugin-".len()..];
        if let Some(end) = rest.find('/') {
            let name = &rest[..end];
            return Some(format!("plugin-{}", name));
        }
    }
    None
}

fn build_all_plugins(workspace_root: &Path, target_triple: &str, release: bool) -> Result<()> {
    let plugins = discover_plugins(workspace_root)?;
    let plugins_refs: Vec<&PluginInfo> = plugins.iter().collect();
    build_plugins_with_info(workspace_root, target_triple, release, &plugins_refs)
}

fn build_plugins(
    workspace_root: &Path,
    target_triple: &str,
    release: bool,
    affected: &[String],
) -> Result<()> {
    let all_plugins = discover_plugins(workspace_root)?;

    let plugins_to_build: Vec<&PluginInfo> = if affected.contains(&"__all__".to_string()) {
        all_plugins.iter().collect()
    } else {
        all_plugins
            .iter()
            .filter(|p| affected.contains(&p.name))
            .collect()
    };

    if plugins_to_build.is_empty() {
        return Ok(());
    }

    build_plugins_with_info(workspace_root, target_triple, release, &plugins_to_build)
}

fn build_plugins_with_info(
    workspace_root: &Path,
    target_triple: &str,
    release: bool,
    plugins: &[&PluginInfo],
) -> Result<()> {
    let profile_flag = if release { "--release" } else { "" };
    let host_target = get_host_target().unwrap_or_default();
    let mut built = 0;

    for plugin in plugins {
        print!("  Building {}... ", plugin.name.cyan());

        let mut args = vec!["build"];
        if !profile_flag.is_empty() {
            args.push(profile_flag);
        }
        if target_triple != host_target {
            args.push("--target");
            args.push(target_triple);
        }
        args.push("--lib");
        args.push("-p");
        args.push(&plugin.name);

        let status = Command::new("cargo")
            .args(&args)
            .current_dir(workspace_root)
            .status()
            .context(format!("Failed to build {}", plugin.name))?;

        if status.success() {
            println!("{}", "OK".green());
            built += 1;

            // Copy plugin to plugins/ directory
            let lib_filename = get_plugin_lib_filename(&plugin.lib_name, target_triple);
            let profile = if release { "release" } else { "debug" };
            let src_dir = if target_triple == host_target {
                workspace_root.join(format!("target/{}", profile))
            } else {
                workspace_root.join(format!("target/{}/{}", target_triple, profile))
            };
            let src = src_dir.join(&lib_filename);
            let dst = workspace_root.join("plugins").join(&lib_filename);

            if src.exists() {
                std::fs::copy(&src, &dst)
                    .context(format!("Failed to copy {} to plugins/", lib_filename))?;
            }
        } else {
            println!("{}", "FAILED".red());
            anyhow::bail!("Failed to build {}", plugin.name);
        }
    }

    println!("  Built {} plugin(s)", built.to_string().green());
    Ok(())
}

fn spawn_command(command: &[String]) -> Result<std::process::Child> {
    let program = command.first().context("No command specified")?;
    let args = &command[1..];

    let child = Command::new(program)
        .args(args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .context(format!("Failed to spawn command: {}", command.join(" ")))?;

    Ok(child)
}

struct PluginInfo {
    name: String,
    dir_name: String,
    lib_name: String,
    version: String,
    manifest_path: PathBuf,
}
