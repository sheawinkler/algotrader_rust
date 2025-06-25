//! Persistence layer traits and implementations
//!
//! This module defines a thin abstraction so the core trading
//! engine / backtester can be decoupled from the concrete storage
//! backend.  The goal is to support multiple implementations
//! (SQLite via SeaORM to start, Postgres later).

use async_trait::async_trait;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

/// A minimal representation of a trade suitable for persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    pub id: Option<i64>,
    pub timestamp: NaiveDateTime,
    pub symbol: String,
    pub side: String,
    pub qty: f64,
    pub price: f64,
    pub pnl: f64,
}

/// A snapshot of the portfolio equity curve at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquitySnapshot {
    pub id: Option<i64>,
    pub timestamp: NaiveDateTime,
    pub equity: f64,
}

/// High-level summary produced by a backtest run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestSummary {
    pub id: Option<i64>,
    pub strategy: String,
    pub timeframe: String,
    pub start_balance: f64,
    pub end_balance: f64,
    pub sharpe: f64,
    pub max_drawdown: f64,
}

#[async_trait]
pub trait Persistence: Send + Sync {
    /// Persist a single trade fill.
    async fn save_trade(&self, trade: &TradeRecord) -> anyhow::Result<()>;

    /// Persist an equity snapshot.
    async fn save_snapshot(&self, snap: &EquitySnapshot) -> anyhow::Result<()>;

    /// Persist a completed backtest summary.
    async fn save_backtest(&self, rpt: &BacktestSummary) -> anyhow::Result<()>;

    /// Flush / close any outstanding connections.
    async fn flush(&self) -> anyhow::Result<()>;
}

pub mod sqlite;

/// A no-op implementation useful in unit tests or if the user
/// has not configured a persistence backend.
#[derive(Clone, Default)]
pub struct NullPersistence;

#[async_trait]
impl Persistence for NullPersistence {
    async fn save_trade(&self, _t: &TradeRecord) -> anyhow::Result<()> { Ok(()) }
    async fn save_snapshot(&self, _s: &EquitySnapshot) -> anyhow::Result<()> { Ok(()) }
    async fn save_backtest(&self, _b: &BacktestSummary) -> anyhow::Result<()> { Ok(()) }
    async fn flush(&self) -> anyhow::Result<()> { Ok(()) }
}
