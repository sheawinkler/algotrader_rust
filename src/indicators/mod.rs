//! Local shim indicators that are no longer available in the upstream `ta` crate.
//! Only the functionality needed by our strategies is implemented – enough to compile
//! and run basic back-tests without pulling another external dependency.

use ta::{Next, High, Low, Close, Period, Reset};
use ta::indicators::{SlowStochastic, ExponentialMovingAverage};
use anyhow::Result;

/// Output of the stochastic oscillator that the legacy code expects.
#[derive(Debug, Clone, Copy)]
pub struct StochasticOutput {
    pub k: f64,
    pub d: f64,
}

/// Simple wrapper around `SlowStochastic` that also calculates a smoothed D line.
/// The historical implementation expected both %K and %D.
#[derive(Debug, Clone)]
pub struct StochasticOscillator {
    k_calc: SlowStochastic,
    // %D is just an EMA of %K in the classic definition
    d_calc: ExponentialMovingAverage,
    period: usize,
}

impl StochasticOscillator {
    pub fn new(stoch_period: usize, d_period: usize, _unused: usize) -> Result<Self> {
        Ok(Self {
            k_calc: SlowStochastic::new(stoch_period, 3)?, // 3-period EMA internally
            d_calc: ExponentialMovingAverage::new(d_period)?,
            period: stoch_period,
        })
    }

    pub fn next_item<T: High + Low + Close>(&mut self, input: &T) -> StochasticOutput {
        let k = self.k_calc.next(input);
        let d = self.d_calc.next(k);
        StochasticOutput { k, d }
    }
}

impl Period for StochasticOscillator {
    fn period(&self) -> usize {
        self.period
    }
}

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

// Re-export so existing `use crate::indicators::*` patterns work.


/// Extremely-minimal Average Directional Index stub.
/// Only returns 0.0; enough for compilation until we decide to implement properly.
#[derive(Debug, Clone)]
pub struct AverageDirectionalIndex {
    _period: usize,
}

impl AverageDirectionalIndex {
    pub fn new(period: usize) -> Result<Self> {
        Ok(Self { _period: period })
    }

    pub fn current(&self) -> f64 { 0.0 }
}

impl Period for AverageDirectionalIndex {
    fn period(&self) -> usize { self._period }
}

impl Reset for AverageDirectionalIndex {
    fn reset(&mut self) {}
}

impl Next<f64> for AverageDirectionalIndex {
    type Output = f64;
    fn next(&mut self, _input: f64) -> f64 { 0.0 }
}

impl<'a, T: High + Low + Close> Next<&'a T> for AverageDirectionalIndex {
    type Output = f64;
    fn next(&mut self, _input: &'a T) -> f64 { 0.0 }
}
