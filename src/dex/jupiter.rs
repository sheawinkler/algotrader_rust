//! Jupiter DEX client implementation

use crate::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

const JUPITER_API_BASE: &str = "https://quote-api.jup.ag/v6";

/// Client for interacting with the Jupiter DEX
pub struct JupiterClient {
    client: Client,
    api_key: Option<String>,
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
    platform_fee: Option<PlatformFee>,
    price_impact_pct: String,
    route: Vec<Vec<RouteStep>>,
    context: Context,
    time_taken: f64,
}

#[derive(Debug, Deserialize)]
struct PlatformFee {
    amount: String,
    fee_bps: u16,
}

#[derive(Debug, Deserialize)]
struct RouteStep {
    // Define fields based on Jupiter API response
}

#[derive(Debug, Deserialize)]
struct Context {
    slot: u64,
    // Other context fields
}

#[derive(Debug, Deserialize)]
struct SwapResponse {
    #[serde(rename = "swapTransaction")]
    swap_transaction: String,
}

use serde_json::json;

impl JupiterClient {
    /// Create a new Jupiter client
    pub fn new() -> Result<Self> {
        let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

        Ok(Self { client, api_key: None })
    }

    /// Create a new Jupiter client with an API key
    pub fn with_api_key(api_key: String) -> Result<Self> {
        let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

        Ok(Self { client, api_key: Some(api_key) })
    }

    async fn get_quote(
        &self, input_mint: &str, output_mint: &str, amount: u64, slippage_bps: u16,
    ) -> Result<QuoteResponse> {
        let url = format!(
            "{}/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}",
            JUPITER_API_BASE, input_mint, output_mint, amount, slippage_bps
        );

        let response = self.client.get(&url).send().await?;
        let quote = response.json::<QuoteResponse>().await?;

        Ok(quote)
    }

    /// Fetch a base64-encoded swap transaction from Jupiter `/swap` endpoint
    async fn get_swap_transaction(
        &self, input_mint: &str, output_mint: &str, amount: u64, slippage_bps: u16,
        user_pubkey: &str,
    ) -> Result<String> {
        let url = "https://quote-api.jup.ag/v6/swap"; // official endpoint
        let body = json!({
            "inputMint": input_mint,
            "outputMint": output_mint,
            "inAmount": amount.to_string(),
            "slippageBps": slippage_bps,
            "userPublicKey": user_pubkey,
            "wrapUnwrapSol": true,
            "feeBps": 0u16
        });
        let resp = self.client.post(url).json(&body).send().await?;
        let sr: SwapResponse = resp.json().await?;
        Ok(sr.swap_transaction)
    }
}

#[async_trait]
impl super::DexClient for JupiterClient {
    fn name(&self) -> &'static str {
        "Jupiter"
    }

    async fn get_price(&self, base_token: &str, quote_token: &str) -> Result<f64> {
        // Get a quote for 1 unit of base token
        let quote = self
            .get_quote(base_token, quote_token, 1_000_000, 50)
            .await?;

        // Convert the output amount to a price
        let out_amount: f64 = quote
            .out_amount
            .parse()
            .map_err(|e| crate::Error::DexError(format!("Failed to parse out_amount: {}", e)))?;

        // Since we requested 1 unit (1_000_000 lamports for SOL), the price is out_amount / 1_000_000
        let price = out_amount / 1_000_000.0;

        Ok(price)
    }

    async fn execute_trade(
        &self, base_token: &str, quote_token: &str, amount: f64, is_buy: bool, slippage_bps: u16,
        max_fee_lamports: u64, order_type: crate::utils::types::OrderType,
        limit_price: Option<f64>, _stop_price: Option<f64>, _take_profit_price: Option<f64>,
        wallet: &crate::wallet::Wallet,
    ) -> Result<String> {
        // Handle different order types
        match order_type {
            | crate::utils::types::OrderType::Market => {
                // proceed
            }
            | crate::utils::types::OrderType::Limit => {
                let current_price = self.get_price(base_token, quote_token).await?;
                if let Some(lp) = limit_price {
                    let condition = if is_buy {
                        current_price <= lp
                    } else {
                        current_price >= lp
                    };
                    if !condition {
                        return Err(crate::Error::DexError("Limit price not satisfied".into()));
                    }
                } else {
                    return Err(crate::Error::InvalidArgument(
                        "limit_price required for Limit order".into(),
                    ));
                }
            }
            | crate::utils::types::OrderType::Stop | crate::utils::types::OrderType::StopLimit => {
                if let Some(sp) = _stop_price {
                    let current_price = self.get_price(base_token, quote_token).await?;
                    let triggered = if is_buy {
                        current_price >= sp
                    } else {
                        current_price <= sp
                    };
                    if !triggered {
                        return Err(crate::Error::DexError("Stop price not triggered".into()));
                    }
                    // For StopLimit treat as Limit if limit_price supplied, else Market
                    if order_type == crate::utils::types::OrderType::StopLimit {
                        if let Some(lp) = limit_price {
                            let cond = if is_buy {
                                current_price <= lp
                            } else {
                                current_price >= lp
                            };
                            if !cond {
                                return Err(crate::Error::DexError(
                                    "Limit condition after stop not satisfied".into(),
                                ));
                            }
                        }
                    }
                } else {
                    return Err(crate::Error::InvalidArgument(
                        "stop_price required for Stop order".into(),
                    ));
                }
            }
        }

        // Convert amount to lamports (assuming 6 decimals for most tokens)
        let amount_lamports = (amount * 1_000_000.0) as u64;

        // Get a quote
        let quote = self
            .get_quote(
                if is_buy { quote_token } else { base_token },
                if is_buy { base_token } else { quote_token },
                amount_lamports,
                50, // 0.5% slippage
            )
            .await?;

        // In a real implementation, we would:
        // 1. Get the swap transaction from Jupiter API
        // 2. Sign it with the user's wallet
        // 3. Send it to the network
        // 4. Return the transaction signature

        // Convert ui amount to lamports (assume 6 decimals typical)
        let amount_lamports = (amount * 1_000_000.0) as u64;
        let user_pubkey = wallet.pubkey().to_string();
        let tx_b64 = self
            .get_swap_transaction(
                if is_buy { quote_token } else { base_token },
                if is_buy { base_token } else { quote_token },
                amount_lamports,
                slippage_bps,
                &user_pubkey,
            )
            .await?;
        let sig = wallet.sign_and_send_serialized_tx(&tx_b64).await?;
        Ok(sig.to_string())
    }

    async fn get_balance(&self, token: &str) -> Result<f64> {
        // In a real implementation, we would:
        // 1. Query the user's wallet for the token balance
        // 2. Convert from lamports to token amount

        // For now, return a placeholder
        Ok(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_jupiter_client_initialization() {
        let client = JupiterClient::new();
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_jupiter_with_api_key() {
        let client = JupiterClient::with_api_key("test_key".to_string());
        assert!(client.is_ok());
    }
}
