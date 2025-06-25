//! Helius WebSocket integration for Solana on-chain and DEX events
// Requires a Helius API key. See https://docs.helius.xyz/ for details.

use crate::engine::market_router::ChannelMarketDataStream;
use crate::utils::market_stream::MarketEvent;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

pub struct HeliusStream {
    pub url: String,
    pub api_key: String,
    pub program_id: Option<String>, // Filter by program, e.g., OpenBook
}

impl HeliusStream {
    pub fn new(api_key: &str, program_id: Option<&str>) -> Self {
        let url = format!("wss://rpc.helius.xyz/v0/websockets/?api-key={}", api_key);
        Self { url, api_key: api_key.to_string(), program_id: program_id.map(|s| s.to_string()) }
    }
}

#[async_trait::async_trait]
impl ChannelMarketDataStream for HeliusStream {
    async fn connect_and_stream_channel(
        &mut self, _symbols: Vec<String>, sender: Sender<MarketEvent>,
    ) -> anyhow::Result<()> {
        let (mut ws_stream, _) = connect_async(&self.url).await?;
        // Subscribe to account/program updates (example: OpenBook program)
        if let Some(program_id) = &self.program_id {
            let subscribe_msg = serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "programSubscribe",
                "params": [program_id, {"encoding": "jsonParsed"}]
            });
            ws_stream
                .send(Message::Text(subscribe_msg.to_string()))
                .await?;
        }
        let (_, mut read) = ws_stream.split();
        while let Some(msg) = read.next().await {
            let msg = msg?;
            if let Message::Text(txt) = msg {
                if let Ok(json) = serde_json::from_str::<Value>(&txt) {
                    // Parse program/account update events
                    // (You may need to adjust this for your specific use case)
                    if let Some(_result) = json.get("result") {
                        // TODO: Map to MarketEvent::Trade/OrderBook as needed
                        // Example: sender.send(event).await.ok();
                    }
                }
            }
        }
        Ok(())
    }
}
