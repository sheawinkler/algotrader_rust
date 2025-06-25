//! Configuration template generation

use crate::config::Config;
use crate::utils::error::{Error, Result};
use std::fs;
use std::path::Path;

/// Generate a default configuration file at the specified path
pub fn generate_config_template<P: AsRef<Path>>(path: P) -> Result<()> {
    let config = Config::default();
    config
        .save(path)
        .map_err(|e| Error::ConfigError(e.to_string()))
}

/// Generate a configuration file with comments explaining each field
pub fn generate_commented_config_template<P: AsRef<Path>>(path: P) -> std::io::Result<()> {
    let toml_str = r#"# AlgoTraderV2 Configuration
# This is a template configuration file with all available options.
# Uncomment and modify the values as needed.

[wallet]
# Private key in base58 format (alternative to keypair_path)
# private_key = ""

# Path to a file containing the keypair (alternative to private_key)
# If both private_key and keypair_path are None, the default path will be used
keypair_path = "wallet.json"

# Minimum SOL balance to maintain (in SOL)
min_sol_balance = 0.1

# Maximum SOL to use for transaction fees (in SOL)
max_fee_sol = 0.001

[solana]
# Solana RPC endpoint URL
rpc_url = "https://api.mainnet-beta.solana.com"

# WebSocket endpoint URL (optional, for real-time updates)
# ws_url = "wss://api.mainnet-beta.solana.com"

# Commitment level (processed, confirmed, finalized)
commitment = "confirmed"

# Timeout for RPC requests in seconds
timeout_seconds = 30

# Maximum number of retries for failed requests
max_retries = 3

# Rate limit in requests per second
rate_limit_rps = 10

[trading]
# Default trading pair
default_pair = "SOL/USDC"

# Default order size in quote currency
default_order_size = 0.1

# Maximum number of open positions
max_open_positions = 5

# Maximum position size as a percentage of portfolio (0-100)
max_position_size_pct = 20.0

# Enable/disable trading (set to false for dry-run mode)
trading_enabled = false

# Enable/disable paper trading (simulated trades)
paper_trading = true

[risk]
# Maximum allowed drawdown percentage (0-100)
max_drawdown_pct = 10.0

# Maximum position risk percentage (0-100)
max_position_risk_pct = 2.0

# Daily loss limit percentage (0-100)
daily_loss_limit_pct = 5.0

# Maximum leverage (1.0 = no leverage)
max_leverage = 1.0

# Enable/disable stop losses
stop_loss_enabled = true

# Default stop loss percentage (0-100)
default_stop_loss_pct = 5.0

# Default take profit percentage (0-100)
default_take_profit_pct = 10.0

[performance]
# Enable/disable performance tracking
enabled = true

# Metrics collection interval in seconds
collection_interval_secs = 60

# Maximum number of data points to keep in memory
max_data_points = 10000

# Enable/disable detailed logging
detailed_logging = true
"#;

    // Create parent directories if they don't exist
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, toml_str)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_generate_config_template() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        generate_commented_config_template(&config_path).unwrap();
        assert!(config_path.exists());

        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("AlgoTraderV2 Configuration"));
        assert!(content.contains("rpc_url"));
    }

    #[test]
    fn test_generate_config_template_with_nonexistent_dir() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config").join("config.toml");

        generate_commented_config_template(&config_path).unwrap();
        assert!(config_path.exists());
    }
}
