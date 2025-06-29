//! Integration test for AlpacaSource stub.
//! Requires no real Alpaca API keys – will fall back to random price generation.

use algotraderv2::signal::alpaca::AlpacaSource;
use algotraderv2::signal::SignalSource;
use tokio::sync::mpsc::unbounded_channel;
use std::time::Duration;

#[tokio::test]
async fn alpaca_source_emits_prices() {
    let (tx, mut rx) = unbounded_channel::<(String, f64)>();
    let src = AlpacaSource::new("AAPL", 1, tx);
    tokio::spawn(async move { let _ = src.run().await; });

    // Wait up to 3 seconds for first price
    let mut got = false;
    for _ in 0..3 {
        if let Ok(Some((sym, price))) = tokio::time::timeout(Duration::from_secs(1), rx.recv()).await {
            assert_eq!(sym, "AAPL");
            assert!(price > 0.0);
            got = true;
            break;
        }
    }
    assert!(got, "AlpacaSource did not emit price within 3 seconds");
}
