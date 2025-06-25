//! Configuration module for the trading bot

pub mod position_sizer;
mod template;

use crate::utils::error::{Error, Result};
use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use position_sizer::PositionSizerConfig;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use solana_sdk::signature::Keypair;
use std::env;
use std::fs;

pub use template::{generate_commented_config_template, generate_config_template};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
/// Main configuration structure
/// TODO: Add support for environment-specific overrides (e.g., config.local.toml)
pub struct Config {
    /// Configuration file version
    pub version: String,
    /// Solana RPC configuration
    pub solana: SolanaConfig,

    /// Trading configuration
    pub trading: TradingConfig,

    /// Risk management configuration
    pub risk: RiskConfig,

    /// Wallet configuration
    pub wallet: WalletConfig,

    /// Performance monitoring configuration
    pub performance: PerformanceConfig,
}

/// Solana RPC configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaConfig {
    /// Optional Helius API key (for enhanced RPC bandwidth)
    pub helius_api_key: Option<String>,
    /// RPC endpoint URL
    pub rpc_url: String,

    /// WebSocket endpoint URL (optional)
    pub ws_url: Option<String>,

    /// Commitment level
    pub commitment: String,

    /// Timeout for RPC requests in seconds
    pub timeout_seconds: u64,

    /// Maximum number of retries for failed requests
    pub max_retries: u8,

    /// Rate limit in requests per second
    pub rate_limit_rps: u32,
}

/// Trading configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingConfig {
    /// Default trading pair (e.g., SOL/USDC)
    pub default_pair: String,

    /// Default order size in quote currency
    pub default_order_size: f64,

    /// Maximum number of open positions
    pub max_open_positions: usize,

    /// Maximum position size as a percentage of portfolio
    pub max_position_size_pct: f64,

    /// Starting cash balance in USD used to seed the portfolio
    #[serde(default = "default_starting_balance_usd")]
    pub starting_balance_usd: f64,

    /// Enable/disable trading
    pub trading_enabled: bool,

    /// Enable/disable paper trading
    pub paper_trading: bool,
    /// List of strategy configurations
    #[serde(default)]
    pub strategies: Vec<crate::strategies::StrategyConfig>,

    /// Maximum allowed slippage (in basis points, 1bp = 0.01%). Default 200 = 2%.
    #[serde(default = "default_slippage_bps")]
    pub slippage_bps: u16,

    /// Maximum fee to pay per transaction in lamports (1e-9 SOL). Default 5_000 lamports = 0.000005 SOL.
    #[serde(default = "default_max_fee_lamports")]
    pub max_fee_lamports: u64,

    /// Trade size above which orders are split (SOL units). Default 1 SOL.
    #[serde(default = "default_split_threshold_sol")]
    pub split_threshold_sol: f64,

    /// Chunk size for split orders (SOL units). Default 0.25 SOL.
    #[serde(default = "default_split_chunk_sol")]
    pub split_chunk_sol: f64,

    /// Maximum random delay (in ms) between split chunks. Default 1200 ms.
    #[serde(default = "default_split_delay_ms")]
    pub split_delay_ms: u64,
    // ---------- helper defaults below ----------
}

/// Risk management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfig {
    /// Maximum allowed drawdown percentage
    pub max_drawdown_pct: f64,

    /// Maximum position risk percentage
    pub max_position_risk_pct: f64,

    /// Daily loss limit percentage
    pub daily_loss_limit_pct: f64,

    /// Maximum leverage (1.0 = no leverage)
    pub max_leverage: f64,

    /// Enable/disable stop losses
    pub stop_loss_enabled: bool,

    /// Default stop loss percentage
    pub default_stop_loss_pct: f64,

    /// Default take profit percentage
    pub default_take_profit_pct: f64,

    /// Optional position sizer configuration
    #[serde(default)]
    pub position_sizer: Option<PositionSizerConfig>,
}

/// Wallet configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConfig {
    /// Wallet private key (base58 encoded)
    pub private_key: Option<String>,

    /// Wallet file path (alternative to private_key)
    pub keypair_path: Option<String>,

    /// Pool of wallet private keys (base58) used for automatic rotation
    #[serde(default)]
    pub wallets: Vec<String>,

    /// Minimum SOL balance to maintain
    pub min_sol_balance: f64,

    /// Maximum SOL to use for transaction fees
    pub max_fee_sol: f64,
}

/// Performance monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Enable/disable performance tracking
    pub enabled: bool,

    /// Metrics collection interval in seconds
    pub collection_interval_secs: u64,

    /// Maximum number of data points to keep in memory
    pub max_data_points: usize,

    /// Enable/disable detailed logging
    pub detailed_logging: bool,
}

impl Default for Config {
    fn default() -> Self {
        // TODO: Support environment-specific overrides (e.g., config.local.toml)

        Self {
            version: "0.2.0".to_string(),
            solana: SolanaConfig::default(),
            trading: TradingConfig::default(),
            risk: RiskConfig::default(),
            wallet: WalletConfig::default(),
            performance: PerformanceConfig::default(),
        }
    }
}

impl Default for SolanaConfig {
    fn default() -> Self {
        Self {
            rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            ws_url: None,
            commitment: "confirmed".to_string(),
            timeout_seconds: 30,
            max_retries: 3,
            rate_limit_rps: 10,
            helius_api_key: None,
        }
    }
}

impl Default for TradingConfig {
    fn default() -> Self {
        Self {
            default_pair: "SOL/USDC".to_string(),
            default_order_size: 0.1, // 0.1 SOL
            max_open_positions: 5,
            max_position_size_pct: 20.0, // 20% of portfolio
            trading_enabled: false,
            paper_trading: true,
            strategies: Vec::new(),
            slippage_bps: default_slippage_bps(),
            max_fee_lamports: default_max_fee_lamports(),
            split_threshold_sol: default_split_threshold_sol(),
            split_chunk_sol: default_split_chunk_sol(),
            split_delay_ms: default_split_delay_ms(),
            starting_balance_usd: default_starting_balance_usd(),
        }
    }
}

// --------- Helper default functions for serde ---------
fn default_slippage_bps() -> u16 {
    200
}
fn default_max_fee_lamports() -> u64 {
    5_000
}
fn default_split_threshold_sol() -> f64 {
    1.0
}
fn default_split_chunk_sol() -> f64 {
    0.25
}
fn default_split_delay_ms() -> u64 {
    1_200
}
fn default_starting_balance_usd() -> f64 {
    1000.0
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            max_drawdown_pct: 10.0, // 10% max drawdown
            max_position_risk_pct: 2.0,
            daily_loss_limit_pct: 5.0,
            max_leverage: 1.0,
            stop_loss_enabled: true,
            default_stop_loss_pct: 5.0,
            default_take_profit_pct: 10.0,
            position_sizer: None,
        }
    }
}

impl Default for WalletConfig {
    fn default() -> Self {
        Self {
            private_key: None,
            keypair_path: Some("wallet.json".to_string()),
            wallets: Vec::new(),
            min_sol_balance: 0.1, // 0.1 SOL
            max_fee_sol: 0.001,   // 0.001 SOL max fee
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            collection_interval_secs: 60, // 1 minute
            max_data_points: 10_000,
            detailed_logging: true,
        }
    }
}

impl Config {
    /// Serialize default config to TOML string
    pub fn default_toml() -> String {
        toml::to_string_pretty(&Self::default()).expect("serialize default config")
    }

    /// Load configuration from a specific file path
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(&path).map_err(|e| {
            Error::ConfigError(format!("Failed to read config file {:?}: {}", path.as_ref(), e))
        })?;
        let mut cfg: Self = toml::from_str(&content)
            .map_err(|e| Error::ConfigError(format!("Failed to parse config file: {}", e)))?;
        cfg.merge_env()?;
        Ok(cfg)
    }

    /// Save the configuration to a file
    pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        let content = toml::to_string_pretty(self)
            .map_err(|e| Error::ConfigError(format!("Failed to serialize config: {}", e)))?;
        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                Error::ConfigError(format!("Failed to create directory {:?}: {}", parent, e))
            })?;
        }
        std::fs::write(path, content).map_err(|e| {
            Error::ConfigError(format!("Failed to write config file {:?}: {}", path, e))
        })?;
        Ok(())
    }

    /// Validate the configuration for required fields and reasonable values
    pub fn validate(&self) -> Result<()> {
        // Check config version
        if self.version.trim().is_empty() {
            return Err(crate::Error::ConfigError(
                "Config version must be set (e.g., '0.2.0')".to_string(),
            ));
        }
        // Solana config
        if self.solana.rpc_url.trim().is_empty() {
            return Err(crate::Error::ConfigError("Solana RPC URL must be set".to_string()));
        }
        if self.solana.commitment.trim().is_empty() {
            return Err(crate::Error::ConfigError("Solana commitment must be set".to_string()));
        }
        if self.solana.timeout_seconds == 0 {
            return Err(crate::Error::ConfigError(
                "Solana timeout_seconds must be > 0".to_string(),
            ));
        }
        // Trading config
        if self.trading.default_pair.trim().is_empty() {
            return Err(crate::Error::ConfigError("Default trading pair must be set".to_string()));
        }
        if self.trading.default_order_size <= 0.0 {
            return Err(crate::Error::ConfigError("Default order size must be > 0".to_string()));
        }
        if self.trading.max_open_positions == 0 {
            return Err(crate::Error::ConfigError("max_open_positions must be > 0".to_string()));
        }
        if self.trading.max_position_size_pct > 100.0 {
            return Err(crate::Error::ConfigError(
                "max_position_size_pct cannot exceed 100".to_string(),
            ));
        }
        // Wallet config
        if self.wallet.private_key.is_none()
            && self.wallet.keypair_path.is_none()
            && self.wallet.wallets.is_empty()
        {
            return Err(crate::Error::ConfigError(
                "Either wallet.private_key or wallet.keypair_path must be set".to_string(),
            ));
        }
        // Risk config
        if self.risk.max_drawdown_pct > 100.0 {
            return Err(crate::Error::ConfigError(
                "max_drawdown_pct cannot exceed 100".to_string(),
            ));
        }
        if self.risk.max_position_risk_pct > 100.0 {
            return Err(crate::Error::ConfigError(
                "max_position_risk_pct cannot exceed 100".to_string(),
            ));
        }
        // TODO: Add more checks as needed
        Ok(())
    }

    /// Load configuration from default locations
    pub fn load() -> Result<Self> {
        // Try to load from current directory
        if let Ok(config) = Self::from_file("config.toml") {
            return Ok(config);
        }

        // Try to load from user config directory
        if let Some(mut path) = dirs::config_dir() {
            path.push("algotraderv2");
            path.push("config.toml");
            if path.exists() {
                return Self::from_file(path);
            }
        }

        // Return default config if no config file found
        let mut config = Self::default();
        config.merge_env()?;
        Ok(config)
    }

    /// Merge environment variables into the configuration
    pub fn merge_env(&mut self) -> Result<()> {
        if let Ok(rpc_url) = env::var("SOLANA_RPC_URL") {
            self.solana.rpc_url = rpc_url;
        }

        if let Ok(ws_url) = env::var("SOLANA_WS_URL") {
            self.solana.ws_url = Some(ws_url);
        }

        if let Ok(private_key) = env::var("WALLET_PRIVATE_KEY") {
            self.wallet.private_key = Some(private_key);
        }

        // Priority env var override for absolute keypair path
        if let Ok(env_keypair) = env::var("SOLANA_KEYPAIR") {
            self.wallet.keypair_path = Some(env_keypair);
        }

        if let Ok(keypair_path) = env::var("WALLET_KEYPAIR_PATH") {
            self.wallet.keypair_path = Some(keypair_path);
        }

        Ok(())
    }

    /// Decrypt an AES-256-GCM encrypted keypair file. The file format is assumed to be:
    /// [12 bytes nonce][ciphertext...]. The key is derived as SHA-256(passphrase).
    pub fn decrypt_keyfile<P: AsRef<std::path::Path>>(
        path: P, passphrase: &str,
    ) -> Result<Vec<u8>> {
        use aes_gcm::aead::Aead;
        use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
        use sha2::{Digest, Sha256};
        use std::fs;

        let data = fs::read(path)?;
        if data.len() < 13 {
            return Err(Error::WalletError("Encrypted keyfile too short".into()));
        }
        let (nonce_bytes, cipher_bytes) = data.split_at(12);
        let key = Sha256::digest(passphrase.as_bytes());
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| Error::WalletError(format!("AES init error: {e}")))?;
        let nonce = Nonce::from_slice(nonce_bytes);
        let plaintext = cipher
            .decrypt(nonce, cipher_bytes.as_ref())
            .map_err(|e| Error::WalletError(format!("Decrypt error: {e}")))?;
        Ok(plaintext)
    }

    pub fn load_keypair(&self) -> Result<Keypair> {
        // Try to load from private key first
        if let Some(ref private_key) = self.wallet.private_key {
            let bytes: Vec<u8> = bs58::decode(private_key).into_vec()?;
            let keypair = Keypair::from_bytes(&bytes)
                .map_err(|e| Error::WalletError(format!("Keypair from_bytes error: {}", e)))?;
            return Ok(keypair);
        }

        // Then try to load from keypair file
        if let Some(ref keypair_path) = self.wallet.keypair_path {
            // First try to read as UTF-8 and decode as base58 (the format written by `algotrader init`)
            match fs::read_to_string(keypair_path) {
                | Ok(s) => {
                    let trimmed = s.trim().trim_matches('"');
                    if let Ok(decoded) = bs58::decode(trimmed).into_vec() {
                        if let Ok(kp) = Keypair::from_bytes(&decoded) {
                            return Ok(kp);
                        }
                    }
                }
                | Err(_) => { /* fallthrough to raw bytes */ }
            }

            // If encrypted file (ends with .enc) attempt decryption first
            if keypair_path.ends_with(".enc") {
                if let Ok(pass) = env::var("KEYFILE_PASSPHRASE") {
                    if let Ok(decrypted) = Self::decrypt_keyfile(keypair_path, &pass) {
                        if let Ok(kp) = Keypair::from_bytes(&decrypted) {
                            return Ok(kp);
                        }
                    }
                }
            }
            // Fallback: treat file contents as raw 64-byte keypair bytes
            let keypair_bytes = fs::read(keypair_path)?;
            let keypair = Keypair::from_bytes(&keypair_bytes)
                .map_err(|e| Error::WalletError(format!("Keypair from_bytes error: {}", e)))?;
            return Ok(keypair);
        }

        Err(Error::WalletError("No wallet private key or keypair file provided".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.solana.rpc_url, "https://api.mainnet-beta.solana.com");
        assert_eq!(config.trading.default_pair, "SOL/USDC");
        assert!(config.wallet.keypair_path.is_some());
    }

    #[test]
    fn test_save_and_load_config() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let mut config = Config::default();
        config.solana.rpc_url = "https://testnet.solana.com".to_string();

        // Save config
        config.save(&config_path).unwrap();

        // Load config
        let loaded_config = Config::from_file(&config_path).unwrap();
        assert_eq!(loaded_config.solana.rpc_url, "https://testnet.solana.com");
    }

    #[test]
    fn test_merge_env() {
        temp_env::with_vars(
            vec![
                ("SOLANA_RPC_URL", Some("https://testnet.solana.com")),
                ("WALLET_PRIVATE_KEY", Some("test_private_key")),
            ],
            || {
                let mut config = Config::default();
                config.merge_env().unwrap();

                assert_eq!(config.solana.rpc_url, "https://testnet.solana.com");
                assert_eq!(config.wallet.private_key, Some("test_private_key".to_string()));
            },
        );
    }
}
