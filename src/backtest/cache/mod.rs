//! Backtest cache module
//! Extends existing report caching to also persist raw market data blobs.
//! Uses `sled` embedded database under $HOME/.algotrader/cache.


use crate::{Error, Result};
use sled::Db;



const DB_SUBDIR: &str = ".algotrader/cache";

fn open_db() -> Result<sled::Db> {
        let mut dir = dirs::home_dir().ok_or_else(|| Error::DataError("cannot find home dir".into()))?;
    dir.push(DB_SUBDIR);
    std::fs::create_dir_all(&dir).map_err(|e| Error::DataError(format!("mkdir error: {e}")))?;
    sled::open(dir).map_err(|e| Error::DataError(format!("sled open error: {e}")))
}

/// Store compressed blob (zstd) keyed by provided key
pub fn put_raw(key: &str, bytes: &[u8]) -> Result<()> {
    let db = open_db()?;
    let compressed = zstd::encode_all(bytes, 1).map_err(|e| Error::DataError(format!("zstd encode error: {e}")))?;
    db.insert(key.as_bytes(), compressed).map_err(|e| Error::DataError(format!("sled write error: {e}")))?;
    db.flush().map_err(|e| Error::DataError(format!("sled flush error: {e}")))?;
    Ok(())
}

/// Retrieve compressed blob if exists
pub fn get_raw(key: &str) -> Result<Option<Vec<u8>>> {
    let db = open_db()?;
    if let Some(val) = db.get(key.as_bytes()).map_err(|e| Error::DataError(format!("sled get error: {e}")))? {
        let decompressed = zstd::decode_all(val.as_ref()).map_err(|e| Error::DataError(format!("zstd decode error: {e}")))?;
        Ok(Some(decompressed))
    } else {
        Ok(None)
    }
}

/// ---------------------------------------------------------------------------
/// Report cache for BacktestReport (was previously in cache.rs)
/// ---------------------------------------------------------------------------

use crate::backtest::BacktestReport;

/// Lightweight embedded cache for backtest results.
#[derive(Clone)]
pub struct BacktestCache {
    db: Db,
}

impl BacktestCache {
    pub fn open(path: &str) -> Result<Self> {
        let db = sled::open(path).map_err(|e| Error::DataError(format!("Cache open error: {e}")))?;
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

    pub fn get(&self, strategy: &str, symbol: &str, timeframe: &str, start_ts: i64, end_ts: i64) -> Result<Option<BacktestReport>> {
        let key = Self::key(strategy, symbol, timeframe, start_ts, end_ts);
        if let Some(ivec) = self.db.get(key).map_err(|e| Error::DataError(format!("Cache read error: {e}")))? {
            let report: BacktestReport = bincode::deserialize(&ivec).map_err(|e| Error::DataError(format!("Cache deserialize error: {e}")))?;
            Ok(Some(report))
        } else { Ok(None) }
    }

    pub fn insert(&self, strategy: &str, symbol: &str, timeframe: &str, start_ts: i64, end_ts: i64, report: &BacktestReport) -> Result<()> {
        let key = Self::key(strategy, symbol, timeframe, start_ts, end_ts);
        let bytes = bincode::serialize(report).map_err(|e| Error::DataError(format!("Cache serialize error: {e}")))?;
        self.db.insert(key, bytes).map_err(|e| Error::DataError(format!("Cache write error: {e}")))?;
        Ok(())
    }
}

/// Utility to build a cache key from arbitrary strings (source,symbol,tf,etc.)
pub fn build_key(parts: &[&str]) -> String {
    use sha2::{Digest, Sha256};
    let concat = parts.join("|");
    let mut hasher = Sha256::new();
    hasher.update(concat.as_bytes());
    format!("{:x}", hasher.finalize())
}
