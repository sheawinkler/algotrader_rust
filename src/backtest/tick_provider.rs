use crate::utils::types::{MarketData, TradingPair};
use crate::Result;
use super::HistoricalDataProvider;
use csv::ReaderBuilder;
use crate::backtest::cache;
use std::fs;
use std::io::Cursor;
use serde::Deserialize;
use std::path::PathBuf;

/// CSV schema for tick data: timestamp,price,qty
#[derive(Debug, Deserialize)]
struct TickRow {
    timestamp: i64,
    price: f64,
    qty: f64,
}

#[derive(Clone)]
pub struct CSVTicksProvider;

impl CSVTicksProvider {
    pub fn new() -> Self { Self }
}

impl HistoricalDataProvider for CSVTicksProvider {
    fn load(&self, data_file: &PathBuf) -> Result<Vec<MarketData>> {
        let no_cache = std::env::var("BACKTEST_NO_CACHE").is_ok();
        let key = cache::build_key(&["tickcsv", data_file.to_string_lossy().as_ref()]);
        let raw_bytes = if !no_cache {
            if let Some(b) = cache::get_raw(&key)? { b } else { fs::read(data_file)? }
        } else { fs::read(data_file)? };
        if !no_cache { cache::put_raw(&key, &raw_bytes).ok(); }
        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(Cursor::new(raw_bytes));
        let mut out = Vec::new();
        for rec in rdr.deserialize::<TickRow>() {
            let row = rec.map_err(|e| crate::Error::DataError(format!("CSV parse error: {e}")))?;
            out.push(MarketData {
                pair: TradingPair::new("UNK", "UNK"),
                symbol: "UNK/UNK".to_string(),
                candles: Vec::new(),
                last_price: row.price,
                volume_24h: 0.0,
                change_24h: 0.0,
                volume: Some(row.qty),
                timestamp: row.timestamp,
                open: Some(row.price),
                high: Some(row.price),
                low: Some(row.price),
                close: row.price,
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
