use std::collections::VecDeque;

use async_trait::async_trait;
use ta::{
    indicators::{BollingerBands, ExponentialMovingAverage, RelativeStrengthIndex, StandardDeviation},
    Next,
};
use tracing::debug;

use crate::trading::{MarketData, Signal, SignalType, Position, Order, OrderSide};
use crate::utils::indicator_ext::IndicatorValue;
use super::{TradingStrategy, TimeFrame};

/// Mean Reversion Strategy that identifies overbought/oversold conditions
#[derive(Debug, Clone)]
pub struct MeanReversionStrategy {
    // Strategy configuration
    symbol: String,
    timeframe: TimeFrame,
    
    // Technical indicators
    rsi: RelativeStrengthIndex,
    bb: BollingerBands,
    ema: ExponentialMovingAverage,
    std_dev: StandardDeviation,
    
    // State
    position: Option<Position>,
    /// Fixed position size in units (can be parameterized later)
    position_size: f64,
    recent_prices: VecDeque<f64>,
    lookback_period: usize,
    zscore_threshold: f64,
    
    // Risk management
    take_profit_pct: f64,
    stop_loss_pct: f64,
}

impl MeanReversionStrategy {
    /// Create a new instance of MeanReversionStrategy
    pub fn new(
        symbol: &str,
        timeframe: TimeFrame,
        lookback_period: usize,
        zscore_threshold: f64,
        take_profit_pct: f64,
        stop_loss_pct: f64,
    ) -> Self {
        Self {
            symbol: symbol.to_string(),
            timeframe,
            rsi: RelativeStrengthIndex::new(14).unwrap(),
            bb: BollingerBands::new(20, 2.0).unwrap(),
            ema: ExponentialMovingAverage::new(lookback_period).unwrap(),
            std_dev: StandardDeviation::new(lookback_period).unwrap(),
            position: None,
            recent_prices: VecDeque::with_capacity(lookback_period * 2),
            lookback_period,
            zscore_threshold,
            take_profit_pct: take_profit_pct / 100.0, // Convert from percentage to decimal
            stop_loss_pct: stop_loss_pct / 100.0,
            position_size: 1.0,
        }
    }
    
    /// Calculate the Z-Score of the current price
    fn calculate_zscore(&self, price: f64) -> f64 {
        let mean = IndicatorValue::value(&self.ema);
        let std = IndicatorValue::value(&self.std_dev).max(0.0001); // Avoid division by zero
        (price - mean) / std
    }
    
    /// Calculate position size based on Z-Score and account balance
    fn calculate_position_size(&self, price: f64, account_balance: f64, risk_percent: f64) -> f64 {
        let zscore = self.calculate_zscore(price).abs();
        let risk_amount = account_balance * (risk_percent / 100.0);
        let position_size = risk_amount * zscore / price;
        position_size.max(0.0)
    }
    
    /// Check if we should exit a position
    fn should_exit_position(&self, price: f64, position: &Position) -> bool {
        if let Some(entry_price) = position.entry_price {
            let pnl_pct = (price - entry_price) / entry_price;
            
            // Check take profit
            if pnl_pct >= self.take_profit_pct {
                debug!("Take profit hit at {:.2}%", pnl_pct * 100.0);
                return true;
            }
            
            // Check stop loss
            if pnl_pct <= -self.stop_loss_pct {
                debug!("Stop loss hit at {:.2}%", pnl_pct * 100.0);
                return true;
            }
            
            // Check if mean reversion is complete
            let zscore = self.calculate_zscore(price);
            if zscore.abs() < self.zscore_threshold * 0.5 {
                debug!("Mean reversion complete, z-score: {:.2}", zscore);
                return true;
            }
        }
        
        false
    }
}

#[async_trait]
impl TradingStrategy for MeanReversionStrategy {
    fn name(&self) -> &str {
        "MeanReversionStrategy"
    }
    
    fn timeframe(&self) -> TimeFrame {
        self.timeframe
    }
    
    fn symbols(&self) -> Vec<String> {
        vec![self.symbol.clone()]
    }
    
    async fn generate_signals(&mut self, market_data: &MarketData) -> Vec<Signal> {
        // Update indicators
        let _ = self.rsi.next(market_data.close);
        let bb = self.bb.next(market_data.close);
        let _ = self.ema.next(market_data.close);
        let _ = self.std_dev.next(market_data.close);
        
        // Track recent prices for volatility calculation
        self.recent_prices.push_back(market_data.close);
        if self.recent_prices.len() > self.lookback_period * 2 {
            self.recent_prices.pop_front();
        }
        
        // Initialize signals vector
        let mut signals = Vec::new();
        
        // Calculate Z-Score and other metrics
        let zscore = self.calculate_zscore(market_data.close);
        let rsi = IndicatorValue::value(&self.rsi);
        
        // Check for existing position
        if let Some(position) = &self.position {
            if self.should_exit_position(market_data.close, position) {
                signals.push(Signal {
                    symbol: self.symbol.clone(),
                    signal_type: SignalType::Sell,
                    size: self.position_size,
                    price: market_data.close,
                    timestamp: market_data.timestamp,
                    confidence: 0.8,
                    metadata: Some(serde_json::json!({
                        "strategy": "MeanReversionExit",
                        "zscore": zscore,
                        "rsi": rsi,
                        "price": market_data.close,
                    })),
                });
            }
        } else if self.recent_prices.len() >= self.lookback_period {
            // Generate entry signals only if we have enough data
            
            // Oversold condition (long entry)
            if zscore < -self.zscore_threshold && rsi < 30.0 && market_data.close < bb.lower {
                signals.push(Signal {
                    symbol: self.symbol.clone(),
                    signal_type: SignalType::Buy,
                    size: self.position_size,
                    price: market_data.close,
                    timestamp: market_data.timestamp,
                    confidence: 0.7,
                    metadata: Some(serde_json::json!({
                        "strategy": "MeanReversionLong",
                        "zscore": zscore,
                        "rsi": rsi,
                        "bb_lower": bb.lower,
                        "bb_middle": bb.average,
                    })),
                });
            } 
            // Overbought condition (short entry)
            else if zscore > self.zscore_threshold && rsi > 70.0 && market_data.close > bb.upper {
                signals.push(Signal {
                    symbol: self.symbol.clone(),
                    signal_type: SignalType::Sell,
                    size: self.position_size,
                    price: market_data.close,
                    timestamp: market_data.timestamp,
                    confidence: 0.7,
                    metadata: Some(serde_json::json!({
                        "strategy": "MeanReversionShort",
                        "zscore": zscore,
                        "rsi": rsi,
                        "bb_upper": bb.upper,
                        "bb_middle": bb.average,
                    })),
                });
            }
        }
        
        signals
    }
    
    fn on_order_filled(&mut self, order: &Order) {
        match order.side {
            OrderSide::Buy => {
                self.position = Some(Position {
                    id: String::new(),
                    symbol: order.symbol.clone(),
                    pair: crate::utils::types::TradingPair::from_str(&order.symbol).unwrap_or(crate::utils::types::TradingPair::new("BASE","QUOTE")),
                    side: order.side,
                    size: order.size,
                    entry_price: Some(order.price),
                    current_price: order.price,
                    unrealized_pnl: 0.0,
                    realized_pnl: 0.0,
                    leverage: 1.0,
                    liquidation_price: None,
                    stop_loss: Some(order.price * (1.0 - self.stop_loss_pct)),
                    take_profit: Some(order.price * (1.0 + self.take_profit_pct)),
                    timestamp: order.timestamp,
                });

            },
            OrderSide::Sell => {
                if let Some(pos) = &self.position {
                    if pos.size <= order.size {
                        self.position = None;
                    } else {
                        self.position.as_mut().unwrap().size -= order.size;
                    }
                }
            },
        }
    }
    
    fn get_positions(&self) -> Vec<&Position> {
        self.position.iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, Duration};
    
    fn create_test_market_data(price: f64) -> MarketData {
        MarketData {
            timestamp: SystemTime::now(),
            open: price,
            high: price * 1.01,
            low: price * 0.99,
            close: price,
            volume: 1000.0,
            symbol: "TEST".to_string(),
        }
    }
    
    #[tokio::test]
    async fn test_mean_reversion_strategy() {
        let mut strategy = MeanReversionStrategy::new(
            "SOL/USDC",
            TimeFrame::OneHour,
            20,   // lookback_period
            2.0,  // zscore_threshold
            2.0,  // take_profit_pct
            1.0,  // stop_loss_pct
        );
        
        // Generate test data with a strong uptrend
        let mut price = 100.0;
        for _ in 0..50 {
            price *= 1.01; // 1% increase per period
            let data = create_test_market_data(price);
            let signals = strategy.generate_signals(&data).await;
            assert!(signals.is_empty()); // No signals during strong trend
        }
        
        // Generate a sharp drop (oversold condition)
        price *= 0.9; // 10% drop
        let data = create_test_market_data(price);
        let signals = strategy.generate_signals(&data).await;
        
        // Should generate a buy signal
        assert!(!signals.is_empty());
        assert_eq!(signals[0].signal_type, SignalType::Buy);
        
        // Simulate order fill
        strategy.on_order_filled(&Order {
            symbol: "SOL/USDC".to_string(),
            side: OrderSide::Buy,
            size: 1.0,
            price,
            order_type: OrderType::Market,
            timestamp: SystemTime::now(),
        });
        
        // Generate more data with price moving up
        for _ in 0..10 {
            price *= 1.001; // Small increase
            let data = create_test_market_data(price);
            let signals = strategy.generate_signals(&data).await;
            assert!(signals.is_empty()); // No exit signals yet
        }
        
        // Price hits take profit
        price = price * 1.03; // Above take profit threshold
        let data = create_test_market_data(price);
        let signals = strategy.generate_signals(&data).await;
        
        // Should generate a sell signal to take profit
        assert!(!signals.is_empty());
        assert_eq!(signals[0].signal_type, SignalType::Sell);
    }
}
