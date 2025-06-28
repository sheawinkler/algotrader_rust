//! Data layer abstraction for AlgoTraderV2
//!
//! This module is gated behind the `db` feature flag and provides
//! async helpers for interacting with our primary data stores:
//!     • TimescaleDB (PostgreSQL)  – tick + trade history
//!     • Redis                     – hot cache / live order-book
//!     • ClickHouse                – heavy analytics & back-test results
//!
//! NOTE: only scaffolding is provided for now so that the crate
//! compiles; concrete query helpers will be filled in incrementally.

#![cfg(feature = "db")]

use anyhow::{Context, Result};
use clickhouse_rs::Pool as ChPool;

use redis::aio::Connection as RedisConn;
use redis::Client as RedisClient;
use tokio_postgres::NoTls;

/// Aggregated handle exposing typed clients for all data stores.
pub struct DataLayer {
    /// TimescaleDB client (pgvector enabled)
    pub pg: tokio_postgres::Client,
    /// Redis async connection
    pub redis: RedisConn,
    /// ClickHouse async pool
    pub clickhouse: ChPool,
}

impl DataLayer {
    /// Build a [`DataLayer`] instance from connection strings.
    pub async fn initialise(pg_url: &str, redis_url: &str, clickhouse_url: &str) -> Result<Self> {
        // --- Postgres / TimescaleDB -----------------------------
        let (pg_client, pg_connection) = tokio_postgres::connect(pg_url, NoTls)
            .await
            .context("failed to connect to postgres")?;
        // Spawn the connection driver so it runs independently
        tokio::spawn(async move {
            if let Err(e) = pg_connection.await {
                eprintln!("Postgres connection error: {e}");
            }
        });

        // --- Redis ---------------------------------------------
        let redis_client = RedisClient::open(redis_url)?;
        let redis = redis_client
            .get_async_connection()
            .await
            .context("failed to connect to redis")?;

        // --- ClickHouse ----------------------------------------
        let clickhouse = ChPool::new(clickhouse_url);

        Ok(Self { pg: pg_client, redis, clickhouse })
    }
}

