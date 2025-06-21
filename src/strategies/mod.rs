//! Advanced trading strategies for the AlgoTraderV2

mod advanced;
mod mean_reversion;
mod trend_following;
mod order_flow;
#[cfg(feature = "ml")]
mod ml_strategy;
mod meme_arbitrage;
mod performance_aware;
mod momentum;
mod bundle_sniper;
mod config_impls;

pub use advanced::AdvancedStrategy;
pub use mean_reversion::MeanReversionStrategy;
pub use trend_following::TrendFollowingStrategy;
pub use order_flow::OrderFlowStrategy;
pub use meme_arbitrage::MemeArbitrageStrategy;
pub use performance_aware::{PerformanceAwareStrategy, AdaptiveStrategy};
pub use momentum::MomentumStrategy;
pub use bundle_sniper::BundleSniperStrategy;

#[cfg(feature = "ml")]
pub use ml_strategy::MLStrategy;

// Re-export performance monitoring types
pub use crate::performance::{
    StrategyMetrics,
    PerformanceMonitor,
    StrategyAnalyzer,
    OptimizationSuggestion,
};

use std::error::Error;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::trading::{MarketData, Signal, Position, Order};


/// Trait for all trading strategies
#[async_trait]
pub trait TradingStrategy: Send + Sync {
    /// Get the name of the strategy
    fn name(&self) -> &str;
    
    /// Get the timeframe this strategy operates on
    fn timeframe(&self) -> TimeFrame;
    
    /// Get the symbols this strategy trades
    fn symbols(&self) -> Vec<String>;
    
    /// Generate trading signals based on market data
    async fn generate_signals(&mut self, market_data: &MarketData) -> Vec<Signal>;
    
    /// Handle order filled events (default does nothing)
    fn on_order_filled(&mut self, _order: &Order) { }
    /// Handle trade execution errors (default no-op)
    fn on_trade_error(&mut self, _order: &Order, _err: &anyhow::Error) { }
    
    /// Get current positions
    fn get_positions(&self) -> Vec<&Position>;
}

// Allow Box<dyn TradingStrategy> to itself satisfy TradingStrategy by delegating
#[async_trait]
impl<T: TradingStrategy + ?Sized> TradingStrategy for Box<T> {
    fn name(&self) -> &str { (**self).name() }
    fn timeframe(&self) -> TimeFrame { (**self).timeframe() }
    fn symbols(&self) -> Vec<String> { (**self).symbols() }
    async fn generate_signals(&mut self, market_data: &MarketData) -> Vec<Signal> {
        (**self).generate_signals(market_data).await
    }
    fn on_order_filled(&mut self, order: &Order) { (**self).on_order_filled(order); }
    fn on_trade_error(&mut self, order: &Order, err: &anyhow::Error) { (**self).on_trade_error(order, err); }
    fn get_positions(&self) -> Vec<&Position> { (**self).get_positions() }
}

/// Time frame for the strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimeFrame {
    OneMinute,
    FiveMinutes,
    FifteenMinutes,
    OneHour,
    FourHours,
    OneDay,
    OneWeek,
}

impl TimeFrame {
    pub fn as_seconds(&self) -> u64 {
        match self {
            TimeFrame::OneMinute => 60,
            TimeFrame::FiveMinutes => 300,
            TimeFrame::FifteenMinutes => 900,
            TimeFrame::OneHour => 3600,
            TimeFrame::FourHours => 14400,
            TimeFrame::OneDay => 86400,
            TimeFrame::OneWeek => 604800,
        }
    }
}

/// Strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    /// Name of the strategy
    pub name: String,
    /// Whether the strategy is enabled
    pub enabled: bool,
    /// Strategy-specific parameters
    pub params: serde_json::Value,
    /// Performance monitoring configuration (optional)
    pub performance: Option<PerformanceConfig>,
}

/// Performance monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// How often to review performance (in minutes)
    pub review_interval_minutes: u64,
    /// Maximum number of consecutive losses before taking action
    pub max_consecutive_losses: u32,
    /// Maximum drawdown percentage before reducing position size
    pub max_drawdown_pct: f64,
    /// Minimum acceptable win rate (as a percentage, e.g., 40.0 for 40%)
    pub min_win_rate_pct: f64,
    /// Number of days to look back for performance analysis
    pub lookback_days: u32,
}

/// Strategy factory
pub struct StrategyFactory;

impl StrategyFactory {
    /// Create a new strategy instance from configuration with performance monitoring
    pub fn create_strategy(
        name: &str,
        config: &StrategyConfig,
    ) -> Result<Box<dyn TradingStrategy>, Box<dyn Error>> {
        // Create the base strategy
        use std::convert::TryFrom;
#[cfg(feature = "perf")] use crate::performance::{PerformanceMonitor, StrategyAnalyzer};
#[cfg(feature = "perf")] use std::time::Duration;
#[cfg(feature = "perf")] use std::collections::HashMap;
        let strategy: Box<dyn TradingStrategy> = match name {
            "advanced" => Box::new(AdvancedStrategy::try_from(config)?),
            "mean_reversion" => Box::new(MeanReversionStrategy::try_from(config)?),
            "trend_following" => Box::new(TrendFollowingStrategy::try_from(config)?),
            "order_flow" => Box::new(OrderFlowStrategy::try_from(config)?),
            "momentum" => Box::new(MomentumStrategy::try_from(config)?),
            "meme_arbitrage" => Box::new(MemeArbitrageStrategy::try_from(config)?),
            "bundle_sniper" => Box::new(BundleSniperStrategy::try_from(config)?),
            #[cfg(feature = "ml")]
            "ml" => Box::new(MLStrategy::from_config(config)?),
            _ => return Err(format!("Unknown strategy: {}", name).into()),
        };

        #[cfg(feature = "perf")]
        if let Some(perf_config) = &config.performance {
            return Ok(Self::wrap_with_performance_monitor(strategy, perf_config));
        }

        // Return base strategy when performance feature disabled or no configuration provided.
        Ok(strategy)
    }

    #[cfg(feature = "perf")]
    fn wrap_with_performance_monitor(
        strategy: Box<dyn TradingStrategy>,
        perf_config: &PerformanceConfig,
    ) -> Box<dyn TradingStrategy> {
        // Create performance monitor
        let monitor = PerformanceMonitor::new(
            Duration::from_secs(perf_config.review_interval_minutes * 60),
            perf_config.max_consecutive_losses,
            perf_config.max_drawdown_pct,
            perf_config.min_win_rate_pct
        );

        // Create strategy analyzer
        let analyzer = StrategyAnalyzer::new(
            10, // min_trades
            perf_config.min_win_rate_pct,
            perf_config.max_drawdown_pct,
            perf_config.lookback_days,
        );

        // For now, use empty initial parameter set until AdaptiveStrategy support is integrated
        let initial_params: HashMap<String, f64> = HashMap::new();

        // Create performance-aware wrapper
        let wrapped = PerformanceAwareStrategy::new(
            strategy,
            monitor,
            analyzer,
            initial_params,
        );

        Box::new(wrapped)
    }
}

/// Parse timeframe string into TimeFrame enum
fn parse_timeframe(tf: &str) -> Result<TimeFrame, String> {
    match tf.to_lowercase().as_str() {
        "1m" | "1min" => Ok(TimeFrame::OneMinute),
        "5m" | "5min" => Ok(TimeFrame::FiveMinutes),
        "15m" | "15min" => Ok(TimeFrame::FifteenMinutes),
        "1h" | "1hour" => Ok(TimeFrame::OneHour),
        "4h" | "4hour" => Ok(TimeFrame::FourHours),
        "1d" | "1day" => Ok(TimeFrame::OneDay),
        "1w" | "1week" => Ok(TimeFrame::OneWeek),
        _ => Err(format!("Invalid timeframe: {}", tf)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_timeframe() {
        assert_eq!(parse_timeframe("1m").unwrap(), TimeFrame::OneMinute);
        assert_eq!(parse_timeframe("5min").unwrap(), TimeFrame::FiveMinutes);
        assert_eq!(parse_timeframe("15m").unwrap(), TimeFrame::FifteenMinutes);
        assert_eq!(parse_timeframe("1h").unwrap(), TimeFrame::OneHour);
        assert_eq!(parse_timeframe("4hour").unwrap(), TimeFrame::FourHours);
        assert_eq!(parse_timeframe("1d").unwrap(), TimeFrame::OneDay);
        assert_eq!(parse_timeframe("1week").unwrap(), TimeFrame::OneWeek);
        assert!(parse_timeframe("invalid").is_err());
    }
}
