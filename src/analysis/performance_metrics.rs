use std::collections::VecDeque;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Represents a single trade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub id: String,
    pub symbol: String,
    pub entry_time: DateTime<Utc>,
    pub exit_time: Option<DateTime<Utc>>,
    pub entry_price: f64,
    pub exit_price: Option<f64>,
    pub quantity: f64,
    pub side: TradeSide,
    pub pnl: Option<f64>,
    pub pnl_percentage: Option<f64>,
    pub fees: f64,
    pub status: TradeStatus,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
    pub strategy: String,
    pub notes: Option<String>,
}

/// Trade side (Long/Short)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TradeSide {
    Long,
    Short,
}

/// Trade status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TradeStatus {
    Open,
    Closed,
    StoppedOut,
    TakeProfit,
    Liquidated,
    Error,
}

/// Performance metrics for a trading strategy or portfolio
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub initial_balance: f64,
    pub current_balance: f64,
    pub total_trades: u64,
    pub winning_trades: u64,
    pub losing_trades: u64,
    pub win_rate: f64,
    pub total_pnl: f64,
    pub total_pnl_percentage: f64,
    pub avg_win: f64,
    pub avg_loss: f64,
    pub profit_factor: f64,
    pub max_drawdown: f64,
    pub max_drawdown_percentage: f64,
    pub sharpe_ratio: f64,
    pub sortino_ratio: f64,
    pub calmar_ratio: f64,
    pub average_trade_duration: Duration,
    pub trade_dates: Vec<DateTime<Utc>>,
    pub equity_curve: Vec<f64>,
    pub daily_returns: Vec<f64>,
    pub monthly_returns: Vec<f64>,
    pub yearly_returns: Vec<f64>,
}

pub type TradeRecord = Trade;
pub type PerformanceSnapshot = PerformanceMetrics;

/// Tracks performance metrics for a trading strategy
pub struct PerformanceTracker {
    trades: Vec<Trade>,
    initial_balance: f64,
    current_balance: f64,
    peak_balance: f64,
    max_drawdown: f64,
    trade_history: VecDeque<Trade>,
    max_trades: usize,
}

impl Default for PerformanceTracker {
    fn default() -> Self {
        Self::new(10_000.0, 10_000)
    }
}

impl PerformanceTracker {
    /// Create a new PerformanceTracker
    pub fn new(initial_balance: f64, max_trades: usize) -> Self {
        Self {
            trades: Vec::new(),
            initial_balance,
            current_balance: initial_balance,
            peak_balance: initial_balance,
            max_drawdown: 0.0,
            trade_history: VecDeque::with_capacity(max_trades),
            max_trades,
        }
    }
    
    /// Record a new trade
    pub fn record_trade(&mut self, trade: Trade) -> anyhow::Result<()> {
        // Add to trades list
        self.trades.push(trade.clone());
        
        // Update trade history (FIFO)
        if self.trade_history.len() >= self.max_trades {
            self.trade_history.pop_front();
        }
        self.trade_history.push_back(trade);
        
        // Update metrics
        self.update_metrics()?;
        
        Ok(())
    }
    
    /// Update performance metrics based on current trades
    fn update_metrics(&mut self) -> anyhow::Result<()> {
        // Calculate current balance from all closed trades
        let mut balance = self.initial_balance; // running equity
        let mut winning_trades = 0;
        let mut total_pnl = 0.0;
        let win_pnl = 0.0;
        let loss_pnl = 0.0;
        let mut trade_durations = Vec::new();
        
        for trade in &self.trades {
            if let (Some(exit_price), Some(pnl)) = (trade.exit_price, trade.pnl) {
                balance += pnl;
                total_pnl += pnl;
                
                if pnl > 0.0 {
                    winning_trades += 1;
                }
                
                if let Some(exit_time) = trade.exit_time {
                    let duration = exit_time - trade.entry_time;
                    trade_durations.push(duration);
                }
            }
        }
        
        // Update current balance and peak
        self.current_balance = balance;
        if balance > self.peak_balance {
            self.peak_balance = balance;
        }
        
        // Calculate drawdown
        let drawdown = if self.peak_balance > 0.0 {
            (self.peak_balance - balance) / self.peak_balance
        } else { 0.0 };
        if drawdown > self.max_drawdown {
            self.max_drawdown = drawdown;
        }
        
        Ok(())
    }
    
    /// Get current performance metrics
    pub fn get_metrics(&self) -> PerformanceMetrics {
        let total_trades = self.trades.len() as u64;
        let winning_trades = self.trades.iter().filter(|t| t.pnl.unwrap_or(0.0) > 0.0).count() as u64;
        let losing_trades = total_trades - winning_trades;
        let win_rate = if total_trades > 0 {
            winning_trades as f64 / total_trades as f64
        } else {
            0.0
        };
        
        let pnl_values: Vec<f64> = self.trades.iter()
            .filter_map(|t| t.pnl)
            .collect();
            
        let avg_win = if winning_trades > 0 {
            pnl_values.iter().filter(|&&p| p > 0.0).sum::<f64>() / winning_trades as f64
        } else {
            0.0
        };
        
        let avg_loss = if losing_trades > 0 {
            pnl_values.iter().filter(|&&p| p < 0.0).sum::<f64>().abs() / losing_trades as f64
        } else {
            0.0
        };
        
        let profit_factor = if avg_loss > 0.0 {
            (avg_win * winning_trades as f64) / (avg_loss * losing_trades as f64)
        } else if winning_trades > 0 {
            f64::INFINITY
        } else {
            0.0
        };
        
        // Calculate equity curve
        let mut equity = self.initial_balance;
        let mut equity_curve = vec![equity];
        let mut daily_returns = Vec::new();
        
        for trade in &self.trades {
            if let Some(pnl) = trade.pnl {
                equity += pnl;
                equity_curve.push(equity);
                
                // Calculate daily return
                let prev_equity = equity - pnl;
                if prev_equity > 0.0 {
                    daily_returns.push(pnl / prev_equity);
                }
            }
        }
        
        // Calculate risk-adjusted returns (simplified)
        let sharpe_ratio = self.calculate_sharpe_ratio(&daily_returns);
        let sortino_ratio = self.calculate_sortino_ratio(&daily_returns);
        let calmar_ratio = self.calculate_calmar_ratio();
        
        PerformanceMetrics {
            start_time: self.trades.first().map(|t| t.entry_time).unwrap_or_else(Utc::now),
            end_time: self.trades.last().and_then(|t| t.exit_time),
            initial_balance: self.initial_balance,
            current_balance: self.current_balance,
            total_trades,
            winning_trades,
            losing_trades: losing_trades,
            win_rate,
            total_pnl: self.current_balance - self.initial_balance,
            total_pnl_percentage: (self.current_balance - self.initial_balance) / self.initial_balance * 100.0,
            avg_win,
            avg_loss,
            profit_factor,
            max_drawdown: self.max_drawdown * self.peak_balance,
            max_drawdown_percentage: self.max_drawdown * 100.0,
            sharpe_ratio,
            sortino_ratio,
            calmar_ratio,
            average_trade_duration: {
                let total_secs: i64 = self.trades.iter()
                    .filter_map(|t| match t.exit_time {
                        Some(exit) => Some((exit - t.entry_time).num_seconds()),
                        None => None,
                    }).sum();
                let completed: i64 = self.trades.iter().filter(|t| t.exit_time.is_some()).count() as i64;
                let avg_secs = if completed > 0 { total_secs / completed } else { 0 };
                Duration::from_secs(avg_secs as u64)
            },
            trade_dates: self.trades.iter().map(|t| t.entry_time).collect(),
            equity_curve,
            daily_returns,
            monthly_returns: Vec::new(), // Would aggregate from daily returns
            yearly_returns: Vec::new(),  // Would aggregate from daily returns
        }
    }
    
    /// Calculate Sharpe ratio (risk-free rate assumed to be 0 for simplicity)
    fn calculate_sharpe_ratio(&self, returns: &[f64]) -> f64 {
        if returns.is_empty() {
            return 0.0;
        }
        
        let mean = returns.iter().sum::<f64>() / returns.len() as f64;
        let std_dev = (returns.iter()
            .map(|r| (r - mean).powi(2))
            .sum::<f64>() / returns.len() as f64)
            .sqrt();
            
        if std_dev > 0.0 {
            mean / std_dev * (365.0_f64).sqrt() // Annualized
        } else {
            0.0
        }
    }
    
    /// Calculate Sortino ratio
    fn calculate_sortino_ratio(&self, returns: &[f64]) -> f64 {
        if returns.is_empty() {
            return 0.0;
        }
        
        let mean = returns.iter().sum::<f64>() / returns.len() as f64;
        let downside_returns: Vec<f64> = returns.iter()
            .filter(|&&r| r < 0.0)
            .map(|&r| r.powi(2))
            .collect();
            
        let downside_dev = if !downside_returns.is_empty() {
            (downside_returns.iter().sum::<f64>() / downside_returns.len() as f64).sqrt()
        } else {
            0.0
        };
        
        if downside_dev > 0.0 {
            mean / downside_dev * (365.0_f64).sqrt() // Annualized
        } else {
            0.0
        }
    }
    
    /// Calculate Calmar ratio
    fn calculate_calmar_ratio(&self) -> f64 {
        if self.max_drawdown > 0.0 {
            let total_return = (self.current_balance - self.initial_balance) / self.initial_balance;
            total_return / self.max_drawdown
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    
    fn create_test_trade(id: &str, pnl: Option<f64>, duration_hours: i64) -> Trade {
        let now = Utc::now();
        Trade {
            id: id.to_string(),
            symbol: "SOL/USDC".to_string(),
            entry_time: now - Duration::hours(duration_hours),
            exit_time: Some(now),
            entry_price: 100.0,
            exit_price: pnl.map(|p| 100.0 + p),
            quantity: 1.0,
            side: TradeSide::Long,
            pnl,
            pnl_percentage: pnl.map(|p| p / 100.0 * 100.0),
            fees: 0.1,
            status: TradeStatus::Closed,
            stop_loss: None,
            take_profit: None,
            strategy: "test".to_string(),
            notes: None,
        }
    }
    
    #[test]
    fn test_performance_tracker() {
        let mut tracker = PerformanceTracker::new(10_000.0, 1000);
        
        // Add some winning and losing trades
        tracker.record_trade(create_test_trade("1", Some(100.0), 24)).unwrap();
        tracker.record_trade(create_test_trade("2", Some(150.0), 48)).unwrap();
        tracker.record_trade(create_test_trade("3", Some(-50.0), 72)).unwrap();
        
        let metrics = tracker.get_metrics();
        
        assert_eq!(metrics.total_trades, 3);
        assert_eq!(metrics.winning_trades, 2);
        assert_eq!(metrics.losing_trades, 1);
        assert!((metrics.win_rate - 0.6667).abs() < 0.01);
        assert!((metrics.total_pnl - 200.0).abs() < 0.01);
        assert!(metrics.profit_factor > 0.0);
    }
    
    #[test]
    fn test_sharpe_ratio() {
        let tracker = PerformanceTracker::default();
        let returns = vec![0.01, -0.005, 0.02, -0.01, 0.015];
        let sharpe = tracker.calculate_sharpe_ratio(&returns);
        
        // Just verify it's calculated without panicking
        assert!(sharpe.is_finite());
    }
}
