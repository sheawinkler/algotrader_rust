//! SQLite persistence backend using `rusqlite`.
//! This is deliberately lightweight – SeaORM can be layered on later.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use rusqlite::{Connection, params};
use chrono::{NaiveDateTime, TimeZone, Utc};
use async_trait::async_trait;

use super::{Persistence, TradeRecord, EquitySnapshot, BacktestSummary};

/// Thread-safe SQLite wrapper shared across async tasks.
#[derive(Clone)]
pub struct SqlitePersistence {
    conn: Arc<Mutex<Connection>>, // wrapped for async use via spawn_blocking
}

impl SqlitePersistence {
    /// Open (or create) the DB file under the user data dir.
    pub async fn new(db_path: Option<PathBuf>) -> anyhow::Result<Self> {
        let path = db_path.unwrap_or_else(|| {
            let mut p = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
            p.push("algotraderv2");
            std::fs::create_dir_all(&p).ok();
            p.push("trades.db");
            p
        });
        let conn = tokio::task::spawn_blocking(move || Connection::open(path)).await??;
        init_schema(&conn)?;
        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }
}

fn init_schema(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;\n
         CREATE TABLE IF NOT EXISTS trade_records (
             id           INTEGER PRIMARY KEY AUTOINCREMENT,
             timestamp    INTEGER NOT NULL,
             symbol       TEXT NOT NULL,
             side         TEXT NOT NULL,
             qty          REAL NOT NULL,
             price        REAL NOT NULL,
             pnl          REAL NOT NULL
         );
         CREATE TABLE IF NOT EXISTS equity_snapshots (
             id           INTEGER PRIMARY KEY AUTOINCREMENT,
             timestamp    INTEGER NOT NULL,
             equity       REAL NOT NULL
         );
         CREATE TABLE IF NOT EXISTS backtests (
             id           INTEGER PRIMARY KEY AUTOINCREMENT,
             strategy     TEXT NOT NULL,
             timeframe    TEXT NOT NULL,
             start_balance REAL NOT NULL,
             end_balance   REAL NOT NULL,
             sharpe        REAL NOT NULL,
             max_drawdown  REAL NOT NULL,
             created_at    INTEGER NOT NULL
         );" )?;
    Ok(())
}

#[async_trait]
impl Persistence for SqlitePersistence {
    async fn save_trade(&self, trade: &TradeRecord) -> anyhow::Result<()> {
        let conn = self.conn.clone();
        let t = trade.clone();
        tokio::task::spawn_blocking(move || {
            let ts = t.timestamp.timestamp();
            conn.lock().unwrap().execute(
                "INSERT INTO trade_records (timestamp, symbol, side, qty, price, pnl) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![ts, t.symbol, t.side, t.qty, t.price, t.pnl],
            )?;
            Ok::<_, rusqlite::Error>(())
        }).await??;
        Ok(())
    }

    async fn save_snapshot(&self, snap: &EquitySnapshot) -> anyhow::Result<()> {
        let conn = self.conn.clone();
        let s = snap.clone();
        tokio::task::spawn_blocking(move || {
            let ts = s.timestamp.timestamp();
            conn.lock().unwrap().execute(
                "INSERT INTO equity_snapshots (timestamp, equity) VALUES (?1, ?2)",
                params![ts, s.equity],
            )?;
            Ok::<_, rusqlite::Error>(())
        }).await??;
        Ok(())
    }

    async fn save_backtest(&self, rpt: &BacktestSummary) -> anyhow::Result<()> {
        let conn = self.conn.clone();
        let r = rpt.clone();
        tokio::task::spawn_blocking(move || {
            let now_ts = Utc::now().timestamp();
            conn.lock().unwrap().execute(
                "INSERT INTO backtests (strategy, timeframe, start_balance, end_balance, sharpe, max_drawdown, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![r.strategy, r.timeframe, r.start_balance, r.end_balance, r.sharpe, r.max_drawdown, now_ts],
            )?;
            Ok::<_, rusqlite::Error>(())
        }).await??;
        Ok(())
    }

    async fn flush(&self) -> anyhow::Result<()> { Ok(()) }
}
