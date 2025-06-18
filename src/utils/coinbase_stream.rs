//! Coinbase WebSocket market data stream integration

use crate::engine::market_router::ChannelMarketDataStream;
use crate::utils::market_stream::MarketEvent;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{StreamExt, SinkExt};
use serde_json::Value;

pub struct CoinbaseStream {
    pub url: String,
}

impl CoinbaseStream {
    pub fn new(symbols: &[String]) -> Self {
        let url = "wss://advanced-trade-ws.coinbase.com".to_string();
        Self { url }
    }
}

#[async_trait::async_trait]
impl ChannelMarketDataStream for CoinbaseStream {
    async fn connect_and_stream_channel(&mut self, symbols: Vec<String>, sender: Sender<MarketEvent>) -> anyhow::Result<()> {
        let (mut ws_stream, _) = connect_async(&self.url).await?;
        // Subscribe to ticker/trades for the given symbols
        let subscribe_msg = serde_json::json!({
            "type": "subscribe",
            "channels": [
                { "name": "ticker", "product_ids": symbols },
                { "name": "matches", "product_ids": symbols }
            ]
        });
        ws_stream.send(Message::Text(subscribe_msg.to_string())).await?;
        let (_, mut read) = ws_stream.split();
        while let Some(msg) = read.next().await {
            let msg = msg?;
            if let Message::Text(txt) = msg {
                if let Ok(json) = serde_json::from_str::<Value>(&txt) {
                    if let Some(event_type) = json.get("type").and_then(|v| v.as_str()) {
                        match event_type {
                            "match" => {
                                let symbol = json.get("product_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let price = json.get("price").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                                let qty = json.get("size").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                                let side = json.get("side").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let timestamp = json.get("time").and_then(|v| v.as_str()).map(|s| chrono::DateTime::parse_from_rfc3339(s).map(|dt| dt.timestamp_millis()).unwrap_or(0)).unwrap_or(0);
                                let event = MarketEvent::Trade {
                                    exchange: "coinbase".to_string(),
                                    symbol,
                                    price,
                                    qty,
                                    side,
                                    timestamp,
                                };
                                let _ = sender.send(event).await;
                            }
                            "ticker" => {
                                let symbol = json.get("product_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let price = json.get("price").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                                let timestamp = json.get("time").and_then(|v| v.as_str()).map(|s| chrono::DateTime::parse_from_rfc3339(s).map(|dt| dt.timestamp_millis()).unwrap_or(0)).unwrap_or(0);
                                let event = MarketEvent::Ticker {
                                    exchange: "coinbase".to_string(),
                                    symbol,
                                    price,
                                    timestamp,
                                };
                                let _ = sender.send(event).await;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
