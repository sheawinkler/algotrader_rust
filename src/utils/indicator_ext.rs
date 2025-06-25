//! Lightweight wrapper around indicators from the `ta` crate that keeps track of
//! the most recently computed value.  This lets strategy code query the "current
//! value" of an indicator without having to store that separately.
//!
//! Example
//! ```rust
//! use ta::indicators::ExponentialMovingAverage;
//! use algotraderv2::utils::indicator_ext::{CachedIndicator, IndicatorValue};
//! let mut ema = CachedIndicator::new(ExponentialMovingAverage::new(9).unwrap());
//! ema.next(100.0);
//! println!("EMA value = {}", ema.value());
//! ```

use ta::{Next, Reset, Period};

/// Simple trait that allows retrieving the last computed value of an indicator.
pub trait IndicatorValue {
    /// Return most recent calculated indicator value.  Default fallback is `0.0`
    /// so that legacy impls for external types compile until they are wrapped in
    /// `CachedIndicator`.
    fn value(&self) -> f64 {
        0.0
    }
}

/// Generic wrapper that caches the last output of an indicator implementing
/// `ta::Next`.
#[derive(Clone, Debug)]
pub struct CachedIndicator<I> {
    inner: I,
    last: f64,
}

impl<I> CachedIndicator<I> {
    pub fn new(inner: I) -> Self {
        Self { inner, last: 0.0 }
    }

    /// Access underlying indicator immutably
    pub fn inner(&self) -> &I { &self.inner }
    /// Mutable access (use with care – changing internal state may desync `last`)
    pub fn inner_mut(&mut self) -> &mut I { &mut self.inner }
}

impl<I> IndicatorValue for CachedIndicator<I> {
    fn value(&self) -> f64 { self.last }
}

// ============ Implement Next ==========
impl<I> Next<f64> for CachedIndicator<I>
where
    I: Next<f64, Output = f64>,
{
    type Output = f64;
    fn next(&mut self, input: f64) -> f64 {
        self.last = self.inner.next(input);
        self.last
    }
}

impl<'a, I, T> Next<&'a T> for CachedIndicator<I>
where
    I: Next<&'a T, Output = f64>,
{
    type Output = f64;
    fn next(&mut self, input: &'a T) -> f64 {
        self.last = self.inner.next(input);
        self.last
    }
}

// ===== Delegate common indicator traits =====
impl<I> Reset for CachedIndicator<I>
where
    I: Reset,
{
    fn reset(&mut self) {
        self.inner.reset();
        self.last = 0.0;
    }
}

impl<I> Period for CachedIndicator<I>
where
    I: Period,
{
    fn period(&self) -> usize {
        self.inner.period()
    }
}

// Implement the trait for the indicator types used in strategies.  Because the
// trait is local to this crate we are allowed to implement it for external
// types under the orphan-rule.

impl IndicatorValue for ta::indicators::ExponentialMovingAverage {}
impl IndicatorValue for ta::indicators::SimpleMovingAverage {}
impl IndicatorValue for ta::indicators::AverageTrueRange {}
impl IndicatorValue for ta::indicators::StandardDeviation {}
impl IndicatorValue for ta::indicators::RelativeStrengthIndex {}
impl IndicatorValue for ta::indicators::MovingAverageConvergenceDivergence {}
impl IndicatorValue for ta::indicators::PercentagePriceOscillator {}
// Additional indicators used by strategies
impl IndicatorValue for crate::indicators::StochasticOscillator {}
impl IndicatorValue for ta::indicators::KeltnerChannel {}
impl IndicatorValue for crate::indicators::AverageDirectionalIndex {}
