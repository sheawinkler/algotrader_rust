//! Trading module – re-exports common types from `utils::types` so that strategy code can
//! depend on a single namespace (`crate::trading`).  These are thin wrappers around the
//! richer definitions in `utils::types`, with a few convenience helpers expected by the
//! strategies such as `Signal::metadata` and `OrderSide::is_buy/is_sell`.


// Re-export core domain types from utils::types
// so we do not maintain two divergent
// versions of every struct.
pub use crate::utils::types::{
    MarketData,
    SignalAction,
    TradingPair,
    Order,
    OrderSide,
    OrderType,
    OrderStatus,
    Trade,
    Position,
    MarketRegime,
};

// ---------- Convenience impls ----------
impl crate::utils::types::OrderSide {
    /// Returns true if the side is Buy
    pub fn is_buy(&self) -> bool { matches!(self, Self::Buy) }
    /// Returns true if the side is Sell
    pub fn is_sell(&self) -> bool { matches!(self, Self::Sell) }
}

// Additional helpful re-exports
// Re-export `BacktestResult` directly from utils so higher-level
// modules do not have to care where it is defined.
pub use crate::utils::types::BacktestResult;

use serde::{Deserialize, Serialize};

/// Type of trading signal produced by strategies.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SignalType {
    Buy,
    Sell,
    Close,
    Cancel,
    /// Arbitrage signal produced by MemeArbitrage strategy
    Arbitrage {
        buy_dex: String,
        sell_dex: String,
        spread_pct: f64,
    },
}

/// Light-weight signal struct that strategies interact with.
///
/// NOTE: This struct purposefully differs from the richer on-chain
/// `utils::types::Signal` used by the trading engine.  During order
/// execution these strategy signals will be converted into that
/// canonical representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    /// Symbol / trading pair as human-readable string, e.g. "SOL/USDC".
    pub symbol: String,
    pub signal_type: SignalType,
    pub price: f64,
    /// Position size in base currency units.
    pub size: f64,
        pub timestamp: i64,
    /// Confidence score 0-1.
    pub confidence: f64,
    /// Additional arbitrary metadata useful for debugging/analytics.
    pub metadata: Option<serde_json::Value>,
}

impl Signal {
    /// Helper: returns true if the signal represents a buy/long entry.
    pub fn is_buy(&self) -> bool { self.signal_type == SignalType::Buy }
    /// Helper: returns true if the signal represents a sell/short entry.
    pub fn is_sell(&self) -> bool { self.signal_type == SignalType::Sell }
}


// Simple order-book representations used by the Order Flow strategy.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BookLevel {
    pub price: f64,
    pub size: f64,
}

pub type OrderBookLevel = BookLevel;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Book {
    pub bids: Vec<BookLevel>,
    pub asks: Vec<BookLevel>,
}

pub type OrderBook = Book;

// --- end of trading module ---
















