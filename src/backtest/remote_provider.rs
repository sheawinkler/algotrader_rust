use async_trait::async_trait;
use serde_json::Value;

use crate::utils::types::{MarketData, TradingPair};
use crate::Result;

/// Trait for providers that fetch historical market data over the network.
#[async_trait]
pub trait RemoteHistoricalDataProvider: Send + Sync {
    async fn fetch(&self, base: &str, quote: &str, timeframe: &str, limit: usize) -> Result<Vec<MarketData>>;
}

/// Helper that only resolves a token symbol to its mint address using DexScreener
#[derive(Clone, Default)]
pub struct DexScreenerResolver;
impl DexScreenerResolver {
    pub async fn resolve_mint(&self, symbol: &str, quote: &str) -> Result<String> {
        // Build list of candidate quote tokens in priority order
        let mut quotes: Vec<String> = vec![quote.to_string()];
        for q in &["USDC", "USD", "USDT", "SOL"] {
            if !quotes.iter().any(|s| s.eq_ignore_ascii_case(q)) {
                quotes.push(q.to_string());
            }
        }

        // Keep best pair found across all quote attempts
        let mut best_pair: Option<Value> = None;
        let mut best_score = 0f64;

        for q in &quotes {
            let query = format!("{}/{}", symbol, q);
            let url = format!("https://api.dexscreener.com/latest/dex/search/?q={}", query);
            let resp = reqwest::get(&url).await?.error_for_status()?;
            let v: Value = resp.json().await?;
            if let Some(arr) = v.get("pairs").and_then(|v| v.as_array()) {
                for p in arr {
                    // Compute score
                    let vol = p["volume"]["h24"].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                    let buys = p["txns"]["h24"]["buys"].as_i64().unwrap_or(0) as f64;
                    let sells = p["txns"]["h24"]["sells"].as_i64().unwrap_or(0) as f64;
                    let score = vol * (buys + sells).max(1.0);
                    if score > best_score {
                        best_score = score;
                        best_pair = Some(p.clone());
                    }
                }
            }
            if best_score > 0.0 {
                // Good enough pair found with preferred quote, break early
                if q.eq_ignore_ascii_case(quote) {
                    break;
                }
            }
        }

        // As last resort search by symbol only
        if best_pair.is_none() {
            let url = format!("https://api.dexscreener.com/latest/dex/search/?q={}", symbol);
            let resp = reqwest::get(&url).await?.error_for_status()?;
            let v: Value = resp.json().await?;
            if let Some(arr) = v.get("pairs").and_then(|v| v.as_array()) {
                for p in arr {
                    let vol = p["volume"]["h24"].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                    let buys = p["txns"]["h24"]["buys"].as_i64().unwrap_or(0) as f64;
                    let sells = p["txns"]["h24"]["sells"].as_i64().unwrap_or(0) as f64;
                    let score = vol * (buys + sells).max(1.0);
                    if score > best_score {
                        best_score = score;
                        best_pair = Some(p.clone());
                    }
                }
            }
        }

        let pair = best_pair.ok_or_else(|| crate::Error::DataError("no suitable pair found on DexScreener".into()))?;
        let mint = pair["baseToken"]["address"].as_str().ok_or_else(|| crate::Error::DataError("missing base address in DexScreener response".into()))?;
        Ok(mint.to_string())
    }

/* DUPLICATE LEGACY BLOCK REMOVED */
/*
            // Try symbol alone when symbol/quote returns no pairs
            let url = format!("https://api.dexscreener.com/latest/dex/search/?q={}", symbol);
            let resp = reqwest::get(&url).await?.error_for_status()?;
            let v: Value = resp.json().await?;
            let pairs = v["pairs"].as_array().ok_or_else(|| crate::Error::DataError("dexscreener malformed json".into()))?;
            if pairs.is_empty() {
                return Err(crate::Error::DataError("symbol not found on DexScreener".into()));
            }
            // rank by 24h volume * 24h txns (buys+sells)
            let mut best_any = None;
            let mut best_any_score = 0f64;
            for p in pairs {
                let vol = p["volume"]["h24"].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                let buys = p["txns"]["h24"]["buys"].as_i64().unwrap_or(0) as f64;
                let sells = p["txns"]["h24"]["sells"].as_i64().unwrap_or(0) as f64;
                let score = vol * (buys + sells).max(1.0);
                if score > best_any_score {
                    best_any_score = score;
                    best_any = Some(p.clone());
                }
            }
            let best_pair = best_any.ok_or_else(|| crate::Error::DataError("no suitable pair found".into()))?;
            let mint = best_pair["baseToken"]["address"].as_str().ok_or_else(|| crate::Error::DataError("missing base address in DexScreener response".into()))?;
            Ok(mint.to_string())
        } else {
            // rank by 24h volume * 24h txns (buys+sells)
            let mut best_any = None;
            let mut best_any_score = 0f64;
            let mut best_with_quote = None;
            let mut best_with_quote_score = 0f64;
            for p in pairs {
                let vol = p["volume"]["h24"].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                let buys = p["txns"]["h24"]["buys"].as_i64().unwrap_or(0) as f64;
                let sells = p["txns"]["h24"]["sells"].as_i64().unwrap_or(0) as f64;
                let score = vol * (buys + sells).max(1.0);
                if score > best_any_score {
                    best_any_score = score;
                    best_any = Some(p.clone());
                }
                let quote_sym = p["quoteToken"]["symbol"].as_str().unwrap_or("");
                if quote_sym.eq_ignore_ascii_case(quote) && score > best_with_quote_score {
                    best_with_quote_score = score;
                    best_with_quote = Some(p.clone());
                }
            }
            let best_pair = best_with_quote.or(best_any).ok_or_else(|| crate::Error::DataError("no suitable pair found".into()))?;
            let mint = best_pair["baseToken"]["address"].as_str().ok_or_else(|| crate::Error::DataError("missing base address in DexScreener response".into()))?;
            */
}

/// Historical data provider using Birdeye OHLCV endpoint (needs API key)
#[derive(Clone, Default)]
pub struct BirdeyeProvider {
    api_key: String,
    resolver: DexScreenerResolver,
}
impl BirdeyeProvider {
    pub fn new() -> Self {
        let key = std::env::var("BIRDEYE_API_KEY").unwrap_or_default();
        Self { api_key: key, resolver: DexScreenerResolver::default() }
    }
}

#[derive(serde::Deserialize)]
struct BirdeyeCandle {
    #[serde(rename = "o")] open: f64,
    #[serde(rename = "h")] high: f64,
    #[serde(rename = "l")] low: f64,
    #[serde(rename = "c")] close: f64,
    #[serde(rename = "v")] volume: f64,
    #[serde(rename = "unixTime")] timestamp: i64,
}

#[async_trait]
impl RemoteHistoricalDataProvider for BirdeyeProvider {
    async fn fetch(&self, base: &str, quote: &str, timeframe: &str, limit: usize) -> Result<Vec<MarketData>> {
        let mint = self.resolver.resolve_mint(base, quote).await?;
        let interval = match timeframe {
            "1d" | "1day" | "day" | "D" => "1d",
            "1h" | "hour" | "H" => "1h",
            _ => "5m",
        };
        let limit = limit.min(1000);
        let url = format!("https://public-api.birdeye.so/defi/ohlcv?address={}&interval={}&limit={}", mint, interval, limit);
        let resp = reqwest::Client::new()
            .get(&url)
            .header("X-API-KEY", &self.api_key)
            .send()
            .await?
            .error_for_status()?;
        let v: Value = resp.json().await?;
        if !v.get("success").and_then(|b| b.as_bool()).unwrap_or(false) {
            let msg = v.get("message").and_then(|m| m.as_str()).unwrap_or("unknown");
            return Err(crate::Error::DataError(format!("Birdeye API error: {}", msg)));
        }
        let data = v["data"].as_array().ok_or_else(|| crate::Error::DataError("missing data in Birdeye response".into()))?;
        let out = data
            .iter()
            .filter_map(|val| serde_json::from_value::<BirdeyeCandle>(val.clone()).ok())
            .map(|c| MarketData {
                pair: TradingPair::new(base, quote),
                symbol: format!("{}/{}", base, quote),
                candles: Vec::new(),
                last_price: c.close,
                volume_24h: 0.0,
                change_24h: 0.0,
                volume: Some(c.volume),
                timestamp: c.timestamp,
                open: Some(c.open),
                high: Some(c.high),
                low: Some(c.low),
                close: c.close,
                order_book: None,
                dex_prices: None,
            })
            .collect();
        Ok(out)
    }
}




/// Historical data provider based on the public CryptoCompare REST API.
///
/// An optional API key can be supplied via the `CRYPTOCOMPARE_API_KEY` env var to
/// unlock higher rate-limits. Only basic OHLCV candles are returned and mapped
/// into the project’s `MarketData` struct so they can be consumed by the
/// back-testing engine.
#[derive(Clone, Default)]
pub struct CryptoCompareProvider {
    api_key: Option<String>,
}

impl CryptoCompareProvider {
    /// Create a new provider instance. Looks for `CRYPTOCOMPARE_API_KEY` in the
    /// environment but works fine without one (public tier).
    pub fn new() -> Self {
        Self { api_key: std::env::var("CRYPTOCOMPARE_API_KEY").ok() }
    }
}

#[derive(serde::Deserialize)]
struct CcResponse {
    #[serde(rename = "Response")] // "Success" | "Error"
    status: String,
    #[serde(rename = "Message", default)]
    message: String,
    #[serde(rename = "Data", default)]
    data: serde_json::Value,
}

#[derive(serde::Deserialize)]
struct CcCandle {
    time: i64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    #[serde(rename = "volumefrom")]
    volume_from: f64,
    #[serde(rename = "volumeto")]
    volume_to: f64,
}

#[async_trait]
impl RemoteHistoricalDataProvider for CryptoCompareProvider {
    async fn fetch(&self, base: &str, quote: &str, timeframe: &str, limit: usize) -> Result<Vec<MarketData>> {
        // Determine endpoint and aggregation based on timeframe string
        let (endpoint, aggregate) = match timeframe.to_lowercase().as_str() {
            "1d" | "1day" | "day" | "d" => ("histoday", 1),
            "1h" | "hour" | "h" => ("histohour", 1),
            tf if tf.ends_with('m') => {
                // parse "5m", "15m" etc.
                let num = tf.trim_end_matches('m').parse::<u32>().unwrap_or(1).max(1);
                ("histominute", num)
            },
            _ => ("histominute", 1),
        };

        // CryptoCompare limits: max 2000 candles per call on most tiers.
        let limit = limit.min(2000);
        let mut url = format!(
            "https://min-api.cryptocompare.com/data/{}?fsym={}&tsym={}&limit={}&aggregate={}",
            endpoint, base, quote, limit, aggregate
        );
        if let Some(key) = &self.api_key {
            url.push_str("&api_key=");
            url.push_str(key);
        }

        let resp = reqwest::get(&url).await?.error_for_status()?;
        let payload: CcResponse = resp.json().await?;
        if payload.status != "Success" {
            return Err(crate::Error::DataError(format!("CryptoCompare API error: {}", payload.message).into()));
        }

        let candles_json = payload.data.get("Data").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let out = candles_json
            .into_iter()
            .filter_map(|val| serde_json::from_value::<CcCandle>(val).ok())
            .map(|c| MarketData {
                pair: TradingPair::new(base, quote),
                symbol: format!("{}/{}", base, quote),
                candles: Vec::new(),
                last_price: c.close,
                volume_24h: 0.0,
                change_24h: 0.0,
                volume: Some(c.volume_to),
                timestamp: c.time,
                open: Some(c.open),
                high: Some(c.high),
                low: Some(c.low),
                close: c.close,
                order_book: None,
                dex_prices: None,
            })
            .collect();
        Ok(out)
    }
}
