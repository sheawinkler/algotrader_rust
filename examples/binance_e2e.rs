//! Example: End-to-end test of TradingEngine with Binance market data stream

use algotraderv2_rust::TradingEngine;
use tokio::runtime::Runtime;

fn main() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let engine = TradingEngine::new();
        // Test with BTCUSDT symbol
        let symbols = vec!["btcusdt".to_string()];
        match engine.start_with_market_router(symbols).await {
            Ok(_) => println!("E2E Binance stream test completed."),
            Err(e) => eprintln!("E2E test failed: {}", e),
        }
    });
}
