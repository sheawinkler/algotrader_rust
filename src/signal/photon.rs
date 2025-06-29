//! Photon-Sol orderflow & liquidity feed stub.
//!
//! Placeholder implementation that emits a random liquidity metric (0–100) for a
//! given Solana pair at a configurable interval. Real implementation will call
//! Photon-Sol or Jupiter APIs for on-chain orderbook depth.

use async_trait::async_trait;
use rand::Rng;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;

use super::SignalSource;

#[derive(Clone)]
pub struct PhotonSource {
    pub pair: String,                        // e.g. "SOL/USDC"
    pub interval_secs: u64,
    pub tx: UnboundedSender<(String, f64)>, // (pair, liquidity_score)
}

impl PhotonSource {
    pub fn new(pair: impl Into<String>, interval_secs: u64, tx: UnboundedSender<(String, f64)>) -> Self {
        Self { pair: pair.into(), interval_secs, tx }
    }
}

#[async_trait]
impl SignalSource for PhotonSource {
    async fn run(&self) -> anyhow::Result<()> {
        loop {
            let score = rand::thread_rng().gen_range(0.0..100.0);
            let _ = self.tx.send((self.pair.clone(), score));
            tokio::time::sleep(Duration::from_secs(self.interval_secs)).await;
        }
    }
}
