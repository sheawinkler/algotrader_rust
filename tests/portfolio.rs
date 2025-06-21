extern crate algotraderv2 as algotraderv2_rust;
use algotraderv2_rust::portfolio::Portfolio;
use algotraderv2_rust::utils::types::TradingPair;

#[test]
fn portfolio_buy_sell_flow() {
    // Start with $1000 USD
    let mut portfolio = Portfolio::new(1000.0);

    // Buy 1 SOL at $20
    portfolio.update_on_buy("SOL/USDC", 1.0, 20.0);
    assert!((portfolio.cash_usd - 980.0).abs() < 1e-6);
    {
        let pos = portfolio.positions.get("SOL/USDC").unwrap();
        assert!((pos.size - 1.0).abs() < 1e-6);
        assert!((pos.average_entry_price - 20.0).abs() < 1e-6);
    }

    // Sell 0.5 SOL at $22 (realises $1 profit)
    let realized = portfolio.update_on_sell("SOL/USDC", 0.5, 22.0);
    assert!((realized - 1.0).abs() < 1e-6);
    assert!((portfolio.cash_usd - 991.0).abs() < 1e-6);

    // Dummy price lookup always returns 22 for SOL
    let price_lookup = |pair: &TradingPair| {
        if pair.base == "SOL" {
            Some(22.0)
        } else {
            None
        }
    };

    let unrealized = portfolio.unrealized_pnl(&price_lookup);
    // Remaining 0.5 SOL has $1 unrealized PnL
    assert!((unrealized - 1.0).abs() < 1e-6);

    let total_equity = portfolio.total_usd_value(&price_lookup);
    // 991 cash + 1 unrealized = 992
    assert!((total_equity - 992.0).abs() < 1e-6);
}
