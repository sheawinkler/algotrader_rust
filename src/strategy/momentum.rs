//! Momentum trading strategy implementation.

use super::*;
use std::collections::HashMap;
use crate::utils::types::MarketData;
use ta::indicators::{ExponentialMovingAverage, RelativeStrengthIndex};
use ta::Next;
use crate::utils::error::Error;

/// Momentum trading strategy
pub struct MomentumStrategy {
    name: String,
    ema_short_period: usize,
    ema_long_period: usize,
    rsi_period: usize,
    rsi_overbought: f64,
    rsi_oversold: f64,
    position_size: f64,
    ema_short: Option<ExponentialMovingAverage>,
    ema_long: Option<ExponentialMovingAverage>,
    rsi: Option<RelativeStrengthIndex>,
}

impl MomentumStrategy {
    /// Create a new MomentumStrategy with default parameters
    pub fn new() -> Self {
        Self {
            name: "Momentum".to_string(),
            ema_short_period: 9,
            ema_long_period: 21,
            rsi_period: 14,
            rsi_overbought: 70.0,
            rsi_oversold: 30.0,
            position_size: 0.1, // 10% of portfolio
            ema_short: None,
            ema_long: None,
            rsi: None,
        }
    }
    
    /// Update the strategy parameters
    pub fn with_parameters(
        mut self,
        ema_short_period: usize,
        ema_long_period: usize,
        rsi_period: usize,
        rsi_overbought: f64,
        rsi_oversold: f64,
        position_size: f64,
    ) -> Self {
        self.ema_short_period = ema_short_period;
        self.ema_long_period = ema_long_period;
        self.rsi_period = rsi_period;
        self.rsi_overbought = rsi_overbought;
        self.rsi_oversold = rsi_oversold;
        self.position_size = position_size;
        self
    }
    
    /// Initialize the indicators with historical data
    fn initialize_indicators(&mut self, prices: &[f64]) -> crate::Result<()> {
        if prices.len() < self.ema_long_period.max(self.rsi_period) {
            return Err(crate::Error::StrategyError(
                "Not enough data to initialize indicators".to_string(),
            ));
        }
        
        // Initialize EMAs
        let mut ema_short = ExponentialMovingAverage::new(self.ema_short_period).map_err(|e| Error::StrategyError(format!("Failed to init ema_short: {}", e)))?;
        let mut ema_long = ExponentialMovingAverage::new(self.ema_long_period).map_err(|e| Error::StrategyError(format!("Failed to init ema_long: {}", e)))?;
        
        // Initialize RSI
        let mut rsi = RelativeStrengthIndex::new(self.rsi_period).map_err(|e| Error::StrategyError(format!("Failed to init RSI: {}", e)))?;
        
        // Warm up indicators with historical data
        for &price in prices {
            let _ = ema_short.next(price);
            let _ = ema_long.next(price);
            let _ = rsi.next(price);
        }
        
        self.ema_short = Some(ema_short);
        self.ema_long = Some(ema_long);
        self.rsi = Some(rsi);
        
        Ok(())
    }
}

#[async_trait]
impl TradingStrategy for MomentumStrategy {
    fn name(&self) -> &'static str {
        "Momentum"
    }
    
    async fn initialize(&mut self, params: HashMap<String, String>) -> crate::Result<()> {
        // Update parameters if provided
        if let Some(ema_short) = params.get("ema_short") {
            self.ema_short_period = ema_short.parse().map_err(|_| 
                crate::Error::StrategyError("Invalid EMA short period".to_string())
            )?;
        }
        
        if let Some(ema_long) = params.get("ema_long") {
            self.ema_long_period = ema_long.parse().map_err(|_| 
                crate::Error::StrategyError("Invalid EMA long period".to_string())
            )?;
        }
        
        if let Some(rsi_period) = params.get("rsi_period") {
            self.rsi_period = rsi_period.parse().map_err(|_| 
                crate::Error::StrategyError("Invalid RSI period".to_string())
            )?;
        }
        
        if let Some(overbought) = params.get("rsi_overbought") {
            self.rsi_overbought = overbought.parse().map_err(|_| 
                crate::Error::StrategyError("Invalid RSI overbought level".to_string())
            )?;
        }
        
        if let Some(oversold) = params.get("rsi_oversold") {
            self.rsi_oversold = oversold.parse().map_err(|_| 
                crate::Error::StrategyError("Invalid RSI oversold level".to_string())
            )?;
        }
        
        if let Some(size) = params.get("position_size") {
            self.position_size = size.parse().map_err(|_| 
                crate::Error::StrategyError("Invalid position size".to_string())
            )?;
        }
        
        Ok(())
    }
    
    async fn analyze(&self, market_data: &crate::utils::types::MarketData) -> crate::Result<Vec<TradeSignal>> {
        if market_data.candles.is_empty() {
            return Ok(Vec::new());
        }
        
        let mut signals = Vec::new();
        let latest = market_data.candles.last().unwrap();
        let current_price = latest.close;
        
        // Extract closing prices
        let prices: Vec<f64> = market_data.candles.iter().map(|c| c.close).collect();
        
        // Initialize indicators if not already done
        if self.ema_short.is_none() {
            // In a real implementation, we would initialize with historical data
            // For simplicity, we'll just use the current data
            let mut strategy = self.clone();
            strategy.initialize_indicators(&prices)?;
            return strategy.analyze(market_data).await;
        }
        
        // Clone indicators to avoid borrowing issues
        let ema_short = self.ema_short.as_ref().unwrap().clone().next(current_price);
        let ema_long = self.ema_long.as_ref().unwrap().clone().next(current_price);
        let rsi = self.rsi.as_ref().unwrap().clone().next(current_price);
        
        // Generate signals based on momentum indicators
        let ema_crossover = ema_short > ema_long; // Bullish when short EMA crosses above long EMA
        let rsi_oversold = rsi < self.rsi_oversold;
        let rsi_overbought = rsi > self.rsi_overbought;
        
        // Buy signal: Bullish EMA crossover and RSI not overbought
        if ema_crossover && !rsi_overbought {
            signals.push(TradeSignal {
                strategy_name: self.name().to_string(),
                symbol: "SOL/USDC".to_string(), // Should be dynamic
                action: Action::Buy,
                quantity: self.position_size,
                price: None, // Market order
                reason: format!(
                    "Bullish EMA crossover (EMA{}: {:.2} > EMA{}: {:.2}) with RSI: {:.2}",
                    self.ema_short_period, ema_short,
                    self.ema_long_period, ema_long,
                    rsi
                ),
                confidence: 0.7,
            });
        }
        // Sell signal: Bearish EMA crossover or RSI overbought
        else if !ema_crossover || rsi_overbought {
            signals.push(TradeSignal {
                strategy_name: self.name().to_string(),
                symbol: "SOL/USDC".to_string(), // Should be dynamic
                action: Action::Sell,
                quantity: self.position_size,
                price: None, // Market order
                reason: if !ema_crossover {
                    format!(
                        "Bearish EMA crossover (EMA{}: {:.2} < EMA{}: {:.2})",
                        self.ema_short_period, ema_short,
                        self.ema_long_period, ema_long
                    )
                } else {
                    format!("RSI overbought: {:.2}", rsi)
                },
                confidence: if rsi_overbought { 0.8 } else { 0.6 },
            });
        }
        
        Ok(signals)
    }
    
    fn get_parameters(&self) -> HashMap<String, String> {
        let mut params = HashMap::new();
        params.insert("ema_short".to_string(), self.ema_short_period.to_string());
        params.insert("ema_long".to_string(), self.ema_long_period.to_string());
        params.insert("rsi_period".to_string(), self.rsi_period.to_string());
        params.insert("rsi_overbought".to_string(), self.rsi_overbought.to_string());
        params.insert("rsi_oversold".to_string(), self.rsi_oversold.to_string());
        params.insert("position_size".to_string(), self.position_size.to_string());
        params
    }
}

// Implement Clone for MomentumStrategy
impl Clone for MomentumStrategy {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            ema_short_period: self.ema_short_period,
            ema_long_period: self.ema_long_period,
            rsi_period: self.rsi_period,
            rsi_overbought: self.rsi_overbought,
            rsi_oversold: self.rsi_oversold,
            position_size: self.position_size,
            ema_short: None, // Will be reinitialized
            ema_long: None,  // Will be reinitialized
            rsi: None,       // Will be reinitialized
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    
    fn create_test_data() -> Vec<OHLCV> {
        // Create test data with an upward trend
        let mut data = Vec::new();
        let base_time = Utc::now().timestamp();
        let mut price = 100.0;
        
        for i in 0..100 {
            // Add some noise to the trend
            let noise = (i as f64 * 0.1).sin() * 2.0;
            price += 0.5 + noise * 0.1; // Upward trend with noise
            
            data.push(OHLCV {
                timestamp: base_time + (i * 60) as i64, // 1 minute intervals
                open: price,
                high: price + 0.1,
                low: price - 0.1,
                close: price,
                volume: 1000.0,
            });
        }
        
        data
    }
    
    #[tokio::test]
    async fn test_momentum_strategy() {
        let mut strategy = MomentumStrategy::new()
            .with_parameters(9, 21, 14, 70.0, 30.0, 0.1);
            
        // Initialize with default parameters
        assert!(strategy.initialize(HashMap::new()).await.is_ok());
        
        // Create test market data
        let ohlcv = create_test_data();
        let market_data = MarketData {
            ohlcv,
            order_book: None,
            recent_trades: None,
        };
        
        // Analyze the market data
        let signals = strategy.analyze(&market_data).await;
        assert!(signals.is_ok());
        let signals = signals.unwrap();
        
        // We might get signals based on the test data
        // The exact number depends on the price movement
        assert!(signals.len() <= 2); // Shouldn't generate too many signals
        
        // Verify the parameters
        let params = strategy.get_parameters();
        assert_eq!(params.get("ema_short").unwrap(), "9");
        assert_eq!(params.get("ema_long").unwrap(), "21");
        assert_eq!(params.get("rsi_period").unwrap(), "14");
        assert_eq!(params.get("rsi_overbought").unwrap(), "70");
        assert_eq!(params.get("rsi_oversold").unwrap(), "30");
        assert_eq!(params.get("position_size").unwrap(), "0.1");
    }
    
    #[tokio::test]
    async fn test_initialize_with_parameters() {
        let mut strategy = MomentumStrategy::new();
        
        let mut params = HashMap::new();
        params.insert("ema_short".to_string(), "5".to_string());
        params.insert("ema_long".to_string(), "20".to_string());
        params.insert("rsi_period".to_string(), "10".to_string());
        params.insert("rsi_overbought".to_string(), "75".to_string());
        params.insert("rsi_oversold".to_string(), "25".to_string());
        params.insert("position_size".to_string(), "0.2".to_string());
        
        assert!(strategy.initialize(params).await.is_ok());
        
        let params = strategy.get_parameters();
        assert_eq!(params.get("ema_short").unwrap(), "5");
        assert_eq!(params.get("ema_long").unwrap(), "20");
        assert_eq!(params.get("rsi_period").unwrap(), "10");
        assert_eq!(params.get("rsi_overbought").unwrap(), "75");
        assert_eq!(params.get("rsi_oversold").unwrap(), "25");
        assert_eq!(params.get("position_size").unwrap(), "0.2");
    }
}
