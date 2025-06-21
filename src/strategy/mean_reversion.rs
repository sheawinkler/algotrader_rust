//! Mean Reversion trading strategy implementation.

use super::*;
use crate::Error;
use crate::utils::types::MarketData;
use std::collections::HashMap;
use ta::indicators::SimpleMovingAverage;
use ta::Next;

/// Mean Reversion trading strategy
pub struct MeanReversionStrategy {
    name: String,
    lookback_period: usize,
    entry_z_score: f64,
    exit_z_score: f64,
    position_size: f64,
    sma: Option<SimpleMovingAverage>,
    std_dev: f64,
    prices: Vec<f64>,
}

impl MeanReversionStrategy {
    /// Create a new MeanReversionStrategy with default parameters
    pub fn new() -> Self {
        Self {
            name: "Mean Reversion".to_string(),
            lookback_period: 20,
            entry_z_score: 2.0,
            exit_z_score: 0.5,
            position_size: 0.1, // 10% of portfolio
            sma: None,
            std_dev: 0.0,
            prices: Vec::new(),
        }
    }
    
    /// Update the strategy parameters
    pub fn with_parameters(
        mut self,
        lookback_period: usize,
        entry_z_score: f64,
        exit_z_score: f64,
        position_size: f64,
    ) -> Self {
        self.lookback_period = lookback_period;
        self.entry_z_score = entry_z_score;
        self.exit_z_score = exit_z_score;
        self.position_size = position_size;
        self
    }
    
    /// Calculate the standard deviation of prices
    fn calculate_std_dev(prices: &[f64], mean: f64) -> f64 {
        if prices.is_empty() {
            return 0.0;
        }
        
        let variance = prices.iter()
            .map(|&p| (p - mean).powi(2))
            .sum::<f64>() / prices.len() as f64;
            
        variance.sqrt()
    }
    
    /// Calculate the z-score of the current price
    fn calculate_z_score(&self, price: f64) -> f64 {
        if self.std_dev == 0.0 {
            return 0.0;
        }
        
        // Calculate mean using the SMA
        let mut mean = 0.0;
        if let Some(sma) = &self.sma {
            // Create a clone to avoid borrowing issues
            let mut sma_clone = sma.clone();
            mean = sma_clone.next(price);
        }
        (price - mean) / self.std_dev
    }
}

#[async_trait]
impl TradingStrategy for MeanReversionStrategy {
    fn name(&self) -> &'static str {
        "Mean Reversion"
    }
    
    async fn initialize(&mut self, params: HashMap<String, String>) -> crate::Result<()> {
        // Update parameters if provided
        if let Some(lookback) = params.get("lookback") {
            self.lookback_period = lookback.parse().map_err(|_| 
                crate::Error::StrategyError("Invalid lookback period".to_string())
            )?;
        }
        
        if let Some(entry) = params.get("entry_z_score") {
            self.entry_z_score = entry.parse().map_err(|_| 
                crate::Error::StrategyError("Invalid entry z-score".to_string())
            )?;
        }
        
        if let Some(exit) = params.get("exit_z_score") {
            self.exit_z_score = exit.parse().map_err(|_| 
                crate::Error::StrategyError("Invalid exit z-score".to_string())
            )?;
        }
        
        if let Some(size) = params.get("position_size") {
            self.position_size = size.parse().map_err(|_| 
                crate::Error::StrategyError("Invalid position size".to_string())
            )?;
        }
        
        // Initialize the SMA
        self.sma = Some(SimpleMovingAverage::new(self.lookback_period).unwrap());
        
        Ok(())
    }
    
    async fn analyze(&self, market_data: &crate::utils::types::MarketData) -> crate::Result<Vec<TradeSignal>> {
        if market_data.candles.is_empty() {
            return Ok(Vec::new());
        }
        
        let mut signals = Vec::new();
        let latest = market_data.candles.last().unwrap();
        let current_price = latest.close;
        
        // Update prices
        let mut prices: Vec<f64> = market_data.candles.iter()
            .map(|x| x.close)
            .collect();
            
        // Calculate SMA and standard deviation
        let mut sma = SimpleMovingAverage::new(self.lookback_period).unwrap();
        let mut mean = 0.0;
        
        // Feed prices to the SMA and get the last value
        for &price in &prices {
            mean = sma.next(price);
        }
        
        let std_dev = Self::calculate_std_dev(&prices, mean);
        
        // Calculate z-score
        let z_score = if std_dev > 0.0 {
            (current_price - mean) / std_dev
        } else {
            0.0
        };
        
        // Generate signals based on z-score
        if z_score >= self.entry_z_score {
            // Price is significantly above the mean - sell signal
            signals.push(TradeSignal {
                strategy_name: self.name().to_string(),
                symbol: "SOL/USDC".to_string(), // Should be dynamic
                action: Action::Sell,
                quantity: self.position_size,
                price: None, // Market order
                reason: format!("Price {} is {} std dev above mean (z-score: {:.2})", 
                    current_price, self.entry_z_score, z_score),
                confidence: 0.8,
            });
        } else if z_score <= -self.entry_z_score {
            // Price is significantly below the mean - buy signal
            signals.push(TradeSignal {
                strategy_name: self.name().to_string(),
                symbol: "SOL/USDC".to_string(), // Should be dynamic
                action: Action::Buy,
                quantity: self.position_size,
                price: None, // Market order
                reason: format!("Price {} is {} std dev below mean (z-score: {:.2})", 
                    current_price, self.entry_z_score, z_score),
                confidence: 0.8,
            });
        } else if z_score.abs() <= self.exit_z_score {
            // Price is close to the mean - close position signal
            signals.push(TradeSignal {
                strategy_name: self.name().to_string(),
                symbol: "SOL/USDC".to_string(), // Should be dynamic
                action: Action::ClosePosition,
                quantity: self.position_size,
                price: None, // Market order
                reason: format!("Price {} is close to mean (z-score: {:.2})", current_price, z_score),
                confidence: 0.6,
            });
        }
        
        Ok(signals)
    }
    
    fn get_parameters(&self) -> HashMap<String, String> {
        let mut params = HashMap::new();
        params.insert("lookback".to_string(), self.lookback_period.to_string());
        params.insert("entry_z_score".to_string(), self.entry_z_score.to_string());
        params.insert("exit_z_score".to_string(), self.exit_z_score.to_string());
        params.insert("position_size".to_string(), self.position_size.to_string());
        params
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    
    fn create_test_data() -> Vec<OHLCV> {
        // Create test data with a clear mean-reverting pattern
        let mut data = Vec::new();
        let base_time = Utc::now().timestamp();
        
        // Create a sine wave pattern
        for i in 0..100 {
            let price = 100.0 + 10.0 * (i as f64 * 0.1).sin();
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
    async fn test_mean_reversion_strategy() {
        let mut strategy = MeanReversionStrategy::new()
            .with_parameters(20, 2.0, 0.5, 0.1);
            
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
        
        // We should have at least one signal
        assert!(!signals.is_empty());
        
        // Verify the parameters
        let params = strategy.get_parameters();
        assert_eq!(params.get("lookback").unwrap(), "20");
        assert_eq!(params.get("entry_z_score").unwrap(), "2");
        assert_eq!(params.get("exit_z_score").unwrap(), "0.5");
        assert_eq!(params.get("position_size").unwrap(), "0.1");
    }
    
    #[tokio::test]
    async fn test_initialize_with_parameters() {
        let mut strategy = MeanReversionStrategy::new();
        
        let mut params = HashMap::new();
        params.insert("lookback".to_string(), "30".to_string());
        params.insert("entry_z_score".to_string(), "1.5".to_string());
        params.insert("exit_z_score".to_string(), "0.3".to_string());
        params.insert("position_size".to_string(), "0.2".to_string());
        
        assert!(strategy.initialize(params).await.is_ok());
        
        let params = strategy.get_parameters();
        assert_eq!(params.get("lookback").unwrap(), "30");
        assert_eq!(params.get("entry_z_score").unwrap(), "1.5");
        assert_eq!(params.get("exit_z_score").unwrap(), "0.3");
        assert_eq!(params.get("position_size").unwrap(), "0.2");
    }
}
