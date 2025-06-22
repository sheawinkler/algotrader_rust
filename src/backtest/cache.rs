//! Backtest cache using sled key-value store
//! Persists `BacktestReport`s to avoid redundant heavy computations.

use crate::backtest::BacktestReport;
use crate::{Error, Result};
use sled::Db;

/// Lightweight embedded cache for backtest results.
/// Uses [`sled`](https://crates.io/crates/sled) which is an embedded, high-performance KV store.
#[derive(Clone)]
pub struct BacktestCache {
    db: Db,
}

impl BacktestCache {
    /// Open or create a cache database at the specified path.
    pub fn open(path: &str) -> Result<Self> {
        let db = sled::open(path)
            .map_err(|e| Error::DataError(format!("Cache open error: {e}")))?;
        Ok(Self { db })
    }

    fn key(
        strategy: &str,
        symbol: &str,
        timeframe: &str,
        start_ts: i64,
        end_ts: i64,
    ) -> Vec<u8> {
        format!("{strategy}:{symbol}:{timeframe}:{start_ts}:{end_ts}").into_bytes()
    }

    /// Retrieve a cached report if present.
    pub fn get(
        &self,
        strategy: &str,
        symbol: &str,
        timeframe: &str,
        start_ts: i64,
        end_ts: i64,
    ) -> Result<Option<BacktestReport>> {
        let key = Self::key(strategy, symbol, timeframe, start_ts, end_ts);
        match self.db.get(key).map_err(|e| Error::DataError(format!("Cache read error: {e}")))? {
            Some(ivec) => {
                let report: BacktestReport = bincode::deserialize(&ivec)
                    .map_err(|e| Error::DataError(format!("Cache deserialize error: {e}")))?;
                Ok(Some(report))
            }
            None => Ok(None),
        }
    }

    /// Insert a new report into the cache.
    pub fn insert(
        &self,
        strategy: &str,
        symbol: &str,
        timeframe: &str,
        start_ts: i64,
        end_ts: i64,
        report: &BacktestReport,
    ) -> Result<()> {
        let key = Self::key(strategy, symbol, timeframe, start_ts, end_ts);
        let bytes = bincode::serialize(report)
            .map_err(|e| Error::DataError(format!("Cache serialize error: {e}")))?;
        self.db
            .insert(key, bytes)
            .map_err(|e| Error::DataError(format!("Cache write error: {e}")))?;
        Ok(())
    }
}
