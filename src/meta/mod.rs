//! Meta-strategy engine: selects the best-performing strategy for a given dataset
//! by running back-tests (with caching) and ranking by Sharpe ratio & draw-down.
//! This is an early, synchronous MVP – can be improved with async and more metrics later.

use std::path::Path;

use anyhow::Result;

use crate::backtest::{
    cache::BacktestCache, providers::CSVHistoricalDataProvider, Backtester, SimMode,
};
use crate::strategies::TradingStrategyClone;

use crate::persistence;
use crate::strategies::{registry::default_strategies, TradingStrategy};
use std::sync::Arc;

/// Simple ranking result
pub struct RankedStrategy {
    pub strategy: Box<dyn TradingStrategy>,
    pub sharpe: f64,
    pub max_drawdown: f64,
}

pub struct MetaStrategyEngine {
    strategies: Vec<Box<dyn TradingStrategyClone>>,
    cache: BacktestCache,
    timeframe: String,
    starting_balance: f64,
}

impl MetaStrategyEngine {
    /// Create engine with default registry strategies.
    pub fn new(timeframe: &str, starting_balance: f64, cache_path: &str) -> Result<Self> {
        let cache = BacktestCache::open(cache_path)?;
        Ok(Self {
            strategies: default_strategies(),
            cache,
            timeframe: timeframe.to_string(),
            starting_balance,
        })
    }

    /// Evaluate strategies on the file & return the best-performing one (highest Sharpe, drawdown < 30%)
    pub fn select_best_strategy(&mut self, data_file: &Path) -> Result<RankedStrategy> {
        let base_provider = CSVHistoricalDataProvider::new();
        let mut best: Option<RankedStrategy> = None;

        for strategy in &self.strategies {
            // Obtain a fresh clone for the backtester run
            let cloned = strategy.box_clone();
            // Backtester consumes a Vec, so wrap the one strategy
            let mut bt = Backtester {
                data_provider: Box::new(base_provider.clone()),
                timeframe: self.timeframe.clone(),
                starting_balance: self.starting_balance,
                strategies: vec![cloned as Box<dyn TradingStrategy>],
                cache: Some(self.cache.clone()),
                sim_mode: SimMode::Bar,
                slippage_bps: 5,
                fee_bps: 3,
                persistence: Some(Arc::new(persistence::NullPersistence)),
                risk_rules: vec![
                    Box::new(crate::risk::StopLossRule::new(0.05)),
                    Box::new(crate::risk::TakeProfitRule::new(0.10)),
                ],
            };

            let rpt = futures::executor::block_on(bt.run(data_file))?;
            let ranked = RankedStrategy {
                strategy: strategy.box_clone(),
                sharpe: rpt.sharpe,
                max_drawdown: rpt.max_drawdown,
            };

            let is_better = match &best {
                | None => true,
                | Some(b) => ranked.sharpe > b.sharpe,
            } && ranked.max_drawdown < 0.3; // filter high DD

            if is_better {
                best = Some(ranked);
            }
        }

        best.ok_or_else(|| anyhow::anyhow!("No suitable strategy found"))
    }
}
