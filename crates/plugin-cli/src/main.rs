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

        /// Target triple for the sd-core binary. Plugins ignore this: they
        /// are always built for wasm32-wasip2.
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

/// The target every plugin is built for.
const WASM_TARGET: &str = "wasm32-wasip2";

/// Find the plugin crates under `plugins/`.
///
/// These are deliberately outside the cargo workspace — their `wit-bindgen`
/// glue emits wasm-only imports that cannot link into a host binary, so a
/// workspace-wide `cargo build` would fail on them. That means workspace
/// metadata cannot see them and the directory has to be walked directly, with
/// one `cargo metadata` call per crate to read its name and version.
fn discover_plugins(workspace_root: &Path) -> Result<Vec<PluginInfo>> {
    let plugins_dir = workspace_root.join("plugins");
    if !plugins_dir.exists() {
        return Ok(Vec::new());
    }

    let mut plugins = Vec::new();

    for entry in std::fs::read_dir(&plugins_dir)
        .with_context(|| format!("Failed to read {}", plugins_dir.display()))?
    {
        let entry = entry?;
        let dir = entry.path();
        let manifest_path = dir.join("Cargo.toml");
        if !manifest_path.is_file() {
            continue;
        }

        let metadata = MetadataCommand::new()
            .manifest_path(&manifest_path)
            .no_deps()
            .exec()
            .with_context(|| format!("Failed to read metadata for {}", manifest_path.display()))?;

        // `no_deps` on a standalone crate yields exactly that crate.
        let Some(package) = metadata.packages.first() else {
            continue;
        };

        // Test fixtures live alongside real plugins but must never be staged
        // into `plugins/`, or the app would load them at startup.
        let is_fixture = package
            .metadata
            .get("sd-plugins")
            .and_then(|m| m.get("fixture"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if is_fixture {
            continue;
        }

        plugins.push(PluginInfo {
            name: package.name.clone(),
            dir_name: entry.file_name().to_string_lossy().into_owned(),
            lib_name: package.name.replace('-', "_"),
            version: package.version.to_string(),
            manifest_path,
        });
    }

    plugins.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(plugins)
}

/// The artifact filename for a plugin.
///
/// One name, every platform. This used to take a target triple and return
/// `.so` / `.dylib` / `.dll`, which is why a release had to carry six copies
/// of each plugin.
fn plugin_artifact_filename(lib_name: &str) -> String {
    format!("{}.wasm", lib_name)
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

    // `--target` now only affects the host binary. Plugins are always built
    // for `wasm32-wasip2`: one artifact runs everywhere, which is the reason
    // the native ABIs were dropped.
    let target_triple = target.unwrap_or_else(|| get_host_target().unwrap_or_default());
    let profile_flag = if release { "--release" } else { "" };

    println!("{}", "=== StreamDeck Plugin Builder ===".cyan().bold());
    println!("Plugin target: {}", WASM_TARGET.yellow());
    if with_core {
        println!("Core target:   {}", target_triple.yellow());
    }
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

        match build_one_plugin(&workspace_root, plugin, release) {
            Ok(Some(dst)) => {
                println!("{}", "OK".green());
                println!("    -> {}", dst.display());
                built += 1;
            }
            Ok(None) => {
                println!("{}", "OK".green());
                println!(
                    "    {} cargo succeeded but no artifact was produced",
                    "warning:".yellow()
                );
                built += 1;
            }
            Err(e) => {
                println!("{}", "FAILED".red());
                eprintln!("    {e:#}");
                failed += 1;
            }
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

    // Plugin crates are outside the workspace, so `cargo clean` above did not
    // touch them; each has its own target directory.
    for plugin in discover_plugins(&workspace_root)? {
        let plugin_dir = plugin
            .manifest_path
            .parent()
            .expect("manifest path always has a parent");
        let status = Command::new("cargo")
            .args(["clean"])
            .current_dir(plugin_dir)
            .status();
        if matches!(status, Ok(s) if s.success()) {
            println!("  {} target/ cleaned", plugin.name);
        }
    }

    // Remove staged plugin artifacts from plugins/. Stale native libraries
    // from before the migration are swept up too, since nothing can load them.
    let plugins_dir = workspace_root.join("plugins");
    if plugins_dir.exists() {
        for entry in std::fs::read_dir(&plugins_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy();
                if matches!(ext_str.as_ref(), "wasm" | "so" | "dylib" | "dll") {
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

    // Optionally build first.
    //
    // Plugins are built once, not once per platform: a component is the same
    // file everywhere, which is the whole reason the native ABIs were dropped.
    // The host binary is the only per-target artifact, and cross-compiling it
    // needs a toolchain for the target that this machine may simply not have —
    // `sd-core` links wasmtime, so there is no cross build without one. Rather
    // than failing the whole run on the first foreign target, each is attempted
    // and a missing toolchain is reported as a skip.
    if build {
        println!("{}", "Building plugins (wasm32-wasip2)...".yellow());
        build_all_plugins(&workspace_root, true)?;
        println!();

        let mut built_any = false;
        for (plat, triple_opt, _) in &targets {
            let triple = triple_opt.clone().unwrap_or_else(|| host_target.clone());
            match build_host_for_target(&workspace_root, &triple, &host_target) {
                Ok(true) => built_any = true,
                Ok(false) => eprintln!(
                    "  {} platform `{}`: no toolchain for {}; packaging whatever is \
                     already in target/{}/release",
                    "skip:".yellow(),
                    plat,
                    triple,
                    triple
                ),
                Err(e) => return Err(e),
            }
        }

        if !built_any {
            eprintln!(
                "  {} no host binary was built for any requested platform",
                "warning:".yellow()
            );
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

/// Build `sd-core` for one target triple.
///
/// Returns `Ok(false)` when the target's std is not installed, which is the
/// ordinary situation for `--all-platforms` on a developer machine — there is
/// nothing wrong with the repository, this host just cannot produce a Windows
/// binary. Only a genuine compile failure is an error.
///
/// It does *not* build the plugin crates. They are outside the workspace (see
/// `discover_plugins`) and are built once for `wasm32-wasip2` by the caller; a
/// workspace-wide `cargo build --target <foreign triple>` would try to
/// cross-compile wasmtime and fail on every host without a cross toolchain,
/// which is what made `--build --all-platforms` unusable.
fn build_host_for_target(workspace_root: &Path, triple: &str, host_target: &str) -> Result<bool> {
    if triple != host_target && !target_is_installed(triple) {
        return Ok(false);
    }

    println!(
        "  {} building sd-core for {}...",
        "build:".yellow(),
        triple.cyan()
    );
    let status = Command::new("cargo")
        .args(["build", "--release", "--target", triple, "-p", "sd-core"])
        .current_dir(workspace_root)
        .status()
        .with_context(|| format!("building sd-core for {triple}"))?;
    if !status.success() {
        anyhow::bail!("cargo build of sd-core for {triple} failed");
    }
    Ok(true)
}

/// Whether rustup reports the target's standard library as installed.
///
/// A missing rustup is treated as "installed" so the build is attempted and
/// cargo gets to produce the real error, rather than this check inventing one.
fn target_is_installed(triple: &str) -> bool {
    let output = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output();

    match output {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
            .lines()
            .any(|line| line.trim() == triple),
        _ => true,
    }
}

/// How the staged artifact in `plugins/` relates to the plugin's source.
#[derive(Debug, PartialEq, Eq)]
enum Staleness {
    /// Built, newer than its source, and its manifest matches.
    Fresh,
    /// No artifact staged yet. Normal on a fresh checkout — the artifacts are
    /// build output and are not committed.
    NotBuilt,
    /// A source file is newer than the staged `.wasm`.
    SourceNewer,
    /// The staged sidecar manifest no longer matches the plugin's own
    /// `plugin.manifest.json`.
    ManifestDrifted,
}

/// Compare a plugin's staged artifact against its source.
///
/// `check` used to validate only that a `Cargo.toml` and a `src/lib.rs` were
/// present, which said nothing about whether the binary in `plugins/` had
/// anything to do with them. Editing a plugin and forgetting to rebuild
/// shipped the old binary silently — and editing `plugin.manifest.json`
/// without rebuilding left the loader reading a staged copy with the previous
/// capability grants, which is the worse half of the same problem.
fn check_staged_artifact(workspace_root: &Path, plugin: &PluginInfo) -> Staleness {
    let plugin_dir = match plugin.manifest_path.parent() {
        Some(dir) => dir,
        None => return Staleness::NotBuilt,
    };
    let plugins_dir = workspace_root.join("plugins");
    let artifact = plugins_dir.join(plugin_artifact_filename(&plugin.lib_name));

    let artifact_time = match std::fs::metadata(&artifact).and_then(|m| m.modified()) {
        Ok(time) => time,
        Err(_) => return Staleness::NotBuilt,
    };

    // The manifest check first: it is the one with a security consequence, and
    // a byte comparison is exact where an mtime comparison is a heuristic.
    let source_manifest = plugin_dir.join("plugin.manifest.json");
    if source_manifest.exists() {
        let staged_manifest = plugins_dir.join(format!(
            "{}.manifest.json",
            plugin_artifact_filename(&plugin.lib_name)
                .trim_end_matches(".wasm")
        ));
        let staged = std::fs::read(&staged_manifest).ok();
        let source = std::fs::read(&source_manifest).ok();
        if staged.as_deref() != source.as_deref() {
            return Staleness::ManifestDrifted;
        }
    }

    if newest_source_time(&plugin_dir.join("src"))
        .into_iter()
        .chain(std::fs::metadata(&plugin.manifest_path).and_then(|m| m.modified()))
        .any(|source_time| source_time > artifact_time)
    {
        return Staleness::SourceNewer;
    }

    Staleness::Fresh
}

/// The most recent modification time anywhere under `dir`.
fn newest_source_time(dir: &Path) -> Option<std::time::SystemTime> {
    let mut newest: Option<std::time::SystemTime> = None;
    let mut stack = vec![dir.to_path_buf()];

    while let Some(current) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if let Ok(modified) = entry.metadata().and_then(|m| m.modified()) {
                newest = Some(match newest {
                    Some(current) if current >= modified => current,
                    _ => modified,
                });
            }
        }
    }

    newest
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

        match check_staged_artifact(&workspace_root, plugin) {
            Staleness::Fresh => println!("{}", "OK".green()),
            Staleness::NotBuilt => println!(
                "{} not built; run `sd-plugins build --release`",
                "STALE".yellow()
            ),
            Staleness::SourceNewer => {
                println!(
                    "{} plugins/{} is older than its source; rebuild before shipping it",
                    "STALE".yellow(),
                    plugin_artifact_filename(&plugin.lib_name)
                );
                errors += 1;
            }
            Staleness::ManifestDrifted => {
                println!(
                    "{} the staged manifest differs from {}/plugin.manifest.json; \
                     the loader is granting the old capabilities",
                    "STALE".red(),
                    plugin.name
                );
                errors += 1;
            }
        }
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

    println!("{}", "=== StreamDeck Dev Mode ===".cyan().bold());
    println!("Watching plugins for changes...");
    println!("Command: {}", command.join(" ").yellow());
    println!();

    // Initial build of all plugins
    println!("{}", "Building plugins...".yellow());
    build_all_plugins(&workspace_root, release)?;
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
                match build_plugins(&workspace_root, release, &affected) {
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
        if path_str.contains("crates/plugin-system/") {
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

fn build_all_plugins(workspace_root: &Path, release: bool) -> Result<()> {
    let plugins = discover_plugins(workspace_root)?;
    let plugins_refs: Vec<&PluginInfo> = plugins.iter().collect();
    build_plugins_with_info(workspace_root, release, &plugins_refs)
}

fn build_plugins(workspace_root: &Path, release: bool, affected: &[String]) -> Result<()> {
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

    build_plugins_with_info(workspace_root, release, &plugins_to_build)
}

/// Build one plugin to `wasm32-wasip2` and stage the artifact in `plugins/`.
///
/// Returns the staged path, or `None` when cargo succeeded but the expected
/// artifact was not where it should be.
fn build_one_plugin(
    workspace_root: &Path,
    plugin: &PluginInfo,
    release: bool,
) -> Result<Option<PathBuf>> {
    // Plugins live outside the cargo workspace, so each is built from its own
    // directory rather than selected with `-p`.
    let plugin_dir = plugin
        .manifest_path
        .parent()
        .expect("manifest path always has a parent");

    let mut args = vec!["build", "--target", WASM_TARGET, "--lib"];
    if release {
        args.push("--release");
    }

    let status = Command::new("cargo")
        .args(&args)
        .current_dir(plugin_dir)
        .status()
        .context(format!("Failed to build {}", plugin.name))?;

    if !status.success() {
        anyhow::bail!("Failed to build {}", plugin.name);
    }

    let artifact = plugin_artifact_filename(&plugin.lib_name);
    let profile = if release { "release" } else { "debug" };
    let src = plugin_dir
        .join("target")
        .join(WASM_TARGET)
        .join(profile)
        .join(&artifact);

    if !src.exists() {
        return Ok(None);
    }

    let plugins_dir = workspace_root.join("plugins");
    std::fs::create_dir_all(&plugins_dir)?;
    let dst = plugins_dir.join(&artifact);
    std::fs::copy(&src, &dst).context(format!("Failed to copy {} to plugins/", artifact))?;

    // A plugin declares its capability grants and resource limits in a
    // `plugin.manifest.json` beside its Cargo.toml. Stage it under the
    // artifact's stem, which is where the loader looks for it.
    let manifest_src = plugin_dir.join("plugin.manifest.json");
    if manifest_src.exists() {
        let stem = dst
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&plugin.lib_name);
        let manifest_dst = plugins_dir.join(format!("{stem}.manifest.json"));
        std::fs::copy(&manifest_src, &manifest_dst)
            .context(format!("Failed to stage manifest for {}", plugin.name))?;
    } else {
        // Not fatal — a plugin needing no capabilities does not need one —
        // but silence here would make a forgotten manifest look like a
        // capability bug at runtime.
        println!(
            "    {} no plugin.manifest.json; {} will be granted no capabilities",
            "note:".yellow(),
            plugin.name
        );
    }

    Ok(Some(dst))
}

fn build_plugins_with_info(
    workspace_root: &Path,
    release: bool,
    plugins: &[&PluginInfo],
) -> Result<()> {
    let mut built = 0;

    for plugin in plugins {
        print!("  Building {}... ", plugin.name.cyan());
        match build_one_plugin(workspace_root, plugin, release)? {
            Some(_) => {
                println!("{}", "OK".green());
                built += 1;
            }
            None => println!("{} (artifact missing)", "OK".green()),
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
