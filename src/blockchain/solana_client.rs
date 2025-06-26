use std::str::FromStr;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use solana_client::{
    nonblocking::rpc_client::RpcClient as AsyncRpcClient, rpc_config::RpcProgramAccountsConfig,
    rpc_filter::RpcFilterType,
};
use solana_sdk::{
    commitment_config::{CommitmentConfig, CommitmentLevel},
    program_pack::Pack,
    pubkey::Pubkey,
    signer::{keypair::Keypair, Signer},
};
use spl_token::state::Account as TokenAccount;
use tokio::sync::RwLock;

use crate::analysis::wallet_analyzer::WalletAnalysis;

/// Configuration for the Solana client
#[derive(Debug, Clone)]
pub struct SolanaClientConfig {
    pub rpc_url: String,
    pub ws_url: Option<String>,
    pub commitment: CommitmentLevel,
    pub timeout_seconds: u64,
    pub max_retries: u8,
    pub rate_limit_requests_per_second: u32,
}

impl Default for SolanaClientConfig {
    fn default() -> Self {
        Self {
            rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            ws_url: None,
            commitment: CommitmentLevel::Confirmed,
            timeout_seconds: 30,
            max_retries: 3,
            rate_limit_requests_per_second: 10, // Be nice to public RPCs
        }
    }
}

/// A client for interacting with the Solana blockchain
pub struct SolanaClient {
    client: AsyncRpcClient,
    config: SolanaClientConfig,
    keypair: Arc<Keypair>,
    last_request: Arc<RwLock<std::time::Instant>>,
}

impl SolanaClient {
    /// Create a new Solana client
    pub fn new(config: SolanaClientConfig, keypair: Keypair) -> Result<Self> {
        let client = AsyncRpcClient::new_with_commitment(
            config.rpc_url.clone(),
            CommitmentConfig { commitment: config.commitment },
        );

        Ok(Self {
            client,
            config,
            keypair: Arc::new(keypair),
            last_request: Arc::new(RwLock::new(std::time::Instant::now())),
        })
    }

    /// Get the RPC client
    pub fn get_client(&self) -> &AsyncRpcClient {
        &self.client
    }

    /// Get the keypair
    pub fn get_keypair(&self) -> &Keypair {
        &self.keypair
    }

    /// Get the public key of the wallet
    pub fn get_public_key(&self) -> Pubkey {
        self.keypair.pubkey()
    }

    /// Get the token accounts for a wallet
    pub async fn get_token_accounts(&self, wallet: &str) -> Result<Vec<(Pubkey, TokenAccount)>> {
        let wallet_pubkey =
            Pubkey::from_str(wallet).map_err(|e| anyhow!("Invalid wallet address: {}", e))?;

        // Get all token accounts for the wallet
        let filters = vec![RpcFilterType::DataSize(spl_token::state::Account::LEN as u64)];

        let config = RpcProgramAccountsConfig { filters: Some(filters), ..Default::default() };

        let token_accounts = self
            .client
            .get_program_accounts_with_config(&spl_token::id(), config)
            .await
            .map_err(|e| anyhow!("Failed to get token accounts: {}", e))?;

        let mut accounts = Vec::new();
        for (pubkey, account) in token_accounts {
            if let Ok(token_account) = TokenAccount::unpack(&account.data) {
                if token_account.owner == wallet_pubkey {
                    accounts.push((pubkey, token_account));
                }
            }
        }

        Ok(accounts)
    }

    /// Get the wallet balance in SOL
    pub async fn get_sol_balance(&self, wallet: &str) -> Result<f64> {
        let pubkey =
            Pubkey::from_str(wallet).map_err(|e| anyhow!("Invalid wallet address: {}", e))?;

        let balance = self
            .client
            .get_balance(&pubkey)
            .await
            .map_err(|e| anyhow!("Failed to get SOL balance: {}", e))?;

        Ok(balance as f64 / 1_000_000_000.0) // Convert lamports to SOL
    }

    /// Get the token balance for a specific token account
    pub async fn get_token_balance(&self, token_account: &Pubkey) -> Result<f64> {
        let account = self
            .client
            .get_token_account_balance(token_account)
            .await
            .map_err(|e| anyhow!("Failed to get token balance: {}", e))?;

        account
            .amount
            .parse::<f64>()
            .map_err(|e| anyhow!("Failed to parse token balance: {}", e))
            .map(|amount| amount / 10_f64.powi(account.decimals as i32))
    }

    /// Get the token info for a mint
    pub async fn get_token_info(&self, mint: &Pubkey) -> Result<TokenInfo> {
        // For now, return a default token info with the mint address
        // In a real implementation, this would fetch metadata from the token program
        let _account = self
            .client
            .get_account(mint)
            .await
            .map_err(|e| anyhow!("Failed to get token account: {}", e))?;

        Ok(TokenInfo::new("UNKNOWN", "Unknown Token", 9))
    }

    /// Get token metadata
    pub async fn get_token_metadata(&self, _mint: &Pubkey) -> Result<TokenInfo> {
        // For now, return a default token info
        // In a real implementation, this would fetch metadata from the token program
        Ok(TokenInfo::new("UNKNOWN", "Unknown Token", 9))
    }

    /// Analyze a wallet's trading activity
    async fn analyze_wallet(&self, _wallet: &str) -> Result<WalletAnalysis> {
        let _token_accounts = self.get_token_accounts(_wallet).await?;
        let _sol_balance = self.get_sol_balance(_wallet).await?;

        // Get SOL balance
        let sol_balance = self.get_sol_balance(_wallet).await?;

        // TODO: Analyze transaction history
        // TODO: Calculate trading metrics

        Ok(WalletAnalysis {
            address: _wallet.to_string(),
            total_trades: 0,
            profitable_trades: 0,
            total_volume: 0.0,
            total_pnl: 0.0,
            win_rate: 0.0,
            avg_trade_size: 0.0,
            last_trade_time: None,
            risk_score: 0.0,
            common_tokens: vec![],
            holding_period: None,
            success_rate: 0.0,
            avg_hold_time: None,
            max_drawdown: 0.0,
            sharpe_ratio: None,
        })
    }

    /// Get recent transactions for a wallet
    async fn get_recent_transactions(
        &self, _wallet: &str, _limit: usize,
    ) -> Result<Vec<TransactionInfo>> {
        // TODO: Implement transaction history fetching
        Ok(vec![])
    }

    /// Get token holders for a specific token
    async fn get_token_holders(&self, _mint: &str, _limit: usize) -> Result<Vec<TokenHolder>> {
        // TODO: Implement token holder analysis
        Ok(vec![])
    }
}

/// Information about a token
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct TokenInfo {
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
}

impl TokenInfo {
    pub fn new(symbol: &str, name: &str, decimals: u8) -> Self {
        Self { symbol: symbol.to_string(), name: name.to_string(), decimals }
    }
}

/// Information about a transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionInfo {
    pub signature: String,
    pub block_time: i64,
    pub slot: u64,
    pub fee: u64,
    pub status: String,
    pub token_transfers: Vec<TokenTransfer>,
}

/// Information about a token transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenTransfer {
    pub from: String,
    pub to: String,
    pub token: String,
    pub amount: f64,
    pub decimals: u8,
}

/// Information about a token holder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenHolder {
    pub address: String,
    pub amount: f64,
    pub decimals: u8,
    pub ui_amount: f64,
    pub owner: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::signer::keypair::Keypair;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_get_sol_balance() {
        let config = SolanaClientConfig::default();
        let keypair = Keypair::new();
        let client = SolanaClient::new(config, keypair).unwrap();

        // Test with a known wallet (e.g., Solana Foundation)
        let balance = client
            .get_sol_balance("vines1vzrYbzLMRdu58ou5XTby4qAqVRLmqo36NKPTg")
            .await;
        assert!(balance.is_ok());

        // Test with invalid address
        let result = client.get_sol_balance("invalid_address").await;
        assert!(result.is_err());
    }

    #[cfg(feature = "solana-online-tests")]
    #[tokio::test]
    #[ignore]
    async fn test_get_token_accounts() {
        let config = SolanaClientConfig::default();
        let keypair = Keypair::new();
        let client = SolanaClient::new(config, keypair).unwrap();

        // Test with a wallet that likely has token accounts
        let accounts = client
            .get_token_accounts("vines1vzrYbzLMRdu58ou5XTby4qAqVRLmqo36NKPTg")
            .await;
        assert!(accounts.is_ok());
    }
}
