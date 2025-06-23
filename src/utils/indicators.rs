//! Convenience re-exports and lightweight custom indicators used across strategies.
//!
//! This module lives under `utils` so that strategy code can simply
//! `use crate::utils::indicators::*` and access both wrappers from the `ta` crate as
//! well as a few bespoke indicators that are not included upstream.

use ta::Next;
use ta::{Period, Reset};

// ---------- Re-export popular TA indicators from the `ta` crate ----------
// Aliases (type aliases) are provided for brevity (e.g. `EMA` instead of the very
// verbose `ta::indicators::ExponentialMovingAverage`).

pub use ta::indicators::{
    BollingerBands as BB,
    ExponentialMovingAverage as EMA,
    MovingAverageConvergenceDivergence as MACD,
    PercentagePriceOscillator as PPO,
    RelativeStrengthIndex as RSI,
    SimpleMovingAverage as SMA,
    StandardDeviation as StdDev,
};

// ---------- Helper traits & wrappers ----------

/// Trait for fetching the latest computed value from an indicator.
/// Falls back to `0.0` for types that do not cache a value (useful for
/// third-party indicators until they are wrapped).
pub trait IndicatorValue {
    fn value(&self) -> f64 { 0.0 }
}

/// Generic wrapper that caches the last output of any `ta::Next` indicator so
/// the most-recent value can be queried cheaply.
#[derive(Clone, Debug)]
pub struct CachedIndicator<I> {
    inner: I,
    last: f64,
}

impl<I> CachedIndicator<I> {
    pub fn new(inner: I) -> Self { Self { inner, last: 0.0 } }
    pub fn inner(&self) -> &I { &self.inner }
    pub fn inner_mut(&mut self) -> &mut I { &mut self.inner }
}

impl<I> IndicatorValue for CachedIndicator<I> {
    fn value(&self) -> f64 { self.last }
}

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
    fn period(&self) -> usize { self.inner.period() }
}

// Implement the trait for external indicator types that are commonly used so
// strategy code can call `IndicatorValue::value()` directly.
impl IndicatorValue for EMA {}
impl IndicatorValue for SMA {}
impl IndicatorValue for MACD {}
impl IndicatorValue for RSI {}
impl IndicatorValue for StdDev {}
impl IndicatorValue for PPO {}

// ---------- Legacy shim indicators ----------
use ta::{High, Low, Close};

#[derive(Debug, Clone, Copy)]
pub struct StochasticOutput {
    pub k: f64,
    pub d: f64,
}



#[derive(Debug, Clone)]
pub struct StochasticOscillator {
    k_calc: ta::indicators::SlowStochastic,
    d_calc: EMA,
    period: usize,
}

impl StochasticOscillator {
    pub fn new(stoch_period: usize, d_period: usize, _unused: usize) -> Self {
        Self {
            k_calc: ta::indicators::SlowStochastic::new(stoch_period, 3).unwrap(),
            d_calc: EMA::new(d_period).unwrap(),
            period: stoch_period,
        }
    }

    pub fn next_item<T: High + Low + Close>(&mut self, input: &T) -> StochasticOutput {
        let k = self.k_calc.next(input);
        let d = self.d_calc.next(k);
        StochasticOutput { k, d }
    }
}

impl Period for StochasticOscillator { fn period(&self) -> usize { self.period } }
impl Reset for StochasticOscillator {
    fn reset(&mut self) {
        self.k_calc.reset();
        self.d_calc.reset();
    }
}

impl<T: High + Low + Close> Next<&T> for StochasticOscillator {
    type Output = StochasticOutput;
    fn next(&mut self, input: &T) -> Self::Output {
        self.next_item(input)
    }
}

/// Extremely-minimal Average Directional Index stub.
#[derive(Debug, Clone)]
pub struct AverageDirectionalIndex { period: usize }
impl AverageDirectionalIndex {
    /// Create a new Average Directional Index with the given period.
    pub fn new(period: usize) -> Self { Self { period } }
    /// Return the current Average Directional Index value.
    pub fn current(&self) -> f64 { 0.0 }
}
impl Period for AverageDirectionalIndex { fn period(&self) -> usize { self.period } }
impl Reset for AverageDirectionalIndex { fn reset(&mut self) {} }
impl Next<f64> for AverageDirectionalIndex { type Output = f64; fn next(&mut self, _input: f64) -> f64 { 0.0 } }
impl<'a, T: High + Low + Close> Next<&'a T> for AverageDirectionalIndex { type Output = f64; fn next(&mut self, _input: &'a T) -> f64 { 0.0 } }

impl IndicatorValue for StochasticOscillator {}
impl IndicatorValue for AverageDirectionalIndex {}

// ---------- Custom indicators ----------

/// Time-weighted VWAP (Volume-Weighted Average Price).
///
/// A very small, streaming implementation that only requires price & volume
/// tuples.  It is *not* a rolling window VWAP – each call to `next` consumes the
/// full series so far (equivalent to session VWAP).  This is adequate for
/// intraday back-tests where we reset state at session boundaries.
#[derive(Clone, Debug, Default)]
pub struct VWAP {
    cum_px_vol: f64,
    cum_vol: f64,
}

impl VWAP {
    /// Create a new VWAP calculator with zeroed state.
    pub fn new() -> Self { Self::default() }

    /// Return the current VWAP.  If no data has been observed the value is `0.0`.
    pub fn value(&self) -> f64 {
        if self.cum_vol.abs() < f64::EPSILON { 0.0 } else { self.cum_px_vol / self.cum_vol }
    }
}

impl Next<(f64, f64)> for VWAP {
    type Output = f64;

    /// Feed the next `(price, volume)` tuple and return the updated VWAP.
    fn next(&mut self, input: (f64, f64)) -> Self::Output {
        let (price, vol) = input;
        self.cum_px_vol += price * vol;
        self.cum_vol += vol;
        self.value()
    }
}

impl Reset for VWAP {
    fn reset(&mut self) {
        self.cum_px_vol = 0.0;
        self.cum_vol = 0.0;
    }
}

impl Period for VWAP {
    fn period(&self) -> usize {
        // Session-wide – not a rolling window.  Return 0 to signal undefined / N/A.
        0
    }
}

// ---------- Tests ----------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vwap_basic() {
        let mut vwap = VWAP::new();
        assert_eq!(vwap.value(), 0.0);

        vwap.next((100.0, 5.0)); // price 100, vol 5
        assert_eq!(vwap.value(), 100.0);

        vwap.next((105.0, 5.0)); // cumulative price*vol = 100*5 + 105*5 = 1025; cum vol = 10
        assert_eq!(vwap.value(), 102.5);

        vwap.reset();
        assert_eq!(vwap.value(), 0.0);
    }
}
