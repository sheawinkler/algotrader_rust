//! Strategy registry providing convenient access to default strategy instances.
//! This is an early, minimal implementation. Over time this should evolve to load
//! strategies from configuration files or at runtime.

use crate::strategies::trend_following::TrendFollowingConfig;
use crate::strategies::TradingStrategyClone;
use crate::strategies::*;

/// Return a list of strategy instances with default parameters suitable for meta-selection.
/// For now we use hard-coded reasonable defaults.
pub fn default_strategies() -> Vec<Box<dyn TradingStrategyClone>> {
    use crate::strategies::TimeFrame;
    vec![
        Box::new(MeanReversionStrategy::new("UNK/UNK", TimeFrame::OneHour, 20, 2.0, 2.0, 1.0))
            as Box<dyn TradingStrategyClone>,
        Box::new(TrendFollowingStrategy::new(TrendFollowingConfig::new(
            "UNK/UNK",
            TimeFrame::OneHour,
            9,
            21,
            55, // EMAs
            12,
            26,
            9,    // MACD fast/slow/signal
            14,   // ADX period
            14,   // ATR period
            2.0,  // trailing stop %
            25.0, // max drawdown %
            10.0, // position size %
        ))) as Box<dyn TradingStrategyClone>,
        Box::new(MomentumStrategy::new("UNK/UNK")) as Box<dyn TradingStrategyClone>,
    ]
}
