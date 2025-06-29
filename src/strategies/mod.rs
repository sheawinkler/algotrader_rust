//! Advanced trading strategies for the AlgoTraderV2

mod advanced;
mod allocation;
mod bundle_sniper;
mod config_impls;
mod mean_reversion;
mod meme_arbitrage;
mod meta;
#[cfg(feature = "ml")]
mod ml_strategy;
mod momentum;
mod order_flow;
mod param_tuner;
mod performance_aware;
pub mod registry;
mod trend_following;

pub use advanced::AdvancedStrategy;
pub use allocation::AllocationStrategy;
pub use bundle_sniper::BundleSniperStrategy;
pub use mean_reversion::MeanReversionStrategy;
pub use meme_arbitrage::MemeArbitrageStrategy;
pub use meta::EnsembleStrategy;
pub use momentum::MomentumStrategy;
pub use order_flow::OrderFlowStrategy;
pub use param_tuner::ParamTuner;
pub use performance_aware::{AdaptiveStrategy, PerformanceAwareStrategy};
pub use trend_following::TrendFollowingStrategy;

#[cfg(feature = "ml")]
pub use ml_strategy::MLStrategy;

// Re-export performance monitoring types
pub use crate::performance::{
    OptimizationSuggestion, PerformanceMonitor, StrategyAnalyzer, StrategyMetrics,
};

use async_trait::async_trait;
use std::error::Error;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::trading::{MarketData, Order, Position, Signal};

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
    fn on_order_filled(&mut self, _order: &Order) {}
    /// Handle trade execution errors (default no-op)
    fn on_trade_error(&mut self, _order: &Order, _err: &anyhow::Error) {}

    /// Update parameters at runtime (default no-op).
    fn update_params(&mut self, _params: &serde_json::Value) {}

    /// Get current positions
    fn get_positions(&self) -> Vec<&Position>;
    /// Downcast helper for dynamic typing
    fn as_any(&self) -> &dyn std::any::Any
    where
        Self: 'static + Sized,
    {
        self
    }
}

// Trait enabling cloning of boxed strategies
#[async_trait]
pub trait TradingStrategyClone: TradingStrategy {
    fn box_clone(&self) -> Box<dyn TradingStrategy>;
}

impl<T> TradingStrategyClone for T
where
    T: TradingStrategy + Clone + 'static,
{
    fn box_clone(&self) -> Box<dyn TradingStrategy> {
        Box::new(self.clone())
    }
}

// Removed custom Clone impl; use box_clone directly where needed

// blanket impl for Box<T> was removed; trait objects are used directly

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
            | TimeFrame::OneMinute => 60,
            | TimeFrame::FiveMinutes => 300,
            | TimeFrame::FifteenMinutes => 900,
            | TimeFrame::OneHour => 3600,
            | TimeFrame::FourHours => 14400,
            | TimeFrame::OneDay => 86400,
            | TimeFrame::OneWeek => 604800,
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
        name: &str, config: &StrategyConfig,
    ) -> Result<Box<dyn TradingStrategy>, Box<dyn Error>> {
        // Create the base strategy
        #[cfg(feature = "perf")]
        use crate::performance::{PerformanceMonitor, StrategyAnalyzer};
        #[cfg(feature = "perf")]
        use std::collections::HashMap;
        use std::convert::TryFrom;
        #[cfg(feature = "perf")]
        use std::time::Duration;
        let strategy: Box<dyn TradingStrategy> = match name {
            | "advanced" => Box::new(AdvancedStrategy::try_from(config)?),
            | "allocation" => {
                // params: { "subs": ["s1","s2"], "weights": [0.6,0.4] }
                let val = config.params.clone();
                let subs: Vec<String> = val
                    .get("subs")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .ok_or("allocation subs missing")?;
                let weights: Vec<f64> = val
                    .get("weights")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .ok_or("allocation weights missing")?;
                if subs.len() != weights.len() {
                    return Err("weights length mismatch".into());
                }
                let mut sub_boxes = Vec::new();
                use serde_json::json;
                for n in &subs {
                    let dummy = StrategyConfig {
                        name: n.clone(),
                        enabled: true,
                        params: json!({}),
                        performance: None,
                    };
                    sub_boxes.push(StrategyFactory::create_strategy(n, &dummy)?);
                }
                Box::new(AllocationStrategy::new(name, sub_boxes, weights))
            }
            | "ensemble" | "meta" => {
                // Expected params: list of strategy names to include
                let names: Vec<String> = serde_json::from_value(config.params.clone())
                    .map_err(|e| format!("Invalid ensemble params: {}", e))?;
                // Recursively create sub strategies using factory
                let mut subs: Vec<Box<dyn TradingStrategy>> = Vec::new();
                use serde_json::json;
                for n in names {
                    if n.eq_ignore_ascii_case("ensemble") || n.eq_ignore_ascii_case("meta") {
                        return Err("Nested ensemble strategies are not supported".into());
                    }
                    // Build a minimal StrategyConfig with empty params; users should pass full configs in TOML for production.
                    let dummy_cfg = StrategyConfig {
                        name: n.clone(),
                        enabled: true,
                        params: json!({}),
                        performance: None,
                    };
                    match StrategyFactory::create_strategy(&n, &dummy_cfg) {
                        | Ok(s) => subs.push(s),
                        | Err(e) => {
                            return Err(format!("Failed to create sub-strategy {}: {}", n, e).into())
                        }
                    }
                }
                Box::new(EnsembleStrategy::new(name, subs))
            }
            | "mean_reversion" => Box::new(MeanReversionStrategy::try_from(config)?),
            | "trend_following" => Box::new(TrendFollowingStrategy::try_from(config)?),
            | "order_flow" => Box::new(OrderFlowStrategy::try_from(config)?),
            | "momentum" => Box::new(MomentumStrategy::try_from(config)?),
            | "meme_arbitrage" => Box::new(MemeArbitrageStrategy::try_from(config)?),
            | "bundle_sniper" => Box::new(BundleSniperStrategy::try_from(config)?),
            #[cfg(feature = "ml")]
            | "ml" => Box::new(MLStrategy::from_config(config)?),
            | _ => return Err(format!("Unknown strategy: {}", name).into()),
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
        strategy: Box<dyn TradingStrategy>, perf_config: &PerformanceConfig,
    ) -> Box<dyn TradingStrategy> {
        // TEMP: Skip PerformanceAware wrapping for now until AdaptiveStrategy is implemented for all.
        strategy

        /* Create performance monitor
                let monitor = PerformanceMonitor::new(
                    Duration::from_secs(perf_config.review_interval_minutes * 60),
                    perf_config.max_consecutive_losses,
        {{ ... }}
                    perf_config.min_win_rate_pct,
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
                let wrapped = PerformanceAwareStrategy::new(strategy, monitor, analyzer, initial_params);

                Box::new(wrapped) */
    }
}

/// Parse timeframe string into TimeFrame enum
fn parse_timeframe(tf: &str) -> Result<TimeFrame, String> {
    match tf.to_lowercase().as_str() {
        | "1m" | "1min" => Ok(TimeFrame::OneMinute),
        | "5m" | "5min" => Ok(TimeFrame::FiveMinutes),
        | "15m" | "15min" => Ok(TimeFrame::FifteenMinutes),
        | "1h" | "1hour" => Ok(TimeFrame::OneHour),
        | "4h" | "4hour" => Ok(TimeFrame::FourHours),
        | "1d" | "1day" => Ok(TimeFrame::OneDay),
        | "1w" | "1week" => Ok(TimeFrame::OneWeek),
        | _ => Err(format!("Invalid timeframe: {}", tf)),
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
