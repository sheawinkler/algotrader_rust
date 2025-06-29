//! Risk management utilities.
//!
//! Converts a fused signal score into a position size (% of equity).
//! Simple linear mapping for prototype: score -1.0 → -max, 0 → flat, +1.0 → +max.

/// Maximum leverage (fraction of equity) we are willing to deploy in either direction.
const MAX_POSITION_FRAC: f64 = 0.3; // 30 % of equity

/// Map fused score in [-1.0, 1.0] to target position fraction in [-MAX, MAX].
pub fn size_from_score(score: f64) -> f64 {
    let clamped = score.clamp(-1.0, 1.0);
    clamped * MAX_POSITION_FRAC
}
