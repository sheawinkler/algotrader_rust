use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, instrument, warn};
use serde::{Serialize, Deserialize};
use rust_decimal::prelude::*;
// use rust_decimal_macros::dec; // REMOVED: crate not present
use chrono::{DateTime, Utc};

use crate::{
    utils::types::{Order, OrderSide, Position, MarketData},
    utils::Result,
    analysis::performance_metrics::{TradeRecord, TradeSide, TradeStatus},
};

use super::{
    metrics::{StrategyMetrics},
    analyzer::StrategyAnalyzer,
};

use crate::utils::types::MarketRegime;

/// Configuration for performance monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Whether performance monitoring is enabled
    pub enabled: bool,
    /// Interval between performance reviews (in seconds)
    pub review_interval_secs: u64,
    /// Maximum consecutive losses before taking action
    pub max_consecutive_losses: u32,
    /// Maximum drawdown percentage before reducing position size
    pub max_drawdown_pct: f64,
    /// Minimum acceptable win rate (as a percentage, e.g., 40.0 for 40%)
    pub min_win_rate_pct: f64,
    /// Risk per trade as a percentage of account equity
    pub risk_per_trade_pct: f64,
    /// Maximum position size as a percentage of account equity
    pub max_position_size_pct: f64,
    /// Number of days to look back for performance analysis
    pub lookback_days: u32,
    /// Enable/disable circuit breaker functionality
    pub enable_circuit_breaker: bool,
    /// Enable/disable adaptive position sizing
    pub enable_adaptive_sizing: bool,
    /// Volatility adjustment factor for position sizing
    pub volatility_factor: f64,
    /// Correlation threshold for portfolio diversification
    pub correlation_threshold: f64,
    /// Enable/disable market regime detection
    pub enable_market_regime_detection: bool,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            review_interval_secs: 300,  // 5 minutes
            max_consecutive_losses: 3,
            max_drawdown_pct: 10.0,
            min_win_rate_pct: 40.0,
            risk_per_trade_pct: 1.0,
            max_position_size_pct: 10.0,
            lookback_days: 7,
            enable_circuit_breaker: true,
            enable_adaptive_sizing: true,
            volatility_factor: 1.5,
            correlation_threshold: 0.7,
            enable_market_regime_detection: true,
        }
    }
}

/// Performance alert types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertType {
    ConsecutiveLosses(u32, u32),       // (current, threshold)
    DrawdownExceeded(f64, f64),        // (current %, threshold %)
    WinRateBelow(f64, f64),            // (current %, threshold %)
    PositionSizeAdjusted(f64, f64),    // (old size %, new size %)
    StrategyPaused(String),            // Reason
    StrategyResumed(String),            // Reason
    RiskLimitExceeded(String, f64),     // (metric, value)
    PerformanceDegraded(String, f64),   // (metric, value)
    CircuitBreakerTriggered(String),    // Reason
    MarketRegimeChanged(MarketRegime),  // New market regime
    VolatilitySpike(f64, f64),          // (current vol, threshold)
    CorrelationAlert(String, String, f64), // (strategy1, strategy2, correlation)
}

/// Monitors and manages strategy performance with advanced features
#[derive(Debug, Clone)]
pub struct PerformanceMonitor {
    /// Strategy metrics storage
    metrics: Arc<RwLock<HashMap<String, StrategyMetrics>>>,
    /// Active strategies
    active_strategies: Arc<RwLock<HashSet<String>>>,
    /// Paused strategies with reasons
    paused_strategies: Arc<RwLock<HashMap<String, String>>>,
    /// Performance configuration
    config: PerformanceConfig,
    /// Last performance review time
    last_review: Arc<Mutex<Instant>>,
    /// Performance alerts
    alerts: Arc<Mutex<VecDeque<(AlertType, DateTime<Utc>)>>>,
    /// Strategy analyzers
    analyzers: Arc<RwLock<HashMap<String, StrategyAnalyzer>>>,
    /// Market regime detector
    market_regime: Arc<Mutex<Option<MarketRegime>>>,
    /// Position sizes by strategy
    position_sizes: Arc<RwLock<HashMap<String, f64>>>,
    /// Trade correlation matrix
    correlation_matrix: Arc<Mutex<HashMap<String, HashMap<String, f64>>>>,
}

impl PerformanceMonitor {
    /// Create a new performance monitor with default configuration
    pub fn new() -> Self {
        Self::with_config(PerformanceConfig::default())
    }
    
    /// Create a new performance monitor with custom configuration
    pub fn with_config(config: PerformanceConfig) -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            active_strategies: Arc::new(RwLock::new(HashSet::new())),
            paused_strategies: Arc::new(RwLock::new(HashMap::new())),
            config,
            last_review: Arc::new(Mutex::new(Instant::now())),
            alerts: Arc::new(Mutex::new(VecDeque::with_capacity(1000))), // Keep last 1000 alerts
            analyzers: Arc::new(RwLock::new(HashMap::new())),
            market_regime: Arc::new(Mutex::new(None)),
            position_sizes: Arc::new(RwLock::new(HashMap::new())),
            correlation_matrix: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Record a new trade with detailed information
    #[instrument(skip(self, order, position))]
    pub async fn record_trade(
        &self,
        strategy_name: &str,
        order: &Order,
        position: Option<&Position>,
        pnl: f64,
        fees: f64,
        market_data: Option<&MarketData>,
    ) -> Result<()> {
        // Skip if strategy is disabled
        if !self.is_strategy_active(strategy_name).await? {
            return Ok(());
        }
        
        // Create trade record
        let trade_record = TradeRecord {
            id: order.id.clone(),
            symbol: order.id.clone(), // No symbol, use id or get from context
            entry_time: Utc::now(), // Should be actual entry time if available
            exit_time: None, // Should be set when trade is closed
            entry_price: order.price,
            exit_price: None, // Should be set when trade is closed
            quantity: order.size,
            side: match order.side {
                OrderSide::Buy => TradeSide::Long,
                OrderSide::Sell => TradeSide::Short,
            },
            pnl: Some(pnl),
            pnl_percentage: Some(if position.is_some() {
                (pnl / (order.price * order.size)) * 100.0
            } else {
                0.0
            }),
            fees,
            status: TradeStatus::Open, // Set appropriately
            stop_loss: None,
            take_profit: None,
            strategy: strategy_name.to_string(),
            notes: None,
        };
        
        // Update metrics
        let mut metrics = self.metrics.write().await;
        let strategy_metrics = metrics.entry(strategy_name.to_string())
            .or_insert_with(|| StrategyMetrics::new(strategy_name));
            
        strategy_metrics.record_trade(trade_record.pnl.unwrap_or(0.0));
        
        // Update analyzer
        {
            let mut analyzers = self.analyzers.write().await;
            let analyzer = analyzers.entry(strategy_name.to_string())
                .or_insert_with(|| {
                    StrategyAnalyzer::new(
                        10, // min_trades
                        self.config.min_win_rate_pct,
                        self.config.max_drawdown_pct,
                        self.config.lookback_days as u32,
                    )
                });
                
            analyzer.analyze_trade(&trade_record);
        }
        
        // Update market regime if data is available
        if let Some(market_data) = market_data {
            self.update_market_regime(market_data).await?;
        }
        
        // Check for performance issues
        self.check_performance(strategy_name).await?;
        
        debug!(
            target: "performance",
            "Trade recorded - Strategy: {}, Symbol: {}, Side: {:?}, PnL: {:.4} ({}%), Total PnL: {:.4}",
            strategy_name,
            order.id,
            order.side,
            pnl,
            trade_record.pnl_percentage.unwrap_or(0.0),
            strategy_metrics.total_pnl
        );
        
        Ok(())
    }
    
    /// Get current metrics for a strategy
    pub async fn get_metrics(&self, strategy_name: &str) -> Result<Option<StrategyMetrics>> {
        let metrics = self.metrics.read().await;
        Ok(metrics.get(strategy_name).cloned())
    }
    
    /// Check if a strategy is currently active
    pub async fn is_strategy_active(&self, strategy_name: &str) -> Result<bool> {
        let active = self.active_strategies.read().await;
        let paused = self.paused_strategies.read().await;
        Ok(active.contains(strategy_name) && !paused.contains_key(strategy_name))
    }
    
    /// Pause a strategy with a reason
    pub async fn pause_strategy(&self, strategy_name: &str, reason: &str) -> Result<()> {
        let mut paused = self.paused_strategies.write().await;
        paused.insert(strategy_name.to_string(), reason.to_string());
        
        self.add_alert(AlertType::StrategyPaused(reason.to_string())).await;
        info!("Strategy paused: {} - Reason: {}", strategy_name, reason);
        
        Ok(())
    }
    
    /// Resume a paused strategy
    pub async fn resume_strategy(&self, strategy_name: &str) -> Result<()> {
        let mut paused = self.paused_strategies.write().await;
        if paused.remove(strategy_name).is_some() {
            self.add_alert(AlertType::StrategyResumed("Manual resume".to_string())).await;
            info!("Strategy resumed: {}", strategy_name);
        }
        
        Ok(())
    }
    
    /// Check strategy performance and take action if needed
    async fn check_performance(&self, strategy_name: &str) -> Result<()> {
        let metrics = match self.get_metrics(strategy_name).await? {
            Some(m) => m,
            None => return Ok(()),
        };
        
        // Skip if not enough data
        if metrics.total_trades < 5 {
            return Ok(());
        }
        
        let mut actions = Vec::new();
        
        // Check consecutive losses
        if metrics.consecutive_losses >= self.config.max_consecutive_losses {
            let msg = format!(
                "Too many consecutive losses ({} >= {})",
                metrics.consecutive_losses, self.config.max_consecutive_losses
            );
            actions.push(("consecutive_losses", msg));
        }
        
        // Check drawdown
        let max_dd = metrics.total_pnl.abs() * (self.config.max_drawdown_pct / 100.0);
        if metrics.current_drawdown > max_dd && max_dd > 0.0 {
            let msg = format!(
                "Excessive drawdown: {:.2}% of equity (max {:.2}%)",
                (metrics.current_drawdown / metrics.total_pnl.abs()) * 100.0,
                self.config.max_drawdown_pct
            );
            actions.push(("drawdown", msg));
        }
        
        // Check win rate (if enough trades)
        if metrics.total_trades >= 20 {
            let win_rate = (metrics.winning_trades as f64 / metrics.total_trades as f64) * 100.0;
            if win_rate < self.config.min_win_rate_pct {
                let msg = format!(
                    "Win rate too low: {:.1}% < {:.1}%",
                    win_rate, self.config.min_win_rate_pct
                );
                actions.push(("win_rate", msg));
            }
        }
        
        // Take action if needed
        if !actions.is_empty() {
            let reasons: Vec<&str> = actions.iter().map(|(_, msg)| msg.as_str()).collect();
            let reason = reasons.join("; ");
            
            if self.config.enable_circuit_breaker {
                self.pause_strategy(strategy_name, &reason).await?;
                
                // Add alert for circuit breaker
                self.add_alert(AlertType::CircuitBreakerTriggered(
                    format!("Strategy {} paused: {}", strategy_name, reason)
                )).await;
            } else {
                // Just log a warning
                warn!(
                    "Performance issue detected for {}: {}",
                    strategy_name, reason
                );
            }
        }
        
        Ok(())
    }
    
    /// Get recommended position size based on strategy performance and market conditions
    pub async fn get_recommended_position_size(
        &self,
        strategy_name: &str,
        symbol: &str,
        account_balance: f64,
        market_data: Option<&MarketData>,
    ) -> Result<f64> {
        // Get base risk per trade from config or use default
        let risk_pct = self.config.risk_per_trade_pct / 100.0;
        let max_position_pct = self.config.max_position_size_pct / 100.0;
        
        // Calculate base position size
        let mut position_size = account_balance * risk_pct;
        
        // Get strategy metrics if available
        if let Ok(Some(metrics)) = self.get_metrics(strategy_name).await {
            // Apply Kelly Criterion for position sizing
            let kelly = metrics.kelly_criterion();
            let ror = metrics.risk_of_ruin();
            
            // Adjust position size based on strategy performance
            let performance_factor = if metrics.total_trades >= 10 {
                // More weight to win rate and profit factor
                let win_rate_factor = (metrics.win_rate * 1.5).min(1.5);
                let profit_factor = metrics.profit_factor.min(3.0) / 2.0; // Normalize to 0-1.5
                (win_rate_factor + profit_factor) / 2.5
            } else {
                // Default to conservative sizing for new strategies
                0.5
            };
            
            // Apply adaptive sizing if enabled
            if self.config.enable_adaptive_sizing {
                position_size *= performance_factor * kelly;
                
                // Reduce position size during high volatility
                if let Some(market_data) = market_data {
                    let volatility = self.calculate_volatility(market_data).await?;
                    let volatility_adjustment = 1.0 / (1.0 + (volatility * self.config.volatility_factor));
                    position_size *= volatility_adjustment;
                }
                
                // Check correlation with other strategies
                let correlation_penalty = self.calculate_correlation_penalty(strategy_name, symbol).await?;
                position_size *= 1.0 - correlation_penalty;
            }
            
            // Apply circuit breaker if risk is too high
            if ror > 0.3 {
                warn!("High risk of ruin ({:.1}%) detected for {}", ror * 100.0, strategy_name);
                position_size *= 0.5; // Halve position size
            }
        }
        
        // Apply absolute limits
        let min_position = account_balance * 0.01;  // At least 1% of account
        let max_position = account_balance * max_position_pct;
        
        // Ensure position size is within bounds
        let position_size = position_size.max(min_position).min(max_position);
        
        // Update position size tracking
        let mut position_sizes = self.position_sizes.write().await;
        position_sizes.insert(format!("{}:{}", strategy_name, symbol), position_size);
        
        debug!(
            "Position size for {}: ${:.2} ({}% of account, ${:.2})",
            symbol, position_size, (position_size / account_balance * 100.0), account_balance
        );
        
        Ok(position_size)
    }
    
    /// Calculate market volatility using ATR or standard deviation
    async fn calculate_volatility(&self, market_data: &MarketData) -> Result<f64> {
        // Simple implementation using price range
        // In production, use ATR or standard deviation over a lookback period
        let price_range = match (market_data.high, market_data.low) {
    (Some(high), Some(low)) => high - low,
    _ => 0.0,
};
        let avg_price = match (market_data.high, market_data.low) {
    (Some(high), Some(low)) => (high + low) / 2.0,
    _ => 0.0,
};
        let volatility = if avg_price > 0.0 { price_range / avg_price } else { 0.0 };
        
        Ok(volatility)
    }
    
    /// Calculate correlation penalty based on strategy and symbol correlations
    async fn calculate_correlation_penalty(&self, strategy_name: &str, symbol: &str) -> Result<f64> {
        let correlation_matrix = self.correlation_matrix.lock().await;
        let mut max_correlation: f64 = 0.0;
        
        // Find maximum correlation with other strategies for this symbol
        for (key, correlations) in correlation_matrix.iter() {
            if key.starts_with(strategy_name) {
                continue; // Skip self-comparison
            }
            
            if let Some(correlation) = correlations.get(symbol) {
                max_correlation = max_correlation.max(*correlation);
            }
        }
        
        // Apply non-linear penalty based on correlation
        let penalty = if max_correlation > self.config.correlation_threshold {
            // Scale penalty from 0 to 0.5 as correlation approaches 1.0
            0.5 * ((max_correlation - self.config.correlation_threshold) / (1.0 - self.config.correlation_threshold))
        } else {
            0.0
        };
        
        Ok(penalty)
    }
    
    /// Get a comprehensive performance summary for all strategies
    pub async fn get_performance_summary(&self) -> Result<String> {
        let metrics = self.metrics.read().await;
        let analyzers = self.analyzers.read().await;
        let paused = self.paused_strategies.read().await;
        let market_regime = self.market_regime.lock().await;
        
        let mut summary = String::from("\n=== STRATEGY PERFORMANCE SUMMARY ===\n\n");
        
        // Market regime info
        let regime_info = match &*market_regime {
            Some(regime) => format!("Current Market Regime: {:?}", regime),
            None => "Market regime: Unknown".to_string(),
        };
        summary.push_str(&format!("{}\n\n", regime_info));
        
        // Strategy performance table
        summary.push_str(&format!(
            "{:<20} | {:<6} | {:<6} | {:<8} | {:<8} | {:<7} | {:<6} | {:<6} | {:<8} | {:<10}\n",
            "Strategy", "Trades", "Win %", "PnL", "Drawdown", "Kelly %", "RoR %", "Active", "Position", "Last Signal"
        ));
        summary.push_str(&"-".repeat(100));
        summary.push('\n');
        
        for (name, metric) in metrics.iter() {
            let analyzer = analyzers.get(name);
            let is_paused = paused.contains_key(name);
            let status = if is_paused { "PAUSED" } else { "ACTIVE" };
            
            // Get position size if available
            let position_size = {
                let sizes = self.position_sizes.read().await;
                sizes.get(name).copied().unwrap_or(0.0)
            };
            
            // Get last signal if analyzer exists
            let last_signal = analyzer.and_then(|a| a.last_signal())
                .unwrap_or_else(|| "N/A".to_string());
            
            // Add strategy performance to summary
            let line = format!(
                "{:<20} | {:<6} | {:<5.1}% | ${:<7.2} | {:<5.1}% | {:<6.1}% | {:<5.1}% | {:<6} | ${:<7.2} | {:<10}\n",
                name,
                metric.total_trades,
                metric.win_rate * 100.0,
                metric.total_pnl,
                (metric.current_drawdown / metric.total_pnl.max(0.01).abs()) * 100.0,
                metric.kelly_criterion() * 100.0,
                metric.risk_of_ruin() * 100.0,
                status,
                position_size,
                last_signal
            );
            summary.push_str(&line);
            
            // Add analyzer insights if available
            if let Some(analyzer) = analyzer {
                if let Some(insights) = analyzer.get_insights() {
                    for (key, value) in insights {
                        summary.push_str(&format!("  {}: {}\n", key, value));
                    }
                }
            }
        }
        
        // Add performance alerts if any
        let alerts = self.alerts.lock().await;
        if !alerts.is_empty() {
            summary.push_str("\n=== RECENT ALERTS ===\n");
            for (alert, timestamp) in alerts.iter().take(5) {
                summary.push_str(&format!("[{}] {:?}\n", timestamp.format("%Y-%m-%d %H:%M:%S"), alert));
            }
        }
        
        Ok(summary)
    }
    
    /// Get correlation matrix for all strategies
    pub async fn get_correlation_matrix(&self) -> Result<HashMap<String, HashMap<String, f64>>> {
        let matrix_guard = self.correlation_matrix.lock().await;
        Ok((*matrix_guard).clone())
    }
    
    /// Update market regime based on current market data
    pub async fn update_market_regime(&self, market_data: &MarketData) -> Result<()> {
        // Simple implementation - in production, use more sophisticated regime detection
        let price_change = (market_data.close - market_data.open.unwrap_or(0.0)) / market_data.open.unwrap_or(1.0);
        let volatility = (market_data.high.unwrap_or(0.0) - market_data.low.unwrap_or(0.0)) / market_data.open.unwrap_or(1.0);
        
        let new_regime = if volatility > 0.05 {
            if price_change > 0.02 {
                MarketRegime::TrendingUp
            } else if price_change < -0.02 {
                MarketRegime::TrendingDown
            } else {
                MarketRegime::Volatile
            }
        } else {
            if price_change.abs() < 0.01 {
                MarketRegime::Ranging
            } else {
                MarketRegime::Unknown
            }
        };
        
        // Only update if regime has changed
        let mut current_regime = self.market_regime.lock().await;
        if *current_regime != Some(new_regime) {
            let old_regime = current_regime.take();
            *current_regime = Some(new_regime);
            
            self.add_alert(AlertType::MarketRegimeChanged(new_regime)).await;
            info!(
                "Market regime changed: {:?} -> {:?}",
                old_regime.unwrap_or(MarketRegime::Unknown),
                new_regime
            );
        }
        
        Ok(())
    }
    
    /// Add a new alert to the alert queue
    pub async fn add_alert(&self, alert: AlertType) {
        let mut alerts = self.alerts.lock().await;
        alerts.push_back((alert, Utc::now()));
        
        // Keep only the most recent alerts
        if alerts.len() > 1000 {
            alerts.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trading::{Order, OrderType};
    
    #[tokio::test]
    async fn test_performance_monitor() {
        let monitor = PerformanceMonitor::new(
            Duration::from_secs(300), // 5 minutes
            3,     // max consecutive losses
            10.0,  // max drawdown %
            40.0,  // min win rate %
        );
        
        let order = Order {
            id: "TEST-ORDER".to_string(),
            symbol: "SOL/USDC".to_string(),
            side: OrderSide::Buy,
            size: 1.0,
            price: 100.0,
            order_type: OrderType::Market,
            timestamp: SystemTime::now(),
        };
        
        // Record some trades
        monitor.record_trade("TestStrategy", &order, 100.0).await;
        monitor.record_trade("TestStrategy", &order, -50.0).await;
        
        // Check metrics
        let metrics = monitor.get_metrics("TestStrategy").await.unwrap();
        assert_eq!(metrics.total_trades, 2);
        assert_eq!(metrics.winning_trades, 1);
        assert_eq!(metrics.losing_trades, 1);
        
        // Check position sizing
        let position_size = monitor
            .get_recommended_position_size("TestStrategy", 10000.0, 0.02)
            .await;
        assert!(position_size > 0.0);
        
        // Check performance summary
        let summary = monitor.get_performance_summary().await;
        assert!(summary.contains("TestStrategy"));
    }
}
