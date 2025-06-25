//! Simple helper that downloads historical OHLCV data via a `RemoteHistoricalDataProvider`
//! and saves it to a CSV file compatible with the existing CSVHistoricalDataProvider.

use crate::backtest::remote_provider::{CryptoCompareProvider, BirdeyeProvider, RemoteHistoricalDataProvider};
use crate::Result;

/// Download candles for `symbol=BASE/QUOTE` and save to `output_csv`.
///
/// * `timeframe` – Accepted values map to CryptoCompare endpoints: "1d", "1h", otherwise minutes.
/// * `limit` – Number of candles to request (CryptoCompare max 2000).
pub async fn download_to_csv(
    base: &str,
    quote: &str,
    timeframe: &str,
    limit: usize,
    output_csv: &std::path::Path,
) -> Result<()> {
    // Try Birdeye first if API key present, fallback to CryptoCompare
    let mut candles = Vec::new();
    if std::env::var("BIRDEYE_API_KEY").is_ok() {
        let birdeye = BirdeyeProvider::new();
        match birdeye.fetch(base, quote, timeframe, limit).await {
            Ok(c) if !c.is_empty() => {
                candles = c;
            },
            Err(e) => {
                log::warn!("Birdeye fetch failed: {} – falling back to CryptoCompare", e);
            },
            _ => {},
        }
    }
    if candles.is_empty() {
        let cc = CryptoCompareProvider::new();
        candles = cc.fetch(base, quote, timeframe, limit).await?;
    }

    // Ensure parent dir exists
    if let Some(p) = output_csv.parent() {
        std::fs::create_dir_all(p)?;
    }

    let mut wtr = csv::WriterBuilder::new()
        .has_headers(true)
        .from_path(output_csv)?;

    // Write header row compatible with CsvRow struct in providers.rs
    wtr.write_record(["timestamp", "open", "high", "low", "close", "volume"])?;
    for c in candles {
        wtr.serialize((
            c.timestamp,
            c.open.unwrap_or(0.0),
            c.high.unwrap_or(0.0),
            c.low.unwrap_or(0.0),
            c.close,
            c.volume.unwrap_or(0.0),
        ))?;
    }
    wtr.flush()?;
    Ok(())
}
