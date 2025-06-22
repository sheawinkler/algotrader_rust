//! Backtesting framework for AlgoTraderV2 Rust

use std::collections::HashMap;
use serde::{Serialize, Deserialize}; // SimulatedTrade and BacktestReport

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
pub struct Portfolio { // Backtest-only lightweight portfolio
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
            crate::utils::types::OrderSide::Buy => {
                self.cash -= trade.qty * trade.price;
                let pos = self.positions.entry(trade.symbol.clone()).or_default();
                pos.update_on_buy(trade.qty, trade.price);
            }
            crate::utils::types::OrderSide::Sell => {
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
        } else { 0.0 };
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
    pub fn print(&self) {
        println!("===== BACKTEST REPORT =====");
        println!("Start Balance : {:.2}", self.starting_balance);
        println!("End Balance   : {:.2}", self.ending_balance);
        println!("Realized PnL  : {:.2}", self.realized_pnl);
        println!("Max Drawdown  : {:.2}%", self.max_drawdown * 100.0);
        println!("Total Trades  : {}", self.total_trades);
        println!("Winning Trades: {} ({:.2}%)", self.winning_trades, self.winning_trades as f64 / self.total_trades.max(1) as f64 * 100.0);
        println!("Sharpe Ratio  : {:.2}", self.sharpe);
        println!("===========================");
    }
}

pub struct Backtester {
    pub data_provider: Box<dyn HistoricalDataProvider>,
    pub timeframe: String,
    pub starting_balance: f64,
    pub strategies: Vec<Box<dyn crate::strategies::TradingStrategy>>, // strategies to evaluate
    pub cache: Option<crate::backtest::cache::BacktestCache>,
    pub risk_rules: Vec<Box<dyn crate::risk::RiskRule>>,
}

impl Backtester {
    pub async fn run(&mut self, data_file: &PathBuf) -> Result<BacktestReport> {
        // determine date range for caching
        let market_data = self.data_provider.load(data_file)?;
        if market_data.is_empty() {
            return Err(crate::Error::DataError("No market data loaded".to_string()));
        }
        let start_ts = market_data.first().unwrap().timestamp;
        let end_ts = market_data.last().unwrap().timestamp;

        // strategy name concat if single strategy else "multi"
        let strat_key = if self.strategies.len()==1 { self.strategies[0].name().to_string() } else { "multi".to_string() };
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

        // Prepare portfolio
        let mut portfolio = Portfolio::new(self.starting_balance);
        let mut prices: HashMap<String, f64> = HashMap::new();

        // metrics
        let mut total_trades = 0usize;
        let mut winning_trades = 0usize;
        let mut peak_equity = self.starting_balance;
        let mut max_drawdown = 0.0;
        let mut equity_curve: Vec<f64> = Vec::new();

        for data_point in &market_data {
            // update last price map
            prices.insert(data_point.pair.to_string(), data_point.last_price);

            // run strategies
            for strategy in &mut self.strategies {
                let signals = strategy.generate_signals(data_point).await;
                for sig in signals {
                    let qty = sig.size;
                    let price = sig.price;
                    let pnl = 0.0; // placeholder for now

                    use crate::trading::SignalType;
                    let side = match sig.signal_type {
                        SignalType::Buy => crate::utils::types::OrderSide::Buy,
                        SignalType::Sell => crate::utils::types::OrderSide::Sell,
                        _ => continue, // skip signals that are not actionable trades
                    };

                    let trade = SimulatedTrade {
                        timestamp: sig.timestamp,
                        symbol: sig.symbol.clone(),
                        side,
                        qty,
                        price,
                        pnl,
                    };
                    portfolio.apply_trade(&trade);
                    total_trades += 1;
                    if pnl > 0.0 {
                        winning_trades += 1;
                    }
                }
            }

            // update drawdown
            let equity = portfolio.equity(&prices);
            if equity > peak_equity { peak_equity = equity; }
            let drawdown = (peak_equity - equity) / peak_equity;
            if drawdown > max_drawdown { max_drawdown = drawdown; }
            equity_curve.push(equity);

            // ---------- Risk rule evaluation ----------
            // iterate over a snapshot of open positions to avoid borrow issues
            let positions_snapshot: std::collections::HashMap<String, crate::portfolio::Position> = portfolio.positions.clone();
            for (sym, pos) in positions_snapshot {
                if pos.size <= 0.0 { continue; }
                if let Some(price) = prices.get(&sym) {
                    for rule in &self.risk_rules {
                        if let Some(RiskAction::ClosePosition) = rule.evaluate(&sym, &pos, *price) {
                            let pnl = portfolio.update_on_sell(&sym, pos.size, *price);
                            total_trades += 1;
                            if pnl > 0.0 { winning_trades += 1; }
                            break; // after close, stop evaluating more rules for this pos
                        }
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
        let var = returns.iter().map(|r| (r - mean_ret).powi(2)).sum::<f64>() / returns.len().max(1) as f64;
        let sd = var.sqrt();
        let sharpe = if sd > 0.0 { mean_ret / sd * (returns.len() as f64).sqrt() } else { 0.0 };

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
        if let Some(cache)=&self.cache{
            let _ = cache.insert(&strat_key, "UNK", &self.timeframe, start_ts, end_ts, &report);
        }
        Ok(report)
    }
}

use crate::Result;
use crate::risk::{RiskRule, RiskAction};

use crate::utils::types::MarketData;

use std::path::PathBuf;

/// Trait for historical data providers
pub trait HistoricalDataProvider: Send + Sync {
    fn load(&self, data_file: &PathBuf) -> Result<Vec<MarketData>>;
    fn box_clone(&self) -> Box<dyn HistoricalDataProvider>;
}

impl Clone for Box<dyn HistoricalDataProvider> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}




pub mod providers;
pub mod cache;

/// Convenience helper used by CLI until full engine integration is ready
pub async fn simple_backtest(data_path: &PathBuf, timeframe: &str) -> Result<()> {
    use crate::strategies::{MeanReversionStrategy, TradingStrategy, TimeFrame};
    // 1. Provider
    let provider = crate::backtest::providers::CSVHistoricalDataProvider::new();
    // 2. Build backtester
    let strategies: Vec<Box<dyn TradingStrategy>> = vec![
        Box::new(MeanReversionStrategy::new("UNK/UNK", TimeFrame::OneHour, 20, 2.0, 2.0, 1.0)),
    ];
    let mut bt = Backtester {
        risk_rules: vec![
            Box::new(crate::risk::StopLossRule::new(0.05)),
            Box::new(crate::risk::TakeProfitRule::new(0.10)),
        ],
        data_provider: Box::new(provider),
        timeframe: timeframe.to_string(),
        starting_balance: 10_000.0,
        strategies,
        cache: None,
    };
    let report = bt.run(data_path).await?;
    report.print();
    Ok(())
}
// TODO: Add result reporting and export utilities
