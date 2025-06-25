//! Capital allocation strategy wrapper.
//!
//! Distributes capital among multiple sub-strategies according to
//! user-defined weights by scaling the `size` field of each `Signal`.

use async_trait::async_trait;
use futures::future::join_all;

use super::{TimeFrame, TradingStrategy};
use crate::trading::{MarketData, Order, Position, Signal};

pub struct AllocationStrategy {
    name: String,
    timeframe: TimeFrame,
    sub_strategies: Vec<Box<dyn TradingStrategy>>,
    weights: Vec<f64>, // length == sub_strategies
}

impl AllocationStrategy {
    pub fn new(name: &str, subs: Vec<Box<dyn TradingStrategy>>, weights: Vec<f64>) -> Self {
        assert_eq!(subs.len(), weights.len(), "weights length must match sub strategies");
        let timeframe = subs
            .get(0)
            .map(|s| s.timeframe())
            .unwrap_or(TimeFrame::OneHour);
        // normalise weights (sum to 1.0)
        let sum: f64 = weights.iter().copied().sum();
        let weights = if (sum - 1.0).abs() > 1e-6 {
            weights.iter().map(|w| w / sum).collect()
        } else {
            weights
        };
        Self { name: name.to_string(), timeframe, sub_strategies: subs, weights }
    }
}

#[async_trait]
impl TradingStrategy for AllocationStrategy {
    fn name(&self) -> &str {
        &self.name
    }

    fn timeframe(&self) -> TimeFrame {
        self.timeframe
    }

    fn symbols(&self) -> Vec<String> {
        let mut set = std::collections::BTreeSet::new();
        for s in &self.sub_strategies {
            for sym in s.symbols() {
                set.insert(sym);
            }
        }
        set.into_iter().collect()
    }

    async fn generate_signals(&mut self, market_data: &MarketData) -> Vec<Signal> {
        let futs = self
            .sub_strategies
            .iter_mut()
            .map(|s| s.generate_signals(market_data));
        let nested: Vec<Vec<Signal>> = join_all(futs).await;
        let mut out = Vec::new();
        for (idx, list) in nested.into_iter().enumerate() {
            let w = self.weights[idx];
            for mut sig in list {
                sig.size *= w; // scale position size
                out.push(sig);
            }
        }
        out
    }

    fn on_order_filled(&mut self, order: &Order) {
        for s in &mut self.sub_strategies {
            s.on_order_filled(order);
        }
    }

    fn on_trade_error(&mut self, order: &Order, err: &anyhow::Error) {
        for s in &mut self.sub_strategies {
            s.on_trade_error(order, err);
        }
    }

    fn get_positions(&self) -> Vec<&Position> {
        let mut v = Vec::new();
        for s in &self.sub_strategies {
            v.extend(s.get_positions());
        }
        v
    }

    fn as_any(&self) -> &dyn std::any::Any
    where
        Self: 'static + Sized,
    {
        self
    }
}
