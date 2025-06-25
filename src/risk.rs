//! Basic risk management rules (stop-loss / take-profit)
//! This module is intentionally lightweight so it can be reused by both the
//! back-tester and the live trading engine without additional dependencies.

use crate::portfolio::Position;

pub mod position_sizer;

/// Action requested by a risk-rule evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskAction {
    /// Close the entire position at market.
    ClosePosition,
}

/// Generic interface for risk-management rules.
pub trait RiskRule: Send + Sync {
    /// Evaluate the rule for the given position and current price.
    ///
    /// * `symbol` – instrument symbol (e.g. "SOL/USDC").
    /// * `pos` – current position (size > 0 means long; we only trade spot longs for now).
    /// * `current_price` – latest trade/mark price.
    ///
    /// Return `Some(RiskAction)` if the rule triggers, otherwise `None`.
    fn evaluate(&self, symbol: &str, pos: &Position, current_price: f64) -> Option<RiskAction>;

    /// Clone boxed trait-objects safely.
    fn box_clone(&self) -> Box<dyn RiskRule>;
}

impl Clone for Box<dyn RiskRule> {
    fn clone(&self) -> Self { self.box_clone() }
}

/// Simple stop-loss rule: close the position if price drops more than `pct` below average entry.
#[derive(Debug, Clone)]
pub struct StopLossRule {
    pct: f64, // e.g. 0.05 = 5%
}

impl StopLossRule {
    pub fn new(pct: f64) -> Self { Self { pct } }
}

impl RiskRule for StopLossRule {
    fn evaluate(&self, _symbol: &str, pos: &Position, current_price: f64) -> Option<RiskAction> {
        if pos.size > 0.0 {
            let threshold = pos.average_entry_price * (1.0 - self.pct);
            if current_price <= threshold {
                return Some(RiskAction::ClosePosition);
            }
        }
        None
    }

    fn box_clone(&self) -> Box<dyn RiskRule> { Box::new(self.clone()) }
}

/// Simple take-profit rule: close position if price rises more than `pct` above entry.
#[derive(Debug, Clone)]
pub struct TakeProfitRule {
    pct: f64,
}

impl TakeProfitRule {
    pub fn new(pct: f64) -> Self { Self { pct } }
}

impl RiskRule for TakeProfitRule {
    fn evaluate(&self, _symbol: &str, pos: &Position, current_price: f64) -> Option<RiskAction> {
        if pos.size > 0.0 {
            let threshold = pos.average_entry_price * (1.0 + self.pct);
            if current_price >= threshold {
                return Some(RiskAction::ClosePosition);
            }
        }
        None
    }

    fn box_clone(&self) -> Box<dyn RiskRule> { Box::new(self.clone()) }
}
