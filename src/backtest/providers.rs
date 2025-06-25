use super::HistoricalDataProvider;
use crate::backtest::cache;
use crate::utils::types::{MarketData, TradingPair};
use crate::Result;
use csv::ReaderBuilder;
use serde::Deserialize;
use std::fs;
use std::io::Cursor;

use crate::backtest::tick_provider::CSVTicksProvider;
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
#[derive(Clone)]
pub struct CSVHistoricalDataProvider;

impl CSVHistoricalDataProvider {
    pub fn new() -> Self {
        Self
    }
}

impl HistoricalDataProvider for CSVHistoricalDataProvider {
    fn load(&self, data_file: &PathBuf) -> Result<Vec<MarketData>> {
        let no_cache = std::env::var("BACKTEST_NO_CACHE").is_ok();
        let key = cache::build_key(&["csv", data_file.to_string_lossy().as_ref()]);
        let raw_bytes = if !no_cache {
            if let Some(b) = cache::get_raw(&key)? {
                b
            } else {
                fs::read(data_file)?
            }
        } else {
            fs::read(data_file)?
        };
        if !no_cache {
            cache::put_raw(&key, &raw_bytes).ok();
        }
        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(Cursor::new(raw_bytes));
        let mut out = Vec::new();
        for rec in rdr.deserialize::<CsvRow>() {
            let row = rec.map_err(|e| crate::Error::DataError(format!("CSV parse error: {e}")))?;
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

    fn box_clone(&self) -> Box<dyn HistoricalDataProvider> {
        Box::new(self.clone())
    }
}
