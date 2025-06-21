use std::collections::{HashMap, VecDeque};
use std::time::{SystemTime, Duration, UNIX_EPOCH};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::trading::{MarketData, Signal, SignalType, OrderType, Position, Order, OrderSide};
use super::{TradingStrategy, TimeFrame};

/// Meme Token Arbitrage Strategy that identifies price discrepancies across DEXs
#[derive(Debug, Clone)]
pub struct MemeArbitrageStrategy {
    // Strategy configuration
    symbol: String,
    timeframe: TimeFrame,
    
    // DEX configurations
    dex_weights: HashMap<String, f64>,  // DEX name -> weight/confidence
    
    // State
    position: Option<Position>,
    recent_signals: VecDeque<Signal>,
    performance_metrics: PerformanceMetrics,
    
    // Risk management
    max_position_size: f64,
    max_slippage_pct: f64,
    max_consecutive_losses: u32,
    
    // Performance tracking
    trade_history: Vec<TradeRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PerformanceMetrics {
    total_trades: u32,
    winning_trades: u32,
    losing_trades: u32,
    consecutive_losses: u32,
    total_pnl: f64,
    win_rate: f64,
    profit_factor: f64,
    sharpe_ratio: f64,
    last_review: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TradeRecord {
    timestamp: i64,
    symbol: String,
    entry_price: f64,
    exit_price: Option<f64>,
    size: f64,
    side: OrderSide,
    pnl: Option<f64>,
    pnl_pct: Option<f64>,
    metadata: serde_json::Value,
}

impl MemeArbitrageStrategy {
    pub fn new(
        symbol: &str,
        timeframe: TimeFrame,
        max_position_size: f64,
        max_slippage_pct: f64,
        max_consecutive_losses: u32,
    ) -> Self {
        let mut dex_weights = HashMap::new();
        dex_weights.insert("jupiter".to_string(), 0.4);
        dex_weights.insert("raydium".to_string(), 0.3);
        dex_weights.insert("orca".to_string(), 0.3);
        
        Self {
            symbol: symbol.to_string(),
            timeframe,
            dex_weights,
            position: None,
            recent_signals: VecDeque::with_capacity(10),
            performance_metrics: PerformanceMetrics {
                total_trades: 0,
                winning_trades: 0,
                losing_trades: 0,
                consecutive_losses: 0,
                total_pnl: 0.0,
                win_rate: 0.0,
                profit_factor: 0.0,
                sharpe_ratio: 0.0,
                last_review: SystemTime::now(),
            },
            max_position_size,
            max_slippage_pct: max_slippage_pct / 100.0,
            max_consecutive_losses,
            trade_history: Vec::new(),
        }
    }
    
    /// Calculate arbitrage opportunity score between DEXs
    fn calculate_arbitrage_score(&self, prices: &HashMap<String, f64>) -> Option<(f64, String, String)> {
        if prices.len() < 2 {
            return None;
        }
        
        let mut best_bid = (f64::MIN, "".to_string());
        let mut best_ask = (f64::MAX, "".to_string());
        
        // Find best bid and ask across all DEXs
        for (dex, &price) in prices {
            let weight = self.dex_weights.get(dex).unwrap_or(&0.0);
            let weighted_price = price * weight;
            
            if weighted_price > best_bid.0 {
                best_bid = (weighted_price, dex.clone());
            }
            
            if weighted_price < best_ask.0 {
                best_ask = (weighted_price, dex.clone());
            }
        }
        
        // Calculate arbitrage score (spread as percentage of ask price)
        if best_ask.0 > 0.0 && best_bid.1 != best_ask.1 {
            let spread_pct = (best_bid.0 - best_ask.0) / best_ask.0 * 100.0;
            if spread_pct > 1.0 {  // Minimum 1% spread to consider
                return Some((spread_pct, best_bid.1, best_ask.1));
            }
        }
        
        None
    }
    
    /// Adjust position size based on recent performance
    fn adjust_position_size(&self, base_size: f64) -> f64 {
        let metrics = &self.performance_metrics;
        let reduction_factor = if metrics.consecutive_losses > 0 {
            1.0 / (metrics.consecutive_losses as f64 + 1.0)
        } else {
            1.0
        };
        
        (base_size * reduction_factor).min(self.max_position_size)
    }
    
    /// Update performance metrics after each trade
    fn update_performance_metrics(&mut self, trade: &TradeRecord) {
        self.performance_metrics.total_trades += 1;
        
        if let (Some(pnl), Some(pnl_pct)) = (trade.pnl, trade.pnl_pct) {
            self.performance_metrics.total_pnl += pnl;
            
            if pnl > 0.0 {
                self.performance_metrics.winning_trades += 1;
                self.performance_metrics.consecutive_losses = 0;
            } else {
                self.performance_metrics.losing_trades += 1;
                self.performance_metrics.consecutive_losses += 1;
            }
            
            // Update win rate
            self.performance_metrics.win_rate = 
                self.performance_metrics.winning_trades as f64 
                / self.performance_metrics.total_trades as f64;
                
            // Update profit factor (gross profits / gross losses)
            // TODO: Track gross profits and losses separately for more accurate calculation
            
            // Update Sharpe ratio (simplified)
            // TODO: Track returns over time for proper Sharpe ratio calculation
        }
        
        // Periodically review and adjust strategy
        if self.performance_metrics.total_trades % 5 == 0 {
            self.review_and_adjust();
        }
    }
    
    /// Review strategy performance and make adjustments
    fn review_and_adjust(&mut self) {
        let metrics = &mut self.performance_metrics;
        
        // Calculate time since last review
        let review_interval = match self.timeframe {
            TimeFrame::OneMinute => Duration::from_secs(300),  // 5 minutes
            TimeFrame::FiveMinutes => Duration::from_secs(900),  // 15 minutes
            TimeFrame::FifteenMinutes => Duration::from_secs(3600),  // 1 hour
            _ => Duration::from_secs(14400),  // 4 hours by default
        };
        
        if metrics.last_review.elapsed().unwrap_or_default() < review_interval {
            return;
        }
        
        info!("Performing strategy review...");
        
        // Adjust DEX weights based on performance
        if metrics.consecutive_losses >= self.max_consecutive_losses {
            warn!("Consecutive losses ({}), adjusting strategy...", metrics.consecutive_losses);
            
            // Reduce position size
            self.max_position_size *= 0.8;  // Reduce by 20%
            
            // Adjust DEX weights (simplified)
            for weight in self.dex_weights.values_mut() {
                *weight *= 0.9;  // Slightly reduce confidence in all DEXs
            }
            
            info!("Adjusted strategy: max_position_size={:.4}, DEX weights: {:?}", 
                self.max_position_size, self.dex_weights);
        }
        
        metrics.last_review = SystemTime::now();
    }
}

#[async_trait]
impl TradingStrategy for MemeArbitrageStrategy {
    fn name(&self) -> &str {
        "MemeArbitrageStrategy"
    }
    
    fn timeframe(&self) -> TimeFrame {
        self.timeframe
    }
    
    fn symbols(&self) -> Vec<String> {
        vec![self.symbol.clone()]
    }
    
    async fn generate_signals(&mut self, market_data: &MarketData) -> Vec<Signal> {
        let mut signals = Vec::new();
        
        // Skip if we don't have DEX price data
        let prices_opt = market_data.dex_prices.as_ref();
        if prices_opt.map_or(true, |m| m.is_empty()) {
            return signals;
        }
        let prices = prices_opt.unwrap();
        
        // Calculate arbitrage opportunities
        if let Some((spread_pct, buy_dex, sell_dex)) = self.calculate_arbitrage_score(prices) {
            let buy_price = prices[&buy_dex];
            let sell_price = prices[&sell_dex];
            
            // Calculate position size based on available balance and risk
            let position_size = self.adjust_position_size(self.max_position_size);
            
            // Create arbitrage signal
            signals.push(Signal {
                symbol: self.symbol.clone(),
                size: position_size,
                signal_type: SignalType::Arbitrage {
                    buy_dex: buy_dex.clone(),
                    sell_dex: sell_dex.clone(),
                    spread_pct,
                },
                price: buy_price,
                 order_type: OrderType::Market,
                 limit_price: None,
                 stop_price: None,
                timestamp: market_data.timestamp,
                confidence: (spread_pct / 10.0).min(0.95), // Scale confidence with spread
                metadata: Some(serde_json::json!({
                    "strategy": "MemeArbitrage",
                    "spread_pct": spread_pct,
                    "buy_price": buy_price,
                    "sell_price": sell_price,
                    "position_size": position_size,
                    "consecutive_losses": self.performance_metrics.consecutive_losses,
                })),
            });
        }
        
        signals
    }
    
    fn on_order_filled(&mut self, order: &Order) {
        let trade_record = TradeRecord {
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
            symbol: self.symbol.clone(),
            entry_price: order.price,
            exit_price: None,
            size: order.size,
            side: order.side.clone(),
            pnl: None,
            pnl_pct: None,
            metadata: serde_json::json!({}),
        };
        
        match order.side {
            OrderSide::Buy => {
                self.position = Some(Position {
                    symbol: self.symbol.clone(),
                    size: order.size,
                    entry_price: Some(order.price),
                    current_price: order.price,
                    stop_loss: Some(order.price * (1.0 - self.max_slippage_pct)),
                    take_profit: Some(order.price * (1.0 + self.max_slippage_pct * 2.0)),
                    side: order.side.clone(),
                    timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
                    ..Default::default()
                });
                
                self.trade_history.push(trade_record);
            },
            OrderSide::Sell => {
                let mut metrics_record: Option<TradeRecord> = None;
                if let Some(pos) = &self.position {
                    if let Some(entry_price) = pos.entry_price {
                        let pnl = if order.side == OrderSide::Buy {
                            (entry_price - order.price) * order.size
                        } else {
                            (order.price - entry_price) * order.size
                        };
                        let pnl_pct = (order.price - entry_price) / entry_price * 100.0;

                        if let Some(last_trade) = self.trade_history.last_mut() {
                            last_trade.exit_price = Some(order.price);
                            last_trade.pnl = Some(pnl);
                            last_trade.pnl_pct = Some(pnl_pct);
                            metrics_record = Some(last_trade.clone());
                        }
                    }

                    if pos.size <= order.size {
                        self.position = None;
                    } else {
                        if let Some(p) = &mut self.position {
                            p.size -= order.size;
                        }
                    }
                }
                // Position borrow has ended; safe to update metrics
                if let Some(rec) = metrics_record {
                    self.update_performance_metrics(&rec);
                }
                
                self.trade_history.push(trade_record);
            },
        }
    }
    
    fn get_positions(&self) -> Vec<&Position> {
        self.position.iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    
    #[test]
    fn test_arbitrage_score_calculation() {
        let strategy = MemeArbitrageStrategy::new(
            "BONK/USDC",
            TimeFrame::FiveMinutes,
            0.1,  // max_position_size
            0.5,  // max_slippage_pct
            3,    // max_consecutive_losses
        );
        
        let mut prices = HashMap::new();
        prices.insert("jupiter".to_string(), 1.05);
        prices.insert("raydium".to_string(), 1.00);
        prices.insert("orca".to_string(), 0.98);
        
        if let Some((spread_pct, buy_dex, sell_dex)) = strategy.calculate_arbitrage_score(&prices) {
            assert!(spread_pct > 0.0);
            assert_eq!(buy_dex, "jupiter");
            assert_eq!(sell_dex, "orca");
        } else {
            panic!("Failed to calculate arbitrage score");
        }
    }
    
    #[tokio::test]
    async fn test_position_size_adjustment() {
        let mut strategy = MemeArbitrageStrategy::new(
            "BONK/USDC",
            TimeFrame::FiveMinutes,
            0.1,  // max_position_size
            0.5,  // max_slippage_pct
            3,    // max_consecutive_losses
        );
        
        // Simulate consecutive losses
        strategy.performance_metrics.consecutive_losses = 2;
        
        let adjusted_size = strategy.adjust_position_size(0.1);
        assert!(adjusted_size < 0.1); // Should be reduced
        
        // Test with no losses
        strategy.performance_metrics.consecutive_losses = 0;
        let full_size = strategy.adjust_position_size(0.1);
        assert_eq!(full_size, 0.1);
    }
}
