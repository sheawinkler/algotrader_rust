//! Unified market data stream trait and event types for multi-source integration

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarketEvent {
    Trade {
        exchange: String,
        symbol: String,
        price: f64,
        qty: f64,
        side: String,
        timestamp: i64,
    },
    OrderBook {
        exchange: String,
        symbol: String,
        bids: Vec<(f64, f64)>,
        asks: Vec<(f64, f64)>,
        timestamp: i64,
    },
    Ticker {
        exchange: String,
        symbol: String,
        price: f64,
        timestamp: i64,
    },
    // Extend for other event types as needed
}

#[async_trait]
pub trait ChannelMarketDataStream {
    async fn connect_and_stream_channel(
        &mut self, symbols: Vec<String>, sender: tokio::sync::mpsc::Sender<MarketEvent>,
    ) -> Result<()>;
}
