use crate::utils::indicators::AverageDirectionalIndex;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use ta::{
    indicators::{
        ExponentialMovingAverage, MovingAverageConvergenceDivergence, PercentagePriceOscillator,
        SimpleMovingAverage,
    },
    Next,
};
use tracing::debug;

use super::{TimeFrame, TradingStrategy};
use crate::trading::{MarketData, Order, OrderSide, OrderType, Position, Signal, SignalType};
use crate::utils::indicators::{CachedIndicator, IndicatorValue};

/// Trend Following Strategy that identifies and rides market trends
#[derive(Debug, Clone)]
pub struct TrendFollowingStrategy {
    // Strategy configuration
    symbol: String,
    timeframe: TimeFrame,

    // Technical indicators
    fast_ema: CachedIndicator<ExponentialMovingAverage>,
    medium_ema: CachedIndicator<ExponentialMovingAverage>,
    slow_ema: CachedIndicator<ExponentialMovingAverage>,
    macd: MovingAverageConvergenceDivergence,
    ppc: PercentagePriceOscillator,
    adx: AverageDirectionalIndex,
    atr: SimpleMovingAverage,

    // State
    position: Option<Position>,
    trend_direction: TrendDirection,

    // Risk management
    trailing_stop_pct: f64,
    max_drawdown_pct: f64,
    position_size_pct: f64,

    // Performance tracking
    peak_equity: f64,
    current_drawdown: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
enum TrendDirection {
    Uptrend,
    Downtrend,
    Sideways,
}

impl TrendFollowingStrategy {
    /// Create a new instance of TrendFollowingStrategy
    pub fn new(
        symbol: &str, timeframe: TimeFrame, fast_ema_period: usize, medium_ema_period: usize,
        slow_ema_period: usize, macd_fast: u8, macd_slow: u16, macd_signal: u8, adx_period: u8,
        atr_period: u8, trailing_stop_pct: f64, max_drawdown_pct: f64, position_size_pct: f64,
    ) -> Self {
        Self {
            symbol: symbol.to_string(),
            timeframe,
            fast_ema: CachedIndicator::new(ExponentialMovingAverage::new(fast_ema_period).unwrap()),
            medium_ema: CachedIndicator::new(
                ExponentialMovingAverage::new(medium_ema_period).unwrap(),
            ),
            slow_ema: CachedIndicator::new(ExponentialMovingAverage::new(slow_ema_period).unwrap()),
            macd: MovingAverageConvergenceDivergence::new(
                macd_fast as usize,
                macd_slow as usize,
                macd_signal as usize,
            )
            .unwrap(),
            ppc: PercentagePriceOscillator::new(
                fast_ema_period,
                slow_ema_period,
                macd_signal as usize,
            )
            .unwrap(),
            adx: AverageDirectionalIndex::new(adx_period as usize),
            atr: SimpleMovingAverage::new(atr_period as usize).unwrap(),
            position: None,
            trend_direction: TrendDirection::Sideways,
            trailing_stop_pct: trailing_stop_pct / 100.0,
            max_drawdown_pct: max_drawdown_pct / 100.0,
            position_size_pct: position_size_pct / 100.0,
            peak_equity: 1.0,
            current_drawdown: 0.0,
        }
    }

    /// Determine the current trend direction
    fn update_trend_direction(&mut self, price: f64) {
        let fast_ema = IndicatorValue::value(&self.fast_ema);
        let medium_ema = IndicatorValue::value(&self.medium_ema);
        let slow_ema = IndicatorValue::value(&self.slow_ema);
        let macd_line = IndicatorValue::value(&self.macd);
        let macd_signal = IndicatorValue::value(&self.macd);
        let adx = IndicatorValue::value(&self.adx);

        // Strong uptrend conditions
        let strong_uptrend =
            fast_ema > medium_ema && medium_ema > slow_ema && macd_line > macd_signal && adx > 25.0;

        // Strong downtrend conditions
        let strong_downtrend =
            fast_ema < medium_ema && medium_ema < slow_ema && macd_line < macd_signal && adx > 25.0;

        self.trend_direction = if strong_uptrend {
            TrendDirection::Uptrend
        } else if strong_downtrend {
            TrendDirection::Downtrend
        } else {
            TrendDirection::Sideways
        };
    }

    /// Check if we should exit a position due to stop loss or trend reversal
    fn should_exit_position(&mut self, price: f64, position: &Position) -> bool {
        if let (Some(entry_price), Some(stop_loss)) = (position.entry_price, position.stop_loss) {
            // Check trailing stop
            if price <= stop_loss {
                debug!("Trailing stop hit at {:.2}", price);
                return true;
            }

            // Check max drawdown
            let current_equity = position.size * price;
            self.peak_equity = self.peak_equity.max(current_equity);
            self.current_drawdown = (self.peak_equity - current_equity) / self.peak_equity;

            if self.current_drawdown > self.max_drawdown_pct {
                debug!("Max drawdown hit: {:.2}%", self.current_drawdown * 100.0);
                return true;
            }

            // Check for trend reversal
            match self.trend_direction {
                | TrendDirection::Uptrend if price < IndicatorValue::value(&self.medium_ema) => {
                    debug!("Uptrend broken");
                    return true;
                }
                | TrendDirection::Downtrend if price > IndicatorValue::value(&self.medium_ema) => {
                    debug!("Downtrend broken");
                    return true;
                }
                | _ => {}
            }
        }

        false
    }
}

#[async_trait]
impl TradingStrategy for TrendFollowingStrategy {
    fn name(&self) -> &str {
        "TrendFollowingStrategy"
    }

    fn timeframe(&self) -> TimeFrame {
        self.timeframe
    }

    fn symbols(&self) -> Vec<String> {
        vec![self.symbol.clone()]
    }

    async fn generate_signals(&mut self, market_data: &MarketData) -> Vec<Signal> {
        // Update indicators
        let _ = self.fast_ema.next(market_data.close);
        let _ = self.medium_ema.next(market_data.close);
        let _ = self.slow_ema.next(market_data.close);
        let _ = self.macd.next(market_data.close);
        let _ = self.ppc.next(market_data.close);
        let _ = self.adx.next(market_data.close);
        let atr_val = self.atr.next(
            market_data.high.unwrap_or(market_data.close)
                - market_data.low.unwrap_or(market_data.close),
        );
        // Publish ATR to global cache so position sizer can access
        crate::utils::atr_cache::update(&self.symbol, atr_val);

        // Update trend direction
        self.update_trend_direction(market_data.close);

        // Initialize signals vector
        let mut signals = Vec::new();

        // Determine if we should exit an existing position without violating borrow rules
        let exit_now = if let Some(pos_clone) = self.position.clone() {
            self.should_exit_position(market_data.close, &pos_clone)
        } else {
            false
        };

        if exit_now {
            signals.push(Signal {
                symbol: self.symbol.clone(),
                signal_type: SignalType::Sell,
                size: 0.0,
                price: market_data.close,
                order_type: OrderType::Market,
                limit_price: None,
                stop_price: None,
                timestamp: market_data.timestamp,
                confidence: 0.8,
                metadata: Some(serde_json::json!({
                    "strategy": "TrendFollowingExit",
                    "trend_direction": format!("{:?}", self.trend_direction),
                    "adx": IndicatorValue::value(&self.adx),
                    "atr": atr_val,
                })),
            });
        } else {
            // Generate entry signals
            match self.trend_direction {
                | TrendDirection::Uptrend => {
                    // Look for pullback to fast or medium EMA for entry
                    if market_data.close > IndicatorValue::value(&self.fast_ema)
                        && market_data.low.unwrap_or(market_data.close)
                            <= IndicatorValue::value(&self.fast_ema)
                    {
                        signals.push(Signal {
                            symbol: self.symbol.clone(),
                            signal_type: SignalType::Buy,
                            size: 0.0,
                            price: market_data.close,
                            order_type: OrderType::Market,
                            limit_price: None,
                            stop_price: None,
                            timestamp: market_data.timestamp,
                            confidence: 0.7,
                            metadata: Some(serde_json::json!({
                                "strategy": "TrendFollowingLong",
                                "trend_direction": "Uptrend",
                                "adx": IndicatorValue::value(&self.adx),
                                "atr": atr_val,
                                "fast_ema": IndicatorValue::value(&self.fast_ema),
                                "medium_ema": IndicatorValue::value(&self.medium_ema),
                            })),
                        });
                    }
                }
                | TrendDirection::Downtrend => {
                    // Look for pullback to fast or medium EMA for short entry
                    if market_data.close < IndicatorValue::value(&self.fast_ema)
                        && market_data.high.unwrap_or(market_data.close)
                            >= IndicatorValue::value(&self.fast_ema)
                    {
                        signals.push(Signal {
                            symbol: self.symbol.clone(),
                            signal_type: SignalType::Sell,
                            size: 0.0,
                            price: market_data.close,
                            order_type: OrderType::Market,
                            limit_price: None,
                            stop_price: None,
                            timestamp: market_data.timestamp,
                            confidence: 0.7,
                            metadata: Some(serde_json::json!({
                                "strategy": "TrendFollowingShort",
                                "trend_direction": "Downtrend",
                                "adx": IndicatorValue::value(&self.adx),
                                "atr": atr_val,
                                "fast_ema": IndicatorValue::value(&self.fast_ema),
                                "medium_ema": IndicatorValue::value(&self.medium_ema),
                            })),
                        });
                    }
                }
                | TrendDirection::Sideways => {
                    // No trades in sideways markets
                }
            }
        }

        signals
    }

    fn on_order_filled(&mut self, order: &Order) {
        match order.side {
            | OrderSide::Buy => {
                let atr = IndicatorValue::value(&self.atr);
                let stop_loss = order.price - (atr * 2.0); // 2x ATR stop loss

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
                    stop_loss: Some(stop_loss),
                    take_profit: None,
                    timestamp: order.timestamp,
                });

                // Reset equity tracking for new position
                // reset equity tracking
                self.peak_equity = order.size * order.price;
                self.current_drawdown = 0.0;
            }
            | OrderSide::Sell => {
                if let Some(pos) = &self.position {
                    if pos.size <= order.size {
                        self.position = None;
                    } else {
                        self.position.as_mut().unwrap().size -= order.size;
                    }
                }
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
    use std::time::{Duration, SystemTime};

    fn create_test_market_data(open: f64, high: f64, low: f64, close: f64) -> MarketData {
        MarketData {
            timestamp: SystemTime::now(),
            open,
            high,
            low,
            close,
            volume: 1000.0,
            symbol: "TEST".to_string(),
        }
    }

    #[tokio::test]
    async fn test_trend_following_strategy() {
        let mut strategy = TrendFollowingStrategy::new(
            "SOL/USDC",
            TimeFrame::OneHour,
            9,   // fast_ema_period
            21,  // medium_ema_period
            50,  // slow_ema_period
            12,  // macd_fast
            26,  // macd_slow
            9,   // macd_signal
            14,  // adx_period
            14,  // atr_period
            1.0, // trailing_stop_pct
            5.0, // max_drawdown_pct
            2.0, // position_size_pct
        );

        // Generate test data with an uptrend
        let mut price = 100.0;
        for i in 0..100 {
            let open = price;
            let close = price * 1.01; // 1% increase
            let high = close * 1.005;
            let low = open * 0.995;

            let data = create_test_market_data(open, high, low, close);
            let signals = strategy.generate_signals(&data).await;

            // After enough data points, we should get a buy signal
            if i == 60 {
                // Give indicators time to warm up
                assert!(!signals.is_empty());
                assert_eq!(signals[0].signal_type, SignalType::Buy);

                // Simulate order fill
                strategy.on_order_filled(&Order {
                    id: "TEST".to_string(),
                    symbol: "SOL/USDC".to_string(),
                    side: OrderSide::Buy,
                    size: 0.0,
                    price: close,
                    order_type: OrderType::Market,
                    order_type: OrderType::Market,
                    timestamp: SystemTime::now(),
                });
            }

            price = close;
        }

        // Generate a reversal
        for _ in 0..20 {
            let open = price;
            let close = price * 0.98; // 2% decrease
            let high = open * 1.01;
            let low = close * 0.99;

            let data = create_test_market_data(open, high, low, close);
            let signals = strategy.generate_signals(&data).await;

            price = close;

            // After the trend breaks, we should get a sell signal
            if signals.iter().any(|s| s.signal_type == SignalType::Sell) {
                return; // Test passed
            }
        }

        panic!("Failed to generate sell signal on trend reversal");
    }
}
