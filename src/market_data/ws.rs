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
        // ---- Jupiter Price Feed ----
        // Try WebSocket first (undocumented; may fail with 404). If it fails,
        // fall back to simple HTTP polling using `https://lite-api.jup.ag/price`.
        let ws_url = "wss://quote-api.jup.ag/v6/ws";
        if let Ok((mut ws, _)) = connect_async(ws_url).await {
            log::info!("Connected to Jupiter WS price stream");
            let sub_msg = serde_json::json!({
                "type": "subscribe",
                "channel": "price",
                "symbols": symbols,
            });
            if ws.send(Message::Text(sub_msg.to_string())).await.is_err() {
                log::error!("Price feed: failed to send subscribe message (WS)");
            } else {
                // Read loop until error
                while let Some(msg) = ws.next().await {
                    match msg {
                        Ok(Message::Text(txt)) => {
                            if let Ok(evt) = serde_json::from_str::<PriceUpdate>(&txt) {
                                let pair = TradingPair::new(&evt.base, &evt.quote);
                                let mut guard = cache_clone.write().await;
                                guard.insert(pair, evt.price);
                            }
                        }
                        Ok(Message::Ping(_)) => { let _ = ws.send(Message::Pong(Vec::new())).await; }
                        Err(e) => { log::warn!("WS price feed error: {}", e); break; }
                        _ => {}
                    }
                }
            }
            log::warn!("Jupiter WS price feed closed; switching to HTTP polling");
        } else {
            log::warn!("Jupiter WS endpoint unavailable – using HTTP polling");
        }

        // ---- HTTP polling fallback ----
        // Build comma-separated list of mint addresses (Jupiter Price API v2 expects mints)
        fn symbol_to_mint(sym: &str) -> String {
            let up = sym.to_ascii_uppercase();
            match up.as_str() {
                "SOL" => "So11111111111111111111111111111111111111112".to_string(),
                // Extend with more symbol→mint mappings as needed.
                _ => sym.to_string(), // assume already a mint address.
            }
        }
        let mint_tokens: Vec<String> = symbols
            .iter()
            .filter_map(|s| s.split('/').next().map(|b| symbol_to_mint(b)))
            .collect();
        let ids_param = mint_tokens.join(",");

        // --- Immediate one-shot fetch so UI has data without waiting 4s ----
        let init_url = format!("https://lite-api.jup.ag/price/v2?ids={}", ids_param);
        match reqwest::get(&init_url).await {
            Ok(resp) => {
                if let Ok(json) = resp.json::<PriceApiResp>().await {
                    let mut guard = cache_clone.write().await;
                    for (sym, data) in json.data {
                        let price_f = data.price.parse::<f64>().unwrap_or(0.0);
                        let base_sym = if sym.eq_ignore_ascii_case("So11111111111111111111111111111111111111112") {
                            "SOL"
                        } else {
                            sym.as_str()
                        };
                        let pair = TradingPair::new(base_sym, "USDC");
                        log::info!("Price update {} = {}", pair, price_f);
                        guard.insert(pair, price_f);
                    }
                }
            }
            Err(e) => {
                log::warn!("Init price fetch failed: {} – trying Coingecko", e);
                // Fallback: only fetch SOL price for now
                if ids_param.contains("SOL") {
                    if let Ok(r2) = reqwest::get("https://api.coingecko.com/api/v3/simple/price?ids=solana&vs_currencies=usd").await {
                        if let Ok(j) = r2.json::<serde_json::Value>().await {
                            if let Some(price) = j.get("solana").and_then(|v| v.get("usd")).and_then(|p| p.as_f64()) {
                                let mut guard = cache_clone.write().await;
                                guard.insert(TradingPair::new("SOL","USDC"), price);
                            }
                        }
                    }
                }
            }
        }
        
        loop {
            let url = format!("https://lite-api.jup.ag/price/v2?ids={}", ids_param);
            match reqwest::get(&url).await {
                Ok(resp) => {
                    if let Ok(json) = resp.json::<PriceApiResp>().await {
                        let mut guard = cache_clone.write().await;
                        for (sym, data) in json.data {
                            let price_f = data.price.parse::<f64>().unwrap_or(0.0);
                            let pair = TradingPair::new(&sym, "USDC");
                            log::info!("Price update {} = {}", pair, price_f);
                            guard.insert(pair, price_f);
                        }
                    } else {
                        log::warn!("Price API: failed to parse JSON");
                    }
                }
                Err(e) => {
                    log::warn!("Jupiter price API request error: {} — falling back to Coingecko", e);
                    // Very limited Coingecko fallback (SOL/USD only for now)
                    if ids_param.contains("SOL") {
                        if let Ok(r2) = reqwest::get("https://api.coingecko.com/api/v3/simple/price?ids=solana&vs_currencies=usd").await {
                            if let Ok(j) = r2.json::<serde_json::Value>().await {
                                if let Some(price) = j.get("solana").and_then(|v| v.get("usd")).and_then(|p| p.as_f64()) {
                                    let mut guard = cache_clone.write().await;
                                    guard.insert(TradingPair::new("SOL","USDC"), price);
                                }
                            }
                        }
                    }
                },
            }
            tokio::time::sleep(Duration::from_secs(4)).await;
        }
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

// ----- HTTP Price API response structs -----
#[derive(Debug, Deserialize)]
struct PriceApiRespEntry {
    price: String,
}

#[derive(Debug, Deserialize)]
struct PriceApiResp {
    data: std::collections::HashMap<String, PriceApiRespEntry>,
}
