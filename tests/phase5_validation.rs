use algotraderv2::backtest::{
    harness::{run_walk_forward, WalkForwardConfig},
    SimMode,
};
use std::io::Write;

/// Smoke test that runs the walk-forward harness on a small synthetic dataset.
#[tokio::test]
async fn walk_forward_smoke() -> anyhow::Result<()> {
    use tempfile::NamedTempFile;

    // Generate minimal synthetic hourly close-only CSV (6 months)
    let mut tmp = NamedTempFile::new()?;
    writeln!(tmp, "timestamp,close")?;
    let start_ts: i64 = 1_700_000_000; // arbitrary epoch
    for i in 0..(24 * 180) {
        // 180 days of hourly bars
        let ts = start_ts + i * 3600;
        let price = 100.0 + (i as f64 * 0.01);
        writeln!(tmp, "{ts},{price}")?;
    }

    // Run harness
    let cfg = WalkForwardConfig { train_days: 90, test_days: 30, step_days: 30 };
    let reports = run_walk_forward(tmp.path(), "1h", SimMode::Bar, cfg).await?;
    assert!(!reports.is_empty(), "no reports generated");
    println!("walk_forward_smoke: generated {} reports", reports.len());
    Ok(())
}
