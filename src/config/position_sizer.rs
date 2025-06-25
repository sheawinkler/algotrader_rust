//! Position sizer configuration structs for serde deserialization.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PositionSizerConfig {
    FixedFractional { pct: f64 },
    Kelly { win_rate: f64, payoff_ratio: f64, cap: f64 },
    Volatility { risk_pct: f64, atr_mult: f64 },
    KellyLive { cap: f64 },
}
