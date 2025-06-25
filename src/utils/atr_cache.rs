//! Global ATR cache for sharing latest ATR values between strategies and the
//! position sizing engine.
//!
//! Strategies should call [`update`] whenever they compute a fresh ATR value.
//! The `VolatilitySizer` reads the most‐recent ATR via [`get`].

use std::collections::HashMap;
use std::sync::RwLock;
use once_cell::sync::Lazy;

static ATR_CACHE: Lazy<RwLock<HashMap<String, f64>>> = Lazy::new(|| RwLock::new(HashMap::new()));

/// Update the cached ATR value for a symbol.
pub fn update(symbol: &str, atr: f64) {
    let mut map = ATR_CACHE.write().unwrap();
    map.insert(symbol.to_string(), atr);
}

/// Fetch the latest ATR for `symbol` if available.
pub fn get(symbol: &str) -> Option<f64> {
    let map = ATR_CACHE.read().unwrap();
    map.get(symbol).copied()
}
