//! Kraken WebSocket market data stream integration

use crate::engine::market_router::ChannelMarketDataStream;
use crate::utils::market_stream::MarketEvent;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

#[derive(Clone, Debug)]
pub struct KrakenStream {
    pub url: String,
}

impl KrakenStream {
    pub fn new() -> Self {
        let url = "wss://ws.kraken.com".to_string();
        Self { url }
    }
}

#[async_trait::async_trait]
impl Default for KrakenStream {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ChannelMarketDataStream for KrakenStream {
    async fn connect_and_stream_channel(
        &mut self, symbols: Vec<String>, sender: Sender<MarketEvent>,
    ) -> anyhow::Result<()> {
        let (mut ws_stream, _) = connect_async(&self.url).await?;
        // Subscribe to ticker/trades for the given symbols
        let subscribe_msg = serde_json::json!({
            "event": "subscribe",
            "pair": symbols,
            "subscription": { "name": "trade" }
        });
        ws_stream
            .send(Message::Text(subscribe_msg.to_string()))
            .await?;
        let (_, mut read) = ws_stream.split();
        while let Some(msg) = read.next().await {
            let msg = msg?;
            if let Message::Text(txt) = msg {
                if let Ok(json) = serde_json::from_str::<Value>(&txt) {
                    // Kraken trade message is an array
                    if json.is_array() && json.as_array().unwrap().len() > 3 {
                        let arr = json.as_array().unwrap();
                        if let Some(trades) = arr.get(1).and_then(|v| v.as_array()) {
                            let symbol = arr
                                .get(3)
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            for trade in trades {
                                if let Some(trade_arr) = trade.as_array() {
                                    let price = trade_arr
                                        .first()
                                        .and_then(|v| v.as_str())
                                        .and_then(|s| s.parse().ok())
                                        .unwrap_or(0.0);
                                    let qty = trade_arr
                                        .get(1)
                                        .and_then(|v| v.as_str())
                                        .and_then(|s| s.parse().ok())
                                        .unwrap_or(0.0);
                                    let side = trade_arr
                                        .get(3)
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let timestamp = trade_arr
                                        .get(2)
                                        .and_then(|v| v.as_f64())
                                        .map(|f| (f * 1000.0) as i64)
                                        .unwrap_or(0);
                                    let event = MarketEvent::Trade {
                                        exchange: "kraken".to_string(),
                                        symbol: symbol.clone(),
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
            }
        }
        Ok(())
    }
}
