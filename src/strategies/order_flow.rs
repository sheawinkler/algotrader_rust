use std::collections::{HashMap, VecDeque};
use std::time::SystemTime;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{TimeFrame, TradingStrategy};
use crate::trading::{
    MarketData, Order, OrderBook, OrderSide, OrderType, Position, Signal, SignalType,
};

/// Order Flow Strategy that analyzes market depth and order flow
#[derive(Debug, Clone)]
pub struct OrderFlowStrategy {
    // Strategy configuration
    symbol: String,
    timeframe: TimeFrame,

    // Order book analysis
    order_book_depth: usize,
    imbalance_threshold: f64,

    // Volume profile
    volume_profile: HashMap<u64, f64>, // Price level -> Volume
    vpoc_levels: Vec<f64>,             // Volume Point of Control levels

    // State
    position: Option<Position>,
    recent_imbalances: VecDeque<f64>,
    window_size: usize,

    // Risk management
    position_size_pct: f64,
    max_slippage_pct: f64,

    // Performance tracking
    trade_history: Vec<TradeRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TradeRecord {
    timestamp: SystemTime,
    price: f64,
    size: f64,
    side: OrderSide,
    pnl: Option<f64>,
    metadata: serde_json::Value,
}

impl OrderFlowStrategy {
    /// Create a new instance of OrderFlowStrategy
    pub fn new(
        symbol: &str, timeframe: TimeFrame, order_book_depth: usize, imbalance_threshold: f64,
        window_size: usize, position_size_pct: f64, max_slippage_pct: f64,
    ) -> Self {
        Self {
            symbol: symbol.to_string(),
            timeframe,
            order_book_depth,
            imbalance_threshold,
            volume_profile: HashMap::new(),
            vpoc_levels: Vec::new(),
            position: None,
            recent_imbalances: VecDeque::with_capacity(window_size * 2),
            window_size,
            position_size_pct: position_size_pct / 100.0,
            max_slippage_pct: max_slippage_pct / 100.0,
            trade_history: Vec::new(),
        }
    }

    /// Calculate order book imbalance
    fn calculate_order_imbalance(&self, order_book: &OrderBook) -> f64 {
        let top_bids = &order_book.bids[..self.order_book_depth.min(order_book.bids.len())];
        let top_asks = &order_book.asks[..self.order_book_depth.min(order_book.asks.len())];

        let bid_volume: f64 = top_bids.iter().map(|l| l.size).sum();
        let ask_volume: f64 = top_asks.iter().map(|l| l.size).sum();

        if bid_volume + ask_volume > 0.0 {
            (bid_volume - ask_volume) / (bid_volume + ask_volume)
        } else {
            0.0
        }
    }

    /// Update volume profile with new trade data
    fn update_volume_profile(&mut self, price: f64, volume: f64) {
        // Round price to nearest tick size (adjust based on market)
        let tick_size = 0.01; // Example tick size
        let price_level = (price / tick_size).round() as u64;

        *self.volume_profile.entry(price_level).or_insert(0.0) += volume;

        // Recalculate VPOC levels periodically
        if self.volume_profile.len() % 100 == 0 {
            self.calculate_vpoc_levels();
        }
    }

    /// Calculate Volume Point of Control (VPOC) levels
    fn calculate_vpoc_levels(&mut self) {
        // Find price levels with highest volume
        let mut sorted_levels: Vec<_> = self.volume_profile.iter().collect();
        sorted_levels.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());

        // Take top 3 VPOC levels
        self.vpoc_levels = sorted_levels
            .iter()
            .take(3)
            .map(|(price_level, _)| **price_level as f64 * 0.01) // Convert back from tick size
            .collect();
    }

    /// Detect large block trades or iceberg orders
    fn detect_large_orders(&self, order_book: &OrderBook) -> Option<(OrderSide, f64)> {
        // Look for large orders in the order book
        let large_bid = order_book.bids.iter()
            .find(|l| l.size > 1000.0) // Threshold for large order
            .map(|l| (OrderSide::Buy, l.price));

        let large_ask = order_book.asks.iter()
            .find(|l| l.size > 1000.0) // Threshold for large order
            .map(|l| (OrderSide::Sell, l.price));

        large_bid.or(large_ask)
    }

    /// Calculate position size based on order book liquidity
    fn calculate_position_size(&self, price: f64, account_balance: f64) -> f64 {
        let notional_size = account_balance * self.position_size_pct;
        let max_position = notional_size / price;

        // Adjust for slippage
        let max_slippage = price * self.max_slippage_pct;
        max_position.min(max_slippage * 10.0) // Example adjustment factor
    }
}

#[async_trait]
impl TradingStrategy for OrderFlowStrategy {
    fn name(&self) -> &str {
        "OrderFlowStrategy"
    }

    fn timeframe(&self) -> TimeFrame {
        self.timeframe
    }

    fn symbols(&self) -> Vec<String> {
        vec![self.symbol.clone()]
    }

    async fn generate_signals(&mut self, market_data: &MarketData) -> Vec<Signal> {
        // Update volume profile with new trade data
        self.update_volume_profile(market_data.close, market_data.volume.unwrap_or(0.0));

        // Get order book data (in a real implementation, this would come from the exchange)
        let order_book = match market_data.order_book.as_ref() {
            | Some(ob) => ob,
            | None => return Vec::new(),
        };

        // Calculate order book imbalance
        let imbalance = self.calculate_order_imbalance(order_book);
        self.recent_imbalances.push_back(imbalance);
        if self.recent_imbalances.len() > self.window_size {
            self.recent_imbalances.pop_front();
        }

        // Calculate moving average of imbalance
        let avg_imbalance: f64 = if !self.recent_imbalances.is_empty() {
            self.recent_imbalances.iter().sum::<f64>() / self.recent_imbalances.len() as f64
        } else {
            0.0
        };

        // Look for large orders
        let large_order = self.detect_large_orders(order_book);

        // Initialize signals vector
        let mut signals = Vec::new();

        // Generate signals based on order flow
        if let Some((side, price)) = large_order {
            // Large order detected - trade in the same direction (momentum)
            signals.push(Signal {
                symbol: self.symbol.clone(),
                signal_type: match side {
                    | OrderSide::Buy => SignalType::Buy,
                    | OrderSide::Sell => SignalType::Sell,
                },
                size: 0.0,
                price,
                order_type: OrderType::Market,
                limit_price: None,
                stop_price: None,
                timestamp: market_data.timestamp,
                confidence: 0.8,
                metadata: Some(serde_json::json!({
                    "strategy": "LargeOrderFlow",
                    "side": format!("{:?}", side),
                    "price": price,
                    "size": order_book.bids[0].size.max(order_book.asks[0].size),
                })),
            });
        } else if imbalance.abs() > self.imbalance_threshold {
            // Significant order book imbalance
            let signal_type = if imbalance > 0.0 {
                SignalType::Buy
            } else {
                SignalType::Sell
            };

            // Check if price is near a VPOC level (support/resistance)
            let near_vpoc = self
                .vpoc_levels
                .iter()
                .any(|&level| (market_data.close - level).abs() / level < 0.005); // Within 0.5%

            if near_vpoc {
                signals.push(Signal {
                    symbol: self.symbol.clone(),
                    signal_type,
                    size: 0.0,
                    price: market_data.close,
                    order_type: OrderType::Market,
                    limit_price: None,
                    stop_price: None,
                    timestamp: market_data.timestamp,
                    confidence: 0.7,
                    metadata: Some(serde_json::json!({
                        "strategy": "VPOCBounce",
                        "imbalance": imbalance,
                        "avg_imbalance": avg_imbalance,
                        "vpoc_levels": self.vpoc_levels,
                    })),
                });
            }
        }

        signals
    }

    fn on_order_filled(&mut self, order: &Order) {
        let trade_record = TradeRecord {
            timestamp: SystemTime::now(),
            price: order.price,
            size: order.size,
            side: order.side.clone(),
            pnl: None, // Will be filled when position is closed
            metadata: serde_json::json!({}),
        };

        match order.side {
            | OrderSide::Buy => {
                self.position = Some(Position {
                    id: String::new(),
                    symbol: order.symbol.clone(),
                    pair: crate::utils::types::TradingPair::from_str(&order.symbol)
                        .unwrap_or(crate::utils::types::TradingPair::new("BASE", "QUOTE")),
                    side: order.side,
                    size: order.size,
                    entry_price: Some(order.price),
                    current_price: order.price,
                    unrealized_pnl: 0.0,
                    realized_pnl: 0.0,
                    leverage: 1.0,
                    liquidation_price: None,
                    stop_loss: Some(order.price * 0.99),
                    take_profit: Some(order.price * 1.02),
                    timestamp: order.timestamp,
                });

                self.trade_history.push(trade_record);
            }
            | OrderSide::Sell => {
                if let Some(pos) = &self.position {
                    // Calculate PnL for closing trade
                    if let Some(entry_price) = pos.entry_price {
                        let pnl = if order.side == OrderSide::Buy {
                            (entry_price - order.price) * order.size // Short position
                        } else {
                            (order.price - entry_price) * order.size // Long position
                        };

                        if let Some(last_trade) = self.trade_history.last_mut() {
                            last_trade.pnl = Some(pnl);
                        }
                    }

                    if pos.size <= order.size {
                        self.position = None;
                    } else {
                        self.position.as_mut().unwrap().size -= order.size;
                    }
                }

                self.trade_history.push(trade_record);
            }
        }
    }

    fn get_positions(&self) -> Vec<&Position> {
        self.position.iter().collect()
    }
}

#[cfg(all(test, feature = "strategy_tests"))]
mod tests {
    use super::*;
    use std::time::SystemTime;

    fn create_test_order_book(
        bid_price: f64, ask_price: f64, bid_size: f64, ask_size: f64,
    ) -> OrderBook {
        OrderBook {
            bids: vec![OrderBookLevel { price: bid_price, size: bid_size }],
            asks: vec![OrderBookLevel { price: ask_price, size: ask_size }],
        }
    }

    #[tokio::test]
    async fn test_order_flow_strategy() {
        let mut strategy = OrderFlowStrategy::new(
            "SOL/USDC",
            TimeFrame::OneMinute,
            10,  // order_book_depth
            0.7, // imbalance_threshold
            20,  // window_size
            2.0, // position_size_pct
            0.1, // max_slippage_pct
        );

        // Create test market data with order book
        let mut market_data = MarketData {
            pair: TradingPair::new("SOL", "USDC"),
            timestamp: SystemTime::now(),
            open: Some(100.0),
            high: Some(101.0),
            low: Some(99.5),
            close: 100.5,
            volume: Some(1000.0),
            symbol: "SOL/USDC".to_string(),
            order_book: Some(create_test_order_book(100.0, 101.0, 500.0, 100.0)),
            ..Default::default()
        };

        // Test with significant buy imbalance
        let signals = strategy.generate_signals(&market_data).await;
        assert!(signals.is_empty()); // No signal yet, need more data

        // Update with large buy order
        market_data.order_book = Some(create_test_order_book(100.0, 101.0, 2000.0, 100.0));
        let signals = strategy.generate_signals(&market_data).await;

        // Should generate a buy signal due to large buy order
        assert!(!signals.is_empty());
        assert_eq!(signals[0].signal_type, SignalType::Buy);

        // Test order fill
        strategy.on_order_filled(&Order {
            id: "TEST_FILL".to_string(),
            symbol: "SOL/USDC".to_string(),
            side: OrderSide::Buy,
            size: 0.0,
            price: 100.5,
            order_type: OrderType::Market,
            timestamp: SystemTime::now(),
        });

        // Should have an open position
        assert!(strategy.position.is_some());

        // Test sell signal
        market_data.order_book = Some(create_test_order_book(99.0, 100.0, 100.0, 2000.0));
        let signals = strategy.generate_signals(&market_data).await;

        // Should generate a sell signal due to large sell order
        assert!(!signals.is_empty());
        assert_eq!(signals[0].signal_type, SignalType::Sell);
    }
}
