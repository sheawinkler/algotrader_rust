//! Simple signal fusion utilities.
//!
//! For now we fuse a vector of numeric signals by taking their arithmetic mean.
//! In future, this will be replaced by a Kalman filter or learned ensemble.

/// Compute a fused score in the range roughly matching the input scale.
pub fn fuse(signals: &[f64]) -> f64 {
    if signals.is_empty() {
        0.0
    } else {
        let sum: f64 = signals.iter().copied().sum();
        sum / signals.len() as f64
    }
}
