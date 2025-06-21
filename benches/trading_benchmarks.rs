//! Benchmarks for trading operations

#![feature(test)]

#[cfg(test)]
mod tests {
    extern crate test;
    use algotraderv2_rust::{
        dex::{DexClient, DexFactory},
        strategy::{MarketData, StrategyFactory, TradingStrategy},
    };
    // use rust_decimal_macros::dec; // REMOVED: crate not present
    use std::sync::Arc;
    use test::Bencher;

    // Helper function to create test market data
    fn create_test_market_data() -> Vec<MarketData> {
        let mut data = Vec::with_capacity(1000);
        let mut price = 100.0;
        
        for i in 0..1000 {
            // Add some randomness to the price movement
            let change = (i as f64).sin() * 0.5;
            price += change;
            
            data.push(MarketData {
                timestamp: chrono::Utc::now() + chrono::Duration::seconds(i as i64),
                open: price - 0.1,
                high: price + 0.1,
                low: price - 0.1,
                close: price,
                volume: 1000.0 + (i as f64) * 10.0,
            });
        }
        
        data
    }

    // Benchmark for mean reversion strategy analysis
    #[bench]
    fn bench_mean_reversion_strategy(b: &mut Bencher) {
        let market_data = create_test_market_data();
        let mut strategy = StrategyFactory::create_strategy("mean_reversion").unwrap();
        
        // Initialize with default parameters
        let params = std::collections::HashMap::from([
            ("lookback".to_string(), "20".to_string()),
            ("entry_z_score".to_string(), "2.0".to_string()),
            ("exit_z_score".to_string(), "0.5".to_string()),
            ("position_size".to_string(), "0.1".to_string()),
        ]);
        
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(strategy.initialize(params))
            .unwrap();
        
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(strategy.analyze(&market_data)).unwrap();
        });
    }

    // Benchmark for momentum strategy analysis
    #[bench]
    fn bench_momentum_strategy(b: &mut Bencher) {
        let market_data = create_test_market_data();
        let mut strategy = StrategyFactory::create_strategy("momentum").unwrap();
        
        // Initialize with default parameters
        let params = std::collections::HashMap::from([
            ("ema_short".to_string(), "9".to_string()),
            ("ema_long".to_string(), "21".to_string()),
            ("rsi_period".to_string(), "14".to_string()),
            ("rsi_overbought".to_string(), "70.0".to_string()),
            ("rsi_oversold".to_string(), "30.0".to_string()),
            ("position_size".to_string(), "0.1".to_string()),
        ]);
        
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(strategy.initialize(params))
            .unwrap();
        
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(strategy.analyze(&market_data)).unwrap();
        });
    }
    
    // Benchmark for DEX price fetching (mocked)
    #[bench]
    fn bench_dex_price_fetch(b: &mut Bencher) {
        // This is a mock benchmark - in a real scenario, this would make actual API calls
        b.iter(|| {
            // Simulate network latency
            std::thread::sleep(std::time::Duration::from_millis(10));
            // Return a mock price
            dec!(100.50)
        });
    }
    
    // Benchmark for order execution (mocked)
    #[bench]
    fn bench_order_execution(b: &mut Bencher) {
        // This is a mock benchmark - in a real scenario, this would execute actual trades
        b.iter(|| {
            // Simulate network latency and exchange processing time
            std::thread::sleep(std::time::Duration::from_millis(50));
            // Return a mock order ID
            "mock_order_123".to_string()
        });
    }
}
