//! Momentum trading strategy adapted to the richer `strategies` trait.

use async_trait::async_trait;
use ta::indicators::{ExponentialMovingAverage, RelativeStrengthIndex};
use ta::Next;

use crate::trading::{MarketData, Signal, SignalType};
use crate::utils::types::TradingPair;

use crate::utils::types::{Position, Order};
use super::{TradingStrategy, TimeFrame};

/// Momentum trading strategy based on EMA crossover and RSI.
#[derive(Debug, Clone)]
pub struct MomentumStrategy {
    pair: TradingPair,
    ema_short_period: usize,
    ema_long_period: usize,
    rsi_period: usize,
    rsi_overbought: f64,
    rsi_oversold: f64,
    position_size: f64,
    ema_short: Option<ExponentialMovingAverage>,
    ema_long: Option<ExponentialMovingAverage>,
    rsi: Option<RelativeStrengthIndex>,
    positions: Vec<Position>,
}

impl MomentumStrategy {
    pub fn new(pair: &str) -> Self {
        Self {
            pair: TradingPair::from_str(pair).expect("invalid pair"),
            ema_short_period: 9,
            ema_long_period: 21,
            rsi_period: 14,
            rsi_overbought: 70.0,
            rsi_oversold: 30.0,
            position_size: 1.0,
            ema_short: None,
            ema_long: None,
            rsi: None,
            positions: Vec::new(),
        }
    }

    /// Initialise indicators with historical prices
    fn warm_up(&mut self, prices: &[f64]) {
        let mut ema_s = ExponentialMovingAverage::new(self.ema_short_period).unwrap();
        let mut ema_l = ExponentialMovingAverage::new(self.ema_long_period).unwrap();
        let mut rsi = RelativeStrengthIndex::new(self.rsi_period).unwrap();
        for &p in prices {
            ema_s.next(p);
            ema_l.next(p);
            rsi.next(p);
        }
        self.ema_short = Some(ema_s);
        self.ema_long = Some(ema_l);
        self.rsi = Some(rsi);
    }
}

#[async_trait]
impl TradingStrategy for MomentumStrategy {
    fn name(&self) -> &str {
        "Momentum"
    }

    fn timeframe(&self) -> TimeFrame {
        TimeFrame::OneHour
    }

    fn symbols(&self) -> Vec<String> {
        vec![self.pair.to_string()]
    }

    async fn generate_signals(&mut self, market_data: &MarketData) -> Vec<Signal> {
        if market_data.pair != self.pair || market_data.candles.is_empty() {
            return Vec::new();
        }
        let latest = market_data.candles.last().unwrap();
        let current_price = latest.close;
        let close_prices: Vec<f64> = market_data.candles.iter().map(|c| c.close).collect();
        if self.ema_short.is_none() {
            self.warm_up(&close_prices);
        }
        let ema_s = self.ema_short.as_mut().unwrap().next(current_price);
        let ema_l = self.ema_long.as_mut().unwrap().next(current_price);
        let rsi_val = self.rsi.as_mut().unwrap().next(current_price);

        let mut signals = Vec::new();
        // Buy condition
        if ema_s > ema_l && rsi_val < self.rsi_overbought {
            signals.push(Signal {
                symbol: self.pair.to_string(),
                signal_type: SignalType::Buy,
                price: current_price,
                size: self.position_size,
                confidence: 0.8,
                timestamp: market_data.timestamp,
                metadata: None,
            });
        }
        // Sell condition
        if ema_s < ema_l && rsi_val > self.rsi_oversold {
            signals.push(Signal {
                symbol: self.pair.to_string(),
                signal_type: SignalType::Sell,
                price: current_price,
                size: self.position_size,
                confidence: 0.8,
                timestamp: market_data.timestamp,
                metadata: None,
            });
        }
        signals
    }

    fn on_order_filled(&mut self, _order: &Order) {
        // simplistic, no state tracking yet
    }

    fn get_positions(&self) -> Vec<&Position> {
        self.positions.iter().collect()
    }
}
