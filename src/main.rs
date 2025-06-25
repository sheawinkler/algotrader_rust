#![cfg(feature = "legacy_cli")]
//! (LEGACY) AlgoTraderV2 CLI implementation – compiled only with `legacy_cli` feature.
//! The new minimal CLI lives in `src/bin/algotrader.rs`. This file is kept for
//! reference and will be modernised later.

use std::process;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::fs;
use std::time::{Duration, SystemTime};

use clap::{Parser, Subcommand};
use serde::Serialize;
use log::{error, info, warn, debug};

use algotraderv2::config::Config;
// use algotraderv2::utils::logging::init_logging; // COMMENTED OUT: logging module is private
use algotraderv2::utils::error::{Error, Result};
use algotraderv2::utils::types::{Signal, MarketData, Position, TradingPair, Balance, OrderSide, Order};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::time::UNIX_EPOCH;
use csv::*;
use algotraderv2::dex::{DexClient, DexFactory};
use algotraderv2::strategy::{TradingStrategy, StrategyFactory, Action};
// TODO: OrderType is not implemented. All usages commented out below.
// TODO: Backtest and market modules are not implemented. All usages commented out below.
// TODO: Remove all references to // TODO: PriceData is not implemented. Usage commented out.
// PriceData (replaced with MarketData or commented out below).
// TODO: Remove all references to init_logging (commented out below).


#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::tempdir;

    #[test]
    fn test_main_help() {
        // Test that the help command works
        let output = std::process::Command::new("cargo")
            .args(["run", "--", "--help"])
            .output()
            .expect("Failed to execute command");
            
        assert!(output.status.success());
    }

    #[test]
    fn test_version_command() {
        // Test that the version command works
        let output = std::process::Command::new("cargo")
            .args(["run", "--", "version"])
            .output()
            .expect("Failed to execute command");
            
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("AlgoTraderV2"));
    }
}

use tokio::{
    sync::{mpsc, Mutex},
    time,
};

/// Command-line arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the configuration file
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,

    /// Command to execute
    #[command(subcommand)]
    command: Commands,
}

/// Available commands
#[derive(Subcommand, Debug)]
enum Commands {
    /// Start the trading bot
    Start {
        /// Run in backtest mode
        #[arg(short, long)]
        backtest: bool,
        
        /// Historical data file for backtesting
        #[arg(long)]
        data_file: Option<PathBuf>,
        
        /// Timeframe for live trading (e.g., 1m, 5m, 15m, 1h, 1d)
        #[arg(short, long, default_value = "1m")]
        timeframe: String,
        
        /// Trading pair (e.g., SOL/USDC)
        #[arg(short, long)]
        pair: Option<String>,
    },
    
    /// Generate a default configuration file
    Config {
        /// Output file path
        #[arg(short, long, default_value = "config.toml")]
        output: PathBuf,
    },
    
    /// Check the connection to exchanges
    Test,
    
    /// List available trading pairs
    ListPairs {
        /// DEX to list pairs from (e.g., jupiter, raydium, photon)
        #[arg(short, long)]
        dex: String,
    },
    
    /// Get account balance
    Balance {
        /// DEX to check balance on (e.g., jupiter, raydium, photon)
        #[arg(short, long)]
        dex: String,
    },
}

/// Main entry point for the trading bot
#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line arguments
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.debug { "debug" } else { "info" };
    // TODO: Logging initialization is currently disabled due to unresolved import. Uncomment when available.
    // init_logging(log_level);

    info!("Starting AlgoTraderV2 v{}", env!("CARGO_PKG_VERSION"));

    // Execute the requested command
    match args.command {
        Commands::Start { backtest, data_file, timeframe, pair } => {
            // Load configuration
            let config_path = args.config;
            let config = if config_path.exists() {
                info!("Loading configuration from {:?}", config_path);
                Config::from_file(&config_path)?
            } else {
                warn!("Configuration file not found, using default settings");
                Config::default()
            };

            // Initialize the trading bot
            let mut bot = TradingBot::new(config, backtest, pair).await?;
            
            // Start the appropriate mode
            if backtest {
                // TODO: Backtest module is not implemented. The following block is commented out until available.
                // use crate::backtest::{Backtester, HistoricalDataProvider};
                // struct CSVHistoricalDataProvider;
                // impl HistoricalDataProvider for CSVHistoricalDataProvider {
                //     fn load(&self, data_file: &std::path::PathBuf) -> crate::error::Result<Vec<crate::market::MarketData>> {
                //         // For now, delegate to bot's loader (TODO: move to this struct)
                //         futures::executor::block_on(bot.load_historical_data(data_file))
                //     }
                // }
                // let data_provider = Box::new(CSVHistoricalDataProvider);
                // let mut backtester = Backtester {
                //     bot: &mut bot,
                //     data_provider,
                //     timeframe: timeframe.clone(),
                // };
                // backtester.run(data_file).await?;
            } else {
                bot.run_live(&timeframe).await?;
            }
        }
        Commands::Config { output } => {
            // Generate a default configuration file
            let default_config = Config::default();
            default_config.save_to_file(output)?;
            info!("Default configuration file generated");
        }
        Commands::Test => {
            // Test connections to exchanges
            test_connections().await?;
        }
        Commands::ListPairs { dex } => {
            list_trading_pairs(&dex).await?;
        }
        Commands::Balance { dex } => {
            get_balance(&dex).await?;
        }
    }

    Ok(())
}

/// Trading performance metrics
#[derive(Debug, Clone, Serialize)]
struct PerformanceMetrics {
    total_return: f64,
    annualized_return: f64,
    max_drawdown: f64,
    sharpe_ratio: f64,
    sortino_ratio: f64,
    win_rate: f64,
    profit_factor: f64,
    total_trades: usize,
    winning_trades: usize,
    losing_trades: usize,
    avg_win: f64,
    avg_loss: f64,
}

/// Main trading bot structure
struct TradingBot {
    config: Config,
    dex_clients: HashMap<String, Box<dyn DexClient>>,
    strategies: Vec<Box<dyn TradingStrategy>>,
    is_backtest: bool,
    trading_pair: Option<TradingPair>,
    positions: HashMap<String, Position>,
    balance: HashMap<String, Balance>,
    order_history: Vec<Order>,
    performance_metrics: Option<PerformanceMetrics>,
    risk_free_rate: f64,  // For Sharpe ratio calculation
}

impl TradingBot {
    /// Create a new trading bot instance
    async fn new(config: Config, is_backtest: bool, pair: Option<String>) -> Result<Self> {
        // Parse trading pair if provided
        let trading_pair = pair.as_ref().and_then(|p| TradingPair::from_str(p).ok());
        
        // Initialize DEX clients
        let mut dex_clients = HashMap::new();
        for (name, dex_config) in &config.dex {
            if dex_config.enabled {
                match DexFactory::create_client(name) {
                    Ok(client) => {
                        info!("Initialized {} DEX client", name);
                        dex_clients.insert(name.clone(), client);
                    }
                    Err(e) => {
                        error!("Failed to initialize {} DEX client: {}", name, e);
                    }
                }
            }
        }

        if dex_clients.is_empty() {
            return Err(Error::ConfigError("No DEX clients enabled in configuration".to_string()));
        }

        // Initialize trading strategies
        let mut strategies = Vec::new();
        for (name, strategy_config) in &config.strategies {
            if strategy_config.enabled {
                match StrategyFactory::create_strategy(name) {
                    Ok(mut strategy) => {
                        // Initialize strategy with parameters
                        if let Err(e) = strategy.initialize(strategy_config.params.clone()).await {
                            error!("Failed to initialize {} strategy: {}", name, e);
                            continue;
                        }
                        info!("Initialized {} strategy", name);
                        strategies.push(strategy);
                    }
                    Err(e) => {
                        error!("Failed to create {} strategy: {}", name, e);
                    }
                }
            }
        }

        if strategies.is_empty() {
            return Err(Error::ConfigError("No trading strategies enabled in configuration".to_string()));
        }

        Ok(Self {
            config,
            dex_clients,
            strategies,
            is_backtest,
            trading_pair,
            positions: HashMap::new(),
            balance: HashMap::new(),
            order_history: Vec::new(),
            performance_metrics: None,
            risk_free_rate: 0.05,  // 5% annual risk-free rate
        })
    }

    /// Run the bot in live trading mode
    async fn run_live(&mut self, timeframe: &str) -> Result<()> {
        info!("Running in live trading mode with timeframe: {}", timeframe);
        
        // Validate trading pair
        let trading_pair = self.trading_pair.as_ref().ok_or_else(|| {
            Error::ConfigError("Trading pair must be specified for live trading".to_string())
        })?;
        
        info!("Trading pair: {}", trading_pair);
        
        // Initialize account balance
        self.update_balance().await?;
        self.log_balance();
        
        // Main trading loop
        let duration = parse_timeframe(timeframe);
        let mut interval = time::interval(duration);
        info!("Trading bot is running. Press Ctrl+C to exit.");
        
        loop {
            interval.tick().await;
            
            match self.process_market_data(trading_pair).await {
                Ok(_) => {
                    // Update positions and balance after processing market data
                    self.update_positions().await?;
                    self.update_balance().await?;
                    self.log_balance();
                }
                Err(e) => {
                    error!("Error processing market data: {}", e);
                    // Add some delay before retrying
                    time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }
    
    /// Run the bot in backtest mode
    async fn run_backtest(&mut self, data_file: Option<PathBuf>, timeframe: &str) -> Result<()> {
        info!("Running in backtest mode with timeframe: {}", timeframe);
        
        // Load historical data
        let data_file = data_file.ok_or_else(|| {
            Error::ConfigError("Data file must be specified for backtesting".to_string())
        })?;
        
        info!("Loading historical data from: {:?}", data_file);
        let market_data = self.load_historical_data(&data_file).await?;
        
        if market_data.is_empty() {
            return Err(Error::DataError("No market data loaded".to_string()));
        }
        
        info!("Loaded {} data points", market_data.len());
        
        // Initialize account balance for backtesting
        self.balance.insert(
            "USDC".to_string(),
            Balance {
                asset: "USDC".to_string(),
                free: dec!(10000),  // Starting with 10,000 USDC
                locked: Decimal::ZERO,
                total: dec!(10000),
            },
        );
        
        // Run backtest
        let mut results = Vec::with_capacity(market_data.len());
        let mut equity_curve = Vec::with_capacity(market_data.len());
        
        for (i, data) in market_data.iter().enumerate() {
            if i % 100 == 0 {
                info!("Processing bar {}/{}", i + 1, market_data.len());
            }
            
            // Process market data and get signals
            let signals = self.process_market_data_with_strategies(data).await?;
            
            // Execute trades based on signals
            self.execute_signals(&signals, data).await?;
            
            // Update positions and balance
            self.update_positions().await?;
            self.update_balance().await?;
            
            // Calculate performance metrics
            let total_balance = self.calculate_total_balance(data.close);
            equity_curve.push(total_balance);
            
            // Log progress
            if i % 100 == 0 {
                debug!("Bar {}: Price = {}, Balance = {:.2}", i, data.close, total_balance);
            }
            
            results.push((data.timestamp, total_balance));
        }
        
        // Calculate and log backtest results
        self.log_backtest_results(&results, &equity_curve);
        
        Ok(())
    }
    
    /// Process market data and generate trading signals
    async fn process_market_data(&self, pair: &TradingPair) -> Result<Vec<Signal>> {
        debug!("Processing market data for {}", pair);
        
        // Fetch latest market data from exchanges
        let mut all_market_data = Vec::new();
        
        for (dex_name, client) in &self.dex_clients {
            match client.get_price(pair).await {
                Ok(price_data) => {
                    let market_data = MarketData {
                        timestamp: SystemTime::now(),
                        open: price_data.open,
                        high: price_data.high,
                        low: price_data.low,
                        close: price_data.close,
                        volume: price_data.volume,
                    };
                    all_market_data.push((dex_name, market_data));
                }
                Err(e) => {
                    error!("Failed to get price from {}: {}", dex_name, e);
                }
            }
        }
        
        if all_market_data.is_empty() {
            return Err(Error::DataError("Failed to fetch market data from all DEXs".to_string()));
        }
        
        // Get signals from all strategies
        let mut all_signals = Vec::new();
        
        for (dex_name, market_data) in all_market_data {
            for strategy in &self.strategies {
                match strategy.analyze(&[market_data.clone()]).await {
                    Ok(signals) => {
                        for signal in signals {
                            all_signals.push(Signal {
                                strategy: signal.strategy,
                                pair: pair.clone(),
                                action: signal.action,
                                price: signal.price,
                                timestamp: SystemTime::now(),
                                dex: Some(dex_name.clone()),
                                confidence: signal.confidence,
                            });
                        }
                    }
                    Err(e) => {
                        error!("Error analyzing market data with strategy: {}", e);
                    }
                }
            }
        }
        
        Ok(all_signals)
    }
    
    /// Process market data with strategies (for backtesting)
    async fn process_market_data_with_strategies(&self, data: &MarketData) -> Result<Vec<Signal>> {
        let mut all_signals = Vec::new();
        
        for strategy in &self.strategies {
            match strategy.analyze(&[data.clone()]).await {
                Ok(signals) => {
                    for signal in signals {
                        all_signals.push(signal);
                    }
                }
                Err(e) => {
                    error!("Error analyzing market data with strategy: {}", e);
                }
            }
        }
        
        Ok(all_signals)
    }
    
    /// Execute trading signals
    async fn execute_signals(&mut self, signals: &[Signal], market_data: &MarketData) -> Result<()> {
        if signals.is_empty() {
            return Ok(());
        }
        
        // Group signals by action type
        let mut buy_signals = Vec::new();
        let mut sell_signals = Vec::new();
        
        for signal in signals {
            match signal.action {
                Action::Buy => buy_signals.push(signal),
                Action::Sell => sell_signals.push(signal),
                _ => {}
            }
        }
        
        // Process buy signals
        if !buy_signals.is_empty() {
            self.execute_buy_signals(&buy_signals, market_data).await?;
        }
        
        // Process sell signals
        if !sell_signals.is_empty() {
            self.execute_sell_signals(&sell_signals, market_data).await?;
        }
        
        Ok(())
    }
    
    /// Execute buy signals
    async fn execute_buy_signals(&mut self, signals: &[&Signal], market_data: &MarketData) -> Result<()> {
        debug!("Executing {} buy signals", signals.len());
        
        for signal in signals {
            let dex_name = signal.dex.as_ref().unwrap_or(&"default".to_string());
            
            // Get the DEX client
            let client = match self.dex_clients.get(dex_name) {
                Some(client) => client,
                None => {
                    error!("DEX client not found: {}", dex_name);
                    continue;
                }
            };
            
            // Calculate position size based on risk management
            let equity = self.current_balance;
            let position_size = self.position_sizer.size(equity, &signal.pair.to_string()).await;
            
            // Place a buy order
            match client.place_order(
                &signal.pair,
                OrderSide::Buy,
                OrderType::Market,
                position_size,
                Some(market_data.close * 1.01),  // 1% above market price
                None,
            ).await {
                Ok(order) => {
                    info!("Placed buy order: {:?}", order);
                    self.order_history.push(order);
                }
                Err(e) => {
                    error!("Failed to place buy order: {}", e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Execute sell signals
    async fn execute_sell_signals(&mut self, signals: &[&Signal], market_data: &MarketData) -> Result<()> {
        debug!("Executing {} sell signals", signals.len());
        
        for signal in signals {
            let dex_name = signal.dex.as_ref().unwrap_or(&"default".to_string());
            
            // Get the DEX client
            let client = match self.dex_clients.get(dex_name) {
                Some(client) => client,
                None => {
                    error!("DEX client not found: {}", dex_name);
                    continue;
                }
            };
            
            // Check if we have an open position to sell
            if let Some(position) = self.positions.get(&signal.pair.to_string()) {
                // Place a sell order for the full position
                match client.place_order(
                    &signal.pair,
                    OrderSide::Sell,
                    OrderType::Market,
                    position.size,
                    Some(market_data.close * 0.99),  // 1% below market price
                    None,
                ).await {
                    Ok(order) => {
                        info!("Placed sell order: {:?}", order);
                        self.order_history.push(order);
                    }
                    Err(e) => {
                        error!("Failed to place sell order: {}", e);
                    }
                }
            }
        }
        
        Ok(())
    }
    

        
    }
    
    /// Update positions based on filled orders
    async fn update_positions(&mut self) -> Result<()> {
        // Process all orders that have been filled since last update
        for order in &self.order_history {
            if !order.filled_at.is_some() {
                continue;
            }
            
            let pair_str = order.pair.to_string();
            
            match order.side {
                OrderSide::Buy => {
                    // Update or create position
                    let position = self.positions.entry(pair_str).or_insert(Position {
                        pair: order.pair.clone(),
                        size: Decimal::ZERO,
                        entry_price: Decimal::ZERO,
                        current_price: Decimal::ZERO,
                        pnl: Decimal::ZERO,
                        pnl_percent: Decimal::ZERO,
                    });
                    
                    // Update position size and calculate new average entry price
                    let total_cost = position.entry_price * position.size + order.price * order.size;
                    position.size += order.size;
                    position.entry_price = total_cost / position.size;
                }
                OrderSide::Sell => {
                    // Reduce or close position
                    if let Some(position) = self.positions.get_mut(&pair_str) {
                        position.size -= order.size;
                        
                        // Remove position if fully closed
                        if position.size <= Decimal::ZERO {
                            self.positions.remove(&pair_str);
                        }
                    }
                }
            }
        }
        
        // Update current prices and PnL for all positions
        for (_, position) in self.positions.iter_mut() {
            if let Ok(price_data) = self.get_best_price(&position.pair).await {
                position.current_price = price_data.close;
                position.pnl = (position.current_price - position.entry_price) * position.size;
                position.pnl_percent = if position.entry_price > Decimal::ZERO {
                    (position.current_price / position.entry_price - Decimal::ONE) * Decimal::from(100)
                } else {
                    Decimal::ZERO
                };
            }
        }
        
        Ok(())
    }
    
    /// Update account balance from exchanges
    async fn update_balance(&mut self) -> Result<()> {
        for (dex_name, client) in &self.dex_clients {
            match client.get_balance().await {
                Ok(balances) => {
                    for balance in balances {
                        self.balance.insert(balance.asset.clone(), balance);
                    }
                }
                Err(e) => {
                    error!("Failed to get balance from {}: {}", dex_name, e);
                }
            }
        }
        Ok(())
    }
    
    /// Calculate total account balance in quote currency (e.g., USDC)
    fn calculate_total_balance(&self, current_price: f64) -> f64 {
        let mut total = 0.0;
        
        for (asset, balance) in &self.balance {
            if asset == "USDC" {
                total += balance.free + balance.locked;
            } else if let Some(position) = self.positions.get(asset) {
                // Add position value at current price
                total += (position.size * position.current_price).to_f64().unwrap_or(0.0);
            }
        }
        
        total
    }
    
    /// Log current account balance
    fn log_balance(&self) {
        info!("=== Account Balance ===");
        for (asset, balance) in &self.balance {
            info!("{}: {:.2} (Free: {:.2}, Locked: {:.2})", 
                asset, balance.total, balance.free, balance.locked);
        }
        
        if !self.positions.is_empty() {
            info!("\n=== Open Positions ===");
            for (_, position) in &self.positions {
                info!("{}: Size: {:.4}, Entry: ${:.2}, Current: ${:.2}, PnL: ${:.2} ({:.2}%)",
                    position.pair,
                    position.size,
                    position.entry_price,
                    position.current_price,
                    position.pnl,
                    position.pnl_percent);
            }
        }
    }
    
    /// Log backtest results
    fn log_backtest_results(&self, results: &[(SystemTime, f64)], equity_curve: &[f64]) {
        if results.is_empty() || equity_curve.is_empty() {
            return;
        }
        
        let initial_balance = equity_curve[0];
        let final_balance = *equity_curve.last().unwrap();
        let total_return = (final_balance / initial_balance - 1.0) * 100.0;
        
        // Calculate max drawdown
        let mut max_balance = initial_balance;
        let mut max_drawdown = 0.0;
        
        for &balance in equity_curve {
            if balance > max_balance {
                max_balance = balance;
            }
            let drawdown = (max_balance - balance) / max_balance * 100.0;
            if drawdown > max_drawdown {
                max_drawdown = drawdown;
            }
        }
        
        // Calculate win/loss stats
        let mut winning_trades = 0;
        let mut losing_trades = 0;
        let mut total_win = 0.0;
        let mut total_loss = 0.0;
        
        for order in &self.order_history {
            if let Some(pnl) = order.realized_pnl {
                if pnl > 0.0 {
                    winning_trades += 1;
                    total_win += pnl;
                } else {
                    losing_trades += 1;
                    total_loss += pnl.abs();
                }
            }
        }
        
        let total_trades = winning_trades + losing_trades;
        let win_rate = if total_trades > 0 {
            winning_trades as f64 / total_trades as f64 * 100.0
        } else {
            0.0
        };
        
        let avg_win = if winning_trades > 0 { total_win / winning_trades as f64 } else { 0.0 };
        let avg_loss = if losing_trades > 0 { total_loss / losing_trades as f64 } else { 0.0 };
        let profit_factor = if total_loss > 0.0 { total_win / total_loss } else { 0.0 };
        
        // Log summary
        info!("\n=== Backtest Results ===");
        info!("Period: {} to {}", 
            results[0].0.duration_since(UNIX_EPOCH).unwrap().as_secs(),
            results.last().unwrap().0.duration_since(UNIX_EPOCH).unwrap().as_secs());
        info!("Initial Balance: ${:.2}", initial_balance);
        info!("Final Balance: ${:.2}", final_balance);
        info!("Total Return: {:.2}%", total_return);
        info!("Max Drawdown: {:.2}%", max_drawdown);
        info!("\n=== Trades ===");
        info!("Total Trades: {}", total_trades);
        info!("Winning Trades: {} ({:.2}%)", winning_trades, win_rate);
        info!("Losing Trades: {} ({:.2}%)", losing_trades, 100.0 - win_rate);
        info!("Average Win: ${:.2}", avg_win);
        info!("Average Loss: ${:.2}", avg_loss);
        info!("Profit Factor: {:.2}", profit_factor);
        
        // Store performance metrics
        self.performance_metrics = Some(PerformanceMetrics {
            total_return,
            annualized_return: 0.0,  // Would need time period to annualize
            max_drawdown,
            sharpe_ratio: 0.0,  // Would need risk-free rate and volatility
            sortino_ratio: 0.0,  // Would need downside deviation
            win_rate,
            profit_factor,
            total_trades,
            winning_trades,
            losing_trades,
            avg_win,
            avg_loss: -avg_loss,  // Store as positive for consistency
        });
    }
    
    /// Get the best bid price from available DEXs (stub)
    async fn get_best_price(&self, pair: &TradingPair) -> Result<f64> {
        let mut best_bid = 0.0;
        let mut best_ask = f64::MAX;
        let mut best_dex = "".to_string();
        
        for (dex_name, client) in &self.dex_clients {
            match client.get_price(pair).await {
                Ok(price_data) => {
                    if price_data.bid > best_bid {
                        best_bid = price_data.bid;
                        best_ask = price_data.ask;
                        best_dex = dex_name.clone();
                    }
                }
                Err(e) => {
                    error!("Failed to get price from {}: {}", dex_name, e);
                }
            }
        }
        
        if best_bid > 0.0 {
            return Ok(best_bid);
        }
        
        Err(Error::DataError("No valid prices available".to_string()))
    }
    
    /// Load historical market data from file
    async fn load_historical_data(&self, file_path: &Path) -> Result<Vec<MarketData>> {
        // Read file contents
        let data = fs::read_to_string(file_path)
            .map_err(|e| Error::DataError(format!("Failed to read data file: {}", e)))?;
        
        // Parse CSV data
        let mut rdr = csv::Reader::from_reader(data.as_bytes());
        let mut market_data = Vec::new();
        
        for result in rdr.deserialize() {
            let record: MarketData = result.map_err(|e| {
                Error::DataError(format!("Failed to parse market data: {}", e))
            })?;
            market_data.push(record);
        }
        
        // Sort by timestamp (oldest first)
        market_data.sort_by_key(|d| d.timestamp);
        
        Ok(market_data)
    }
}

/// Test connections to exchanges
async fn test_connections() -> Result<()> {
    info!("Testing connections to exchanges...");
    
    // Load configuration
    let config_path = Path::new("config.toml");
    let config = if config_path.exists() {
        info!("Loading configuration from {:?}", config_path);
        Config::from_file(config_path)?
    } else {
        warn!("Configuration file not found, using default settings");
        Config::default()
    };
    
    let mut success = true;
    
    // Test each enabled DEX connection
    for (dex_name, dex_config) in &config.dex {
        if !dex_config.enabled {
            info!("Skipping disabled DEX: {}", dex_name);
            continue;
        }
        
        info!("Testing connection to {}...", dex_name);
        
        match DexFactory::create_client(dex_name) {
            Ok(client) => {
                // Test connectivity
                match client.ping().await {
                    Ok(_) => {
                        info!("✓ {} connection successful", dex_name);
                        
                        // Test price feed
                        match client.get_price(&TradingPair::from_str("SOL/USDC").unwrap()).await {
                            Ok(price) => {
                                info!("  Current SOL/USC price: ${:.4}", price.close);
                            }
                            Err(e) => {
                                error!("  Failed to get price: {}", e);
                                success = false;
                            }
                        }
                    }
                    Err(e) => {
                        error!("✗ {} connection failed: {}", dex_name, e);
                        success = false;
                    }
                }
            }
            Err(e) => {
                error!("✗ Failed to create {} client: {}", dex_name, e);
                success = false;
            }
        }
    }
    
    if success {
        info!("All connections tested successfully");
        Ok(())
    } else {
        Err(Error::ConnectionError("One or more connections failed".to_string()))
    }
}

/// List available trading pairs for a DEX
async fn list_trading_pairs(dex_name: &str) -> Result<()> {
    info!("Fetching available trading pairs from {}...", dex_name);
    
    // Create DEX client
    let client = DexFactory::create_client(dex_name)
        .map_err(|e| Error::DexError(format!("Failed to create {} client: {}", dex_name, e)))?;
    
    // Fetch trading pairs
    let pairs = client.get_trading_pairs().await
        .map_err(|e| Error::DexError(format!("Failed to fetch trading pairs: {}", e)))?;
    
    if pairs.is_empty() {
        info!("No trading pairs found on {}", dex_name);
        return Ok(());
    }
    
    // Group pairs by quote asset
    let mut pairs_by_quote: HashMap<String, Vec<String>> = HashMap::new();
    
    for pair in pairs {
        let quote_asset = pair.quote.clone();
        pairs_by_quote
            .entry(quote_asset)
            .or_default()
            .push(pair.base);
    }
    
    // Display pairs grouped by quote asset
    info!("\n=== Available Trading Pairs on {} ===", dex_name);
    
    let mut quote_assets: Vec<_> = pairs_by_quote.keys().collect();
    quote_assets.sort();
    
    for quote in quote_assets {
        let mut bases = pairs_by_quote[quote].clone();
        bases.sort();
        
        info!("\n{} Markets ({}):", quote, bases.len());
        
        // Print in columns
        let chunk_size = 4;
        for chunk in bases.chunks(chunk_size) {
            let line = chunk.iter()
                .map(|base| format!("{}/{}", base, quote))
                .collect::<Vec<_>>()
                .join("  ");
            info!("  {}", line);
        }
    }
    
    info!("\nTotal pairs: {}", pairs.len());
    Ok(())
}

/// Get account balance for a DEX
async fn get_balance(dex_name: &str) -> Result<()> {
    info!("Fetching account balance from {}...", dex_name);
    
    // Create DEX client
    let client = DexFactory::create_client(dex_name)
        .map_err(|e| Error::DexError(format!("Failed to create {} client: {}", dex_name, e)))?;
    
    // Fetch balance
    let balances = client.get_balance().await
        .map_err(|e| Error::DexError(format!("Failed to fetch balance: {}", e)))?;
    
    if balances.is_empty() {
        info!("No balance information available");
        return Ok(());
    }
    
    // Calculate total value in USD (simplified)
    let mut total_value = Decimal::ZERO;
    let mut balance_list: Vec<&Balance> = balances.iter().collect();
    
    // Sort by value (descending)
    balance_list.sort_by(|a, b| b.total.cmp(&a.total));
    
    info!("\n=== Account Balance on {} ===", dex_name);
    
    // Display balances
    for balance in balance_list {
        if balance.total > Decimal::ZERO {
            // In a real implementation, you would convert to USD using current prices
            let usd_value = balance.total;  // This should be replaced with actual conversion
            total_value += usd_value;
            
            info!(
                "{:>10}: {:>15.4} (Free: {:>10.4}, Locked: {:>10.4}) ~ ${:>10.2}",
                balance.asset,
                balance.total,
                balance.free,
                balance.locked,
                usd_value
            );
        }
    }
    
    info!("\nTotal Balance: ~ ${:.2}", total_value);
    
    Ok(())
}

/// Parse timeframe string (e.g., "1h", "15m") into Duration
fn parse_timeframe(timeframe: &str) -> Duration {
    let len = timeframe.len();
    if len < 2 {
        return Duration::from_secs(60);  // Default to 1 minute
    }
    
    let (num, unit) = timeframe.split_at(len - 1);
    let num = num.parse::<u64>().unwrap_or(1);
    
    match unit.to_lowercase().as_str() {
        "s" => Duration::from_secs(num),
        "m" => Duration::from_secs(num * 60),
        "h" => Duration::from_secs(num * 60 * 60),
        "d" => Duration::from_secs(num * 60 * 60 * 24),
        "w" => Duration::from_secs(num * 60 * 60 * 24 * 7),
        _ => Duration::from_secs(60),  // Default to 1 minute
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[tokio::test]
    async fn test_config_generation() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let config_path = temp_file.path();
        
        let config = Config::default();
        config.save_to_file(config_path).unwrap();
        
        assert!(config_path.exists());
        
        let loaded_config = Config::from_file(config_path).unwrap();
        assert_eq!(config.app.log_level, loaded_config.app.log_level);
    }
    
    #[tokio::test]
    async fn test_trading_bot_initialization() {
        let config = Config::default();
        let bot = TradingBot::new(config, false).await;
        assert!(bot.is_ok());
    }
}
