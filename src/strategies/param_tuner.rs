//! Parameter tuner that applies updated configs to running strategy instances.
use std::collections::HashMap;

use super::{TradingStrategy, StrategyConfig};

/// Helper that maps updated `StrategyConfig` objects to existing strategy
/// instances and calls their `update_params` hooks so they can adapt at
/// runtime.  This is the core of the dynamic parameter tuning framework.
pub struct ParamTuner {
    configs: HashMap<String, StrategyConfig>,
}

impl ParamTuner {
    /// Build a new tuner from a list of configs (typically reloaded from disk).
    pub fn new(configs: Vec<StrategyConfig>) -> Self {
        let mut map = HashMap::new();
        for cfg in configs {
            map.insert(cfg.name.clone(), cfg);
        }
        Self { configs: map }
    }

    /// Apply updated parameters to the provided strategy list.
    pub fn apply(&self, strategies: &mut [Box<dyn TradingStrategy>]) {
        for strat in strategies.iter_mut() {
            if let Some(cfg) = self.configs.get(strat.name()) {
                strat.update_params(&cfg.params);
            }
        }
    }
}
