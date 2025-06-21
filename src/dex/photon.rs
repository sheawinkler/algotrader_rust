//! Photon DEX client implementation

use crate::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use reqwest::Client;

const PHOTON_API_BASE: &str = "https://api.photon-solana.com";

/// Client for interacting with the Photon DEX aggregator
pub struct PhotonClient {
    client: Client,
    api_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct QuoteRequest {
    input_mint: String,
    output_mint: String,
    amount: String,
    slippage_bps: u16,
}

#[derive(Debug, Deserialize)]
struct QuoteResponse {
    input_mint: String,
    output_mint: String,
    in_amount: String,
    out_amount: String,
    other_amount_threshold: String,
    swap_mode: String,
    slippage_bps: u16,
    price_impact_pct: String,
    time_taken: f64,
}

impl PhotonClient {
    /// Create a new Photon client
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;
            
        Ok(Self {
            client,
            api_key: None,
        })
    }
    
    /// Create a new Photon client with an API key
    pub fn with_api_key(api_key: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;
            
        Ok(Self {
            client,
            api_key: Some(api_key),
        })
    }
    
    async fn get_quote(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
        slippage_bps: u16,
    ) -> Result<QuoteResponse> {
        let url = format!("{}/v1/quote", PHOTON_API_BASE);
        
        let request = QuoteRequest {
            input_mint: input_mint.to_string(),
            output_mint: output_mint.to_string(),
            amount: amount.to_string(),
            slippage_bps,
        };
        
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await?;
            
        let quote = response.json::<QuoteResponse>().await?;
        
        Ok(quote)
    }
}

#[async_trait]
impl super::DexClient for PhotonClient {
    fn name(&self) -> &'static str {
        "Photon"
    }
    
    async fn get_price(&self, base_token: &str, quote_token: &str) -> Result<f64> {
        // Get a quote for 1 unit of base token
        let quote = self.get_quote(base_token, quote_token, 1_000_000, 50).await?;
        
        // Convert the output amount to a price
        let out_amount: f64 = quote.out_amount.parse().map_err(|e| {
            crate::Error::DexError(format!("Failed to parse out_amount: {}", e))
        })?;
        
        // Since we requested 1 unit (1_000_000 lamports for SOL), the price is out_amount / 1_000_000
        let price = out_amount / 1_000_000.0;
        
        Ok(price)
    }
    
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
        _stop_price: Option<f64>,
        _take_profit_price: Option<f64>,
        signer: &str,
    ) -> Result<String> {
        // Handle order types
        match order_type {
            crate::utils::types::OrderType::Market => {},
            crate::utils::types::OrderType::Limit => {
                let current_price = self.get_price(base_token, quote_token).await?;
                if let Some(lp) = limit_price {
                    let cond = if is_buy { current_price <= lp } else { current_price >= lp };
                    if !cond {
                        return Err(crate::Error::DexError("Limit price not satisfied".into()));
                    }
                } else {
                    return Err(crate::Error::InvalidArgument("limit_price required for Limit order".into()));
                }
            },
            crate::utils::types::OrderType::Stop | crate::utils::types::OrderType::StopLimit => {
                if let Some(sp) = _stop_price {
                    let current_price = self.get_price(base_token, quote_token).await?;
                    let triggered = if is_buy { current_price >= sp } else { current_price <= sp };
                    if !triggered {
                        return Err(crate::Error::DexError("Stop price not triggered".into()));
                    }
                    if order_type == crate::utils::types::OrderType::StopLimit {
                        if let Some(lp) = limit_price {
                            let cond = if is_buy { current_price <= lp } else { current_price >= lp };
                            if !cond {
                                return Err(crate::Error::DexError("Limit condition after stop not satisfied".into()));
                            }
                        }
                    }
                } else {
                    return Err(crate::Error::InvalidArgument("stop_price required for Stop order".into()));
                }
            }
        }

        // Convert amount to lamports (assuming 6 decimals for most tokens)
        let amount_lamports = (amount * 1_000_000.0) as u64;
        
        // Get a quote
        let quote = self.get_quote(
            if is_buy { quote_token } else { base_token },
            if is_buy { base_token } else { quote_token },
            amount_lamports,
            50, // 0.5% slippage
        ).await?;
        
        // In a real implementation, we would:
        // 1. Get the swap transaction from Photon API
        // 2. Sign it with the user's wallet
        // 3. Send it to the network
        // 4. Return the transaction signature
        
        // For now, return a placeholder
        Ok("tx_signature_placeholder".to_string())
    }
    
    async fn get_balance(&self, _token: &str) -> Result<f64> {
        // In a real implementation, we would query the user's wallet
        // For now, return a placeholder
        Ok(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_photon_client_initialization() {
        let client = PhotonClient::new();
        assert!(client.is_ok());
    }
    
    #[tokio::test]
    async fn test_photon_with_api_key() {
        let client = PhotonClient::with_api_key("test_key".to_string());
        assert!(client.is_ok());
    }
}
