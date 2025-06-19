//! `deploy` subcommand implementation for AlgoTraderV2 CLI
//!
//! This is an initial skeleton for the v0.2.0 Deployment & self-configuration
//! feature.  It currently performs a very small set of responsibilities:
//!
//! 1. Ensures a config file exists – if not, copies the built-in template
//!    from `config/default.toml` to `config.toml` (or another path provided
//!    via `--config`).
//! 2. Performs a few environment checks (Rust toolchain & Solana CLI).
//! 3. Prints a success summary to the console.
//!
//! Future expansions will connect to the selected network, upload on-chain
//! programs, and perform any other first-time setup required to run the bot
//! in a production environment.

use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use dialoguer::{Input, Confirm};
use crate::config::{self, Config};

use super::config_wizard;
use config_wizard::interactive_fill_config;
use clap::Args;

/// Networks currently supported by the `deploy` command.
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum Network {
    Mainnet,
    Testnet,
    Devnet,
}

impl std::fmt::Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Network::Mainnet => write!(f, "mainnet"),
            Network::Testnet => write!(f, "testnet"),
            Network::Devnet => write!(f, "devnet"),
        }
    }
}

/// Arguments accepted by the `deploy` subcommand.
#[derive(Debug, Clone, Args)]
#[command(next_help_heading = "Deployment options")]
pub struct DeployArgs {
    /// Target Solana cluster/network.
    #[arg(long, value_enum, default_value = "devnet")]
    pub network: Network,

    /// Path to keypair JSON file to use for deployment & fees.
    #[arg(long, value_name = "FILE", default_value = "~/.config/solana/id.json")]
    pub wallet: PathBuf,

    /// Path to configuration file. If it doesn't exist, a template will be created.
    #[arg(short, long, value_name = "FILE", default_value = "config.toml")]
    pub config: PathBuf,
}

/// Entry-point invoked by the CLI.
pub fn handle_deploy(args: DeployArgs) -> Result<()> {
    println!("🔧 Starting deployment wizard (network: {})", args.network);

    ensure_config_exists(&args.config)?;
    perform_env_checks()?;

    // Interactive config filling
    interactive_fill_config(&args.config)?;

    println!("✅ Deployment pre-checks passed. You're ready to deploy!");
    println!("  • Config file: {}", args.config.display());
    println!("  • Wallet: {}", args.wallet.display());
    println!("  • Network: {}", args.network);

    // Future TODO: actual deployment logic (program upload, initialisation, etc.)

    Ok(())
}

/// Copy built-in template to the requested path if the file does not exist.
fn ensure_config_exists(path: &Path) -> Result<()> {
    if path.exists() {
        return Ok(());
    }

    let template = Path::new("config/default.toml");
    std::fs::create_dir_all(
        path.parent().unwrap_or_else(|| Path::new(".")),
    )?;
    std::fs::copy(template, path).with_context(|| {
        format!(
            "Failed to copy default config template from {} to {}",
            template.display(),
            path.display()
        )
    })?;

    println!(
        "📝 Created new configuration file from template at {}",
        path.display()
    );
    Ok(())
}

/// Basic environment validation.
fn perform_env_checks() -> Result<()> {
    // Check rustup installation (for compiling binaries)
    if which::which("rustup").is_err() {
        anyhow::bail!(
            "rustup is not installed – please install Rust toolchain before deploying"
        );
    }

    // Check Solana CLI for program deployment
    if which::which("solana").is_err() {
        anyhow::bail!(
            "solana CLI tools not found – install from https://docs.solana.com/cli/install"
        );
    }

    Ok(())
}
