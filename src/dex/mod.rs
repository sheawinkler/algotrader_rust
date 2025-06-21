//! DEX (Decentralized Exchange) module for interacting with various DEX protocols.

mod jupiter;
mod raydium;
mod photon;

use async_trait::async_trait;
use std::collections::HashMap;

pub use jupiter::JupiterClient;
pub use raydium::RaydiumClient;
pub use photon::PhotonClient;

/// Trait defining the common interface for all DEX clients
#[async_trait]
pub trait DexClient: Send + Sync {
    /// Get the name of the DEX
    fn name(&self) -> &'static str;
    
    /// Get the current price for a trading pair
    async fn get_price(&self, base_token: &str, quote_token: &str) -> crate::Result<f64>;
    
    /// Execute a trade
    async fn execute_trade(
        &self,
        base_token: &str,
        quote_token: &str,
        amount: f64,
        is_buy: bool,
        slippage_bps: u16,
        max_fee_lamports: u64,
        order_type: crate::utils::types::OrderType,
        limit_price: Option<f64>,
        stop_price: Option<f64>,
        take_profit_price: Option<f64>,
        signer: &str,
    ) -> crate::Result<String>; // Returns transaction hash
    
    /// Get the current balance of a token
    async fn get_balance(&self, token: &str) -> crate::Result<f64>;
}

/// Factory for creating DEX clients
pub struct DexFactory;

impl DexFactory {
    /// Create a new DEX client based on the DEX name
    pub fn create_client(platform: &str) -> crate::Result<Box<dyn DexClient>> {
        match platform.to_lowercase().as_str() {
            "jupiter" => Ok(Box::new(JupiterClient::new()?)),
            "raydium" => Ok(Box::new(RaydiumClient::new()?)),
            "photon" => Ok(Box::new(PhotonClient::new()?)),
            _ => Err(crate::Error::DexError(format!("Unsupported DEX: {}", platform))),
        }
    }
    
    /// Create multiple DEX clients at once
    pub fn create_clients(platforms: &[&str]) -> crate::Result<HashMap<String, Box<dyn DexClient>>> {
        let mut clients = HashMap::new();
        
        for platform in platforms {
            let client = Self::create_client(platform)?;
            clients.insert(platform.to_string(), client);
        }
        
        Ok(clients)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_dex_factory() {
        // Test creating a single client
        let jupiter = DexFactory::create_client("jupiter");
        assert!(jupiter.is_ok());
        
        // Test creating multiple clients
        let clients = DexFactory::create_clients(&["jupiter", "raydium"]);
        assert!(clients.is_ok());
        let clients = clients.unwrap();
        assert_eq!(clients.len(), 2);
        assert!(clients.contains_key("jupiter"));
        assert!(clients.contains_key("raydium"));
        
        // Test unsupported DEX
        let unsupported = DexFactory::create_client("unsupported");
        assert!(matches!(unsupported, Err(crate::Error::DexError(_))));
    }
}
