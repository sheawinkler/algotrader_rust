//! Command-line interface for the AlgoTraderV2 trading bot

use clap::{Parser, Subcommand};
use clap::ValueEnum;
use crate::backtest::SimMode;
use crate::backtest;
use std::path::PathBuf;
use anyhow::Result;
use crate::config;

/// Main CLI structure using clap derive
#[derive(Debug, Parser)]
#[command(name = "algotraderv2")]
#[command(about = "A high-performance algorithmic trading bot for Solana", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Available subcommands
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Start the trading bot
    Start {
        /// Path to the configuration file
        #[arg(short, long, value_name = "FILE")]
        config: Option<PathBuf>,
        
        /// Enable debug logging
        #[arg(short, long)]
        debug: bool,
    },
    
    /// Generate a new configuration file
    Init {
        /// Output path for the config file
        #[arg(short, long, value_name = "FILE", default_value = "config.toml")]
        output: PathBuf,
        
        /// Generate a commented config with explanations
        #[arg(long)]
        commented: bool,
    },
    
    /// Check the configuration file for errors
    CheckConfig {
        /// Path to the configuration file
        #[arg(short, long, value_name = "FILE")]
        config: PathBuf,
    },
    
    /// Show wallet information
    Wallet {
        #[command(subcommand)]
        command: WalletCommands,
    },
    
    /// Run a simple CSV backtest
    Backtest {
        /// Path to CSV market data file
        #[arg(short, long, value_name = "FILE")]
        data: PathBuf,

        /// Timeframe string e.g. 1m, 1h or "tick" for raw tick simulation
        #[arg(short = 't', long, default_value = "1h", value_name="TF")]
        timeframe: String,

        /// Simulation mode: bar (default) or tick
        #[arg(long, value_enum, default_value = "bar", value_name="MODE")]
        sim_mode: SimMode,
    },

    /// Run walk-forward optimization
    WalkForward {
        /// Path to CSV market data file
        #[arg(short, long, value_name = "FILE")]
        data: PathBuf,

        /// Timeframe string e.g. 1h
        #[arg(short = 't', long, default_value = "1h", value_name="TF")]
        timeframe: String,

        /// Simulation mode: bar (default) or tick
        #[arg(long, value_enum, default_value = "bar", value_name="MODE")]
        sim_mode: crate::backtest::SimMode,

        /// Training window length in days
        #[arg(long, default_value_t = 90)]
        train: i64,

        /// Test window length in days
        #[arg(long, default_value_t = 30)]
        test: i64,

        /// Step size between windows in days
        #[arg(long, default_value_t = 30)]
        step: i64,
    },

    /// Show version information
    Version,
}

/// Wallet-related subcommands
#[derive(Debug, Subcommand)]
pub enum WalletCommands {
    /// Show wallet balance and tokens
    Info {
        /// Path to the configuration file
        #[arg(short, long, value_name = "FILE")]
        config: Option<PathBuf>,
    },
    
    /// Generate a new wallet keypair
    New {
        /// Output path for the keypair file
        #[arg(short, long, value_name = "FILE", default_value = "wallet.json")]
        output: PathBuf,
        
        /// Overwrite existing file
        #[arg(short, long)]
        force: bool,
    },
}

impl Cli {
    /// Parse command line arguments
    pub fn parse() -> Self {
        Self::parse()
    }
    
    /// Execute the CLI command
    async fn execute(self) -> Result<()> {
        match self.command {
            Commands::Start { config, debug } => {
                self.handle_start(config, debug).await
            }
            Commands::Backtest { data, timeframe, sim_mode } => {
                let path = data;
                if !path.exists() {
                    anyhow::bail!("Data file not found: {}", path.display());
                }
                println!("Running backtest on {}...", path.display());
                crate::backtest::simple_backtest(&path, &timeframe, sim_mode, None).await
            }
            Commands::Init { output, commented } => {
                self.handle_init(output, commented)
            }
            Commands::CheckConfig { config } => {
                self.handle_check_config(config)
            }
            Commands::Wallet { command } => {
                self.handle_wallet(command).await
            }
            Commands::WalkForward { data, timeframe, sim_mode, train, test, step } => {
                if !data.exists() { anyhow::bail!("Data file not found: {}", data.display()); }
                println!("Running walk-forward optimization on {}...", data.display());
                let cfg = crate::backtest::harness::WalkForwardConfig { train_days: train, test_days: test, step_days: step };
                let reports = crate::backtest::harness::run_walk_forward(&data, &timeframe, sim_mode, cfg).await?;
                if reports.is_empty() {
                    println!("No reports generated (perhaps insufficient data)");
                } else {
                    let avg_sharpe: f64 = reports.iter().map(|r| r.sharpe).sum::<f64>() / reports.len() as f64;
                    let total_return = (reports.last().unwrap().ending_balance / reports.first().unwrap().starting_balance - 1.0) * 100.0;
                    println!("Walk-forward completed: {} windows, Avg Sharpe {:.2}, Total Return {:.1}%", reports.len(), avg_sharpe, total_return);
                }
                Ok(())
            }
            Commands::Version => {
                self.handle_version()
            }
        }
    }
    
    async fn handle_start(&self, config_path: Option<PathBuf>, debug: bool) -> Result<()> {
        println!("Starting trading bot...");
        
        // Load configuration
        let config = match config_path {
            Some(path) => config::Config::from_file(path)?,
            None => config::Config::load()?,
        };
        
        // Initialize logging
        crate::utils::logging::init_logging(debug)?;
        
        log::info!("Configuration loaded successfully");
        log::info!("RPC URL: {}", config.solana.rpc_url);
        log::info!("Trading enabled: {}", config.trading.trading_enabled);
        log::info!("Paper trading: {}", config.trading.paper_trading);
        
        // TODO: Initialize and start the trading engine
        
        Ok(())
    }
    
    fn handle_init(&self, output: PathBuf, commented: bool) -> Result<()> {
        if commented {
            config::generate_commented_config_template(&output)?;
            println!("Generated commented configuration at: {}", output.display());
        } else {
            config::generate_config_template(&output)?;
            println!("Generated minimal configuration at: {}", output.display());
        }
        
        println!("\nPlease edit the configuration file before starting the bot.");
        Ok(())
    }
    
    fn handle_check_config(&self, config_path: PathBuf) -> Result<()> {
        println!("Checking configuration file: {}", config_path.display());
        
        match config::Config::from_file(&config_path) {
            Ok(config) => {
                println!("✓ Configuration is valid");
                println!("\nConfiguration summary:");
                println!("  RPC URL: {}", config.solana.rpc_url);
                println!("  Wallet: {}", 
                    config.wallet.private_key.as_ref()
                        .map(|_| "[private key]")
                        .or_else(|| config.wallet.keypair_path.as_deref())
                        .unwrap_or("Not configured")
                );
                println!("  Trading enabled: {}", config.trading.trading_enabled);
                println!("  Paper trading: {}", config.trading.paper_trading);
                
                Ok(())
            }
            Err(e) => {
                eprintln!("✗ Invalid configuration: {}", e);
                std::process::exit(1);
            }
        }
    }
    
    async fn handle_backtest(&self, data_path: PathBuf, timeframe: String) -> Result<()> {
        // Use the CSV provider for now
        let path = data_path;
        if !path.exists() {
            anyhow::bail!("Data file not found: {}", path.display());
        }
        println!("Running backtest on {}...", path.display());
        crate::backtest::simple_backtest(&path, &timeframe, sim_mode, None).await
    }

    async fn handle_wallet(&self, command: WalletCommands) -> Result<()> {
        match command {
            WalletCommands::Info { config } => {
                let config = match config {
                    Some(path) => config::Config::from_file(path)?,
                    None => config::Config::load()?,
                };
                
                let keypair = config.load_keypair()?;
                let pubkey = keypair.pubkey();
                
                println!("Wallet Information:");
                println!("  Public Key: {}", pubkey);
                println!("  Key Source: {}", 
                    if config.wallet.private_key.is_some() {
                        "Private Key"
                    } else if config.wallet.keypair_path.is_some() {
                        "Keypair File"
                    } else {
                        "Unknown"
                    }
                );
                
                // TODO: Show balance and tokens
                println!("\nNote: Balance and token information not yet implemented");
                
                Ok(())
            }
            
            WalletCommands::New { output, force } => {
                if output.exists() && !force {
                    anyhow::bail!(
                        "File already exists: {}. Use --force to overwrite.",
                        output.display()
                    );
                }
                
                // Generate a new keypair
                let keypair = solana_sdk::signer::keypair::generate_keypair();
                
                // Save to file
                std::fs::write(&output, &keypair.to_bytes())
                    .with_context(|| format!("Failed to write keypair to {}", output.display()))?;
                
                println!("Generated new wallet keypair:");
                println!("  Public Key: {}", keypair.pubkey());
                println!("  Saved to: {}", output.display());
                println!("\nWARNING: Keep this file secure and never share it with anyone!");
                
                Ok(())
            }
        }
    }
    
    fn handle_version(&self) -> Result<()> {
        println!(
            "AlgoTraderV2 v{}\n{}",
            env!("CARGO_PKG_VERSION"),
            env!("CARGO_PKG_DESCRIPTION")
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    use tempfile::tempdir;
    
    #[test]
    fn verify_cli() {
        // This will panic if the CLI structure is invalid
        Cli::command().debug_assert();
    }
    
    #[test]
    fn test_init_command() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        let cli = Cli::parse_from(&["algotraderv2", "init", "-o", config_path.to_str().unwrap()]);
        
        if let Commands::Init { output, commented: _ } = cli.command {
            assert_eq!(output, config_path);
        } else {
            panic!("Expected Init command");
        }
    }
    
    #[test]
    fn test_start_command() {
        let cli = Cli::parse_from(&["algotraderv2", "start", "--debug"]);
        
        if let Commands::Start { config: _, debug } = cli.command {
            assert!(debug);
        } else {
            panic!("Expected Start command");
        }
    }
}
