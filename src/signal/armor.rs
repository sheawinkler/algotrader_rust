//! Armor Crypto Intelligence feed stub.
//!
//! Queries Armor API (or placeholder) for distressed wallet alerts / rug pulls. Emits a
//! generic risk score (0.0 – 1.0) for a given symbol every N seconds. For now, we
//! generate a dummy random risk score so that downstream fusion logic can consume
//! the stream.

use async_trait::async_trait;
use rand::Rng;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;

use super::SignalSource;

#[derive(Clone)]
pub struct ArmorSource {
    pub symbol: String,                     // e.g. "BTC"
    pub interval_secs: u64,                 // polling interval
    pub tx: UnboundedSender<(String, f64)>, // channel to emit (symbol, risk_score)
}

impl ArmorSource {
    pub fn new(symbol: impl Into<String>, interval_secs: u64, tx: UnboundedSender<(String, f64)>) -> Self {
        Self { symbol: symbol.into(), interval_secs, tx }
    }
}

#[async_trait]
impl SignalSource for ArmorSource {
    async fn run(&self) -> anyhow::Result<()> {
        loop {
            // Placeholder: generate random risk score
            let risk = rand::thread_rng().gen_range(0.0..1.0);
            let _ = self.tx.send((self.symbol.clone(), risk));
            tokio::time::sleep(Duration::from_secs(self.interval_secs)).await;
        }
    }
}
