//! SolanaSniffer MCP feed stub.
//!
//! Emits a boolean rug-risk alert (0/1) or whale-buy indicator for a Solana token.
//! For now this emits a random 0.0 or 1.0 every N seconds to simulate alerts.

use async_trait::async_trait;
use rand::Rng;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;

use super::SignalSource;

#[derive(Clone)]
pub struct SolanaSnifferSource {
    pub token: String,
    pub interval_secs: u64,
    pub tx: UnboundedSender<(String, f64)>, // (token, alert_value)
}

impl SolanaSnifferSource {
    pub fn new(token: impl Into<String>, interval_secs: u64, tx: UnboundedSender<(String, f64)>) -> Self {
        Self { token: token.into(), interval_secs, tx }
    }
}

#[async_trait]
impl SignalSource for SolanaSnifferSource {
    async fn run(&self) -> anyhow::Result<()> {
        loop {
            let alert = if rand::thread_rng().gen_bool(0.05) { 1.0 } else { 0.0 }; // 5% chance alert
            let _ = self.tx.send((self.token.clone(), alert));
            tokio::time::sleep(Duration::from_secs(self.interval_secs)).await;
        }
    }
}
