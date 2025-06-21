//! Backtesting framework for AlgoTraderV2 Rust

use std::collections::HashMap;

// Backtester struct with portfolio simulation and basic reporting
/// Result of a single simulated trade
#[derive(Debug, Clone)]
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
    pub cash: f64,
    pub positions: HashMap<String, f64>, // symbol -> qty
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
                *self.positions.entry(trade.symbol.clone()).or_default() += trade.qty;
            }
            crate::utils::types::OrderSide::Sell => {
                self.cash += trade.qty * trade.price;
                *self.positions.entry(trade.symbol.clone()).or_default() -= trade.qty;
                self.realized_pnl += trade.pnl;
            }
        }
    }

    pub fn equity(&self, prices: &HashMap<String, f64>) -> f64 {
        let mut eq = self.cash;
        for (sym, qty) in &self.positions {
            if let Some(price) = prices.get(sym) {
                eq += qty * price;
            }
        }
        eq
    }
}

/// Summary of backtest results
#[derive(Debug, Clone)]
pub struct BacktestReport {
    pub starting_balance: f64,
    pub ending_balance: f64,
    pub realized_pnl: f64,
    pub max_drawdown: f64,
    pub total_trades: usize,
    pub winning_trades: usize,
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
        println!("===========================");
    }
}

pub struct Backtester {
    pub data_provider: Box<dyn HistoricalDataProvider>,
    pub timeframe: String,
    pub starting_balance: f64,
    pub strategies: Vec<Box<dyn crate::strategies::TradingStrategy>>, // strategies to evaluate
}

impl Backtester {
    pub async fn run(&mut self, data_file: &PathBuf) -> Result<BacktestReport> {
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
        }

        let ending_balance = portfolio.equity(&prices);
        Ok(BacktestReport {
            starting_balance: self.starting_balance,
            ending_balance,
            realized_pnl: portfolio.realized_pnl,
            max_drawdown,
            total_trades,
            winning_trades,
        })
    }
}

use crate::Result;

use crate::utils::types::MarketData;

use std::path::PathBuf;

/// Trait for historical data providers
pub trait HistoricalDataProvider {
    fn load(&self, data_file: &PathBuf) -> Result<Vec<MarketData>>;
}




pub mod providers;


/// Convenience helper used by CLI until full engine integration is ready
pub async fn simple_backtest(data_path: &PathBuf, timeframe: &str) -> Result<()> {
    
                                    let provider = crate::backtest::providers::CSVHistoricalDataProvider;
    let rows = provider.load(data_path)?;
    println!("Loaded {} rows for timeframe {}", rows.len(), timeframe);
    // TODO: integrate with TradingBot; for now just success
    Ok(())
}
// TODO: Add result reporting and export utilities
