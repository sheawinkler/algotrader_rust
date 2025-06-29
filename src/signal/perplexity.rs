//! Perplexity-based sentiment signal source.
//!
//! This source is a placeholder that **should** query an external sentiment
//! provider such as the Perplexity MCP server (or Twitter API) and emit a
//! numeric confidence score per symbol. Until credentials are wired, we fall
//! back to a random score so the rest of the pipeline compiles and runs.
//! Replace `fetch_sentiment()` with a real implementation when available.

use crate::signal::SignalSource;
use anyhow::Result;
use async_trait::async_trait;
use rand::Rng;
use std::time::Duration;
use tokio::{sync::mpsc::UnboundedSender, time::sleep};

pub struct PerplexitySource {
    symbols: Vec<String>,
    interval_secs: u64,
    tx: UnboundedSender<(String, f64)>,
}

impl PerplexitySource {
    pub fn new(symbols: &[&str], interval_secs: u64, tx: UnboundedSender<(String, f64)>) -> Self {
        Self {
            symbols: symbols.iter().map(|s| s.to_string()).collect(),
            interval_secs,
            tx,
        }
    }

    async fn fetch_sentiment(&self, symbol: &str) -> Result<f64> {
        // TODO: integrate real Perplexity/Twitter sentiment API.
        // Dummy implementation: random score in [-1, 1].
        let mut rng = rand::thread_rng();
        let val: f64 = rng.gen_range(-1.0..1.0);
        Ok(val)
    }
}

#[async_trait]
impl SignalSource for PerplexitySource {
    async fn run(&self) -> Result<()> {
        loop {
            for sym in &self.symbols {
                match self.fetch_sentiment(sym).await {
                    Ok(score) => {
                        // Map score to 0..1 for consistency
                        let norm = (score + 1.0) / 2.0;
                        let _ = self.tx.send((sym.clone(), norm));
                    }
                    Err(e) => {
                        log::warn!("perplexity sentiment fetch failed for {}: {}", sym, e);
                    }
                }
            }
            sleep(Duration::from_secs(self.interval_secs)).await;
        }
    }
}
