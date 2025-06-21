//! Quick compile-check stubs for each strategy implementation.
//! These tests simply attempt to instantiate each strategy via the
//! `StrategyFactory` using a minimal `StrategyConfig`. If the code builds
//! and the factory returns `Ok`, we consider the test successful.

use algotraderv2::strategies::{StrategyConfig, StrategyFactory};
use serde_json::json;

fn dummy_cfg(name: &str) -> StrategyConfig {
    StrategyConfig {
        name: name.to_string(),
        enabled: true,
        params: json!({}),
        performance: None,
    }
}

macro_rules! strategy_compile_test {
    ($test_name:ident, $strategy_name:expr) => {
        #[test]
        fn $test_name() {
            let cfg = dummy_cfg($strategy_name);
            StrategyFactory::create_strategy(&$strategy_name, &cfg)
                .expect("strategy should compile and instantiate");
        }
    };
}

strategy_compile_test!(advanced_strategy_compiles, "advanced");
strategy_compile_test!(mean_reversion_strategy_compiles, "mean_reversion");
strategy_compile_test!(trend_following_strategy_compiles, "trend_following");
strategy_compile_test!(order_flow_strategy_compiles, "order_flow");
strategy_compile_test!(momentum_strategy_compiles, "momentum");
strategy_compile_test!(meme_arbitrage_strategy_compiles, "meme_arbitrage");
strategy_compile_test!(bundle_sniper_strategy_compiles, "bundle_sniper");
