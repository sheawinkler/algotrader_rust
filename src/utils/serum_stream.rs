//! Serum/OpenBook relayer WebSocket integration (Solana DEX)
// Note: This is a stub using Mango's public OpenBook relayer as an example.
// For production, consider Triton, Helius, or running your own relayer/indexer.

use crate::engine::market_router::ChannelMarketDataStream;
use crate::utils::market_stream::MarketEvent;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::StreamExt;
use serde_json::Value;

pub struct SerumStream {
    pub url: String,
    pub market: String, // e.g. "SOL/USDC"
}

impl SerumStream {
    pub fn new(market: &str) -> Self {
        // Example: Mango OpenBook relayer (replace with your relayer if needed)
        let url = format!("wss://api.mngo.cloud/v1/ws/openbook/{}", market.replace("/", "-"));
        Self { url, market: market.to_string() }
    }
}

#[async_trait::async_trait]
impl ChannelMarketDataStream for SerumStream {
    async fn connect_and_stream_channel(&mut self, _symbols: Vec<String>, sender: Sender<MarketEvent>) -> anyhow::Result<()> {
        let (ws_stream, _) = connect_async(&self.url).await?;
        let (_, mut read) = ws_stream.split();
        while let Some(msg) = read.next().await {
            let msg = msg?;
            if let Message::Text(txt) = msg {
                if let Ok(json) = serde_json::from_str::<Value>(&txt) {
                    // Example: handle trade events (structure may differ by relayer)
                    if let Some(event_type) = json.get("type").and_then(|v| v.as_str()) {
                        if event_type == "trade" {
                            let symbol = self.market.clone();
                            let price = json.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            let qty = json.get("size").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            let side = json.get("side").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let timestamp = json.get("timestamp").and_then(|v| v.as_i64()).unwrap_or(0);
                            let event = MarketEvent::Trade {
                                exchange: "serum".to_string(),
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
