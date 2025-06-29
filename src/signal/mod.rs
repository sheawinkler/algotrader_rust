//! Signal ingestion sources – CCXT, sentiment feeds, etc.

pub mod ccxt;
pub mod hub;
pub mod perplexity;
pub mod alpaca;
pub mod armor;
pub mod financial;
pub mod photon;
pub mod solanasniffer;
pub mod websentiment;

use async_trait::async_trait;

/// Trait implemented by any async signal source producing events.
#[async_trait]
pub trait SignalSource {
    /// Run the source until cancelled. Should internally handle retries.
    async fn run(&self) -> anyhow::Result<()>;
}
