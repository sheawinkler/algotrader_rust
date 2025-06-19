//! Stub Machine Learning based strategy (placeholder until implemented)
//! This compiles when the `ml` feature flag is enabled.

use anyhow::Result;
use async_trait::async_trait;

use super::{TradingStrategy, TimeFrame, StrategyConfig};
use crate::trading::{MarketData, Signal, Position, Order};

/// A minimal no-op ML strategy used as a placeholder so the crate compiles
/// with the `ml` feature enabled.
pub struct MLStrategy;

impl MLStrategy {
    /// Build an MLStrategy from configuration. The current implementation
    /// ignores the provided parameters and returns a default instance.
    pub fn from_config(_config: &StrategyConfig) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self)
    }
}

#[async_trait]
impl TradingStrategy for MLStrategy {
    fn name(&self) -> &str {
        "ml"
    }

    fn timeframe(&self) -> TimeFrame {
        // Default to 1-hour timeframe for the stub
        TimeFrame::OneHour
    }

    fn symbols(&self) -> Vec<String> {
        Vec::new()
    }

    async fn generate_signals(&mut self, _market_data: &MarketData) -> Vec<Signal> {
        // No trading logic yet – return empty vector
        Vec::new()
    }

    fn on_order_filled(&mut self, _order: &Order) { }

    fn on_trade_error(&mut self, _order: &Order, _err: &anyhow::Error) { }

    fn get_positions(&self) -> Vec<&Position> {
        Vec::new()
    }
}
