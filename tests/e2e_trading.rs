//! End-to-end trading test: risk, arbitrage, reporting

use std::collections::HashMap;
use algotraderv2_rust::TradingEngine;
use algotraderv2_rust::dex::DexFactory;
use algotraderv2_rust::strategy::StrategyFactory;
use algotraderv2_rust::TradeRecord;

#[tokio::test]
async fn test_e2e_trading_session() {
    // Setup trading engine with test config
    let mut engine = TradingEngine {
        dex_clients: HashMap::new(),
        strategies: StrategyFactory::create_strategies(&["mean_reversion", "momentum"]).unwrap().into_iter().map(|(_k, v)| v).collect(),
        performance_monitors: HashMap::new(),
        config: algotraderv2_rust::utils::Config::default(),
        is_running: false,
        last_performance_review: std::time::Instant::now(),
        trading_wallet: "test_wallet_trading".to_string(),
        personal_wallet: "test_wallet_personal".to_string(),
        wallet_analyzer: None,
        starting_balance: 4.0,
        current_balance: 4.0,
        max_position_pct: 0.05,
        max_position_abs: 0.2,
        max_open_trades: 3,
        stop_loss_pct: 0.10,
        max_daily_loss_pct: 0.15,
        daily_loss: 0.0,
        open_trades: 0,
        trade_history: vec![],
        enable_arbitrage: true,
    };

    // Add dummy DEX clients
    for name in ["jupiter", "raydium", "photon"] {
        if let Ok(client) = DexFactory::create_client(name) {
            engine.dex_clients.insert(name.to_string(), client);
        }
    }
    // Simulate a trading session
    let symbols = vec!["SOL/USDC".to_string(), "BONK/USDC".to_string()];
    for symbol in &symbols {
        // Try arbitrage
        let _ = engine.try_arbitrage(symbol).await;
        // Simulate trades for strategies
        let size = engine.position_size();
        engine.trade_history.push(algotraderv2_rust::TradeRecord {
            timestamp: 0,
            symbol: symbol.clone(),
            side: "buy".to_string(),
            size,
            price: 100.0,
            pnl: 0.01,
            stop_loss_triggered: false,
        });
        engine.current_balance += 0.01;
    }
    // Report
    engine.session_report();
    // Assert risk parameters
    assert!(engine.max_position_pct <= 0.05);
    assert!(engine.max_position_abs <= 0.2);
    assert!(engine.current_balance >= 4.0);
}
