//! # AlgoTraderV2 Rust
//! Main library file for AlgoTraderV2
//! A high-performance algorithmic trading system built in Rust.

// Temporarily silence common unused-code warnings while modules are under active development.
#![allow(dead_code, unused_imports, unused_variables)]

pub use crate::utils::error::{Error, Result};

pub mod analysis;
pub mod backtest;
pub mod blockchain;
pub mod config;
pub mod dex;
pub mod engine;
pub mod indicators;
pub mod meta;
pub mod metrics;
pub mod performance;
pub mod persistence;
pub mod portfolio;
pub mod risk;
#[cfg(feature = "sidecar")]
pub mod sidecar;
pub mod strategies; // unified rich strategy module
pub mod trading;
pub mod utils;
pub mod signal;

pub mod dashboard;
pub mod market_data;
pub mod wallet;

#[cfg(feature = "db")]
pub mod data_layer;

// Import wallet module
use crate::wallet::Wallet;

use crate::dex::DexFactory;
use crate::engine::market_router::MarketRouter;
use serde::Deserialize;
use std::collections::HashMap;

use crate::analysis::wallet_analyzer::WalletAnalyzer;
use crate::market_data::ws::{self as market_ws, PriceCache};
use crate::performance::PerformanceMonitor;
use crate::persistence::{EquitySnapshot, Persistence, TradeRecord};
use crate::risk::position_sizer::{FixedFractionalSizer, PositionSizer};
use crate::risk::{RiskAction, RiskRule};
use crate::strategies::TradingStrategy;
use crate::trading::{Signal as StratSignal, SignalType};
use crate::utils::types::PendingOrder;
use tokio_postgres::types::ToSql;
use crate::signal::SignalSource;
use crate::utils::types::{MarketData, Order, OrderSide, Signal, SignalAction, TradingPair};
use chrono::{NaiveDateTime, Utc};
use solana_sdk::signature::Keypair;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

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
    pub trading_wallet: String,  // For all trade execution
    pub personal_wallet: String, // For review and analytics
    pub wallet_analyzer: Option<WalletAnalyzer>,
    // Wallet (None in paper mode)
    pub wallet: Option<Wallet>,
    // Portfolio tracking
    pub portfolio: crate::portfolio::Portfolio,
    // Position sizing
    position_sizer: Box<dyn PositionSizer>,
    // Risk management rules
    pub risk_rules: Vec<Box<dyn crate::risk::RiskRule>>,
    // --- EXECUTION PARAMETERS ---
    pub slippage_bps: u16,
    pub max_fee_lamports: u64,
    pub split_threshold_sol: f64,
    pub split_chunk_sol: f64,
    pub split_delay_ms: u64,
    // Wallet rotation
    /// Optional pool of additional wallets for rotation
    pub wallet_pool: Vec<crate::wallet::Wallet>,
    wallet_index: usize,

    // --- RISK PARAMETERS ---
    pub starting_balance: f64,           // e.g. 4.0 SOL
    pub current_balance: f64,            // updated after each trade
    pub max_position_pct: f64,           // kept for backward-compat but superseded by sizer
    pub max_position_abs: f64,           // e.g. 0.2 SOL cap
    pub max_open_trades: usize,          // e.g. 3
    pub stop_loss_pct: f64,              // e.g. 0.10 (10% SL)
    pub max_daily_loss_pct: f64,         // e.g. 0.15 (15% daily loss)
    pub daily_loss: f64,                 // tracked daily
    pub open_trades: usize,              // tracked live
    pub trade_history: Vec<TradeRecord>, // for analytics
    pub open_positions: std::collections::HashMap<String, (f64, f64)>,

    // --- MARKET DATA CACHE ---
    pub price_cache: PriceCache,
    price_feed_handle: Option<JoinHandle<()>>,
    // Pending stop/stop-limit orders
    pending_orders: Arc<tokio::sync::Mutex<Vec<crate::utils::types::PendingOrder>>>,
    scheduler_handle: Option<JoinHandle<()>>,
    dashboard_handle: Option<JoinHandle<()>>,
    dashboard_state: Option<crate::dashboard::SharedSnapshot>,
    /// Persistence backend (sqlite or null)
    pub persistence: std::sync::Arc<dyn Persistence + Send + Sync>,
    /// Aggregated access to TimescaleDB / Redis / ClickHouse (only when compiled with `db` feature)
    #[cfg(feature = "db")]
    pub data_layer: Option<crate::data_layer::DataLayer>,
    retry_tx: tokio::sync::mpsc::UnboundedSender<crate::utils::types::PendingOrder>,
    retry_rx: Option<tokio::sync::mpsc::UnboundedReceiver<crate::utils::types::PendingOrder>>,
    #[cfg(feature = "sidecar")]
    sidecar_client: Option<crate::sidecar::SidecarClient>,

    // Arbitrage stub
    pub paper_trading: bool,
    enable_arbitrage: bool,
}

impl TradingEngine {
    // Add stub for on_market_event to satisfy MarketEventHandler
    pub fn on_market_event(
        &mut self, _event: crate::utils::market_stream::MarketEvent,
    ) -> anyhow::Result<()> {
        // TODO: Implement event handling logic
        Ok(())
    }
    /// Create a new trading engine with the given configuration
    pub fn new() -> Self {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(Self::with_config_async(crate::config::Config::default(), false))
    }

    /// Synchronous helper that mirrors `with_config_async` but blocks on its future.
    /// Provided mainly for tests and simple CLI usage.
    pub fn with_config(config: crate::config::Config, paper_trading: bool) -> Self {
        tokio::runtime::Runtime::new()
            .expect("failed to create runtime")
            .block_on(Self::with_config_async(config, paper_trading))
    }
    /// Construct engine from configuration
    pub async fn with_config_async(config: crate::config::Config, paper_trading: bool) -> Self {
        use bs58;
        use solana_client::nonblocking::rpc_client::RpcClient;
        use solana_sdk::signature::{Keypair, Signer};
        // Build position sizer from config
        use crate::config::position_sizer::PositionSizerConfig;
        use crate::risk::position_sizer::{KellySizer, LiveKellySizer, VolatilitySizer};
        let position_sizer: Box<dyn PositionSizer> = match &config.risk.position_sizer {
            | Some(PositionSizerConfig::FixedFractional { pct }) => {
                Box::new(FixedFractionalSizer::new(*pct))
            }
            | Some(PositionSizerConfig::Kelly { win_rate, payoff_ratio, cap }) => {
                Box::new(KellySizer::new(*win_rate, *payoff_ratio, *cap))
            }
            | Some(PositionSizerConfig::KellyLive { cap }) => {
                use crate::performance::PerformanceMonitor;
                use std::sync::Arc;
                let pm = Arc::new(PerformanceMonitor::new());
                Box::new(LiveKellySizer::new(*cap, pm))
            }
            | Some(PositionSizerConfig::Volatility { risk_pct, atr_mult }) => {
                // Live ATR fetcher from global cache
                let fetcher = |sym: &str| -> Option<f64> { crate::utils::atr_cache::get(sym) };
                Box::new(VolatilitySizer::new(*risk_pct, *atr_mult, fetcher))
            }
            | None => Box::new(FixedFractionalSizer::new(0.01)),
        };

        // Build strategies first
        // Extract execution parameters before moving config
        let slippage_bps = config.trading.slippage_bps;
        let max_fee_lamports = config.trading.max_fee_lamports;
        let split_threshold_sol = config.trading.split_threshold_sol;
        let split_chunk_sol = config.trading.split_chunk_sol;
        let split_delay_ms = config.trading.split_delay_ms;
        // Build wallet rotation pool
        let mut wallet_pool: Vec<Wallet> = Vec::new();
        if !paper_trading {
            // instantiate new RpcClient for each wallet below
            for secret in &config.wallet.wallets {
                if let Ok(bytes) = bs58::decode(secret.trim()).into_vec() {
                    if let Ok(kp) = Keypair::from_bytes(&bytes) {
                        wallet_pool
                            .push(Wallet::new(RpcClient::new(config.solana.rpc_url.clone()), kp));
                    } else {
                        log::warn!("Failed to parse keypair bytes in wallet pool");
                    }
                } else {
                    log::warn!("Failed to decode base58 private key in wallet pool");
                }
            }
        }
        // Build persistence (TODO: load backend choice from config)
        use crate::persistence::sqlite::SqlitePersistence;
        let persistence: std::sync::Arc<dyn Persistence + Send + Sync> =
            match SqlitePersistence::new(None).await {
                | Ok(db) => std::sync::Arc::new(db),
                | Err(_) => std::sync::Arc::new(crate::persistence::NullPersistence),
            };

        let mut strategies_vec: Vec<Box<dyn crate::strategies::TradingStrategy>> = Vec::new();
        for scfg in &config.trading.strategies {
            if scfg.enabled {
                match crate::strategies::StrategyFactory::create_strategy(&scfg.name, scfg) {
                    | Ok(s) => strategies_vec.push(s),
                    | Err(e) => log::error!("Failed to init strategy {}: {}", scfg.name, e),
                }
            }
        }
        // Build performance monitors map
        // Initialize price cache and WebSocket feed
        let price_cache: PriceCache = Arc::new(tokio::sync::RwLock::new(HashMap::new()));
        let default_pair = TradingPair::from_str(&config.trading.default_pair)
            .unwrap_or_else(|| TradingPair::new("SOL", "USDC"));
        // Always include SOL/USDC so equity helpers have a USD price reference
        let mut price_pairs = vec![default_pair.clone()];

        if !(default_pair.base == "SOL" && default_pair.quote == "USDC") {
            price_pairs.push(TradingPair::new("SOL", "USDC"));
        }
        let price_feed_handle = market_ws::spawn_price_feed(&price_pairs, price_cache.clone());



        let starting_cash = config.trading.starting_balance_usd;
        let wallet_instance = if !paper_trading {
            match config.load_keypair() {
                | Ok(kp) => {
                    let rpc = RpcClient::new(config.solana.rpc_url.clone());
                    Some(Wallet::new(rpc, kp))
                }
                | Err(e) => {
                    log::error!("Failed to load keypair: {e}. Running in paper mode");
                    None
                }
            }
        } else {
            None
        };

        let portfolio = crate::portfolio::Portfolio::new(starting_cash);
        let risk_rules: Vec<Box<dyn crate::risk::RiskRule>> = vec![
            Box::new(crate::risk::StopLossRule::new(0.05)),
            Box::new(crate::risk::TakeProfitRule::new(0.10)),
        ];
        let pending_orders: Arc<tokio::sync::Mutex<Vec<crate::utils::types::PendingOrder>>> =
            Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let (retry_tx, retry_rx) =
            tokio::sync::mpsc::unbounded_channel::<crate::utils::types::PendingOrder>();
        // Start dashboard server if feature enabled
        let (dashboard_handle_opt, dashboard_state_opt) = {
            #[cfg(feature = "dashboard")]
            {
                let state = std::sync::Arc::new(tokio::sync::RwLock::new(
                    crate::dashboard::DashboardSnapshot::default(),
                ));
                let handle = tokio::spawn(crate::dashboard::run(state.clone()));
                (Some(handle), Some(state))
            }
            #[cfg(not(feature = "dashboard"))]
            {
                (None, None)
            }
        };
        // --- Optional data layer initialisation ---------------------------------
        #[cfg(feature = "db")]
        let data_layer_opt = {
            use crate::data_layer::DataLayer;
            let pg = std::env::var("PG_URL").unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/postgres".to_string());
            let redis = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".to_string());
            let ch = std::env::var("CH_URL").unwrap_or_else(|_| "tcp://127.0.0.1:9000".to_string());
            match DataLayer::initialise(&pg, &redis, &ch).await {
                Ok(dl) => Some(dl),
                Err(e) => {
                    log::error!("Failed to initialise data layer: {e}");
                    None
                }
            }
        };

        // spawn scheduler
        let tx_clone = retry_tx.clone();
        // -------------------------------- Price cache → DB sink -------------------
        #[cfg(feature = "db")]
        if let Some(dl_ref) = data_layer_opt.as_ref() {
            let pg_sink = dl_ref.pg.clone();
            let cache_for_sink = price_cache.clone();
            tokio::spawn(async move {
                use tokio::time::{sleep, Duration};
                loop {
                    sleep(Duration::from_secs(5)).await;
                    let snapshot = {
                        let guard = cache_for_sink.read().await;
                        guard.clone()
                    };
                    for (pair, price) in snapshot {
                        let pair_str = format!("{}/{}", pair.base, pair.quote);
                        let _ = (*pg_sink).execute(
                            "INSERT INTO price_ticks (pair, price, ts) VALUES ($1, $2, now())",
                            &[&pair_str as &(dyn ToSql + Sync), &price],
                        ).await;
                    }
                }
            });
        }

        // ---- External signal sources (Binance via CCXT-like REST) ----
        {
            use crate::signal::{ccxt::CcxtSource, perplexity::PerplexitySource, hub::SignalHub};
            let (sig_tx, sig_rx) = tokio::sync::mpsc::unbounded_channel::<(String, f64)>();
            // Spawn Binance BTCUSDT poller every 5 seconds
            let binance_src = CcxtSource::new("BTCUSDT", 5, sig_tx.clone());
            tokio::spawn(async move { let _ = binance_src.run().await; });

            // Spawn Perplexity sentiment source every 60 seconds for BTC
            let perp_src = PerplexitySource::new(&["BTC"], 60, sig_tx.clone());
            tokio::spawn(async move { let _ = perp_src.run().await; });

            #[cfg(feature = "db")]
            let pg_for_hub = data_layer_opt.as_ref().map(|dl| dl.pg.clone());
            #[cfg(not(feature = "db"))]
            let pg_for_hub: Option<std::sync::Arc<tokio_postgres::Client>> = None;

            let hub = SignalHub {
                rx: sig_rx,
                price_cache: price_cache.clone(),
                #[cfg(feature = "db")]
                pg: pg_for_hub,
                #[cfg(feature = "db")]
                ch: data_layer_opt.as_ref().map(|dl| dl.clickhouse.clone()),
            };
            tokio::spawn(hub.run());
        }

        let scheduler_handle = {
            let tx = tx_clone;
            let cache_clone = price_cache.clone();
            let orders_clone = pending_orders.clone();
            tokio::spawn(async move {
                use tokio::time::{sleep, Duration};
                loop {
                    sleep(Duration::from_secs(5)).await;
                    let mut orders = orders_clone.lock().await;
                    if orders.is_empty() {
                        continue;
                    }
                    let mut i = 0;
                    while i < orders.len() {
                        let o = &orders[i];
                        if let Some(price) = {
                            let guard = cache_clone.read().await;
                            guard.get(&o.pair).cloned()
                        } {
                            let triggered = match o.order_type {
                                | crate::utils::types::OrderType::Stop
                                | crate::utils::types::OrderType::StopLimit => {
                                    if let Some(sp) = o.stop_price {
                                        if o.is_buy {
                                            price >= sp
                                        } else {
                                            price <= sp
                                        }
                                    } else {
                                        false
                                    }
                                }
                                | _ => false,
                            };
                            if triggered {
                                // Currently we just remove it; engine will need explicit retry logic
                                let triggered_order = orders.remove(i);
                                let _ = tx.send(triggered_order);
                                continue;
                            }
                        }
                        i += 1;
                    }
                }
            })
        };

        let mut perf_map = std::collections::HashMap::new();
        for strat in &strategies_vec {
            perf_map.insert(strat.name().to_string(), PerformanceMonitor::new());
        }
        #[cfg(feature = "sidecar")]
        let sidecar_client_opt = if let Some(scfg) = &config.sidecar {
            if scfg.enabled {
                Some(crate::sidecar::SidecarClient::new(scfg.endpoint.clone()))
            } else {
                None
            }
        } else {
            None
        };
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
            starting_balance: starting_cash,
            current_balance: starting_cash,
            max_position_pct: 0.0,
            max_position_abs: 0.0,
            max_open_trades: 0,
            stop_loss_pct: 0.0,
            max_daily_loss_pct: 0.0,
            daily_loss: 0.0,
            open_trades: 0,
            trade_history: Vec::new(),
            open_positions: std::collections::HashMap::new(),
            position_sizer,

            portfolio,
            price_cache,
            price_feed_handle: Some(price_feed_handle),
            pending_orders,
            scheduler_handle: Some(scheduler_handle),
            dashboard_handle: dashboard_handle_opt,
            dashboard_state: dashboard_state_opt,
            persistence: persistence.clone(),
            #[cfg(feature = "db")]
            data_layer: data_layer_opt,
            retry_tx,
            retry_rx: Some(retry_rx),
            #[cfg(feature = "sidecar")]
            sidecar_client: sidecar_client_opt,
            slippage_bps,
            max_fee_lamports,
            split_threshold_sol,
            split_chunk_sol,
            split_delay_ms,
            wallet_pool,
            wallet_index: 0,
            paper_trading,
            enable_arbitrage: false,
            risk_rules,
            wallet: wallet_instance,
        }
    }

    /// Get latest cached mid-price for a pair, if available.
    pub async fn get_live_price(&self, pair: &TradingPair) -> Option<f64> {
        let guard = self.price_cache.read().await;
        guard.get(pair).cloned()
    }

    /// Rotate to next wallet in the configured pool. Returns Some(wallet) or None if pool empty.
    /// Rotate to next wallet in the pool (round-robin) and return a reference.
    pub fn next_wallet(&mut self) -> Option<crate::wallet::Wallet> {
        if self.wallet_pool.is_empty() {
            return None;
        }
        let idx = self.wallet_index % self.wallet_pool.len();
        self.wallet_index = (self.wallet_index + 1) % self.wallet_pool.len();
        Some(self.wallet_pool[idx].clone())
    }

    /// Start the trading engine with market data orchestrator
    /// Generate session report: PnL, win rate, drawdown, best/worst trades, risk metrics
    pub fn session_report(&self) {
        let total_trades = self.trade_history.len();
        let wins = self.trade_history.iter().filter(|t| t.pnl > 0.0).count();
        let losses = self.trade_history.iter().filter(|t| t.pnl <= 0.0).count();
        let pnl: f64 = self.trade_history.iter().map(|t| t.pnl).sum();
        let win_rate = if total_trades > 0 {
            wins as f64 / total_trades as f64
        } else {
            0.0
        };
        let max_drawdown = self.max_drawdown();
        let best_trade = self
            .trade_history
            .iter()
            .max_by(|a, b| a.pnl.partial_cmp(&b.pnl).unwrap());
        let worst_trade = self
            .trade_history
            .iter()
            .min_by(|a, b| a.pnl.partial_cmp(&b.pnl).unwrap());
        println!("=== SESSION REPORT ===");
        println!("Total Trades: {} | Wins: {} | Losses: {}", total_trades, wins, losses);
        println!("PnL: {:.4} SOL | Win Rate: {:.2}%", pnl, win_rate * 100.0);
        println!("Max Drawdown: {:.2}%", max_drawdown * 100.0);
        if let Some(best) = best_trade {
            println!(
                "Best Trade: {} {} @ {:.4} (PnL: {:.4})",
                best.symbol, best.side, best.price, best.pnl
            );
        }
        if let Some(worst) = worst_trade {
            println!(
                "Worst Trade: {} {} @ {:.4} (PnL: {:.4})",
                worst.symbol, worst.side, worst.price, worst.pnl
            );
        }
        println!(
            "Risk: Max Position {:.2} SOL ({}%), Max Daily Loss {:.2}%",
            self.max_position_abs,
            self.max_position_pct * 100.0,
            self.max_daily_loss_pct * 100.0
        );
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
            if equity > peak {
                peak = equity;
            }
            let dd = (peak - equity) / peak;
            if dd > max_dd {
                max_dd = dd;
            }
        }
        max_dd
    }

    /// Core arbitrage logic
    pub async fn try_arbitrage(&self, symbol: &str) -> Option<ArbitrageOpportunity> {
        use futures::future::join_all;
        if !self.enable_arbitrage {
            return None;
        }
        let mut price_futures = vec![];
        for (dex_name, dex_client) in &self.dex_clients {
            if let Some((base, quote)) = symbol.split_once('/') {
                let fut = dex_client.get_price(base, quote);
                price_futures.push((dex_name.clone(), fut));
            }
        }
        let mut prices = vec![];
        let results = join_all(
            price_futures
                .into_iter()
                .map(|(dex_name, fut)| async move { (dex_name, fut.await) }),
        )
        .await;
        for (dex_name, price_res) in results {
            if let Ok(price) = price_res {
                prices.push((dex_name, price));
            }
        }
        if prices.len() < 2 {
            return None;
        }
        let (buy_dex, buy_price) = prices
            .iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();
        let (sell_dex, sell_price) = prices
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();
        let spread = sell_price - buy_price;
        if spread > 0.002 * buy_price {
            println!(
                "[ARBITRAGE] Opportunity: Buy {} @ {} ({}), Sell @ {} ({})",
                symbol, buy_price, buy_dex, sell_price, sell_dex
            );
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
    async fn update_dashboard_snapshot(&self) {
        // Persist equity snapshot
        let snap_equity = {
            let cache = self.price_cache.read().await;
            let price_lookup = |pair: &crate::utils::types::TradingPair| cache.get(pair).cloned();
            self.portfolio.total_usd_value(&price_lookup)
        };
        let snapshot =
            EquitySnapshot { id: None, timestamp: Utc::now().naive_utc(), equity: snap_equity };
        let _ = self.persistence.save_snapshot(&snapshot).await;

        #[cfg(feature = "dashboard")]
        if let Some(state) = &self.dashboard_state {
            // Hold a read lock on the price cache for consistent snapshot
            let cache = self.price_cache.read().await;
            let price_lookup = |pair: &crate::utils::types::TradingPair| cache.get(pair).cloned();
            let mut snap = state.write().await;
            snap.equity_usd = self.portfolio.total_usd_value(&price_lookup);
            snap.equity_sol = self.portfolio.total_sol_value(&price_lookup);
            snap.pnl_usd = self.portfolio.total_realized_pnl;
            snap.open_positions = self.portfolio.positions.len();
        }
    }

    /// Enforce risk parameters and dynamic scaling
    /// Sync Solana wallet balance into portfolio cash based on live SOL price
    pub async fn sync_wallet_balance(&mut self) {
        use solana_client::nonblocking::rpc_client::RpcClient;
        use solana_sdk::signer::Signer;

        // Load trading keypair defined in config (same helper used by CLI)
        let keypair = match self.config.load_keypair() {
            | Ok(kp) => kp,
            | Err(e) => {
                log::warn!("sync_wallet_balance: cannot load keypair: {e}");
                return;
            }
        };
        let rpc = RpcClient::new(self.config.solana.rpc_url.clone());
        let lamports = match rpc.get_balance(&keypair.pubkey()).await {
            | Ok(l) => l,
            | Err(e) => {
                log::warn!("sync_wallet_balance: get_balance failed: {e}");
                return;
            }
        };
        let sol = lamports as f64 / 1_000_000_000.0;

        // Need a price to convert SOL -> USD
        let cache = self.price_cache.read().await;
        let pair = crate::utils::types::TradingPair::new("SOL", "USDC");
        if let Some(price) = cache.get(&pair) {
            self.portfolio.cash_usd = sol * price;
            self.current_balance = self.portfolio.cash_usd;
        }
    }

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

    /// Apply trade effects of a single trade chunk, updating portfolio and returning realized PnL.
    pub fn apply_trade_effects(&mut self, sig: &Signal, chunk: f64) -> f64 {
        let symbol_key = sig.pair.to_string();
        let pnl = match sig.action {
            | SignalAction::Buy => {
                self.open_trades += 1;
                self.portfolio.update_on_buy(&symbol_key, chunk, sig.price);
                0.0
            }
            | SignalAction::Sell => {
                let realized = self.portfolio.update_on_sell(&symbol_key, chunk, sig.price);
                if self.open_trades > 0 {
                    self.open_trades -= 1;
                }
                realized
            }
            | _ => 0.0,
        };

        // Sync legacy fields for backward compatibility
        self.current_balance = self.portfolio.cash_usd;
        self.open_positions = self
            .portfolio
            .positions
            .iter()
            .map(|(sym, pos)| (sym.clone(), (pos.size, pos.average_entry_price)))
            .collect();
        // Update daily loss (USD cash for now)
        self.daily_loss = (self.starting_balance - self.current_balance).max(0.0);

        // Evaluate risk after trade
        self.evaluate_risk_rules();

        pnl
    }

    /// Evaluate risk rules
    pub fn evaluate_risk_rules(&mut self) {
        let cache_guard = self.price_cache.try_read();
        if cache_guard.is_err() {
            return;
        }
        let cache = cache_guard.unwrap();
        let positions_snapshot = self.portfolio.positions.clone();
        for (sym, pos) in positions_snapshot {
            if pos.size <= 0.0 {
                continue;
            }
            if let Some(pair) = TradingPair::from_str(&sym) {
                if let Some(price) = cache.get(&pair) {
                    for rule in &self.risk_rules {
                        if let Some(RiskAction::ClosePosition) = rule.evaluate(&sym, &pos, *price) {
                            let _ = self.portfolio.update_on_sell(&sym, pos.size, *price);
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Return total equity in USD (cash + unrealized)
    pub fn equity_usd(&self) -> f64 {
        let cache_ref = self.price_cache.try_read().ok();
        self.portfolio
            .total_usd_value(&|pair| cache_ref.as_ref().and_then(|c| c.get(pair)).cloned())
    }

    /// Return total equity in SOL using the SOL/USDC mid-price
    pub fn equity_sol(&self) -> f64 {
        let cache_ref = self.price_cache.try_read().ok();
        self.portfolio
            .total_sol_value(&|pair| cache_ref.as_ref().and_then(|c| c.get(pair)).cloned())
    }

    pub async fn start_with_market_router(
        &mut self, symbols: Vec<String>, helius_api_key: Option<String>,
        openbook_program_id: Option<String>,
    ) -> anyhow::Result<()> {
        let mut router = MarketRouter::new();

        // Prepare symbols for streams
        let symbol_strs = symbols.to_vec();

        // Add Binance stream
        let binance_stream =
            Box::new(crate::utils::binance_stream::BinanceStream::new(&symbol_strs));
        router.add_stream(binance_stream);

        // Add Helius stream if API key provided
        if let Some(api_key) = helius_api_key {
            let helius_stream = Box::new(crate::utils::helius_stream::HeliusStream::new(
                &api_key,
                openbook_program_id.as_deref(),
            ));
            router.add_stream(helius_stream);
        }

        // Add more streams as needed (Coinbase, Kraken, Serum, Triton, etc)
        let coinbase_stream =
            Box::new(crate::utils::coinbase_stream::CoinbaseStream::new(&symbol_strs));
        router.add_stream(coinbase_stream);

        let kraken_stream = Box::new(crate::utils::kraken_stream::KrakenStream::new());
        router.add_stream(kraken_stream);
        // SerumStream requires a market symbol, use the first symbol or fallback
        let serum_market = symbol_strs
            .first()
            .cloned()
            .unwrap_or_else(|| "SOL/USDC".to_string());
        let serum_stream = Box::new(crate::utils::serum_stream::SerumStream::new(&serum_market));
        router.add_stream(serum_stream);
        // TritonStream requires api_key and market, use dummy key for now
        let triton_api_key =
            std::env::var("TRITON_API_KEY").unwrap_or_else(|_| "demo-key".to_string());
        let triton_market = symbol_strs
            .first()
            .cloned()
            .unwrap_or_else(|| "SOL/USDC".to_string());
        let triton_stream = Box::new(crate::utils::triton_stream::TritonStream::new(
            &triton_api_key,
            &triton_market,
        ));
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
        let mut snap_interval = tokio::time::interval(tokio::time::Duration::from_secs(3));

        // Spawn router (all streams)
        // Initial wallet sync and dashboard snapshot
        self.sync_wallet_balance().await;
        self.update_dashboard_snapshot().await;

        log::info!("Subscribed symbols: {:?}", symbols);
        let mut router_task = router;
        let symbols_clone = symbols.clone();
        let router_handle = tokio::spawn(async move { router_task.run(symbols_clone, tx).await });

        // Event loop: process market events until router terminates
        // Pending order receiver
        let mut retry_rx_opt = self.retry_rx.take();
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
                        po_opt = async {
                    if let Some(rx) = &mut retry_rx_opt { rx.recv().await } else { None }
                } => {
                    if let Some(order) = po_opt {
                        self.process_pending_order(order).await?;
                    }
                    Ok::<(), anyhow::Error>(())
                },
            _ = snap_interval.tick() => {
                self.sync_wallet_balance().await;
                self.update_dashboard_snapshot().await;
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
                | SignalType::Buy => SignalAction::Buy,
                | SignalType::Sell => SignalAction::Sell,
                | SignalType::Close => SignalAction::Close,
                | SignalType::Cancel => SignalAction::Cancel,
                | SignalType::Arbitrage { .. } => {
                    // Map arbitrage signal to Buy for now; improve later with side fields.
                    SignalAction::Buy
                }
            },
            price: sig.price,
            size: sig.size,
            confidence: sig.confidence,
            order_type: sig.order_type,
            limit_price: sig.limit_price,
            stop_price: sig.stop_price,
            stop_loss: None,
            take_profit: None,
            timestamp: sig.timestamp,
            metadata: std::collections::HashMap::new(),
        })
    }

    async fn process_pending_order(
        &mut self, po: crate::utils::types::PendingOrder,
    ) -> anyhow::Result<()> {
        // Convert order type
        let new_order_type = match po.order_type {
            | crate::utils::types::OrderType::Stop => crate::utils::types::OrderType::Market,
            | crate::utils::types::OrderType::StopLimit => {
                if po.limit_price.is_some() {
                    crate::utils::types::OrderType::Limit
                } else {
                    crate::utils::types::OrderType::Market
                }
            }
            | other => other,
        };
        let preferred = [
            &po.dex_preference[..],
            &["jupiter".to_string(), "raydium".to_string(), "photon".to_string()],
        ]
        .concat();
        let mut executed = false;
        for dex_name in preferred.iter() {
            if let Some(dex) = self.dex_clients.get(dex_name) {
                match dex
                    .execute_trade(
                        &po.pair.base,
                        &po.pair.quote,
                        po.amount,
                        po.is_buy,
                        self.slippage_bps,
                        self.max_fee_lamports,
                        new_order_type,
                        po.limit_price,
                        None,
                        None,
                        self.wallet.as_ref().expect("wallet not available"),
                    )
                    .await
                {
                    | Ok(_) => {
                        executed = true;
                        break;
                    }
                    | Err(e) => {
                        log::warn!("Retry {} failed: {}", dex_name, e);
                        continue;
                    }
                }
            }
        }
        if !executed {
            // push back to queue?
            log::error!("Failed to execute retried order: {:?}", po);
        }
        Ok(())
    }

    #[cfg_attr(not(feature = "sidecar"), allow(unused_mut))]
    async fn handle_signals(&mut self, mut signals: Vec<Signal>) -> anyhow::Result<()> {
        #[cfg(feature = "sidecar")]
        if let Some(sc) = &self.sidecar_client {
            if let Some(cfg) = &self.config.sidecar {
                if cfg.enabled {
                    let weight = cfg.weight.clamp(0.0, 1.0);
                    // Down-weight local engine signals
                    for sig in signals.iter_mut() {
                        sig.confidence *= 1.0 - weight;
                    }
                    if let Ok(feat) = serde_json::to_value(&signals) {
                        match sc.predict(feat).await {
                            | Ok(resp) => {
                                let sidecar_sigs: Vec<Signal> = if let Ok(vec) =
                                    serde_json::from_value::<Vec<Signal>>(resp.clone())
                                {
                                    vec
                                } else if let Some(arr) =
                                    resp.get("signals").and_then(|v| v.as_array())
                                {
                                    serde_json::from_value::<Vec<Signal>>(serde_json::Value::Array(
                                        arr.clone(),
                                    ))
                                    .unwrap_or_default()
                                } else {
                                    Vec::new()
                                };
                                if !sidecar_sigs.is_empty() {
                                    log::info!(
                                        "Blended {} sidecar signals (weight {:.2})",
                                        sidecar_sigs.len(),
                                        weight
                                    );
                                    let mut scaled = sidecar_sigs
                                        .into_iter()
                                        .map(|mut s| {
                                            s.confidence *= weight;
                                            s
                                        })
                                        .collect::<Vec<_>>();
                                    signals.append(&mut scaled);
                                }
                            }
                            | Err(e) => log::warn!("Sidecar predict error: {}", e),
                        }
                    }
                }
            }
        }
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
            // Decide amount via configurable position sizer
            let mut chunk = self
                .position_sizer
                .size(self.current_balance, &sig.pair.base)
                .await;
            if chunk > self.max_position_abs {
                chunk = self.max_position_abs;
            }
            // Determine trade splitting based on config and strategy type
            let mut trade_chunks: Vec<f64> = Vec::new();
            if chunk > self.split_threshold_sol
                && !sig.strategy_id.to_lowercase().contains("arbitrage")
            {
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
            let wallet_ref = self.next_wallet().unwrap_or_else(|| {
                self.wallet
                    .clone()
                    .expect("wallet not available for live trading")
            });

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
                                sig.order_type,
                                sig.limit_price,
                                sig.stop_price,
                                None,
                                &wallet_ref,
                            )
                            .await
                            {
                                | Ok(_) => {
                                    // Persist trade record (pnl unknown at entry)
                                    let rec = TradeRecord {
                                        id: None,
                                        timestamp: Utc::now().naive_utc(),
                                        symbol: sig.pair.to_string(),
                                        side: if matches!(
                                            sig.action,
                                            crate::utils::types::SignalAction::Buy
                                        ) {
                                            "buy".into()
                                        } else {
                                            "sell".into()
                                        },
                                        qty: chunk,
                                        price: sig.price,
                                        pnl: 0.0,
                                    };
                                    let _ = self.persistence.save_trade(&rec).await;
                                    executed = true;
                                    break;
                                }
                                | Err(e) => {
                                    log::warn!("{} execution failed: {}", dex_name, e);
                                    last_err = Some(e.into());
                                    continue;
                                }
                            }
                        }
                    }
                    if !executed {
                        #[allow(clippy::collapsible_if)]
                        if let Some(err) = &last_err {
                            if sig.order_type == crate::utils::types::OrderType::Stop
                                || sig.order_type == crate::utils::types::OrderType::StopLimit
                            {
                                if err.to_string().contains("Stop price not triggered") {
                                    let po = PendingOrder {
                                        pair: sig.pair.clone(),
                                        amount: chunk,
                                        is_buy: matches!(
                                            sig.action,
                                            crate::utils::types::SignalAction::Buy
                                        ),
                                        order_type: sig.order_type,
                                        limit_price: sig.limit_price,
                                        stop_price: sig.stop_price,
                                        wallet: String::new(),
                                        dex_preference: Vec::new(),
                                        timestamp: sig.timestamp,
                                    };
                                }
                            }
                        }
                    } else {
                        log::info!("[PAPER] would execute trade: {:?}", sig);
                    }
                }
                // bookkeeping per chunk
                let pnl_chunk = self.apply_trade_effects(&sig, chunk);
                // Build order record for this chunk
                let order = Order {
                    id: format!("{}-{}-{}", sig.strategy_id, sig.timestamp, rand::random::<u16>()),
                    symbol: sig.pair.to_string(),
                    price: sig.price,
                    size: chunk,
                    side: if sig.action == SignalAction::Buy {
                        OrderSide::Buy
                    } else {
                        OrderSide::Sell
                    },
                    order_type: crate::utils::types::OrderType::Market,
                    timestamp: sig.timestamp,
                };
                // Performance monitor
                if let Some(mon) = self.performance_monitors.get(&sig.strategy_id) {
                    let _ = mon
                        .record_trade(&sig.strategy_id, &order, None, pnl_chunk, 0.0001, None)
                        .await;
                }
                // Notify strategy
                if let Some(strat) = self
                    .strategies
                    .iter_mut()
                    .find(|s| s.name() == sig.strategy_id)
                {
                    strat.on_order_filled(&order);
                }
                // Append history
                self.trade_history.push(TradeRecord {
                    id: None,
                    timestamp: chrono::DateTime::<chrono::Utc>::from_timestamp(sig.timestamp, 0)
                        .unwrap()
                        .naive_utc(),
                    symbol: sig.pair.to_string(),
                    side: match sig.action {
                        | SignalAction::Buy => "buy".into(),
                        | SignalAction::Sell => "sell".into(),
                        | _ => "other".into(),
                    },
                    qty: chunk,
                    price: sig.price,
                    pnl: pnl_chunk,
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
                        symbol: sig.pair.to_string(),
                        price: sig.price,
                        size: chunk,
                        side: if sig.action == SignalAction::Buy { OrderSide::Buy } else { OrderSide::Sell },
                        order_type: crate::utils::types::OrderType::Market,
                        timestamp: sig.timestamp,
                    };
                    // Notify strategy
                    if let Some(strat) = self.strategies.iter_mut().find(|s| s.name() == sig.strategy_id) {
                        strat.on_order_filled(&order);
                    }
                    // Append history
                    self.trade_history.push(TradeRecord {
                            id: None,
                            timestamp: chrono::DateTime::<chrono::Utc>::from_timestamp(sig.timestamp, 0).unwrap().naive_utc(),
                            symbol: sig.pair.to_string(),
                            side: match sig.action { SignalAction::Buy => "buy".into(), SignalAction::Sell => "sell".into(), _ => "other".into() },
                            qty: chunk,
                            price: sig.price,
                            pnl: pnl_chunk,
                        });
                        total_pnl_chunked += pnl_chunk;
        {{ ... }}

        */
        Ok(())
    }

    /// Convert incoming MarketEvent to simple MarketData for strategy consumption
    fn convert_market_event(
        event: &crate::utils::market_stream::MarketEvent,
    ) -> Option<MarketData> {
        use crate::utils::market_stream::MarketEvent::*;
        match event {
            | Trade { symbol, price, qty, timestamp, .. } => {
                let pair = TradingPair::from_str(symbol)
                    .unwrap_or_else(|| TradingPair::new(symbol, "USDC"));
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
            | Ticker { symbol, price, timestamp, .. } => {
                let pair = TradingPair::from_str(symbol)
                    .unwrap_or_else(|| TradingPair::new(symbol, "USDC"));
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
            | _ => None,
        }
    }

    /// Start the trading engine
    pub async fn start(&self) -> Result<()> {
        // Main trading loop
        Ok(())
    }
}

impl Default for TradingEngine {
    fn default() -> Self {
        Self::new()
    }
}

mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn test_trading_engine_initialization() {
        let engine = TradingEngine::new();
        // Add assertions
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_trading_engine_start() {
        let engine = TradingEngine::with_config_async(Config::default(), false).await;
        assert!(engine.start().await.is_ok());
    }
}
