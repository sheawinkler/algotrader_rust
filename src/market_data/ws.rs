//! WebSocket price feed (initial implementation)
//!
//! Currently supports Jupiter v6 price stream. It maintains a shared in-memory
//! cache of the most recent mid-price for each subscribed trading pair.
//! Future: extend with Raydium/Orca/Serum streams or switch to Pyth.

use std::{collections::HashMap, sync::Arc, time::Duration};

use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::{sync::RwLock, task::JoinHandle};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::utils::types::TradingPair;

/// Shared cache of latest prices.
pub type PriceCache = Arc<RwLock<HashMap<TradingPair, f64>>>;

/// Spawn the background WebSocket task. The handle should be kept so the task
/// lives as long as the engine. If it crashes, the caller may decide to
/// restart it.
pub fn spawn_price_feed(pairs: &[TradingPair], cache: PriceCache) -> JoinHandle<()> {
    let symbols: Vec<String> = pairs.iter().map(|p| format!("{}/{}", p.base, p.quote)).collect();
    let cache_clone = cache.clone();

    tokio::spawn(async move {
        // Jupiter WS endpoint
        let url = "wss://quote-api.jup.ag/v6/ws";
        match connect_async(url).await {
            Ok((mut ws, _resp)) => {
                // Subscribe once connected
                let sub_msg = serde_json::json!({
                    "type": "subscribe",
                    "channel": "price",
                    "symbols": symbols,
                });
                if ws.send(Message::Text(sub_msg.to_string())).await.is_err() {
                    log::error!("Price feed: failed to send subscribe message");
                    return;
                }

                // Read loop
                while let Some(msg) = ws.next().await {
                    match msg {
                        Ok(Message::Text(txt)) => {
                            if let Ok(evt) = serde_json::from_str::<PriceUpdate>(&txt) {
                                let pair = TradingPair::new(&evt.base, &evt.quote);
                                let mut guard = cache_clone.write().await;
                                guard.insert(pair, evt.price);
                            }
                        }
                        Ok(Message::Ping(_)) => {
                            // tungstenite handles pongs internally, but we respond anyway
                            let _ = ws.send(Message::Pong(Vec::new())).await;
                        }
                        Err(e) => {
                            log::warn!("Price feed error: {}", e);
                            break;
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => log::error!("Failed to connect to Jupiter price feed: {}", e),
        }

        // If we get here the connection closed – wait a bit and exit. The
        // caller may decide to respawn.
        tokio::time::sleep(Duration::from_secs(5)).await;
    })
}

#[derive(Debug, Deserialize)]
struct PriceUpdate {
    #[serde(rename = "type")]
    _ty: String,
    base: String,
    quote: String,
    price: f64,
}
