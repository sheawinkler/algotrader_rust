//! Conversions from generic `StrategyConfig` to concrete strategy instances.
//! Currently this provides basic default parameter mappings. In future we
//! should parse `config.params` for fine-grained tuning.

use super::trend_following::TrendFollowingConfig;
use super::*;
use std::convert::TryFrom;

mod _removed_brace {}

impl TryFrom<&StrategyConfig> for AdvancedStrategy {
    type Error = Box<dyn std::error::Error>;
    fn try_from(_cfg: &StrategyConfig) -> Result<Self, Self::Error> {
        Ok(AdvancedStrategy::new(
            "SOL/USDC",
            TimeFrame::OneHour,
            14,
            20,
            2.0,
            20,
            1.5,
            14,
            14,
            14,
            100,
        ))
    }
}

impl TryFrom<&StrategyConfig> for MeanReversionStrategy {
    type Error = Box<dyn std::error::Error>;
    fn try_from(_cfg: &StrategyConfig) -> Result<Self, Self::Error> {
        Ok(MeanReversionStrategy::new("SOL/USDC", TimeFrame::OneHour, 20, 2.0, 1.0, 1.0))
    }
}

impl TryFrom<&StrategyConfig> for TrendFollowingStrategy {
    type Error = Box<dyn std::error::Error>;
    fn try_from(_cfg: &StrategyConfig) -> Result<Self, Self::Error> {
        Ok(TrendFollowingStrategy::new(TrendFollowingConfig::new(
            "SOL/USDC",
            TimeFrame::OneHour,
            9,
            21,
            50,
            12,
            26,
            9,
            14,
            14,
            1.0,
            10.0,
            1.0,
        )))
    }
}

impl TryFrom<&StrategyConfig> for OrderFlowStrategy {
    type Error = Box<dyn std::error::Error>;
    fn try_from(_cfg: &StrategyConfig) -> Result<Self, Self::Error> {
        Ok(OrderFlowStrategy::new("SOL/USDC", TimeFrame::OneHour, 20, 0.5, 50, 1.0, 0.5))
    }
}

impl TryFrom<&StrategyConfig> for MemeArbitrageStrategy {
    type Error = Box<dyn std::error::Error>;
    fn try_from(_cfg: &StrategyConfig) -> Result<Self, Self::Error> {
        Ok(MemeArbitrageStrategy::new("SOL/USDC", TimeFrame::OneHour, 1.0, 0.5, 3))
    }
}

impl TryFrom<&StrategyConfig> for MomentumStrategy {
    type Error = Box<dyn std::error::Error>;
    fn try_from(_cfg: &StrategyConfig) -> Result<Self, Self::Error> {
        Ok(MomentumStrategy::new("SOL/USDC"))
    }
}
