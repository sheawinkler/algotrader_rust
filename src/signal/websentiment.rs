//! WebSentiment (Playwright) MCP feed stub.
//!
//! Intended to crawl crypto news/social pages via Playwright headless browser and
//! extract a sentiment score. Real implementation will live in a separate async
//! task calling the MCP Playwright server. For now, generates a random sentiment
//! in the range [-1.0, 1.0] for a watched symbol.

use async_trait::async_trait;
use rand::Rng;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;

use super::SignalSource;

#[derive(Clone)]
pub struct WebSentimentSource {
    pub symbol: String,
    pub interval_secs: u64,
    pub tx: UnboundedSender<(String, f64)>, // (symbol, sentiment)
}

impl WebSentimentSource {
    pub fn new(symbol: impl Into<String>, interval_secs: u64, tx: UnboundedSender<(String, f64)>) -> Self {
        Self { symbol: symbol.into(), interval_secs, tx }
    }
}

#[async_trait]
impl SignalSource for WebSentimentSource {
    async fn run(&self) -> anyhow::Result<()> {
        loop {
            let sentiment = rand::thread_rng().gen_range(-1.0..1.0);
            let _ = self.tx.send((self.symbol.clone(), sentiment));
            tokio::time::sleep(Duration::from_secs(self.interval_secs)).await;
        }
    }
}
