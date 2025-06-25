//! Triton One WebSocket integration for Solana DEX data
// Requires a Triton API key. See https://triton.one/ for details.

use crate::engine::market_router::ChannelMarketDataStream;
use crate::utils::market_stream::MarketEvent;
use async_trait::async_trait;
use futures_util::StreamExt;
use serde_json::Value;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

pub struct TritonStream {
    pub url: String,
    pub api_key: String,
    pub market: String,
}

impl TritonStream {
    pub fn new(api_key: &str, market: &str) -> Self {
        let url = format!("wss://api.triton.one/ws?api-key={}", api_key);
        Self { url, api_key: api_key.to_string(), market: market.to_string() }
    }
}

#[async_trait]
impl ChannelMarketDataStream for TritonStream {
    async fn connect_and_stream_channel(
        &mut self, _symbols: Vec<String>, sender: Sender<MarketEvent>,
    ) -> anyhow::Result<()> {
        let (ws_stream, _) = connect_async(&self.url).await?;
        let (_, mut read) = ws_stream.split();
        while let Some(msg) = read.next().await {
            let msg = msg?;
            if let Message::Text(txt) = msg {
                if let Ok(json) = serde_json::from_str::<Value>(&txt) {
                    // Triton trade event example (structure may differ)
                    if let Some(event_type) = json.get("type").and_then(|v| v.as_str()) {
                        if event_type == "trade" {
                            let symbol = self.market.clone();
                            let price = json.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            let qty = json.get("qty").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            let side = json
                                .get("side")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let timestamp =
                                json.get("timestamp").and_then(|v| v.as_i64()).unwrap_or(0);
                            let event = MarketEvent::Trade {
                                exchange: "triton".to_string(),
                                symbol,
                                price,
                                qty,
                                side,
                                timestamp,
                            };
                            let _ = sender.send(event).await;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
