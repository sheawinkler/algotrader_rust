//! Simple fan-in hub that receives `(symbol, price)` ticks from many SignalSources,
//! updates the shared PriceCache, and optionally persists to TimescaleDB.

use crate::market_data::ws::PriceCache;
use crate::utils::types::TradingPair;
#[cfg(feature = "db")]
use tokio_postgres::Client as PgClient;
use async_trait::async_trait;
#[cfg(feature = "db")]
use tokio_postgres::types::ToSql;
#[cfg(feature = "db")]
use clickhouse_rs::Pool as ChPool;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::RwLock;

pub struct SignalHub {
    pub rx: UnboundedReceiver<(String, f64)>,
    pub price_cache: PriceCache,
    #[cfg(feature = "db")]
    pub pg: Option<std::sync::Arc<PgClient>>, // optional DB sink
    #[cfg(feature = "db")]
    pub ch: Option<ChPool>, // optional ClickHouse sink
}

impl SignalHub {
    pub async fn run(mut self) {
        while let Some((sym, price)) = self.rx.recv().await {
            let pair = TradingPair::new(&sym, "USDC");
            {
                let mut guard = self.price_cache.write().await;
                guard.insert(pair.clone(), price);
            }
            #[cfg(feature = "db")]
            if let Some(pg) = &self.pg {
                let pair_str = format!("{}/USDC", sym);
                let _ = (**pg)
                    .execute(
                        "INSERT INTO price_ticks (pair, price, ts) VALUES ($1, $2, now())",
                        &[&pair_str as &(dyn ToSql + Sync), &price],
                    )
                    .await;
            }
            #[cfg(feature = "db")]
            if let Some(pool) = &self.ch {
                let pair_str = format!("{}/USDC", sym);
                if let Ok(mut conn) = pool.get_handle().await {
                    let q = format!("INSERT INTO signals_metrics (pair, value, ts) VALUES ('{}', {}, now())", pair_str, price);
                    let _ = conn.execute(q).await;
                }
            }
        }
    }
}
