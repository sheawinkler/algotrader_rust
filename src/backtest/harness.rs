//! Walk-forward optimisation harness
//! Iteratively runs backtests on rolling train/test windows and aggregates reports.

use crate::backtest::{Backtester, SimMode, HistoricalDataProvider, providers::CSVHistoricalDataProvider, tick_provider::CSVTicksProvider};
use std::sync::Arc;
use crate::persistence;
use crate::strategies::{MeanReversionStrategy, TradingStrategy, TimeFrame};
use crate::Result;
use crate::utils::types::MarketData;
use tempfile::NamedTempFile;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Simple config expressed in days for training & testing spans
pub struct WalkForwardConfig {
    pub train_days: i64,
    pub test_days: i64,
    pub step_days: i64, // slide step between windows
}

/// Run walk-forward analysis on a single data CSV file.
/// Returns backtest reports for each test window.
/// NOTE: This uses a temporary filtered CSV per window for simplicity.
/// Run walk-forward analysis on a single data CSV file.
/// `data_path` is borrowed as a `Path` (no needless `PathBuf`).
pub async fn run_walk_forward(
    data_path: &Path,
    timeframe: &str,
    sim_mode: SimMode,
    cfg: WalkForwardConfig,
) -> Result<Vec<crate::backtest::BacktestReport>> {
    // Load full dataset once (bar or tick provider just for timestamps)
    let provider: Box<dyn HistoricalDataProvider> = match sim_mode {
        SimMode::Bar => Box::new(CSVHistoricalDataProvider::new()),
        SimMode::Tick => Box::new(CSVTicksProvider::new()),
    };
    let all_data = provider.load(data_path)?;
    if all_data.is_empty() { return Err(crate::Error::DataError("empty dataset".into())); }

    let mut reports: Vec<crate::backtest::BacktestReport> = Vec::new();
    let start_ts = all_data.first().unwrap().timestamp;
    let end_ts = all_data.last().unwrap().timestamp;
    let mut window_start = start_ts;
    let train_secs = cfg.train_days * 86_400;
    let test_secs = cfg.test_days * 86_400;
    let step_secs = cfg.step_days * 86_400;

    while window_start + train_secs + test_secs <= end_ts {
        let train_end = window_start + train_secs;
        let test_end = train_end + test_secs;
        // Extract test slice
        let test_slice: Vec<&MarketData> = all_data.iter()
            .filter(|d| d.timestamp >= train_end && d.timestamp < test_end)
            .collect();
        if test_slice.is_empty() {
            window_start += step_secs;
            continue;
        }
        // Write temp CSV for provider reuse
        let mut tmp = NamedTempFile::new()?;
        // assume MarketData close column exists
        writeln!(tmp, "timestamp,close")?;
        for d in &test_slice {
            writeln!(tmp, "{},{}", d.timestamp, d.close)?;
        }
        let tmp_path = tmp.path().to_path_buf();

        // Build a fresh backtester similar to simple_backtest but capture the report
        let provider: Box<dyn HistoricalDataProvider> = match sim_mode {
            SimMode::Bar => Box::new(CSVHistoricalDataProvider::new()),
            SimMode::Tick => Box::new(CSVTicksProvider::new()),
        };
        let strategies: Vec<Box<dyn TradingStrategy>> = vec![
            Box::new(MeanReversionStrategy::new("UNK/UNK", TimeFrame::OneHour, 20, 2.0, 2.0, 1.0)),
        ];
        let mut bt = Backtester {
            risk_rules: vec![
                Box::new(crate::risk::StopLossRule::new(0.05)),
                Box::new(crate::risk::TakeProfitRule::new(0.10)),
            ],
            data_provider: provider,
            timeframe: timeframe.to_string(),
            starting_balance: 10_000.0,
            strategies,
            cache: None,
            sim_mode,
            slippage_bps: 0,
            fee_bps: 8,
            persistence: Some(Arc::new(persistence::NullPersistence)),
        };
        let rpt = bt.run(tmp_path.as_path()).await?;
        reports.push(rpt);
        window_start += step_secs;
    }
    Ok(reports)
}
