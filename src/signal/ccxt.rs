//! Simple CCXT-like exchange price poller.
//!
//! For now we use public Binance REST to fetch latest price every N seconds.
//! Later this will be replaced by async MCP integration or websocket feeds.

use async_trait::async_trait;
use serde::Deserialize;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;

use super::SignalSource;

#[derive(Clone)]
pub struct CcxtSource {
    pub symbol: String,            // e.g. "BTCUSDT"
    pub interval_secs: u64,        // polling interval
    pub tx: UnboundedSender<(String, f64)>, // rudimentary channel to emit (symbol, price)
}

impl CcxtSource {
    pub fn new(symbol: impl Into<String>, interval_secs: u64, tx: UnboundedSender<(String, f64)>) -> Self {
        Self { symbol: symbol.into(), interval_secs, tx }
    }
}

#[derive(Debug, Deserialize)]
struct PriceResp {
    price: String,
}

#[async_trait]
impl SignalSource for CcxtSource {
    async fn run(&self) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        loop {
            let url = format!(
                "https://api.binance.com/api/v3/ticker/price?symbol={}",
                self.symbol
            );
            match client.get(&url).send().await {
                Ok(r) => {
                    if let Ok(resp) = r.json::<PriceResp>().await {
                        if let Ok(p) = resp.price.parse::<f64>() {
                            let _ = self.tx.send((self.symbol.clone(), p));
                        }
                    }
                }
                Err(e) => {
                    log::warn!("CcxtSource fetch error: {}", e);
                }
            }
            tokio::time::sleep(Duration::from_secs(self.interval_secs)).await;
        }
    }
}
