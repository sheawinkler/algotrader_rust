//! Common types used throughout the trading system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a trading pair (e.g., SOL/USDC)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TradingPair {
    pub base: String,
    pub quote: String,
}

impl TradingPair {
    /// Create a new trading pair
    pub fn new(base: &str, quote: &str) -> Self {
        Self {
            base: base.to_uppercase(),
            quote: quote.to_uppercase(),
        }
    }
    
    /// Parse a trading pair from a string (e.g., "SOL/USDC")
    pub fn from_str(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() == 2 {
            Some(Self::new(parts[0], parts[1]))
        } else {
            None
        }
    }
    
    /// Convert to string representation (e.g., "SOL/USDC")
    pub fn to_string(&self) -> String {
        format!("{}/{}", self.base, self.quote)
    }
}

impl Default for TradingPair {
    fn default() -> Self {
        Self::new("", "")
    }
}

impl std::fmt::Display for TradingPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.base, self.quote)
    }
}

/// Represents an order in the order book
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    /// Human readable symbol, e.g. "SOL/USDC" – some strategies rely on this convenience field.
    pub symbol: String,
    pub price: f64,
    pub size: f64,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub timestamp: i64,
}

impl Default for Order {
    fn default() -> Self {
        Self {
            id: String::new(),
            symbol: String::new(),
            price: 0.0,
            size: 0.0,
            side: OrderSide::Buy,
            order_type: OrderType::Market,
            timestamp: 0,
        }
    }
}

/// Side of an order (buy or sell)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Type of an order (market, limit, etc.)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderType {
    Market,
    Limit,
    Stop,
    StopLimit,
}

/// Status of an order
/// Represents a pending order waiting for stop/limit trigger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingOrder {
    pub pair: TradingPair,
    pub amount: f64,
    pub is_buy: bool,
    pub order_type: OrderType,
    pub limit_price: Option<f64>,
    pub stop_price: Option<f64>,
    pub wallet: String,
    pub dex_preference: Vec<String>,
    pub timestamp: i64,
}

pub enum OrderStatus {
    New,
    PartiallyFilled,
    Filled,
    Canceled,
    Rejected,
    Expired,
}

/// Represents a trade that was executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub id: String,
    pub order_id: String,
    pub price: f64,
    pub size: f64,
    pub side: OrderSide,
    pub fee: f64,
    pub fee_currency: String,
    pub timestamp: i64,
}

/// Represents a position in the market
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub id: String,
    /// Human readable symbol for quick lookups (e.g. "SOL/USDC")
    pub symbol: String,
    pub pair: TradingPair,
    pub side: OrderSide,
    pub size: f64,
    pub entry_price: Option<f64>,
    pub current_price: f64,
    pub unrealized_pnl: f64,
    pub realized_pnl: f64,
    pub leverage: f64,
    pub liquidation_price: Option<f64>,
    /// Optional stop loss price
    pub stop_loss: Option<f64>,
    /// Optional take profit price
    pub take_profit: Option<f64>,
    pub timestamp: i64,
}

impl Default for Position {
    fn default() -> Self {
        Self {
            id: String::new(),
            symbol: String::new(),
            pair: TradingPair::new("UNK", "UNK"),
            side: OrderSide::Buy,
            size: 0.0,
            entry_price: None,
            current_price: 0.0,
            unrealized_pnl: 0.0,
            realized_pnl: 0.0,
            leverage: 1.0,
            liquidation_price: None,
            stop_loss: None,
            take_profit: None,
            timestamp: 0,
        }
    }
}

/// Represents the current account balance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub free: f64,
    pub used: f64,
    pub total: f64,
    pub currency: String,
}

/// Represents a candlestick (OHLCV) data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    pub timestamp: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

/// Represents market data for a trading pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketData {
    pub pair: TradingPair,
    /// Convenience symbol string (e.g. "SOL/USDC") used by some strategies
    pub symbol: String,
    pub candles: Vec<Candle>,
    pub last_price: f64,
    pub volume_24h: f64,
    pub change_24h: f64,
    /// Optional last trade volume for latest tick
    pub volume: Option<f64>,
    pub timestamp: i64,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub close: f64,
    /// Optional L2 order book snapshot used by Order Flow strategy
    pub order_book: Option<crate::trading::OrderBook>,
    /// Optional DEX price map used by Meme Arbitrage strategy
    pub dex_prices: Option<HashMap<String, f64>>,
}

impl Default for MarketData {
    fn default() -> Self {
        Self {
            pair: TradingPair::new("", ""),
            symbol: String::new(),
            candles: Vec::new(),
            last_price: 0.0,
            volume_24h: 0.0,
            change_24h: 0.0,
            volume: None,
            timestamp: 0,
            open: None,
            high: None,
            low: None,
            close: 0.0,
            order_book: None,
            dex_prices: None,
        }
    }
}

/// Market regime for analytics and strategy adaptation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum MarketRegime {
    TrendingUp,
    TrendingDown,
    Ranging,
    Volatile,
    Unknown,
}

/// Represents a trading signal generated by a strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    pub strategy_id: String,
    pub pair: TradingPair,
    pub action: SignalAction,
    pub price: f64,
    pub size: f64,
    pub order_type: OrderType,
    pub limit_price: Option<f64>,
    pub stop_price: Option<f64>,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
    pub timestamp: i64,
    pub metadata: HashMap<String, String>,
}

/// Action to take based on a trading signal
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SignalAction {
    Buy,
    Sell,
    Close,
    Cancel,
}

/// Represents the result of a backtest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestResult {
    pub strategy_id: String,
    pub pair: TradingPair,
    pub start_time: i64,
    pub end_time: i64,
    pub initial_balance: f64,
    pub final_balance: f64,
    pub total_return: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown: f64,
    pub win_rate: f64,
    pub total_trades: usize,
    pub winning_trades: usize,
    pub losing_trades: usize,
    pub trades: Vec<Trade>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_trading_pair() {
        let pair = TradingPair::new("sol", "usdc");
        assert_eq!(pair.base, "SOL");
        assert_eq!(pair.quote, "USDC");
        
        let pair_str = "BTC/USDT".to_string();
        let pair = TradingPair::from_str(&pair_str).unwrap();
        assert_eq!(pair.base, "BTC");
        assert_eq!(pair.quote, "USDT");
        assert_eq!(pair.to_string(), "BTC/USDT");
        
        let invalid = TradingPair::from_str("invalid");
        assert!(invalid.is_none());
    }
    
    #[test]
    fn test_candle() {
        let candle = Candle {
            timestamp: 1_234_567_890,
            open: 100.0,
            high: 105.0,
            low: 95.0,
            close: 102.5,
            volume: 1000.0,
        };
        
        assert_eq!(candle.timestamp, 1_234_567_890);
        assert_eq!(candle.open, 100.0);
        assert_eq!(candle.high, 105.0);
        assert_eq!(candle.low, 95.0);
        assert_eq!(candle.close, 102.5);
        assert_eq!(candle.volume, 1000.0);
    }
    
    #[test]
    fn test_signal() {
        let pair = TradingPair::new("SOL", "USDC");
        let mut metadata = HashMap::new();
        metadata.insert("confidence".to_string(), "0.85".to_string());
        
        let signal = Signal {
            strategy_id: "mean_reversion".to_string(),
            pair: pair.clone(),
            action: SignalAction::Buy,
            price: 100.0,
            size: 1.0,
            order_type: OrderType::Market,
            limit_price: None,
            stop_price: None,
        stop_loss: Some(95.0),
            take_profit: Some(110.0),
            timestamp: 1_234_567_890,
            metadata: metadata.clone(),
        };
        
        assert_eq!(signal.strategy_id, "mean_reversion");
        assert_eq!(signal.pair, pair);
        assert_eq!(signal.action, SignalAction::Buy);
        assert_eq!(signal.price, 100.0);
        assert_eq!(signal.size, 1.0);
        assert_eq!(signal.stop_loss, Some(95.0));
        assert_eq!(signal.take_profit, Some(110.0));
        assert_eq!(signal.timestamp, 1_234_567_890);
        assert_eq!(signal.metadata.get("confidence"), Some(&"0.85".to_string()));
    }
}
