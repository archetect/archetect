//! Build automation tasks for Archetect
//!
//! Usage: cargo xtask <command>

use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Build automation for Archetect")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Install the archetect binary to ~/.cargo/bin
    Install {
        /// Statically compile OpenSSL into the binary
        #[arg(long = "static-openssl", visible_alias = "static-ssl", default_value_t = true)]
        openssl_static: bool,
    },

    /// Run archetect with arguments
    Run {
        /// Arguments to pass to archetect
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Run all tests
    Test,

    /// Run tests for a specific crate
    TestCrate {
        /// Crate name (e.g., archetect-core)
        name: String,
    },

    /// Build release binary
    Build,

    /// Check code without building
    Check,

    /// Run clippy lints (deny warnings)
    Clippy,

    /// Format code
    Fmt {
        /// Check formatting without modifying files
        #[arg(long)]
        check: bool,
    },

    /// Sweep stale build artifacts from target/
    Sweep,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Install { openssl_static } => {
            sweep()?;
            if openssl_static {
                cargo_env(&["install", "--path", "archetect-bin"], &[("OPENSSL_STATIC", "1")])?;
            } else {
                cargo(&["install", "--path", "archetect-bin"])?;
            }
        }

        Commands::Run { args } => {
            let mut cmd_args = vec!["run", "--package", "archetect-bin", "--"];
            cmd_args.extend(args.iter().map(|s| s.as_str()));
            cargo(&cmd_args)?;
        }

        Commands::Test => {
            sweep()?;
            cargo(&["test", "--workspace"])?;
        }

        Commands::TestCrate { name } => {
            sweep()?;
            cargo(&["test", "-p", &name])?;
        }

        Commands::Build => {
            sweep()?;
            cargo(&["build", "--release"])?;
        }

        Commands::Check => {
            sweep()?;
            cargo(&["check", "--workspace", "--all-targets"])?;
        }

        Commands::Clippy => {
            sweep()?;
            cargo(&["clippy", "--workspace", "--all-targets", "--all-features", "--", "-D", "warnings"])?;
        }

        Commands::Fmt { check } => {
            if check {
                cargo(&["fmt", "--all", "--", "--check"])?;
            } else {
                cargo(&["fmt", "--all"])?;
            }
        }

        Commands::Sweep => {
            sweep()?;
        }
    }

    Ok(())
}

fn cargo(args: &[&str]) -> Result<()> {
    cargo_env(args, &[])
}

fn cargo_env(args: &[&str], env: &[(&str, &str)]) -> Result<()> {
    println!("cargo {}", args.join(" "));

    let mut command = Command::new("cargo");
    command.args(args);
    for (key, value) in env {
        command.env(key, value);
    }

    let status = command.status()?;
    if !status.success() {
        anyhow::bail!("cargo command failed with status: {}", status);
    }

    Ok(())
}

/// Sweep build artifacts older than 7 days. Installs cargo-sweep if missing.
fn sweep() -> Result<()> {
    ensure_cargo_sweep()?;
    println!("==> Sweeping stale artifacts (>7 days)...");
    let status = Command::new("cargo")
        .args(["sweep", "--time", "7"])
        .status()
        .context("failed to run cargo sweep")?;
    if !status.success() {
        eprintln!("    Warning: cargo sweep failed, continuing anyway");
    }
    Ok(())
}

/// Install cargo-sweep if it isn't already present.
fn ensure_cargo_sweep() -> Result<()> {
    let cargo_bin = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".cargo/bin/cargo-sweep");

    if cargo_bin.exists() {
        return Ok(());
    }

    println!("==> Installing cargo-sweep...");
    let status = Command::new("cargo")
        .args(["install", "cargo-sweep"])
        .status()
        .context("failed to install cargo-sweep")?;
    if !status.success() {
        anyhow::bail!("cargo install cargo-sweep failed (exit {})", status);
    }
    Ok(())
}
