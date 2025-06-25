use std::collections::HashMap;

use crate::utils::types::TradingPair;

#[derive(Debug, Clone, Default)]
pub struct Position {
    pub size: f64, // positive = long, negative = short (we only trade spot longs for now)
    pub average_entry_price: f64, // weighted avg
    pub realized_pnl: f64,
}

impl Position {
    pub fn update_on_buy(&mut self, qty: f64, price: f64) {
        let new_size = self.size + qty;
        if new_size.abs() < f64::EPSILON {
            // fully closed, reset avg
            self.average_entry_price = 0.0;
        } else {
            self.average_entry_price =
                (self.size * self.average_entry_price + qty * price) / new_size;
        }
        self.size = new_size;
    }

    pub fn update_on_sell(&mut self, qty: f64, price: f64) -> f64 {
        // realise pnl on portion sold relative to avg entry
        let close_qty = qty.min(self.size);
        let pnl = (price - self.average_entry_price) * close_qty;
        self.size -= close_qty;
        if self.size.abs() < f64::EPSILON {
            self.average_entry_price = 0.0;
        }
        self.realized_pnl += pnl;
        pnl
    }

    pub fn unrealized_pnl(&self, current_price: f64) -> f64 {
        (current_price - self.average_entry_price) * self.size
    }
}

#[derive(Debug, Default)]
pub struct Portfolio {
    pub cash_usd: f64,
    pub positions: HashMap<String, Position>, // key = symbol "BASE/QUOTE"
    pub total_realized_pnl: f64,
}

impl Portfolio {
    pub fn new(starting_cash_usd: f64) -> Self {
        Self { cash_usd: starting_cash_usd, ..Default::default() }
    }

    pub fn update_on_buy(&mut self, symbol: &str, qty: f64, price: f64) {
        let cost = qty * price;
        self.cash_usd -= cost;
        let pos = self.positions.entry(symbol.to_string()).or_default();
        pos.update_on_buy(qty, price);
    }

    pub fn update_on_sell(&mut self, symbol: &str, qty: f64, price: f64) -> f64 {
        let proceeds = qty * price;
        self.cash_usd += proceeds;
        let pos = self.positions.entry(symbol.to_string()).or_default();
        let pnl = pos.update_on_sell(qty, price);
        self.total_realized_pnl += pnl;
        if pos.size.abs() < f64::EPSILON {
            // remove empty position to keep map clean
            if let Some(p) = self.positions.get(symbol) {
                if p.size.abs() < f64::EPSILON {
                    self.positions.remove(symbol);
                }
            }
        }
        pnl
    }

    pub fn unrealized_pnl(&self, price_lookup: &impl Fn(&TradingPair) -> Option<f64>) -> f64 {
        self.positions
            .iter()
            .map(|(sym, pos)| {
                let pair_parts: Vec<&str> = sym.split('/').collect();
                if pair_parts.len() != 2 {
                    return 0.0;
                }
                let pair = TradingPair::new(pair_parts[0], pair_parts[1]);
                if let Some(price) = price_lookup(&pair) {
                    pos.unrealized_pnl(price)
                } else {
                    0.0
                }
            })
            .sum()
    }

    pub fn total_usd_value(&self, price_lookup: &impl Fn(&TradingPair) -> Option<f64>) -> f64 {
        self.cash_usd + self.unrealized_pnl(price_lookup)
    }
    /// Return total equity expressed in SOL terms using SOL/USDC mid-price.
    pub fn total_sol_value(&self, price_lookup: &impl Fn(&TradingPair) -> Option<f64>) -> f64 {
        let sol_pair = TradingPair::new("SOL", "USDC");
        if let Some(sol_usd) = price_lookup(&sol_pair) {
            if sol_usd > 0.0 {
                return self.total_usd_value(price_lookup) / sol_usd;
            }
        }
        0.0
    }

    /// Export current portfolio state to a CSV file at the given path.
    /// CSV columns: symbol,size,avg_entry_price,realized_pnl,unrealized_pnl
    pub fn export_csv(
        &self, path: &std::path::Path, price_lookup: &impl Fn(&TradingPair) -> Option<f64>,
    ) -> std::io::Result<()> {
        use std::io::Write;
        let mut wtr = csv::Writer::from_path(path)?;
        wtr.write_record(&["symbol", "size", "avg_entry_price", "realized_pnl", "unrealized_pnl"])?;
        for (sym, pos) in &self.positions {
            let pair_parts: Vec<&str> = sym.split('/').collect();
            let unreal = if pair_parts.len() == 2 {
                let pair = TradingPair::new(pair_parts[0], pair_parts[1]);
                price_lookup(&pair)
                    .map(|p| pos.unrealized_pnl(p))
                    .unwrap_or(0.0)
            } else {
                0.0
            };
            wtr.write_record(&[
                sym,
                &pos.size.to_string(),
                &pos.average_entry_price.to_string(),
                &pos.realized_pnl.to_string(),
                &unreal.to_string(),
            ])?;
        }
        // Cash row (for completeness)
        wtr.write_record(&["CASH", "0", "0", &self.total_realized_pnl.to_string(), "0"])?;
        wtr.flush()?;
        Ok(())
    }
}
