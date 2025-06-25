//! Position sizing policies.
//! Each implementation converts account equity into a position size (in base currency units).

use async_trait::async_trait;
use std::sync::Arc;

/// Position sizing interface.
#[async_trait]
pub trait PositionSizer: Send + Sync {
    /// Return the position size (base currency amount) to trade.
    ///
    /// * `equity` – account equity in base currency (e.g. SOL or USDN equivalent).
    /// * `symbol` – trading symbol (e.g. "SOL/USDC") – useful for symbol-specific sizing.
    async fn size(&self, equity: f64, symbol: &str) -> f64;

    fn box_clone(&self) -> Box<dyn PositionSizer>;
}

impl Clone for Box<dyn PositionSizer> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

/// Fixed-fractional risk model – trades a constant percentage of equity.
#[derive(Debug, Clone)]
pub struct FixedFractionalSizer {
    pub pct: f64, // e.g. 0.01 = 1% of equity
}

impl FixedFractionalSizer {
    pub fn new(pct: f64) -> Self {
        Self { pct }
    }
}

#[async_trait]
impl PositionSizer for FixedFractionalSizer {
    async fn size(&self, equity: f64, _symbol: &str) -> f64 {
        equity * self.pct
    }
    fn box_clone(&self) -> Box<dyn PositionSizer> {
        Box::new(self.clone())
    }
}

/// Kelly Criterion sizer (static values).

/// kelly_fraction = (bp - q)/b  where
///   b = payoff ratio (avg win / avg loss)
///   p = win rate, q = 1-p
/// size = equity * kelly_fraction * leverage_cap
#[derive(Debug, Clone)]
pub struct KellySizer {
    pub win_rate: f64,     // p
    pub payoff_ratio: f64, // b
    pub cap: f64,          // max fraction (e.g. 0.25 to half-Kelly)
}

impl KellySizer {
    pub fn new(win_rate: f64, payoff_ratio: f64, cap: f64) -> Self {
        Self { win_rate, payoff_ratio, cap }
    }
}

#[async_trait]
impl PositionSizer for KellySizer {
    async fn size(&self, equity: f64, _symbol: &str) -> f64 {
        let p = self.win_rate.clamp(0.0, 1.0);
        let b = if self.payoff_ratio <= 0.0 {
            1.0
        } else {
            self.payoff_ratio
        };
        let q = 1.0 - p;
        let kelly = ((b * p) - q) / b;
        let fraction = kelly.max(0.0).min(self.cap);
        equity * fraction
    }
    fn box_clone(&self) -> Box<dyn PositionSizer> {
        Box::new(self.clone())
    }
}

/// Live Kelly Criterion sizer pulling metrics from PerformanceMonitor.
#[derive(Clone)]
pub struct LiveKellySizer {
    pub cap: f64,
    monitor: Arc<crate::performance::PerformanceMonitor>,
}

impl std::fmt::Debug for LiveKellySizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LiveKellySizer")
            .field("cap", &self.cap)
            .finish()
    }
}

impl LiveKellySizer {
    pub fn new(cap: f64, monitor: Arc<crate::performance::PerformanceMonitor>) -> Self {
        Self { cap, monitor }
    }
}

#[async_trait]
impl PositionSizer for LiveKellySizer {
    async fn size(&self, equity: f64, _symbol: &str) -> f64 {
        let metrics = self.monitor.metrics_snapshot().await;
        if metrics.is_empty() {
            return 0.0;
        }
        // Aggregate
        let total_trades: u32 = metrics.iter().map(|m| m.total_trades).sum();
        let total_wins: u32 = metrics.iter().map(|m| m.winning_trades).sum();
        let total_losses: u32 = metrics.iter().map(|m| m.losing_trades).sum();
        if total_trades == 0 {
            return 0.0;
        }
        let p = total_wins as f64 / total_trades as f64;
        let avg_win = metrics
            .iter()
            .filter(|m| m.winning_trades > 0)
            .map(|m| m.total_pnl.max(0.0))
            .sum::<f64>()
            / (total_wins.max(1) as f64);
        let avg_loss = metrics
            .iter()
            .filter(|m| m.losing_trades > 0)
            .map(|m| m.total_pnl.min(0.0).abs())
            .sum::<f64>()
            / (total_losses.max(1) as f64);
        let b = if avg_loss > 0.0 {
            avg_win / avg_loss
        } else {
            1.0
        };
        let q = 1.0 - p;
        let kelly = ((b * p) - q) / b;
        let frac = kelly.max(0.0).min(self.cap);
        equity * frac
    }
    fn box_clone(&self) -> Box<dyn PositionSizer> {
        Box::new(self.clone())
    }
}

/// Volatility-scaled sizer using ATR.
/// position = equity * risk_pct / (atr * atr_mult)

#[derive(Clone)]
pub struct VolatilitySizer {
    pub risk_pct: f64, // equity risk fraction
    pub atr_mult: f64, // stop distance in ATRs
    atr_fetcher: Arc<dyn Fn(&str) -> Option<f64> + Send + Sync>,
}

impl std::fmt::Debug for VolatilitySizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VolatilitySizer")
            .field("risk_pct", &self.risk_pct)
            .field("atr_mult", &self.atr_mult)
            .finish()
    }
}

impl VolatilitySizer {
    pub fn new<F>(risk_pct: f64, atr_mult: f64, atr_fetcher: F) -> Self
    where
        F: Fn(&str) -> Option<f64> + Send + Sync + 'static,
    {
        Self { risk_pct, atr_mult, atr_fetcher: Arc::new(atr_fetcher) }
    }
}

#[async_trait]
impl PositionSizer for VolatilitySizer {
    async fn size(&self, equity: f64, symbol: &str) -> f64 {
        if let Some(atr) = (self.atr_fetcher)(symbol) {
            if atr > 0.0 {
                return (equity * self.risk_pct) / (atr * self.atr_mult);
            }
        }
        0.0
    }
    fn box_clone(&self) -> Box<dyn PositionSizer> {
        Box::new(self.clone())
    }
}
