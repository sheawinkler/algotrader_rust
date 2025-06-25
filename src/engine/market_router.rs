//! MarketRouter: orchestrates multiple MarketDataStream sources and routes MarketEvents to trading logic

use crate::utils::market_stream::MarketEvent;
use tokio::sync::mpsc::Sender;

/// Trait for trading logic that can handle MarketEvents
pub trait MarketEventHandler: Send + Sync + 'static {
    fn on_market_event(&self, event: MarketEvent);
}

/// New trait for streams that can send events via channel
#[async_trait::async_trait]
pub trait ChannelMarketDataStream: Send + Sync {
    async fn connect_and_stream_channel(
        &mut self, symbols: Vec<String>, sender: Sender<MarketEvent>,
    ) -> anyhow::Result<()>;
}

/// MarketRouter manages multiple market data streams and routes events to the trading engine
pub struct MarketRouter {
    // Event streams feeding market data
    streams: Vec<Box<dyn ChannelMarketDataStream + Send + Sync>>,
}

impl Default for MarketRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl MarketRouter {
    pub fn new() -> Self {
        Self { streams: Vec::new() }
    }

    pub fn add_stream(&mut self, stream: Box<dyn ChannelMarketDataStream + Send + Sync>) {
        self.streams.push(stream);
    }

    pub async fn run(
        &mut self, symbols: Vec<String>, sender: Sender<MarketEvent>,
    ) -> anyhow::Result<()> {
        let mut handles = Vec::new();
        // Drain the streams so each is moved into its own task
        for mut stream in self.streams.drain(..) {
            let s = symbols.clone();
            let tx = sender.clone();
            handles
                .push(tokio::spawn(async move { stream.connect_and_stream_channel(s, tx).await }));
        }
        for handle in handles {
            handle.await??;
        }
        Ok(())
    }
}
