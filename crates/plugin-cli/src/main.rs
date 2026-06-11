use anyhow::{Context, Result};
use cargo_metadata::MetadataCommand;
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::{Path, PathBuf};
use std::process::Command;

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
    },

    /// Validate plugin configurations
    Check,
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
        Commands::Package { version, output } => cmd_package(&version, &output),
        Commands::Check => cmd_check(),
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
            let status = Command::new("npm")
                .args(["ci"])
                .current_dir(&web_dir)
                .status()
                .context("Failed to run npm ci")?;

            if !status.success() {
                anyhow::bail!("npm ci failed");
            }

            let status = Command::new("npm")
                .args(["run", "build"])
                .current_dir(&web_dir)
                .status()
                .context("Failed to run npm build")?;

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
            let src_dir = if target_triple == get_host_target().unwrap_or_default() {
                workspace_root.join("target/release")
            } else {
                workspace_root.join(format!("target/{}/release", target_triple))
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

fn cmd_package(version: &str, output_dir: &str) -> Result<()> {
    let workspace_root = find_workspace_root()?;
    let plugins = discover_plugins(&workspace_root)?;
    let host_target = get_host_target()?;

    println!("{}", "=== Packaging Release ===".cyan().bold());
    println!("Version: {}", version.yellow());
    println!("Target:  {}", host_target.yellow());
    println!();

    let release_dir = workspace_root.join(output_dir).join(version);
    let platform_dir = match host_target.as_str() {
        t if t.contains("linux") && t.contains("x86_64") => "linux-x64",
        t if t.contains("linux") && t.contains("aarch64") => "linux-arm64",
        t if t.contains("windows") && t.contains("x86_64") => "windows-x64",
        t if t.contains("windows") && t.contains("aarch64") => "windows-arm64",
        t if t.contains("apple") && t.contains("x86_64") => "macos-x64",
        t if t.contains("apple") && t.contains("aarch64") => "macos-arm64",
        _ => "unknown",
    };

    let pkg_dir = release_dir.join(platform_dir);
    let plugins_out = pkg_dir.join("plugins");

    std::fs::create_dir_all(&plugins_out).context("Failed to create release directory")?;

    // Copy sd-core binary
    let core_ext = if host_target.contains("windows") {
        ".exe"
    } else {
        ""
    };
    let core_src = workspace_root
        .join("target/release")
        .join(format!("sd-core{}", core_ext));
    let core_dst = pkg_dir.join(format!("sd-core{}", core_ext));

    if core_src.exists() {
        std::fs::copy(&core_src, &core_dst)?;
        println!("  Copied sd-core{}", core_ext);
    } else {
        println!(
            "  {} sd-core not found (build with --with-core)",
            "Warning:".yellow()
        );
    }

    // Copy plugins
    for plugin in &plugins {
        let lib_filename = get_plugin_lib_filename(&plugin.lib_name, &host_target);
        let src = workspace_root.join("target/release").join(&lib_filename);
        let dst = plugins_out.join(&lib_filename);

        if src.exists() {
            std::fs::copy(&src, &dst)?;
            println!("  Copied {}", lib_filename);
        } else {
            println!("  {} {} not found", "Warning:".yellow(), lib_filename);
        }
    }

    // Copy web frontend
    let web_src = workspace_root.join("web/dist");
    let web_dst = pkg_dir.join("web");
    if web_src.exists() {
        copy_dir_recursive(&web_src, &web_dst)?;
        println!("  Copied web/");
    } else {
        println!(
            "  {} web/dist not found (build with --with-web)",
            "Warning:".yellow()
        );
    }

    // Create archive
    let archive_ext = if host_target.contains("windows") {
        "zip"
    } else {
        "tar.gz"
    };
    let archive_name = format!("streamdeck-{}.{}", platform_dir, archive_ext);

    if host_target.contains("windows") {
        let status = Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "Compress-Archive -Path '{}' -DestinationPath '{}'",
                    pkg_dir.display(),
                    release_dir.join(&archive_name).display()
                ),
            ])
            .status()?;
        if status.success() {
            println!("\n  Created {}", archive_name.green().bold());
        }
    } else {
        let status = Command::new("tar")
            .args([
                "czf",
                &release_dir.join(&archive_name).to_string_lossy(),
                "-C",
                &release_dir.to_string_lossy(),
                platform_dir,
            ])
            .status()?;
        if status.success() {
            println!("\n  Created {}", archive_name.green().bold());
        }
    }

    println!("\nRelease packaged in: {}", pkg_dir.display());

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

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

struct PluginInfo {
    name: String,
    dir_name: String,
    lib_name: String,
    version: String,
    manifest_path: PathBuf,
}
