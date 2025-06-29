//! Financial Datasets MCP feed stub.
//!
//! Emits a macro sentiment score or economic indicator value for a symbol at a
//! fixed interval. Currently generates a dummy value from N(0,1).

use async_trait::async_trait;
use rand_distr::{Distribution, Normal};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;

use super::SignalSource;

#[derive(Clone)]
pub struct FinancialSource {
    pub symbol: String,
    pub interval_secs: u64,
    pub tx: UnboundedSender<(String, f64)>,
}

impl FinancialSource {
    pub fn new(symbol: impl Into<String>, interval_secs: u64, tx: UnboundedSender<(String, f64)>) -> Self {
        Self { symbol: symbol.into(), interval_secs, tx }
    }
}

#[async_trait]
impl SignalSource for FinancialSource {
    async fn run(&self) -> anyhow::Result<()> {
        let normal = Normal::new(0.0, 1.0).unwrap();
        loop {
            let val = normal.sample(&mut rand::thread_rng());
            let _ = self.tx.send((self.symbol.clone(), val));
            tokio::time::sleep(Duration::from_secs(self.interval_secs)).await;
        }
    }
}
