//! Alpaca market data poller (stub).
//!
//! In production this will call Alpaca v2/api or websocket for live prices. For now we
//! hit the free "last trade" endpoint every N seconds, requiring `APCA_API_KEY_ID`
//! and `APCA_API_SECRET_KEY` in env. If missing, we simply emit random prices so
//! the rest of the pipeline continues to flow.

use async_trait::async_trait;
use rand::Rng;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;

use super::SignalSource;

#[derive(Clone)]
pub struct AlpacaSource {
    pub symbol: String,            // e.g. "BTCUSD" (for crypto) or "AAPL"
    pub interval_secs: u64,        // polling interval
    pub tx: UnboundedSender<(String, f64)>, // channel to emit (symbol, price)
}

impl AlpacaSource {
    pub fn new(symbol: impl Into<String>, interval_secs: u64, tx: UnboundedSender<(String, f64)>) -> Self {
        Self { symbol: symbol.into(), interval_secs, tx }
    }
}

#[async_trait]
impl SignalSource for AlpacaSource {
    async fn run(&self) -> anyhow::Result<()> {
        let api_key = std::env::var("APCA_API_KEY_ID").ok();
        let api_secret = std::env::var("APCA_API_SECRET_KEY").ok();
        let client = reqwest::Client::new();
        loop {
            let price_opt = if let (Some(key), Some(sec)) = (&api_key, &api_secret) {
                let url = format!("https://data.alpaca.markets/v2/stocks/{}/trades/latest", self.symbol);
                match client
                    .get(&url)
                    .header("APCA-API-KEY-ID", key)
                    .header("APCA-API-SECRET-KEY", sec)
                    .send()
                    .await
                {
                    Ok(r) => {
                        #[derive(serde::Deserialize)]
                        struct Resp { trade: Trade }
                        #[derive(serde::Deserialize)]
                        struct Trade { p: f64 }
                        if let Ok(resp) = r.json::<Resp>().await {
                            Some(resp.trade.p)
                        } else {
                            None
                        }
                    }
                    Err(e) => {
                        log::warn!("AlpacaSource fetch error: {}", e);
                        None
                    }
                }
            } else {
                // No API credentials – generate dummy price around 100.0
                Some(100.0 + rand::thread_rng().gen_range(-1.0..1.0))
            };

            if let Some(p) = price_opt {
                let _ = self.tx.send((self.symbol.clone(), p));
            }
            tokio::time::sleep(Duration::from_secs(self.interval_secs)).await;
        }
    }
}
