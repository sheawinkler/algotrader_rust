use crate::utils::types::{MarketData, TradingPair};
use crate::Result;
use super::HistoricalDataProvider;
use csv::ReaderBuilder;
use serde::Deserialize;
use std::path::PathBuf;

/// Simple CSV row matching the extended MarketData struct
#[derive(Debug, Deserialize)]
struct CsvRow {
    timestamp: i64,
    open: Option<f64>,
    high: Option<f64>,
    low: Option<f64>,
    close: Option<f64>,
    volume: Option<f64>,
}

/// CSV provider that reads OHLCV rows into `MarketData` records
pub struct CSVHistoricalDataProvider;

impl HistoricalDataProvider for CSVHistoricalDataProvider {
    fn load(&self, data_file: &PathBuf) -> Result<Vec<MarketData>> {
        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .from_path(data_file)
            .map_err(|e| crate::Error::DataError(format!("CSV read error: {e}")))?;
        let mut out = Vec::new();
        for rec in rdr.deserialize::<CsvRow>() {
            let row = rec
                .map_err(|e| crate::Error::DataError(format!("CSV parse error: {e}")))?;
            out.push(MarketData {
                pair: TradingPair::new("UNK", "UNK"),
                symbol: "UNK/UNK".to_string(),
                candles: Vec::new(),
                last_price: row.close.unwrap_or(0.0),
                volume_24h: 0.0,
                change_24h: 0.0,
                volume: row.volume,
                timestamp: row.timestamp,
                open: row.open,
                high: row.high,
                low: row.low,
                close: row.close.unwrap_or(0.0),
                order_book: None,
                dex_prices: None,
            });
        }
        Ok(out)
    }
}
