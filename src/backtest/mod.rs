//! Backtesting framework for AlgoTraderV2 Rust

use clap::ValueEnum;
use serde::{Deserialize, Serialize}; // SimulatedTrade and BacktestReport
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SimMode {
    Bar,
    Tick,
}

// Backtester struct with portfolio simulation and basic reporting
/// Result of a single simulated trade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedTrade {
    pub timestamp: i64,
    pub symbol: String,
    pub side: crate::utils::types::OrderSide,
    pub qty: f64,
    pub price: f64,
    pub pnl: f64,
}

/// Simple portfolio representation used during backtest
#[derive(Debug, Default, Clone)]
pub struct Portfolio {
    // Backtest-only lightweight portfolio
    pub cash: f64,
    pub positions: HashMap<String, crate::portfolio::Position>, // symbol -> detailed position
    pub realized_pnl: f64,
}

impl Portfolio {
    pub fn new(starting_cash: f64) -> Self {
        Self { cash: starting_cash, positions: HashMap::new(), realized_pnl: 0.0 }
    }

    /// Apply a simulated trade, updating cash / positions
    pub fn apply_trade(&mut self, trade: &SimulatedTrade) {
        match trade.side {
            | crate::utils::types::OrderSide::Buy => {
                self.cash -= trade.qty * trade.price;
                let pos = self.positions.entry(trade.symbol.clone()).or_default();
                pos.update_on_buy(trade.qty, trade.price);
            }
            | crate::utils::types::OrderSide::Sell => {
                self.cash += trade.qty * trade.price;
                let pos = self.positions.entry(trade.symbol.clone()).or_default();
                let pnl = pos.update_on_sell(trade.qty, trade.price);
                self.realized_pnl += pnl;
            }
        }
    }

    pub fn update_on_sell(&mut self, symbol: &str, qty: f64, price: f64) -> f64 {
        let pnl = if let Some(pos) = self.positions.get_mut(symbol) {
            pos.update_on_sell(qty, price)
        } else {
            0.0
        };
        self.cash += qty * price;
        self.realized_pnl += pnl;
        pnl
    }

    pub fn equity(&self, prices: &HashMap<String, f64>) -> f64 {
        let mut eq = self.cash;
        for (sym, pos) in &self.positions {
            if let Some(price) = prices.get(sym) {
                eq += pos.size * price;
            }
        }
        eq
    }
}

/// Summary of backtest results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestReport {
    pub starting_balance: f64,
    pub ending_balance: f64,
    pub realized_pnl: f64,
    pub max_drawdown: f64,
    pub total_trades: usize,
    pub winning_trades: usize,
    /// Equity value after each bar for curve plotting
    pub equity_curve: Vec<f64>,
    /// Simple returns based on equity curve
    pub returns: Vec<f64>,
    /// Annualised Sharpe ratio (using simple return series)
    pub sharpe: f64,
}

impl BacktestReport {
    /// Save equity curve plot (PNG)
    pub fn to_png<P: AsRef<std::path::Path>>(&self, path: P) -> anyhow::Result<()> {
        use plotters::prelude::*;
        use plotters::series::LineSeries;
        use plotters_bitmap::bitmap_pixel::RGBPixel;
        use plotters_bitmap::BitMapBackend;
        let root = BitMapBackend::<RGBPixel>::new(path.as_ref(), (800, 480));
        let root = root.into_drawing_area();
        root.fill(&WHITE)?;
        let max_eq = self
            .equity_curve
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);
        let min_eq = self.equity_curve.iter().cloned().fold(f64::MAX, f64::min);
        let mut chart = ChartBuilder::on(&root)
            .margin(20)
            .caption("Equity Curve", ("sans-serif", 30))
            .x_label_area_size(30)
            .y_label_area_size(50)
            .build_cartesian_2d(0..self.equity_curve.len(), min_eq..max_eq)?;
        chart.configure_mesh().draw()?;
        chart.draw_series(LineSeries::new(
            self.equity_curve.iter().enumerate().map(|(i, v)| (i, *v)),
            &BLUE,
        ))?;
        Ok(())
    }

    /// Export equity curve and summary metrics to CSV
    /// Export equity curve and summary metrics to CSV
    pub fn to_csv<P: AsRef<std::path::Path>>(&self, path: P) -> std::io::Result<()> {
        let mut wtr = csv::Writer::from_path(path)?;
        // header
        wtr.write_record(["index", "equity"])?;
        for (idx, eq) in self.equity_curve.iter().enumerate() {
            wtr.write_record(&[idx.to_string(), eq.to_string()])?;
        }
        wtr.flush()?;
        Ok(())
    }
    pub fn print(&self) {
        println!("===== BACKTEST REPORT =====");
        println!("Start Balance : {:.2}", self.starting_balance);
        println!("End Balance   : {:.2}", self.ending_balance);
        println!("Realized PnL  : {:.2}", self.realized_pnl);
        println!("Max Drawdown  : {:.2}%", self.max_drawdown * 100.0);
        println!("Total Trades  : {}", self.total_trades);
        println!(
            "Winning Trades: {} ({:.2}%)",
            self.winning_trades,
            self.winning_trades as f64 / self.total_trades.max(1) as f64 * 100.0
        );
        println!("Sharpe Ratio  : {:.2}", self.sharpe);
        println!("===========================");
    }
}

pub struct Backtester {
    /// Historical data provider implementation
    pub data_provider: Box<dyn HistoricalDataProvider>,
    /// Candle timeframe label (e.g. "1h")
    pub timeframe: String,
    /// Starting quote balance (e.g. USD)
    pub starting_balance: f64,
    /// Strategies to evaluate during the run
    pub strategies: Vec<Box<dyn crate::strategies::TradingStrategy>>,
    /// Optional cache for storing/retrieving back-test reports
    pub cache: Option<crate::backtest::cache::BacktestCache>,
    /// Optional persistence backend
    pub persistence: Option<std::sync::Arc<dyn crate::persistence::Persistence + Send + Sync>>,
    /// Risk rules (stop-loss, take-profit, etc.)
    pub risk_rules: Vec<Box<dyn crate::risk::RiskRule>>,
    /// Simulation mode (bar vs tick)
    pub sim_mode: SimMode,
    /// Per-trade slippage expressed in basis points (100 bps = 1 %)
    pub slippage_bps: u16,
    /// Trading fee expressed in basis points (paid on notional)
    pub fee_bps: u16,
}

impl Backtester {
    pub async fn run(&mut self, data_file: &std::path::Path) -> Result<BacktestReport> {
        // determine date range for caching
        let market_data = self.data_provider.load(data_file)?;
        if market_data.is_empty() {
            return Err(crate::Error::DataError("No market data loaded".to_string()));
        }
        let start_ts = market_data.first().unwrap().timestamp;
        let end_ts = market_data.last().unwrap().timestamp;

        // strategy name concat if single strategy else "multi"
        let strat_key = if self.strategies.len() == 1 {
            self.strategies[0].name().to_string()
        } else {
            "multi".to_string()
        };
        if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get(&strat_key, "UNK", &self.timeframe, start_ts, end_ts)? {
                return Ok(cached);
            }
        }

        // use loaded data for simulation
        let market_data = self.data_provider.load(data_file)?;
        if market_data.is_empty() {
            return Err(crate::Error::DataError("No market data loaded".to_string()));
        }

        // Prepare portfolio and event queue
        let mut portfolio = Portfolio::new(self.starting_balance);
        let mut prices: HashMap<String, f64> = HashMap::new();
        let mut queue = crate::backtest::event::EventQueue::new();
        use crate::backtest::event::BacktestEvent;

        // prime queue with historical market data
        for dp in market_data {
            queue.push(BacktestEvent::Market(dp));
        }

        // metrics
        let mut total_trades = 0usize;
        let mut winning_trades = 0usize;
        let mut peak_equity = self.starting_balance;
        let mut max_drawdown = 0.0;
        let mut equity_curve: Vec<f64> = Vec::new();

        while let Some(evt) = queue.pop() {
            match evt {
                | BacktestEvent::Market(data_point) => {
                    // update last price
                    prices.insert(data_point.pair.to_string(), data_point.last_price);

                    // generate signals
                    for strategy in &mut self.strategies {
                        let signals =
                            futures::executor::block_on(strategy.generate_signals(&data_point));
                        for sig in signals {
                            // convert to trade later via slippage/fee; push Trade event
                            let qty = sig.size;
                            let raw_price = sig.price;
                            let exec_price = match sig.signal_type {
                                | crate::trading::SignalType::Buy => {
                                    raw_price * (1.0 + (self.slippage_bps as f64) / 10_000.0)
                                }
                                | crate::trading::SignalType::Sell => {
                                    raw_price * (1.0 - (self.slippage_bps as f64) / 10_000.0)
                                }
                                | _ => raw_price,
                            };
                            use crate::trading::SignalType;
                            let side = match sig.signal_type {
                                | SignalType::Buy => crate::utils::types::OrderSide::Buy,
                                | SignalType::Sell => crate::utils::types::OrderSide::Sell,
                                | _ => continue,
                            };
                            let trade_evt = BacktestEvent::Trade(SimulatedTrade {
                                timestamp: sig.timestamp,
                                symbol: sig.symbol.clone(),
                                side,
                                qty,
                                price: exec_price,
                                pnl: 0.0,
                            });
                            queue.push(trade_evt);
                        }
                    }

                    // ---------- Risk rule evaluation ----------
                    let positions_snapshot: std::collections::HashMap<
                        String,
                        crate::portfolio::Position,
                    > = portfolio.positions.clone();
                    for (sym, pos) in positions_snapshot {
                        if pos.size <= 0.0 {
                            continue;
                        }
                        if let Some(price) = prices.get(&sym) {
                            for rule in &self.risk_rules {
                                if let Some(RiskAction::ClosePosition) =
                                    rule.evaluate(&sym, &pos, *price)
                                {
                                    // push a trade event to close the position
                                    let close_evt = BacktestEvent::Trade(SimulatedTrade {
                                        timestamp: data_point.timestamp,
                                        symbol: sym.clone(),
                                        side: crate::utils::types::OrderSide::Sell,
                                        qty: pos.size,
                                        price: *price,
                                        pnl: 0.0,
                                    });
                                    queue.push(close_evt);
                                    break;
                                }
                            }
                        }
                    }

                    // update equity/drawdown after processing bar
                    let equity = portfolio.equity(&prices);
                    if equity > peak_equity {
                        peak_equity = equity;
                    }
                    let drawdown = (peak_equity - equity) / peak_equity;
                    if drawdown > max_drawdown {
                        max_drawdown = drawdown;
                    }
                    equity_curve.push(equity);
                }
                | BacktestEvent::Trade(mut trade) => {
                    let notional = trade.qty * trade.price;
                    let fee = notional * (self.fee_bps as f64) / 10_000.0;
                    let before_pnl = portfolio.realized_pnl;
                    portfolio.apply_trade(&trade);
                    portfolio.cash -= fee;
                    let realized = portfolio.realized_pnl - before_pnl;
                    trade.pnl = realized;
                    if realized > 0.0 {
                        winning_trades += 1;
                    }
                    total_trades += 1;
                }
            }
        }

        // ---------- Risk rule evaluation (after full run) ----------
        for (sym, pos) in portfolio.positions.clone() {
            if pos.size <= 0.0 {
                continue;
            }
            if let Some(price) = prices.get(&sym) {
                for rule in &self.risk_rules {
                    if let Some(RiskAction::ClosePosition) = rule.evaluate(&sym, &pos, *price) {
                        let pnl = portfolio.update_on_sell(&sym, pos.size, *price);
                        total_trades += 1;
                        if pnl > 0.0 {
                            winning_trades += 1;
                        }
                        break;
                    }
                }
            }
        }

        // compute returns & sharpe
        let mut returns: Vec<f64> = Vec::new();
        for w in equity_curve.windows(2) {
            let prev = w[0];
            let curr = w[1];
            if prev > 0.0 {
                returns.push((curr - prev) / prev);
            }
        }
        let mean_ret = returns.iter().copied().sum::<f64>() / returns.len().max(1) as f64;
        let var = returns.iter().map(|r| (r - mean_ret).powi(2)).sum::<f64>()
            / returns.len().max(1) as f64;
        let sd = var.sqrt();
        let sharpe = if sd > 0.0 {
            mean_ret / sd * (returns.len() as f64).sqrt()
        } else {
            0.0
        };

        let ending_balance = portfolio.equity(&prices);
        let report = BacktestReport {
            starting_balance: self.starting_balance,
            ending_balance,
            realized_pnl: portfolio.realized_pnl,
            max_drawdown,
            total_trades,
            winning_trades,
            equity_curve,
            returns,
            sharpe,
        };
        // store in cache
        if let Some(cache) = &self.cache {
            let _ = cache.insert(&strat_key, "UNK", &self.timeframe, start_ts, end_ts, &report);
        }
        // Persist summary if configured
        if let Some(p) = &self.persistence {
            let summary = crate::persistence::BacktestSummary {
                id: None,
                strategy: strat_key.clone(),
                timeframe: self.timeframe.clone(),
                start_balance: self.starting_balance,
                end_balance: ending_balance,
                sharpe,
                max_drawdown,
            };
            let _ = p.save_backtest(&summary).await;
        }
        Ok(report)
    }
}

use crate::risk::{RiskAction, RiskRule};
use crate::Result;

use crate::utils::types::MarketData;

use std::path::PathBuf;

/// Trait for historical data providers
pub trait HistoricalDataProvider: Send + Sync {
    fn load(&self, data_file: &std::path::Path) -> Result<Vec<MarketData>>;
    fn box_clone(&self) -> Box<dyn HistoricalDataProvider>;
}

impl Clone for Box<dyn HistoricalDataProvider> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

pub mod cache;
pub mod event;
pub mod importer;
pub mod providers;
pub mod remote_provider;
pub mod tick_provider;
pub mod harness;

/// Convenience helper used by CLI until full engine integration is ready
use std::path::Path;

pub async fn simple_backtest(
    data_path: &Path, timeframe: &str, sim_mode: SimMode, output: Option<&Path>,
) -> Result<()> {
    use crate::strategies::{MeanReversionStrategy, TimeFrame, TradingStrategy};
    // 1. Provider
    let provider: Box<dyn HistoricalDataProvider> = match sim_mode {
        | SimMode::Bar => Box::new(crate::backtest::providers::CSVHistoricalDataProvider::new()),
        | SimMode::Tick => Box::new(crate::backtest::tick_provider::CSVTicksProvider::new()),
    };
    // 2. Build backtester
    let strategies: Vec<Box<dyn TradingStrategy>> = vec![Box::new(MeanReversionStrategy::new(
        "UNK/UNK",
        TimeFrame::OneHour,
        20,
        2.0,
        2.0,
        1.0,
    ))];
    let mut bt = Backtester {
        risk_rules: vec![
            Box::new(crate::risk::StopLossRule::new(0.05)),
            Box::new(crate::risk::TakeProfitRule::new(0.10)),
        ],
        data_provider: provider,
        timeframe: timeframe.to_string(),
        starting_balance: 10_000.0,
        strategies,
        cache: None,
        persistence: Some(std::sync::Arc::new(crate::persistence::NullPersistence)),
        sim_mode,
        slippage_bps: 0,
        fee_bps: 8, // 0.03 %
    };
    let rpt = bt.run(data_path).await?;
    if let Some(path) = output {
        if let Err(e) = rpt.to_csv(path) {
            log::error!("Failed to write CSV report: {e}");
        } else {
            log::info!("CSV report written to {}", path.display());
            // also render PNG next to CSV
            let mut png_path = std::path::PathBuf::from(path);
            png_path.set_extension("png");
            if let Err(e) = rpt.to_png(&png_path) {
                log::error!("Failed to write PNG chart: {e}");
            } else {
                log::info!("PNG chart written to {}", png_path.display());
            }
        }
    }
    rpt.print();
    Ok(())
}
// TODO: Add result reporting and export utilities
