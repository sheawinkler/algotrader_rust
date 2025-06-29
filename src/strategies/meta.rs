//! Composite / meta strategies that combine multiple sub-strategies.

use async_trait::async_trait;
use futures::future::join_all;

use super::{TimeFrame, TradingStrategy, TradingStrategyClone};
use crate::trading::{MarketData, Order, Position, Signal};

/// A simple ensemble strategy that aggregates signals from multiple
/// sub-strategies and applies a majority-vote filter. If conflicting
/// signals are produced for the *same* symbol in the same bar, they
/// cancel each other out.
pub struct EnsembleStrategy {
    name: String,
    timeframe: TimeFrame,
    sub_strategies: Vec<Box<dyn TradingStrategy>>, // already boxed
}

impl EnsembleStrategy {
    /// Build a new ensemble. All sub-strategies must share the same timeframe.
    pub fn new(name: &str, strategies: Vec<Box<dyn TradingStrategy>>) -> Self {
        let timeframe = strategies
            .first()
            .map(|s| s.timeframe())
            .unwrap_or(TimeFrame::OneHour);
        Self { name: name.to_string(), timeframe, sub_strategies: strategies }
    }
}

#[async_trait]
impl TradingStrategy for EnsembleStrategy {
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
        // Gather futures of each sub-strategy.
        let mut futs = Vec::with_capacity(self.sub_strategies.len());
        for s in &mut self.sub_strategies {
            futs.push(s.generate_signals(market_data));
        }
        let results: Vec<Vec<Signal>> = join_all(futs).await;

        // Flatten and group by (symbol, signal_type)
        use std::collections::HashMap;
        use std::mem;
        let mut counts: HashMap<
            (String, mem::Discriminant<crate::trading::SignalType>),
            (usize, Signal),
        > = HashMap::new();
        for list in results {
            for sig in list {
                let key = (sig.symbol.clone(), mem::discriminant(&sig.signal_type));
                let entry = counts.entry(key).or_insert((0, sig.clone()));
                entry.0 += 1;
            }
        }
        let majority = (self.sub_strategies.len() / 2) + 1;
        counts
            .into_iter()
            .filter_map(
                |((_sym, _disc), (cnt, sig))| if cnt >= majority { Some(sig) } else { None },
            )
            .collect()
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
        let mut out = Vec::new();
        for s in &self.sub_strategies {
            out.extend(s.get_positions());
        }
        out
    }

    fn as_any(&self) -> &dyn std::any::Any
    where
        Self: 'static + Sized,
    {
        self
    }
}
