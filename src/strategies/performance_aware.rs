//! Performance-aware strategy wrapper that adds performance monitoring and adaptation

use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::Mutex;

use super::{TimeFrame, TradingStrategy};
use crate::performance::{PerformanceMonitor, StrategyAnalyzer};
use crate::trading::{MarketData, Order, Position, Signal};

/// Wraps a trading strategy with performance monitoring and adaptation
pub struct PerformanceAwareStrategy<T: TradingStrategy + AdaptiveStrategy + Send + Sync + 'static> {
    inner: T,
    monitor: PerformanceMonitor,
    analyzer: StrategyAnalyzer,
    params: Mutex<HashMap<String, f64>>,
    last_analysis: Mutex<Option<std::time::Instant>>,
    analysis_interval: Duration,
}

impl<T: TradingStrategy + AdaptiveStrategy + Send + Sync + 'static> PerformanceAwareStrategy<T> {
    /// Create a new performance-aware strategy wrapper
    pub fn new(
        inner: T, monitor: PerformanceMonitor, analyzer: StrategyAnalyzer,
        initial_params: HashMap<String, f64>,
    ) -> Self {
        Self {
            inner,
            monitor,
            analyzer,
            params: Mutex::new(initial_params),
            last_analysis: Mutex::new(None),
            analysis_interval: Duration::from_secs(3600), // Analyze every hour
        }
    }

    /// Check if it's time to analyze performance and potentially adjust the strategy
    /// Analyze performance; returns suggested parameter updates (if any).
    async fn check_and_analyze(&self) -> Option<std::collections::HashMap<String, f64>> {
        let now = std::time::Instant::now();
        let mut last_analysis = self.last_analysis.lock().await;

        // Check if enough time has passed since last analysis
        if let Some(last) = *last_analysis {
            if now.duration_since(last) < self.analysis_interval {
                return None;
            }
        }

        // Get current metrics
        let metrics = match self.monitor.get_metrics(self.inner.name()).await {
            | Ok(Some(m)) => m,
            | _ => {
                *last_analysis = Some(now);
                return None;
            }
        };

        // Get current parameters
        let params_snapshot = self.params.lock().await.clone();

        // Generate suggestions
        let suggestions = self.analyzer.analyze_strategy(&metrics, &params_snapshot);

        // Build updates map
        let mut updates = std::collections::HashMap::new();
        for s in &suggestions {
            updates.insert(s.parameter.clone(), s.suggested_value);
        }

        // Generate performance report (for logging)
        let report = self.analyzer.generate_report(&metrics, &params_snapshot);
        println!("\n=== Performance Analysis ===\n{}\n===========================\n", report);

        // Update last analysis time
        *last_analysis = Some(now);
        if updates.is_empty() {
            None
        } else {
            Some(updates)
        }
    }

    /// Apply parameter updates to the inner strategy
    async fn apply_parameter_updates(&mut self, updates: std::collections::HashMap<String, f64>) {
        if updates.is_empty() {
            return;
        }

        // Update stored parameters
        let mut params = self.params.lock().await;
        for (key, value) in updates {
            params.insert(key, value);
        }

        // Apply updates to inner strategy
        self.inner.update_parameters(&params).await;
    }
}

#[async_trait]
impl<T: TradingStrategy + AdaptiveStrategy + Send + Sync + 'static> TradingStrategy
    for PerformanceAwareStrategy<T>
{
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn timeframe(&self) -> TimeFrame {
        self.inner.timeframe()
    }

    fn symbols(&self) -> Vec<String> {
        self.inner.symbols()
    }

    async fn generate_signals(&mut self, market_data: &MarketData) -> Vec<Signal> {
        // Run analysis and apply any suggested parameter updates once per call
        if let Some(updates) = self.check_and_analyze().await {
            self.apply_parameter_updates(updates).await;
        }

        // Get position size based on performance metrics
        let account_balance = 10000.0; // TODO: fetch actual balance from portfolio
        let symbol = self.symbols().get(0).cloned().unwrap_or_default();

        let position_size = match self
            .monitor
            .get_recommended_position_size(self.name(), &symbol, account_balance, None)
            .await
        {
            | Ok(sz) => sz,
            | Err(_) => 1.0, // fallback default
        };

        // Generate signals from inner strategy
        let mut signals = self.inner.generate_signals(market_data).await;

        // Adjust position sizes based on performance
        for signal in &mut signals {
            if let Some(meta) = &mut signal.metadata {
                if let Some(size) = meta.get_mut("position_size") {
                    if let Some(size_val) = size.as_f64() {
                        *size = serde_json::json!(size_val * position_size);
                    }
                }
            }
        }

        signals
    }

    fn on_order_filled(&mut self, order: &Order) {
        // Record the trade with performance monitor
        let pnl = order.price * order.size * if order.side.is_buy() { -1.0 } else { 1.0 };

        // In a real implementation, we'd track the actual PnL based on entry/exit prices
        tokio::spawn({
            let monitor = self.monitor.clone();
            let strategy_name = self.name().to_string();
            let order = order.clone();

            async move {
                let _ = monitor
                    .record_trade(&strategy_name, &order, None, pnl, 0.0, None)
                    .await;
            }
        });

        // Forward to inner strategy
        self.inner.on_order_filled(order);
    }

    fn get_positions(&self) -> Vec<&Position> {
        self.inner.get_positions()
    }
}

/// Trait for strategies that can adapt their parameters based on performance
#[async_trait]
pub trait AdaptiveStrategy: TradingStrategy {
    /// Get current strategy parameters
    async fn get_parameters(&self) -> HashMap<String, f64> {
        HashMap::new()
    }
    /// Update strategy parameters
    async fn update_parameters(&mut self, _params: &HashMap<String, f64>) {}
}

// Blanket default implementation for any TradingStrategy
#[async_trait]
impl<T> AdaptiveStrategy for T
where
    T: TradingStrategy + Send + Sync,
{
    // default methods already provided above; we can leave them or override
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trading::MarketData;
    use std::time::Instant;

    struct MockStrategy {
        name: String,
        params: HashMap<String, f64>,
    }

    #[async_trait]
    impl TradingStrategy for MockStrategy {
        fn name(&self) -> &str {
            &self.name
        }
        fn timeframe(&self) -> TimeFrame {
            TimeFrame::OneHour
        }
        fn symbols(&self) -> Vec<String> {
            vec!["TEST/USD".to_string()]
        }
        async fn generate_signals(&mut self, _: &MarketData) -> Vec<Signal> {
            vec![]
        }
        fn on_order_filled(&mut self, _: &Order) {}
        fn get_positions(&self) -> Vec<&Position> {
            vec![]
        }
    }

    #[async_trait]
    impl AdaptiveStrategy for MockStrategy {
        async fn get_parameters(&self) -> HashMap<String, f64> {
            self.params.clone()
        }

        async fn update_parameters(&mut self, params: &HashMap<String, f64>) {
            for (k, v) in params {
                self.params.insert(k.clone(), *v);
            }
        }
    }

    #[tokio::test]
    async fn test_performance_aware_strategy() {
        let monitor = PerformanceMonitor::new();

        let analyzer = StrategyAnalyzer::new(
            10,   // min trades
            50.0, // min win rate %
            15.0, // max drawdown %
            7,    // lookback days
        );

        let mut params = HashMap::new();
        params.insert("position_size".to_string(), 0.1);

        let inner = MockStrategy { name: "TestStrategy".to_string(), params: params.clone() };

        let mut strategy = PerformanceAwareStrategy::new(inner, monitor, analyzer, params);

        // Test that the wrapper delegates to the inner strategy
        assert_eq!(strategy.name(), "TestStrategy");
        assert_eq!(strategy.timeframe(), TimeFrame::OneHour);
        assert_eq!(strategy.symbols(), vec!["TEST/USD"]);

        // Test signal generation (should not panic)
        let signals = strategy.generate_signals(&MarketData::default()).await;
        assert!(signals.is_empty());
    }
}
