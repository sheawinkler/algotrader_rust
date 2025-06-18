//! # AlgoTraderV2 Rust
//! Main library file for AlgoTraderV2
//! A high-performance algorithmic trading system built in Rust.

// Temporarily silence common unused-code warnings while modules are under active development.
#![allow(dead_code, unused_imports, unused_variables)]

pub use crate::utils::error::{Error, Result};

pub mod analysis;
pub mod blockchain;
pub mod config;
pub mod dex;
pub mod performance;
pub mod strategies;  // unified rich strategy module
pub mod indicators;
pub mod utils;
pub mod trading;
pub mod engine;
pub mod backtest;



use crate::engine::market_router::MarketRouter;
use std::collections::HashMap;
use crate::dex::DexFactory;
use crate::strategies::TradingStrategy;
use crate::performance::PerformanceMonitor;
use crate::utils::types::{MarketData, TradingPair, Signal, SignalAction, Order, OrderSide};
use crate::trading::{Signal as StratSignal, SignalType};
use tokio::sync::mpsc;
use solana_sdk::signature::Keypair;
use crate::analysis::wallet_analyzer::WalletAnalyzer;





/// Arbitrage opportunity struct
#[derive(Debug, Clone)]
pub struct ArbitrageOpportunity {
    pub symbol: String,
    pub buy_dex: String,
    pub buy_price: f64,
    pub sell_dex: String,
    pub sell_price: f64,
    pub spread: f64,
}

/// Record of a trade for analytics
#[derive(Debug, Clone)]
pub struct TradeRecord {
    pub timestamp: i64,
    pub symbol: String,
    pub side: String,
    pub size: f64,
    pub price: f64,
    pub pnl: f64,
    pub stop_loss_triggered: bool,
}

/// Main trading engine that coordinates between DEX and strategies
pub struct TradingEngine {
    // DEX clients
    dex_clients: std::collections::HashMap<String, Box<dyn dex::DexClient>>,
    
    // Trading strategies with performance monitoring
    strategies: Vec<Box<dyn strategies::TradingStrategy>>,
    
    // Performance monitoring
    performance_monitors: std::collections::HashMap<String, performance::PerformanceMonitor>,
    
    // Configuration
    config: crate::config::Config,
    
    // Runtime state
    is_running: bool,
    last_performance_review: std::time::Instant,
    
    // Wallet addresses for monitoring
    pub trading_wallet: String, // For all trade execution
    pub personal_wallet: String, // For review and analytics
    pub wallet_analyzer: Option<WalletAnalyzer>,
    // --- EXECUTION PARAMETERS ---
    pub slippage_bps: u16,
    pub max_fee_lamports: u64,
    pub split_threshold_sol: f64,
    pub split_chunk_sol: f64,
    pub split_delay_ms: u64,
    // Wallet rotation
    pub wallet_pool: Vec<String>,
    wallet_index: usize,
    
    // --- RISK PARAMETERS ---
    pub starting_balance: f64, // e.g. 4.0 SOL
    pub current_balance: f64,  // updated after each trade
    pub max_position_pct: f64, // e.g. 0.05 (5% of balance, capped)
    pub max_position_abs: f64, // e.g. 0.2 SOL cap
    pub max_open_trades: usize, // e.g. 3
    pub stop_loss_pct: f64, // e.g. 0.10 (10% SL)
    pub max_daily_loss_pct: f64, // e.g. 0.15 (15% daily loss)
    pub daily_loss: f64, // tracked daily
    pub open_trades: usize, // tracked live
    pub trade_history: Vec<TradeRecord>, // for analytics
    pub open_positions: std::collections::HashMap<String, (f64, f64)>,
    
    // Arbitrage stub
    pub paper_trading: bool,
    enable_arbitrage: bool,
}

impl TradingEngine {
    // Add stub for on_market_event to satisfy MarketEventHandler
    pub fn on_market_event(&mut self, _event: crate::utils::market_stream::MarketEvent) -> anyhow::Result<()> {
        // TODO: Implement event handling logic
        Ok(())
    }
    /// Create a new trading engine with the given configuration
    pub fn new() -> Self {
        Self::with_config(crate::config::Config::default(), false)
    }
    /// Construct engine from configuration
    pub fn with_config(config: crate::config::Config, paper_trading: bool) -> Self {
        // Build strategies first
        // Extract execution parameters before moving config
        let slippage_bps = config.trading.slippage_bps;
        let max_fee_lamports = config.trading.max_fee_lamports;
        let split_threshold_sol = config.trading.split_threshold_sol;
        let split_chunk_sol = config.trading.split_chunk_sol;
        let split_delay_ms = config.trading.split_delay_ms;
        let wallet_pool = config.wallet.wallets.clone();
        let mut strategies_vec: Vec<Box<dyn crate::strategies::TradingStrategy>> = Vec::new();
        for scfg in &config.trading.strategies {
            if scfg.enabled {
                match crate::strategies::StrategyFactory::create_strategy(&scfg.name, scfg) {
                    Ok(s) => strategies_vec.push(s),
                    Err(e) => log::error!("Failed to init strategy {}: {}", scfg.name, e),
                }
            }
        }
        // Build performance monitors map
        let mut perf_map = std::collections::HashMap::new();
        for strat in &strategies_vec {
            perf_map.insert(strat.name().to_string(), PerformanceMonitor::new());
        }
        TradingEngine {
            dex_clients: std::collections::HashMap::new(),
            strategies: strategies_vec,
            performance_monitors: perf_map,
            config,
            is_running: false,
            last_performance_review: std::time::Instant::now(),
            trading_wallet: String::new(),
            personal_wallet: String::new(),
            wallet_analyzer: None,
            starting_balance: 0.0,
            current_balance: 0.0,
            max_position_pct: 0.0,
            max_position_abs: 0.0,
            max_open_trades: 0,
            stop_loss_pct: 0.0,
            max_daily_loss_pct: 0.0,
            daily_loss: 0.0,
            open_trades: 0,
            trade_history: Vec::new(),
            open_positions: std::collections::HashMap::new(),
            slippage_bps,
            max_fee_lamports,
            split_threshold_sol,
            split_chunk_sol,
            split_delay_ms,
            wallet_pool,
            wallet_index: 0,
            paper_trading,
            enable_arbitrage: false,
        }
    }

    /// Rotate to next wallet in the configured pool. Returns Some(wallet) or None if pool empty.
    pub fn next_wallet(&mut self) -> Option<String> {
        if self.wallet_pool.is_empty() {
            return None;
        }
        let w = self.wallet_pool[self.wallet_index % self.wallet_pool.len()].clone();
        self.wallet_index = (self.wallet_index + 1) % self.wallet_pool.len();
        Some(w)
    }

    /// Start the trading engine with market data orchestrator
    /// Generate session report: PnL, win rate, drawdown, best/worst trades, risk metrics
    pub fn session_report(&self) {
        let total_trades = self.trade_history.len();
        let wins = self.trade_history.iter().filter(|t| t.pnl > 0.0).count();
        let losses = self.trade_history.iter().filter(|t| t.pnl <= 0.0).count();
        let pnl: f64 = self.trade_history.iter().map(|t| t.pnl).sum();
        let win_rate = if total_trades > 0 { wins as f64 / total_trades as f64 } else { 0.0 };
        let max_drawdown = self.max_drawdown();
        let best_trade = self.trade_history.iter().max_by(|a, b| a.pnl.partial_cmp(&b.pnl).unwrap());
        let worst_trade = self.trade_history.iter().min_by(|a, b| a.pnl.partial_cmp(&b.pnl).unwrap());
        println!("=== SESSION REPORT ===");
        println!("Total Trades: {} | Wins: {} | Losses: {}", total_trades, wins, losses);
        println!("PnL: {:.4} SOL | Win Rate: {:.2}%", pnl, win_rate * 100.0);
        println!("Max Drawdown: {:.2}%", max_drawdown * 100.0);
        if let Some(best) = best_trade { println!("Best Trade: {} {} @ {:.4} (PnL: {:.4})", best.symbol, best.side, best.price, best.pnl); }
        if let Some(worst) = worst_trade { println!("Worst Trade: {} {} @ {:.4} (PnL: {:.4})", worst.symbol, worst.side, worst.price, worst.pnl); }
        println!("Risk: Max Position {:.2} SOL ({}%), Max Daily Loss {:.2}%", self.max_position_abs, self.max_position_pct * 100.0, self.max_daily_loss_pct * 100.0);
        if self.daily_loss / self.starting_balance > self.max_daily_loss_pct {
            println!("[ALERT] Max daily loss breached!");
        }
        if self.open_trades > self.max_open_trades {
            println!("[ALERT] Max open trades exceeded!");
        }
        println!("=====================");
    }

    /// Compute max drawdown from trade history
    pub fn max_drawdown(&self) -> f64 {
        let mut peak = self.starting_balance;
        let mut max_dd = 0.0;
        let mut equity = self.starting_balance;
        for t in &self.trade_history {
            equity += t.pnl;
            if equity > peak { peak = equity; }
            let dd = (peak - equity) / peak;
            if dd > max_dd { max_dd = dd; }
        }
        max_dd
    }

    /// Core arbitrage logic
    pub async fn try_arbitrage(&self, symbol: &str) -> Option<ArbitrageOpportunity> {
        use futures::future::join_all;
        if !self.enable_arbitrage { return None; }
        let mut price_futures = vec![];
        for (dex_name, dex_client) in &self.dex_clients {
            if let Some((base, quote)) = symbol.split_once('/') {
                let fut = dex_client.get_price(base, quote);
                price_futures.push((dex_name.clone(), fut));
            }
        }
        let mut prices = vec![];
        let results = join_all(price_futures.into_iter().map(|(dex_name, fut)| async move {
            (dex_name, fut.await)
        })).await;
        for (dex_name, price_res) in results {
            if let Ok(price) = price_res {
                prices.push((dex_name, price));
            }
        }
        if prices.len() < 2 { return None; }
        let (buy_dex, buy_price) = prices.iter().min_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).unwrap();
        let (sell_dex, sell_price) = prices.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).unwrap();
        let spread = sell_price - buy_price;
        if spread > 0.002 * buy_price {
            println!("[ARBITRAGE] Opportunity: Buy {} @ {} ({}), Sell @ {} ({})", symbol, buy_price, buy_dex, sell_price, sell_dex);
            return Some(ArbitrageOpportunity {
                symbol: symbol.to_string(),
                buy_dex: buy_dex.clone(),
                buy_price: *buy_price,
                sell_dex: sell_dex.clone(),
                sell_price: *sell_price,
                spread,
            });
        }
        None
    }
}

impl TradingEngine {
    /// Enforce risk parameters and dynamic scaling
    pub fn enforce_risk(&mut self) {
        if self.daily_loss / self.starting_balance > self.max_daily_loss_pct {
            // Halt trading for the day
            println!("[RISK] Max daily loss hit. Trading halted.");
            // Add logic to halt trading (set a flag, etc.)
        }
        if self.open_trades >= self.max_open_trades {
            println!("[RISK] Max open trades reached.");
        }
    }

    /// Evaluate position size for next trade
    pub fn position_size(&self) -> f64 {
        let pct_size = self.current_balance * self.max_position_pct;
        let capped = pct_size.min(self.max_position_abs);
        capped.min(self.current_balance)
    }

    /// Update risk parameters as capital grows
    pub fn adjust_risk(&mut self) {
        if self.current_balance > 50.0 {
            self.max_position_pct = 0.03;
            self.stop_loss_pct = 0.07;
            self.max_position_abs = 1.0;
        }
        if self.current_balance > 500.0 {
            self.max_position_pct = 0.02;
            self.stop_loss_pct = 0.05;
            self.max_position_abs = 5.0;
        }
        if self.current_balance > 5000.0 {
            self.max_position_pct = 0.01;
            self.stop_loss_pct = 0.04;
            self.max_position_abs = 20.0;
        }
    }

    /// Apply trade effects of a single trade chunk, updating engine state and returning realized PnL.
    pub fn apply_trade_effects(&mut self, sig: &Signal, chunk: f64) -> f64 {
        let mut pnl: f64 = 0.0;
        let symbol_key = sig.pair.to_string();
        match sig.action {
            SignalAction::Buy => {
                // Cost of buying decreases balance
                self.current_balance -= chunk * sig.price;
                self.open_trades += 1;

                // Update or create position entry (size, weighted avg entry price)
                let entry = self.open_positions.entry(symbol_key.clone()).or_insert((0.0, 0.0));
                let (prev_size, prev_avg) = *entry;
                let new_size = prev_size + chunk;
                let new_avg = if new_size > 0.0 {
                    (prev_size * prev_avg + chunk * sig.price) / new_size
                } else {
                    0.0
                };
                *entry = (new_size, new_avg);
            }
            SignalAction::Sell => {
                // Realize PnL against any open position
                if let Some((pos_size, entry_price)) = self.open_positions.get_mut(&symbol_key) {
                    // Close up to the held position size
                    let close_size = chunk.min(*pos_size);
                    pnl = (sig.price - *entry_price) * close_size;

                    // Selling credits balance (includes PnL implicitly)
                    self.current_balance += close_size * sig.price;
                    *pos_size -= close_size;

                    // Remove position if fully closed
                    if *pos_size <= 0.0 {
                        self.open_positions.remove(&symbol_key);
                    }

                    if self.open_trades > 0 {
                        self.open_trades -= 1;
                    }
                } else {
                    // No open position – treat as flat sell, credit balance
                    self.current_balance += chunk * sig.price;
                }
            }
            _ => {}
        }

        // Update tracked daily loss relative to starting balance
        self.daily_loss = (self.starting_balance - self.current_balance).max(0.0);
        pnl
    }

    pub async fn start_with_market_router(&mut self, symbols: Vec<String>, helius_api_key: Option<String>, openbook_program_id: Option<String>) -> anyhow::Result<()> {
        let mut router = MarketRouter::new();

        // Prepare symbols for streams
        let symbol_strs: Vec<String> = symbols.iter().cloned().collect();

        // Add Binance stream
        let binance_stream = Box::new(crate::utils::binance_stream::BinanceStream::new(&symbol_strs));
        router.add_stream(binance_stream);

        // Add Helius stream if API key provided
        if let Some(api_key) = helius_api_key {
            let helius_stream = Box::new(crate::utils::helius_stream::HeliusStream::new(&api_key, openbook_program_id.as_deref()));
            router.add_stream(helius_stream);
        }

        // Add more streams as needed (Coinbase, Kraken, Serum, Triton, etc)
        let coinbase_stream = Box::new(crate::utils::coinbase_stream::CoinbaseStream::new(&symbol_strs));
        router.add_stream(coinbase_stream);
        let kraken_stream = Box::new(crate::utils::kraken_stream::KrakenStream::new());
        router.add_stream(kraken_stream);
        // SerumStream requires a market symbol, use the first symbol or fallback
        let serum_market = symbol_strs.get(0).cloned().unwrap_or_else(|| "SOL/USDC".to_string());
        let serum_stream = Box::new(crate::utils::serum_stream::SerumStream::new(&serum_market));
        router.add_stream(serum_stream);
        // TritonStream requires api_key and market, use dummy key for now
        let triton_api_key = std::env::var("TRITON_API_KEY").unwrap_or_else(|_| "demo-key".to_string());
        let triton_market = symbol_strs.get(0).cloned().unwrap_or_else(|| "SOL/USDC".to_string());
        let triton_stream = Box::new(crate::utils::triton_stream::TritonStream::new(&triton_api_key, &triton_market));
        router.add_stream(triton_stream);

        // --- DEX Integration ---
        // Initialize all DEX clients and store in registry
        let dex_names = ["jupiter", "raydium", "photon"];
        let mut dex_clients = HashMap::new();
        for name in dex_names.iter() {
            if let Ok(client) = DexFactory::create_client(name) {
                dex_clients.insert(name.to_string(), client);
            }
        }
        self.dex_clients = dex_clients;

        // --- Wallet Setup ---
        // Set trading and personal wallet addresses
        self.trading_wallet = "5RS2mgUqL1CDDxXQakzMSFdS8HBjW8LMwnoFtuuFvrtF".to_string();
        self.personal_wallet = "2qCe3m9K22cGH9UXhaHDafqk47zJLnwA1d13m8j5PBbB".to_string();
        // Initialize wallet analyzer if not already set
        if self.wallet_analyzer.is_none() {
            // Example: use mainnet Solana RPC and default config
            let rpc_url = "https://api.mainnet-beta.solana.com";
            let keypair = Keypair::new(); // Replace with actual keypair for trading wallet
            self.wallet_analyzer = WalletAnalyzer::new(rpc_url, keypair, None).ok();
        }

        // Channel for event delivery
        let (tx, mut rx) = mpsc::channel(512);

        // Spawn router (all streams)
        let mut router_task = router;
        let symbols_clone = symbols.clone();
        let router_handle = tokio::spawn(async move {
            router_task.run(symbols_clone, tx).await
        });

        // Event loop: process market events until router terminates
        tokio::select! {
            res = async {
                

                while let Some(evt) = rx.recv().await {
                    if let Some(data) = TradingEngine::convert_market_event(&evt) {
                        let mut collected_signals: Vec<Signal> = Vec::new();
                        for strat in self.strategies.iter_mut() {
                            let sigs = strat.generate_signals(&data).await;
                            for s in sigs {
                                if let Some(engine_sig) = TradingEngine::convert_strategy_signal(&s, strat.name()) {
                                    collected_signals.push(engine_sig);
                                }
                            }
                        }
                        self.handle_signals(collected_signals).await?;
                    }
                }
                Ok::<(), anyhow::Error>(())
            } => { res?;
                    Ok::<(), anyhow::Error>(())
                },
            router_res = router_handle => {
                router_res??;
                Ok::<(), anyhow::Error>(())
            },
        }?;
        Ok(())
    }

    /// Convert a strategy-facing Signal into engine/internal Signal format
    fn convert_strategy_signal(sig: &StratSignal, strat_name: &str) -> Option<Signal> {
        // Parse symbol like "SOL/USDC" into TradingPair
        let parts: Vec<&str> = sig.symbol.split('/').collect();
        let pair = if parts.len() == 2 {
            TradingPair::new(parts[0], parts[1])
        } else {
            TradingPair::new(&sig.symbol, "USDC")
        };
        Some(Signal {
            strategy_id: strat_name.to_string(),
            pair,
            action: match sig.signal_type {
                SignalType::Buy => SignalAction::Buy,
                SignalType::Sell => SignalAction::Sell,
                SignalType::Close => SignalAction::Close,
                SignalType::Cancel => SignalAction::Cancel,
                SignalType::Arbitrage { .. } => {
                    // Map arbitrage signal to Buy for now; improve later with side fields.
                    SignalAction::Buy
                },
            },
            price: sig.price,
            size: sig.size,
            stop_loss: None,
            take_profit: None,
            timestamp: sig.timestamp,
            metadata: std::collections::HashMap::new(),
        })
    }

    async fn handle_signals(&mut self, signals: Vec<Signal>) -> anyhow::Result<()> {
        for sig in signals {
            // Risk checks
            if self.open_trades >= self.max_open_trades {
                log::warn!("Max open trades reached – signal ignored: {:?}", sig);
                continue;
            }
            if self.daily_loss / self.starting_balance >= self.max_daily_loss_pct {
                log::warn!("Daily loss limit reached – signal ignored: {:?}", sig);
                continue;
            }
            // Decide amount: simple fixed % of balance
            let chunk = (self.current_balance * self.max_position_pct).min(self.max_position_abs);
            // Determine trade splitting based on config and strategy type
            let mut trade_chunks: Vec<f64> = Vec::new();
            if chunk > self.split_threshold_sol && !sig.strategy_id.to_lowercase().contains("arbitrage") {
                let mut remaining = chunk;
                while remaining > 0.0 {
                    let piece = remaining.min(self.split_chunk_sol);
                    trade_chunks.push(piece);
                    remaining -= piece;
                }
            } else {
                trade_chunks.push(chunk);
            }
            if chunk == 0.0 {
                continue;
            }
            // Attempt trade on available DEX clients in preferred order
            // Determine signer wallet using rotation (falls back to trading_wallet)
            let wallet = self.next_wallet().unwrap_or_else(|| self.trading_wallet.clone());
            let mut total_pnl_chunked = 0.0;
            for &chunk in trade_chunks.iter() {
            if !self.paper_trading {
                let preferred = ["jupiter", "raydium", "photon"];
                let mut executed = false;
                let mut last_err: Option<anyhow::Error> = None;
                for dex_name in preferred.iter() {
                    if let Some(dex) = self.dex_clients.get(*dex_name) {
                        match dex.execute_trade(
                                &sig.pair.base,
                                &sig.pair.quote,
                                chunk,
                                matches!(sig.action, crate::utils::types::SignalAction::Buy),
                                self.slippage_bps,
                                self.max_fee_lamports,
                                &wallet,
                            ).await {
                            Ok(_tx) => {
                                log::info!("Executed trade via {}: {:?}", dex_name, sig);
                                executed = true;
                                break;
                            }
                            Err(e) => {
                                log::warn!("{} execution failed: {}", dex_name, e);
                                last_err = Some(e.into());
                                continue;
                            }
                        }
                    }
                }
                if !executed {
                    log::error!("All DEX clients failed to execute trade for {:?}", sig);
                    if let Some(strat) = self.strategies.iter_mut().find(|s| s.name() == sig.strategy_id) {
                        let failed_order = Order {
                            id: "".into(),
                            symbol: sig.pair.to_string(),
                            price: sig.price,
                            size: chunk,
                            side: if sig.action == SignalAction::Buy { OrderSide::Buy } else { OrderSide::Sell },
                            timestamp: sig.timestamp,
                        };
                        let err_anyhow: anyhow::Error = last_err.unwrap_or_else(|| anyhow::anyhow!("Unknown DEX error"));
                        strat.on_trade_error(&failed_order, &err_anyhow);
                    }
                    continue; // skip state update on failure
                }
            } else {
                log::info!("[PAPER] would execute trade: {:?}", sig);
            }
            // bookkeeping per chunk
                let pnl_chunk = self.apply_trade_effects(&sig, chunk);
                // Build order record for this chunk
                let order = Order {
                    id: format!("{}-{}-{}", sig.strategy_id, sig.timestamp, rand::random::<u16>()),
                    symbol: sig.pair.to_string(),
                    price: sig.price,
                    size: chunk,
                    side: if sig.action == SignalAction::Buy { OrderSide::Buy } else { OrderSide::Sell },
                    timestamp: sig.timestamp,
                };
                // Performance monitor
                if let Some(mon) = self.performance_monitors.get(&sig.strategy_id) {
                    let _ = mon.record_trade(&sig.strategy_id, &order, None,  pnl_chunk, 0.0001, None).await;
                }
                // Notify strategy
                if let Some(strat) = self.strategies.iter_mut().find(|s| s.name() == sig.strategy_id) {
                    strat.on_order_filled(&order);
                }
                // Append history
                self.trade_history.push(TradeRecord {
                    timestamp: sig.timestamp,
                    symbol: sig.pair.to_string(),
                    side: match sig.action { SignalAction::Buy => "buy".into(), SignalAction::Sell => "sell".into(), _ => "other".into() },
                    size: chunk,
                    price: sig.price,
                    pnl: pnl_chunk,
                    stop_loss_triggered: false,
                });
                total_pnl_chunked += pnl_chunk;
            }
        
            }

            



            
            
/* LEGACY DUPLICATE BLOCK START (commented out)
            match sig.action {
                SignalAction::Buy => {
                    self.current_balance -= chunk * sig.price;
                    self.open_trades += 1;
                    // Track position (accumulate size, weighted avg entry)
                    let entry = self.open_positions.entry(symbol_key.clone()).or_insert((0.0, 0.0));
                    let (prev_size, prev_avg) = *entry;
                    let new_size = prev_size + chunk;
                    let new_avg = if new_size > 0.0 {
                        (prev_size * prev_avg + chunk * sig.price) / new_size
                    } else {
                        0.0
                    };
                    *entry = (new_size, new_avg);
                }
                SignalAction::Sell => {
                    if let Some((pos_size, entry_price)) = self.open_positions.get_mut(&symbol_key) {
                        let close_size = chunk.min(*pos_size);
                        pnl = (sig.price - *entry_price) * close_size;
                        self.current_balance += close_size * sig.price; // realized revenue includes PnL implicitly
                        *pos_size -= close_size;
                        if *pos_size <= 0.0 { self.open_positions.remove(&symbol_key); }
                        if self.open_trades > 0 { self.open_trades -= 1; }
                        // update daily PnL
                        self.daily_loss = (self.starting_balance - self.current_balance).max(0.0);
                    } else {
                        // no position, treat as flat sell; just credit balance
                        self.current_balance += chunk * sig.price;
                    }
                }
                _ => {}
            }
            
            self.daily_loss = (self.starting_balance - self.current_balance).max(0.0);

            // Construct order record
            let order = Order {
                id: format!("{}-{}", sig.strategy_id, sig.timestamp),
                symbol: symbol_key.clone(),
                price: sig.price,
                size: chunk,
                side: if sig.action == SignalAction::Buy { OrderSide::Buy } else { OrderSide::Sell },
                timestamp: sig.timestamp,
            };
            // Call performance monitor if exists
            if let Some(mon) = self.performance_monitors.get(&sig.strategy_id) {
                let _ = mon.record_trade(&sig.strategy_id, &order, None,  pnl_chunk, 0.0001, None).await;
            }
            // Notify strategy
            if let Some(strat) = self.strategies.iter_mut().find(|s| s.name() == sig.strategy_id) {
                strat.on_order_filled(&order);
            }
            // Append history
            self.trade_history.push(TradeRecord {
                    timestamp: sig.timestamp,
                    symbol: sig.pair.to_string(),
                    side: match sig.action { SignalAction::Buy => "buy".into(), SignalAction::Sell => "sell".into(), _ => "other".into() },
                    size: chunk,
                    price: sig.price,
                    pnl: pnl_chunk,
                    stop_loss_triggered: false,
                });
                total_pnl_chunked += pnl_chunk;
                timestamp: sig.timestamp,
                symbol: sig.pair.to_string(),
                side: match sig.action { SignalAction::Buy => "buy".into(), SignalAction::Sell => "sell".into(), _ => "other".into() },
                },
                size: chunk,
                price: sig.price,
                

*/
        Ok(())
    }

    /// Convert incoming MarketEvent to simple MarketData for strategy consumption

    fn convert_market_event(event: &crate::utils::market_stream::MarketEvent) -> Option<MarketData> {
        use crate::utils::market_stream::MarketEvent::*;
        match event {
            Trade { symbol, price, qty, timestamp, .. } => {
                let pair = TradingPair::from_str(symbol).unwrap_or_else(|| TradingPair::new(symbol, "USDC"));
                Some(MarketData {
                    pair,
                    symbol: symbol.clone(),
                    candles: Vec::new(),
                    last_price: *price,
                    volume_24h: 0.0,
                    change_24h: 0.0,
                    volume: Some(*qty),
                    timestamp: *timestamp,
                    open: None,
                    high: None,
                    low: None,
                    close: *price,
                    order_book: None,
                    dex_prices: None,
                })
            }
            Ticker { symbol, price, timestamp, .. } => {
                let pair = TradingPair::from_str(symbol).unwrap_or_else(|| TradingPair::new(symbol, "USDC"));
                Some(MarketData {
                    pair,
                    symbol: symbol.clone(),
                    candles: Vec::new(),
                    last_price: *price,
                    volume_24h: 0.0,
                    change_24h: 0.0,
                    volume: None,
                    timestamp: *timestamp,
                    open: None,
                    high: None,
                    low: None,
                    close: *price,
                    order_book: None,
                    dex_prices: None,
                })
            }
            _ => None,
        }
    }

    /// Start the trading engine
    pub async fn start(&self) -> Result<()> {
        // Main trading loop
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trading_engine_initialization() {
        let engine = TradingEngine::new();
        // Add assertions
    }

    #[tokio::test]
    async fn test_trading_engine_start() {
        let engine = TradingEngine::new();
        assert!(engine.start().await.is_ok());
    }
}
