use std::sync::Arc;

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use spl_token::state::Account as TokenAccount;
use tokio::sync::RwLock;

/// Convenience wrapper around an on-chain Solana wallet (keypair + RPC)
#[derive(Clone)]
pub struct Wallet {
    rpc: Arc<RpcClient>,
    keypair: Arc<Keypair>,
    // cached SOL balance (lamports) to reduce RPC load; refreshed on every query
    sol_balance: Arc<RwLock<u64>>,
}

impl Wallet {
    /// Create wallet from pre-constructed RpcClient and Keypair
    pub fn new(rpc: RpcClient, keypair: Keypair) -> Self {
        Self {
            rpc: Arc::new(rpc),
            keypair: Arc::new(keypair),
            sol_balance: Arc::new(RwLock::new(0)),
        }
    }

    /// Public key of the wallet
    pub fn pubkey(&self) -> Pubkey {
        self.keypair.pubkey()
    }

    /// Return SOL balance in lamports, refreshing the cache
    pub async fn get_sol_balance(&self) -> Result<u64> {
        let lamports = self
            .rpc
            .get_balance(&self.pubkey())
            .await
            .context("fetch balance")?;
        *self.sol_balance.write().await = lamports;
        Ok(lamports)
    }

    /// Get SOL balance in SOL units (f64)
    pub async fn get_sol_balance_f64(&self) -> Result<f64> {
        Ok(self.get_sol_balance().await? as f64 / 1_000_000_000.0)
    }

    /// Fetch SPL token balances for this wallet. Returns mapping mint -> amount (raw token units).
    pub async fn get_spl_balances(&self) -> Result<std::collections::HashMap<Pubkey, u64>> {
        use solana_account_decoder::UiAccountEncoding;
        use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
        use solana_sdk::program_pack::Pack;

        let program_id = spl_token::id();

        #[allow(deprecated)]
        let filters = Some(vec![solana_client::rpc_filter::RpcFilterType::Memcmp(
            solana_client::rpc_filter::Memcmp {
                offset: 32, // owner field start
                bytes: solana_client::rpc_filter::MemcmpEncodedBytes::Base58(
                    self.pubkey().to_string(),
                ),
                encoding: None,
            },
        )]);

        let acc_cfg = RpcProgramAccountsConfig {
            filters,
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                ..RpcAccountInfoConfig::default()
            },
            with_context: None,
        };

        let accounts = self
            .rpc
            .get_program_accounts_with_config(&program_id, acc_cfg)
            .await?;
        let mut map = std::collections::HashMap::new();
        for (_pubkey, acc) in accounts {
            let data_b64 = acc.data;
            if let Ok(raw) = STANDARD.decode(data_b64) {
                if let Ok(ta) = TokenAccount::unpack(&raw) {
                    *map.entry(ta.mint).or_insert(0) += ta.amount;
                }
            }
        }
        Ok(map)
    }

    /// Sign and send a transaction, returning signature.
    pub async fn sign_and_send(
        &self, mut tx: solana_sdk::transaction::Transaction,
    ) -> Result<solana_sdk::signature::Signature> {
        let recent_blockhash = self.rpc.get_latest_blockhash().await?;
        tx.try_sign(&[self.keypair.as_ref()], recent_blockhash)?;
        let sig = self.rpc.send_and_confirm_transaction(&tx).await?;
        Ok(sig)
    }

    /// Sign and send a base64-encoded VersionedTransaction produced by Jupiter swap API
    pub async fn sign_and_send_serialized_tx(
        &self, tx_b64: &str,
    ) -> Result<solana_sdk::signature::Signature> {
        use solana_sdk::signer::signers::Signers;
        use solana_sdk::transaction::VersionedTransaction;
        let tx_bytes = STANDARD.decode(tx_b64.trim())?;
        let vtx: VersionedTransaction = bincode::deserialize(&tx_bytes)?;
        // Create a freshly signed copy using our keypair
        let signed_tx =
            VersionedTransaction::try_new(vtx.message.clone(), &[self.keypair.as_ref()])?;
        let sig = self.rpc.send_and_confirm_transaction(&signed_tx).await?;
        Ok(sig)
    }
}
