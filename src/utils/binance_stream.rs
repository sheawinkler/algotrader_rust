//! Binance WebSocket market data stream integration

use crate::engine::market_router::ChannelMarketDataStream;
use crate::utils::market_stream::MarketEvent;
use futures_util::StreamExt;
use serde_json::Value;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

pub struct BinanceStream {
    pub url: String,
}

impl BinanceStream {
    pub fn new(symbols: &[String]) -> Self {
        // Binance expects lowercase and concatenated symbols, e.g. "btcusdt"
        let streams = symbols
            .iter()
            .map(|s| format!("{}@trade", s.to_lowercase()))
            .collect::<Vec<_>>()
            .join("/");
        let url = format!("wss://stream.binance.com:9443/stream?streams={}", streams);
        Self { url }
    }
}

#[async_trait::async_trait]
impl ChannelMarketDataStream for BinanceStream {
    async fn connect_and_stream_channel(
        &mut self, _symbols: Vec<String>, sender: Sender<MarketEvent>,
    ) -> anyhow::Result<()> {
        let (ws_stream, _) = connect_async(&self.url).await?;
        let (_, mut read) = ws_stream.split();
        while let Some(msg) = read.next().await {
            let msg = msg?;
            if let Message::Text(txt) = msg {
                if let Ok(json) = serde_json::from_str::<Value>(&txt) {
                    if let Some(_stream) = json.get("stream") {
                        if let Some(data) = json.get("data") {
                            // Handle trade events
                            if let Some(event_type) = data.get("e") {
                                if event_type == "trade" {
                                    let symbol = data
                                        .get("s")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let price = data
                                        .get("p")
                                        .and_then(|v| v.as_str())
                                        .and_then(|s| s.parse().ok())
                                        .unwrap_or(0.0);
                                    let qty = data
                                        .get("q")
                                        .and_then(|v| v.as_str())
                                        .and_then(|s| s.parse().ok())
                                        .unwrap_or(0.0);
                                    let side =
                                        if data.get("m").and_then(|v| v.as_bool()).unwrap_or(false)
                                        {
                                            "sell"
                                        } else {
                                            "buy"
                                        };
                                    let timestamp =
                                        data.get("T").and_then(|v| v.as_i64()).unwrap_or(0);
                                    let event = MarketEvent::Trade {
                                        exchange: "binance".to_string(),
                                        symbol,
                                        price,
                                        qty,
                                        side: side.to_string(),
                                        timestamp,
                                    };
                                    let _ = sender.send(event).await;
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
