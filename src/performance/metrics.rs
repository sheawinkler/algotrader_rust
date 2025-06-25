use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

/// Tracks performance metrics for a trading strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyMetrics {
    pub strategy_name: String,
    pub total_trades: u32,
    pub winning_trades: u32,
    pub losing_trades: u32,
    pub consecutive_losses: u32,
    pub max_consecutive_losses: u32,
    pub total_pnl: f64,
    pub win_rate: f64,
    pub profit_factor: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown: f64,
    pub current_drawdown: f64,
    pub last_trade_time: Option<SystemTime>,
    pub last_review: SystemTime,
    pub custom_metrics: HashMap<String, f64>,
}

impl StrategyMetrics {
    /// Create new strategy metrics
    pub fn new(strategy_name: &str) -> Self {
        Self {
            strategy_name: strategy_name.to_string(),
            total_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            consecutive_losses: 0,
            max_consecutive_losses: 0,
            total_pnl: 0.0,
            win_rate: 0.0,
            profit_factor: 0.0,
            sharpe_ratio: 0.0,
            max_drawdown: 0.0,
            current_drawdown: 0.0,
            last_trade_time: None,
            last_review: SystemTime::now(),
            custom_metrics: HashMap::new(),
        }
    }

    /// Record a trade and update metrics
    pub fn record_trade(&mut self, pnl: f64) {
        self.total_trades += 1;
        self.last_trade_time = Some(SystemTime::now());

        // Update PnL
        self.total_pnl += pnl;

        // Update win/loss counts
        if pnl > 0.0 {
            self.winning_trades += 1;
            self.consecutive_losses = 0;
        } else {
            self.losing_trades += 1;
            self.consecutive_losses += 1;
            self.max_consecutive_losses = self.max_consecutive_losses.max(self.consecutive_losses);
        }

        // Update win rate
        self.win_rate = self.winning_trades as f64 / self.total_trades as f64;

        // Update drawdown
        self.current_drawdown = if pnl < 0.0 {
            self.current_drawdown + pnl.abs()
        } else {
            0.0
        };

        self.max_drawdown = self.max_drawdown.max(self.current_drawdown);

        // Calculate profit factor (gross profits / gross losses)
        // Note: This is a simplified version - in practice, track gross profits/losses separately
        self.profit_factor = if self.losing_trades > 0 {
            (self.winning_trades as f64 * self.win_rate) / self.losing_trades as f64
        } else {
            f64::INFINITY
        };
    }

    /// Add a custom metric
    pub fn add_custom_metric(&mut self, name: &str, value: f64) {
        self.custom_metrics.insert(name.to_string(), value);
    }

    /// Get a custom metric
    pub fn get_custom_metric(&self, name: &str) -> Option<f64> {
        self.custom_metrics.get(name).copied()
    }

    /// Calculate the Kelly Criterion for position sizing
    pub fn kelly_criterion(&self) -> f64 {
        if self.winning_trades == 0 || self.losing_trades == 0 {
            return 0.1; // Default to 10% if not enough data
        }

        let win_prob = self.win_rate;
        let avg_win = self.total_pnl / self.winning_trades as f64;
        let avg_loss = if self.losing_trades > 0 {
            self.total_pnl.abs() / self.losing_trades as f64
        } else {
            1.0
        };

        let win_loss_ratio = avg_win / avg_loss;
        let kelly = (win_prob * win_loss_ratio - (1.0 - win_prob)) / win_loss_ratio;

        // Use half-Kelly for more conservative position sizing
        (kelly * 0.5).max(0.01).min(0.5) // Between 1% and 50%
    }

    /// Calculate risk of ruin
    pub fn risk_of_ruin(&self) -> f64 {
        if self.winning_trades == 0 || self.losing_trades == 0 {
            return 0.5; // 50% if not enough data
        }

        let win_rate = self.win_rate;
        let loss_rate = 1.0 - win_rate;
        let avg_win = self.total_pnl / self.winning_trades as f64;
        let avg_loss = if self.losing_trades > 0 {
            self.total_pnl.abs() / self.losing_trades as f64
        } else {
            1.0
        };

        let win_loss_ratio = avg_win / avg_loss;

        // Simplified risk of ruin calculation
        let risk = ((1.0 - (win_rate - loss_rate / win_loss_ratio))
            / (1.0 + (win_rate - loss_rate / win_loss_ratio)))
            .powf(1.0 / 0.02);

        risk.max(0.0).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_tracking() {
        let mut metrics = StrategyMetrics::new("TestStrategy");

        // Record a winning trade
        metrics.record_trade(100.0);
        assert_eq!(metrics.total_trades, 1);
        assert_eq!(metrics.winning_trades, 1);
        assert_eq!(metrics.consecutive_losses, 0);
        assert_eq!(metrics.total_pnl, 100.0);

        // Record a losing trade
        metrics.record_trade(-50.0);
        assert_eq!(metrics.total_trades, 2);
        assert_eq!(metrics.losing_trades, 1);
        assert_eq!(metrics.consecutive_losses, 1);
        assert_eq!(metrics.total_pnl, 50.0);

        // Test win rate
        assert_eq!(metrics.win_rate, 0.5);

        // Test Kelly Criterion
        let kelly = metrics.kelly_criterion();
        assert!(kelly > 0.0 && kelly <= 0.5);

        // Test risk of ruin
        let ror = metrics.risk_of_ruin();
        assert!(ror >= 0.0 && ror <= 1.0);
    }
}
