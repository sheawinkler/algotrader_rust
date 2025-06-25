use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::utils::indicators::{CachedIndicator, IndicatorValue, StochasticOscillator};
use async_trait::async_trait;
use ta::{
    indicators::{
        AverageTrueRange, BollingerBands, ExponentialMovingAverage, KeltnerChannel, MoneyFlowIndex,
        RelativeStrengthIndex,
    },
    DataItem, Next,
};
use tracing::debug;

use crate::trading::{MarketData, Order, OrderSide, OrderType, Position, Signal, SignalType};
use crate::utils::types::MarketRegime;

use super::{TimeFrame, TradingStrategy};

/// Advanced trading strategy that combines multiple indicators and adapts to market conditions
#[derive(Debug, Clone)]
pub struct AdvancedStrategy {
    // Strategy configuration
    symbol: String,
    timeframe: TimeFrame,

    // Technical indicators
    rsi: RelativeStrengthIndex,
    bb: BollingerBands,
    kc: KeltnerChannel,
    mfi: MoneyFlowIndex,
    stoch: StochasticOscillator,
    atr: CachedIndicator<AverageTrueRange>,
    fast_ema: CachedIndicator<ExponentialMovingAverage>,
    slow_ema: CachedIndicator<ExponentialMovingAverage>,

    // State
    last_signal: Option<Signal>,
    position: Option<Position>,
    recent_prices: VecDeque<f64>,
    window_size: usize,
    market_regime: MarketRegime,
    volatility: f64,
}

impl AdvancedStrategy {
    /// Create a new instance of AdvancedStrategy
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        symbol: &str, timeframe: TimeFrame, rsi_period: usize, bb_period: usize,
        bb_multiplier: f64, kc_period: usize, kc_multiplier: f64, mfi_period: usize,
        stoch_period: usize, atr_period: usize, window_size: usize,
    ) -> Self {
        Self {
            symbol: symbol.to_string(),
            timeframe,
            rsi: RelativeStrengthIndex::new(rsi_period).unwrap(),
            bb: BollingerBands::new(bb_period, bb_multiplier).unwrap(),
            kc: KeltnerChannel::new(kc_period, kc_multiplier).unwrap(),
            mfi: MoneyFlowIndex::new(mfi_period).unwrap(),
            stoch: StochasticOscillator::new(stoch_period, 3, 3),
            atr: CachedIndicator::new(AverageTrueRange::new(atr_period).unwrap()),
            fast_ema: CachedIndicator::new(ExponentialMovingAverage::new(9).unwrap()),
            slow_ema: CachedIndicator::new(ExponentialMovingAverage::new(21).unwrap()),
            last_signal: None,
            position: None,
            recent_prices: VecDeque::with_capacity(window_size * 2),
            window_size,
            market_regime: MarketRegime::Ranging,
            volatility: 0.0,
        }
    }

    /// Detect the current market regime based on price action and volatility
    fn detect_market_regime(&mut self, price: f64) -> MarketRegime {
        self.recent_prices.push_back(price);
        if self.recent_prices.len() > self.window_size {
            self.recent_prices.pop_front();
        }

        if self.recent_prices.len() < self.window_size {
            return MarketRegime::Ranging;
        }

        // Calculate simple trend and volatility
        let prices: Vec<f64> = self.recent_prices.iter().cloned().collect();
        let returns: Vec<f64> = prices.windows(2).map(|w| (w[1] - w[0]) / w[0]).collect();

        let mean_return: f64 = returns.iter().sum::<f64>() / returns.len() as f64;
        let variance: f64 = returns
            .iter()
            .map(|r| (r - mean_return).powi(2))
            .sum::<f64>()
            / returns.len() as f64;
        let std_dev = variance.sqrt();

        self.volatility = std_dev * (252.0f64).sqrt(); // Annualized volatility

        // Detect trend using EMAs
        let fast_ema = IndicatorValue::value(&self.fast_ema);
        let slow_ema = IndicatorValue::value(&self.slow_ema);

        // Simple regime detection
        if std_dev > 0.02 {
            // High volatility
            MarketRegime::Volatile
        } else if fast_ema > slow_ema * 1.005 {
            // Upward trend
            MarketRegime::TrendingUp
        } else if fast_ema < slow_ema * 0.995 {
            // Downward trend
            MarketRegime::TrendingDown
        } else {
            MarketRegime::Ranging
        }
    }
}

#[async_trait]
impl TradingStrategy for AdvancedStrategy {
    fn name(&self) -> &str {
        "AdvancedStrategy"
    }

    fn timeframe(&self) -> TimeFrame {
        self.timeframe
    }

    fn symbols(&self) -> Vec<String> {
        vec![self.symbol.clone()]
    }

    async fn generate_signals(&mut self, market_data: &MarketData) -> Vec<Signal> {
        // Update market regime detection
        self.market_regime = self.detect_market_regime(market_data.close);

        // Create a data item for the technical indicators
        let item = DataItem::builder()
            .high(market_data.high.unwrap_or(market_data.close))
            .low(market_data.low.unwrap_or(market_data.close))
            .close(market_data.close)
            .volume(market_data.volume.unwrap_or(0.0))
            .build()
            .expect("Failed to build data item");

        // Update all indicators
        let rsi = self.rsi.next(market_data.close);
        let bb = self.bb.next(market_data.close);
        let kc = self.kc.next(&item);
        let mfi = self.mfi.next(&item);
        let stoch = self.stoch.next(&item);
        let atr = self.atr.next(&item);
        // update global ATR cache
        crate::utils::atr_cache::update(&self.symbol, atr);
        let fast_ema = self.fast_ema.next(market_data.close);
        let slow_ema = self.slow_ema.next(market_data.close);

        // Initialize signals vector
        let mut signals = Vec::new();

        // Generate signals based on market regime
        match self.market_regime {
            | MarketRegime::TrendingUp => {
                // Look for pullback entries in uptrend
                if rsi < 40.0 && market_data.close > kc.average {
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
                            "strategy": "UptrendPullback",
                            "rsi": rsi,
                            "bb_width": bb.upper - bb.lower,
                            "kc_upper": kc.upper,
                            "kc_lower": kc.lower,
                            "atr": atr,
                        })),
                    });
                }
            }
            | MarketRegime::TrendingDown => {
                // Look for pullback shorts in downtrend
                if rsi > 60.0 && market_data.close < kc.average {
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
                            "strategy": "DowntrendPullback",
                            "rsi": rsi,
                            "bb_width": bb.upper - bb.lower,
                            "kc_upper": kc.upper,
                            "kc_lower": kc.lower,
                            "atr": atr,
                        })),
                    });
                }
            }
            | MarketRegime::Ranging => {
                // Mean reversion strategy
                if rsi < 30.0 && market_data.close < bb.lower {
                    signals.push(Signal {
                        symbol: self.symbol.clone(),
                        signal_type: SignalType::Buy,
                        size: 0.0,
                        price: market_data.close,
                        order_type: OrderType::Market,
                        limit_price: None,
                        stop_price: None,
                        timestamp: market_data.timestamp,
                        confidence: 0.6,
                        metadata: Some(serde_json::json!({
                            "strategy": "MeanReversionBuy",
                            "rsi": rsi,
                            "bb_lower": bb.lower,
                            "bb_middle": bb.average,
                            "atr": atr,
                        })),
                    });
                } else if rsi > 70.0 && market_data.close > bb.upper {
                    signals.push(Signal {
                        symbol: self.symbol.clone(),
                        signal_type: SignalType::Sell,
                        size: 0.0,
                        price: market_data.close,
                        order_type: OrderType::Market,
                        limit_price: None,
                        stop_price: None,
                        timestamp: market_data.timestamp,
                        confidence: 0.6,
                        metadata: Some(serde_json::json!({
                            "strategy": "MeanReversionSell",
                            "rsi": rsi,
                            "bb_upper": bb.upper,
                            "bb_middle": bb.average,
                            "atr": atr,
                        })),
                    });
                }
            }
            | MarketRegime::Volatile | MarketRegime::Unknown => {
                // Use tighter stops and take profits in volatile markets
                if stoch.k < 20.0 && stoch.d < 20.0 && stoch.k > stoch.d {
                    signals.push(Signal {
                        symbol: self.symbol.clone(),
                        signal_type: SignalType::Buy,
                        size: 0.0,
                        price: market_data.close,
                        order_type: OrderType::Market,
                        limit_price: None,
                        stop_price: None,
                        timestamp: market_data.timestamp,
                        confidence: 0.5,
                        metadata: Some(serde_json::json!({
                            "strategy": "VolatilityBuy",
                            "stoch_k": stoch.k,
                            "stoch_d": stoch.d,
                            "atr": atr,
                            "volatility": self.volatility,
                        })),
                    });
                } else if stoch.k > 80.0 && stoch.d > 80.0 && stoch.k < stoch.d {
                    signals.push(Signal {
                        symbol: self.symbol.clone(),
                        signal_type: SignalType::Sell,
                        size: 0.0,
                        price: market_data.close,
                        order_type: OrderType::Market,
                        limit_price: None,
                        stop_price: None,
                        timestamp: market_data.timestamp,
                        confidence: 0.5,
                        metadata: Some(serde_json::json!({
                            "strategy": "VolatilitySell",
                            "stoch_k": stoch.k,
                            "stoch_d": stoch.d,
                            "atr": atr,
                            "volatility": self.volatility,
                        })),
                    });
                }
            }
        }

        // Add volume confirmation
        if let Some(signal) = signals.last_mut() {
            if (mfi < 20.0 && signal.signal_type == SignalType::Buy)
                || (mfi > 80.0 && signal.signal_type == SignalType::Sell)
            {
                signal.confidence += 0.1;
            }
        }

        // Log the signals
        if let Some(signal) = signals.last() {
            debug!(
                "Generated signal: {:?} at {:.2} (confidence: {:.2})",
                signal.signal_type, signal.price, signal.confidence
            );
        }

        signals
    }

    fn on_order_filled(&mut self, order: &Order) {
        // Update position management based on filled orders
        match order.side {
            | OrderSide::Buy => {
                self.position = Some(Position {
                    symbol: order.symbol.clone(),
                    size: order.size,
                    entry_price: Some(order.price),
                    current_price: order.price,
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64,
                    stop_loss: Some(order.price * 0.99), // 1% stop loss
                    take_profit: Some(order.price * 1.02), // 2% take profit
                    side: order.side,
                    ..Default::default()
                });
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trading::{MarketData, TradingPair};
    use chrono::Utc;
    use std::time::{Duration, SystemTime};

    #[allow(clippy::field_reassign_with_default)]
    fn create_test_market_data(price: f64, volume: f64) -> MarketData {
        #[allow(clippy::field_reassign_with_default)]
        let mut md = MarketData::default();
        md.pair = TradingPair::new("TEST", "USD");
        md.symbol = "TEST/USD".to_string();
        md.timestamp = Utc::now().timestamp();
        md.open = Some(price * 0.99);
        md.high = Some(price * 1.01);
        md.low = Some(price * 0.99);
        md.close = price;
        md.last_price = price;
        md.volume = Some(volume);
        md
    }

    #[tokio::test]
    async fn test_advanced_strategy_initialization() {
        let strategy = AdvancedStrategy::new(
            "SOL/USDC",
            TimeFrame::OneHour,
            14,  // rsi_period
            20,  // bb_period
            2.0, // bb_multiplier
            20,  // kc_period
            1.5, // kc_multiplier
            14,  // mfi_period
            14,  // stoch_period
            14,  // atr_period
            50,  // window_size
        );

        assert_eq!(strategy.name(), "AdvancedStrategy");
        assert_eq!(strategy.symbols(), vec!["SOL/USDC"]);
        assert_eq!(strategy.timeframe(), TimeFrame::OneHour);
    }

    #[tokio::test]
    async fn test_market_regime_detection() {
        let mut strategy = AdvancedStrategy::new(
            "SOL/USDC",
            TimeFrame::OneHour,
            14,
            14,
            2.0,
            20,
            1.5,
            14,
            14,
            14,
            20,
        );

        // Generate some test data
        let mut price = 100.0;
        for _ in 0..50 {
            price += 0.5; // Upward trend
            let data = create_test_market_data(price, 1000.0);
            strategy.generate_signals(&data).await;
        }

        // Should detect uptrend
        assert_eq!(strategy.market_regime, MarketRegime::TrendingUp);

        // Generate ranging data
        for _ in 0..50 {
            price += if rand::random::<f64>() > 0.5 {
                0.1
            } else {
                -0.1
            };
            let data = create_test_market_data(price, 1000.0);
            strategy.generate_signals(&data).await;
        }

        // Should detect ranging
        assert_eq!(strategy.market_regime, MarketRegime::Ranging);
    }
}
