//! init-config – create a default configuration file for AlgoTraderV2
use anyhow::Result;
use clap::Parser;
use std::{fs, path::PathBuf};

use algotraderv2::config::Config;

#[derive(Parser, Debug)]
#[command(
    name    = "init-config",
    version = env!("CARGO_PKG_VERSION"),
    about   = "Write a default `config.toml` for AlgoTraderV2"
)]
struct Args {
    /// Output path (default: ./config.toml)
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,

    /// Overwrite if the file already exists
    #[arg(short, long)]
    force: bool,
}

fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();

    if args.config.exists() && !args.force {
        eprintln!(
            "Config file {} exists. Use --force to overwrite.",
            args.config.display()
        );
        std::process::exit(1);
    }

    if let Some(parent) = args.config.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&args.config, Config::default_toml())?;
    println!("✅ Wrote default configuration to {}", args.config.display());
    Ok(())
}
