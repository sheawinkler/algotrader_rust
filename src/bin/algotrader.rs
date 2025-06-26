//! Minimal CLI entrypoint for AlgoTraderV2
//! At this stage it only loads the configuration and exits.  We'll extend it
//! once the engine API is finalised.

use algotraderv2::config::Config;
use anyhow::{Context, Result};
use axum::{response::IntoResponse, routing::get, Router};

use clap::Parser;
use std::{net::SocketAddr, path::Path};

use clap::Subcommand;

#[derive(Debug, Parser)]
#[command(name = "algotrader", author, version, about = "AlgoTraderV2 CLI", long_about = None)]
struct Args {
    /// Path to the configuration file (TOML)
    #[arg(short, long, default_value = "config.toml")]
    config: String,

    /// Print the default configuration to stdout and exit
    #[arg(long)]
    print_default_config: bool,

    /// Command to execute
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run a historical backtest against market data
    Backtest {
        #[arg(long, value_name = "CSV")]
        /// Path to historical market data (e.g. CSV)
        #[arg(long)]
        data: String,
        /// Timeframe, e.g. 1m, 5m, 1h (optional)
        #[arg(long)]
        timeframe: Option<String>,
        /// Output CSV file path
        #[arg(long)]
        output: Option<String>,
        /// Use meta-strategy engine to pick best strategy
        #[arg(long)]
        meta: bool,
    },
    /// Import historical data from CryptoCompare and save to CSV
    Import {
        /// Base symbols (e.g., BTC, ETH)
        #[arg(long, value_name = "SYM", num_args = 1..)]
        base: Vec<String>,
        /// Quote symbol (e.g., USDC)
        #[arg(long, default_value = "USDC")]
        quote: String,
        /// Timeframe (1d, 1h, 1m)
        #[arg(long, default_value = "1d")]
        timeframe: String,
        /// Number of candles (<=2000)
        #[arg(long, default_value_t = 365)]
        limit: usize,
        /// Wallet address to auto-include SPL tokens (optional)
        #[arg(long)]
        wallet: Option<String>,
        /// Output directory for CSV files
        #[arg(long, value_name = "DIR", default_value = "data")]
        out_dir: String,
    },
    /// Start live / paper trading using the configured engine
    Run {
        /// Enable paper-trading (no real orders)
        #[arg(long)]
        paper: bool,
    },
    /// Generate a default configuration and wallet keypair
    Init {
        /// Output path for config file
        #[arg(short, long, default_value = "config.toml")]
        config: String,
        /// Output path for keypair file
        #[arg(long, default_value = "wallet.json")]
        keypair: String,
        /// Overwrite existing files
        #[arg(long)]
        force: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    tracing_subscriber::fmt::init();
    algotraderv2::metrics::init().expect("metrics init");

    let args = Args::parse();

    if args.print_default_config {
        println!("{}", Config::default_toml());
        return Ok(());
    }

    // Handle subcommands first so we can fall back to legacy default behaviour
    if let Some(cmd) = &args.command {
        match cmd {
            | Command::Backtest { data, timeframe, output, meta } => {
                println!(
                    "⚙️  Starting backtest on {} (tf={})",
                    data,
                    timeframe.clone().unwrap_or_else(|| "default".into())
                );
                use algotraderv2::backtest::simple_backtest;
                use algotraderv2::meta::MetaStrategyEngine;

                // Run simple backtest
                if *meta {
                    let tf = timeframe.as_deref().unwrap_or("default");
                    let mut engine = MetaStrategyEngine::new(tf, 10_000.0, "meta_cache")?;
                    let ranked = engine.select_best_strategy(&std::path::PathBuf::from(&data))?;
                    println!(
                        "🏆 Best strategy: {} (Sharpe {:.2}, DD {:.2}%)",
                        ranked.strategy.name(),
                        ranked.sharpe,
                        ranked.max_drawdown * 100.0
                    );
                } else {
                    let tf = timeframe.as_deref().unwrap_or("default");
                    let out_path = output.as_ref().map(std::path::Path::new);
                    use algotraderv2::backtest::SimMode;
                    simple_backtest(std::path::Path::new(&data), tf, SimMode::Bar, out_path)
                        .await?;
                }
                return Ok(());
            }
            | Command::Import { base, quote, timeframe, limit, wallet, out_dir } => {
                use algotraderv2::blockchain::wallet_scanner::get_wallet_token_symbols;
                use std::path::PathBuf;

                // Aggregate symbols from CLI and wallet (if provided)
                let mut symbols: Vec<String> = base.clone();
                if let Some(w) = wallet {
                    let rpc = std::env::var("SOLANA_RPC_URL")
                        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".into());
                    match get_wallet_token_symbols(w, &rpc) {
                        | Ok(mut s) => symbols.append(&mut s),
                        | Err(e) => eprintln!("Wallet scan failed: {e}"),
                    }
                }
                symbols.sort();
                symbols.dedup();
                if symbols.is_empty() {
                    eprintln!("No symbols to import");
                    return Ok(());
                }

                std::fs::create_dir_all(Path::new(&out_dir))?;
                println!(
                    "⬇️  Importing {} symbols (quote {}, tf {}, limit {})",
                    symbols.len(),
                    quote,
                    timeframe,
                    limit
                );
                for sym in &symbols {
                    let file = PathBuf::from(&out_dir).join(format!(
                        "{}_{}_{}.csv",
                        sym.to_uppercase(),
                        quote.to_uppercase(),
                        timeframe
                    ));
                    if let Err(e) = algotraderv2::backtest::importer::download_to_csv(
                        sym, quote, timeframe, *limit, &file,
                    )
                    .await
                    {
                        eprintln!("❗ Failed to import {}: {}", sym, e);
                    }
                }
                println!("✅ Import complete. Files in {}", out_dir);
                return Ok(());
            }
            | Command::Run { paper } => {
                println!("🚀 Starting trading engine (paper={})", paper);
                let config = if Path::new(&args.config).exists() {
                    Config::from_file(&args.config).context("Failed to load configuration")?
                } else {
                    log::warn!("Configuration file '{}' not found – using defaults", args.config);
                    Config::default()
                };
                run_service(&config, *paper).await?;
                return Ok(());
            }
            | Command::Init { config, keypair, force } => {
                use solana_sdk::signature::{Keypair, Signer};
                use std::fs;
                use std::path::PathBuf;

                let cfg_path = PathBuf::from(config);
                let kp_path = PathBuf::from(keypair);

                if (cfg_path.exists() || kp_path.exists()) && !force {
                    eprintln!("Config or keypair already exists. Use --force to overwrite.");
                    std::process::exit(1);
                }

                if let Some(parent) = cfg_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                if let Some(parent) = kp_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                fs::write(&cfg_path, Config::default_toml())?;
                println!("✅ Wrote default config to {}", cfg_path.display());

                let kp = Keypair::new();
                let secret = bs58::encode(kp.to_bytes()).into_string();
                fs::write(&kp_path, format!("\"{}\"", secret))?;
                println!("✅ Wrote new keypair to {} (pubkey={})", kp_path.display(), kp.pubkey());
                return Ok(());
            }
        }
    } // end match & if

    // Try to load an existing configuration, otherwise fall back to defaults.
    let config = if Path::new(&args.config).exists() {
        Config::from_file(&args.config).context("Failed to load configuration")?
    } else {
        log::warn!("Configuration file '{}' not found – using defaults", args.config);
        Config::default()
    };

    log::info!("Starting trading engine via default command");
    return run_service(&config, false).await;
}

async fn health() -> impl IntoResponse {
    "OK"
}

async fn run_service(config: &Config, paper: bool) -> Result<()> {
    // structured tracing already initialised in main

    log::info!("Trading engine starting (paper={})", paper);

    // --- Example minimal runtime task: periodically log wallet balance ---
    use std::net::TcpListener;

    // --- Launch TradingEngine -------------------------------------------------
    // Spawn a placeholder background task (system metrics collection TBD)
    tokio::spawn(async {
        loop {
            // Placeholder: hook real system metrics here once sysinfo API stabilized
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        }
    });
    use algotraderv2::TradingEngine;
    let mut engine = TradingEngine::with_config_async(config.clone(), paper).await;
    // Determine symbol list: use config default_pair
    let symbols = vec![config.trading.default_pair.clone()];
    // Spawn the async trading loop – runs until cancelled
    let engine_handle = tokio::spawn(async move {
        if let Err(e) = engine.start_with_market_router(symbols, None, None).await {
            log::error!("TradingEngine exited with error: {e}");
        }
    });

    // Health and metrics endpoints
    async fn metrics_handler() -> impl IntoResponse {
        algotraderv2::metrics::handle().render()
    }

    let app = Router::new()
        .route("/", get(|| async { "AlgoTraderV2 running" }))
        .route("/healthz", get(health))
        .route("/metrics", get(metrics_handler));
    let primary_addr: SocketAddr = "127.0.0.1:8888".parse().unwrap();
    let listener = match TcpListener::bind(primary_addr) {
        | Ok(l) => l,
        | Err(e) => {
            log::warn!("Port 8888 unavailable: {} – binding to random port", e);
            TcpListener::bind("127.0.0.1:0").expect("failed to bind random port")
        }
    };
    let addr = listener.local_addr().expect("no local_addr");
    log::info!("Serving /healthz on http://{}", addr);

    let server = axum::Server::from_tcp(listener)
        .expect("failed to create server from listener")
        .serve(app.into_make_service());
    let server_handle = tokio::spawn(server);

    tokio::signal::ctrl_c().await?;
    log::info!("Shutdown signal received. Stopping...");
    server_handle.abort();

    // Attempt graceful shutdown of engine task
    engine_handle.abort();

    Ok(())
}
