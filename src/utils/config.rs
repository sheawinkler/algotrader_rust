//! Configuration management for the trading system.

use crate::strategies::StrategyConfig as RichStrategyConfig;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::Path};

/// Main configuration structure for the trading system
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct Config {
    /// General application settings
    pub app: AppConfig,
    /// Trading settings
    pub trading: TradingConfig,
    /// DEX configurations
    pub dex: HashMap<String, DexConfig>,
    /// Legacy strategy settings map (deprecated in favour of rich configs)
    pub strategy_settings: HashMap<String, StrategySettings>,
    /// Rich strategy configurations list
    #[serde(default)]
    pub strategies: Vec<RichStrategyConfig>,
}

/// Application-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Log level (debug, info, warn, error)
    pub log_level: String,
    /// Path to the data directory
    pub data_dir: String,
    /// Whether to run in backtest mode
    pub backtest: bool,
}

/// Trading-related configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingConfig {
    /// Default trading pair (e.g., "SOL/USDC")
    pub default_pair: String,
    /// Default position size (as a percentage of portfolio)
    pub default_position_size: f64,
    /// Default slippage tolerance (in basis points, e.g., 50 = 0.5%)
    pub default_slippage_bps: u16,
    /// Maximum number of open positions
    pub max_open_positions: usize,
}

/// DEX-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexConfig {
    /// Whether this DEX is enabled
    pub enabled: bool,
    /// DEX-specific configuration parameters
    pub params: HashMap<String, String>,
}

/// Simple key-value strategy settings (legacy)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategySettings {
    /// Whether this strategy is enabled
    pub enabled: bool,
    /// Strategy-specific parameters
    pub params: HashMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        let mut dex_config = HashMap::new();
        dex_config
            .insert("jupiter".to_string(), DexConfig { enabled: true, params: HashMap::new() });

        let mut strategy_settings = HashMap::new();
        strategy_settings.insert(
            "mean_reversion".to_string(),
            StrategySettings {
                enabled: true,
                params: [
                    ("lookback".to_string(), "20".to_string()),
                    ("entry_z_score".to_string(), "2.0".to_string()),
                    ("exit_z_score".to_string(), "0.5".to_string()),
                    ("position_size".to_string(), "0.1".to_string()),
                ]
                .iter()
                .cloned()
                .collect(),
            },
        );

        strategy_settings.insert(
            "momentum".to_string(),
            StrategySettings {
                enabled: true,
                params: [
                    ("ema_short".to_string(), "9".to_string()),
                    ("ema_long".to_string(), "21".to_string()),
                    ("rsi_period".to_string(), "14".to_string()),
                    ("rsi_overbought".to_string(), "70.0".to_string()),
                    ("rsi_oversold".to_string(), "30.0".to_string()),
                    ("position_size".to_string(), "0.1".to_string()),
                ]
                .iter()
                .cloned()
                .collect(),
            },
        );

        Self {
            app: AppConfig {
                log_level: "info".to_string(),
                data_dir: "./data".to_string(),
                backtest: false,
            },
            trading: TradingConfig {
                default_pair: "SOL/USDC".to_string(),
                default_position_size: 0.1,
                default_slippage_bps: 50,
                max_open_positions: 5,
            },
            dex: dex_config,
            strategy_settings,
            strategies: Vec::new(),
        }
    }
}

impl Config {
    /// Load configuration from a file
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to a file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Get a DEX configuration by name
    pub fn get_dex_config(&self, name: &str) -> Option<&DexConfig> {
        self.dex.get(name)
    }

    /// Get a strategy configuration by name
    pub fn get_strategy_config(&self, name: &str) -> Option<&RichStrategyConfig> {
        self.strategies.iter().find(|s| s.name == name)
    }

    /// Get the default configuration as a TOML string
    pub fn default_toml() -> String {
        toml::to_string_pretty(&Self::default()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.app.log_level, "info");
        assert_eq!(config.trading.default_pair, "SOL/USDC");
        assert!(config.dex.contains_key("jupiter"));
        assert!(config.strategy_settings.contains_key("mean_reversion"));
        assert!(config.strategy_settings.contains_key("momentum"));
    }

    #[test]
    fn test_save_and_load_config() {
        let config = Config::default();
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Save config
        config.save_to_file(path).unwrap();

        // Load config
        let loaded_config = Config::from_file(path).unwrap();

        assert_eq!(config.app.log_level, loaded_config.app.log_level);
        assert_eq!(config.trading.default_pair, loaded_config.trading.default_pair);
    }

    #[test]
    fn test_default_toml() {
        let toml = Config::default_toml();
        assert!(toml.contains("[app]"));
        assert!(toml.contains("[trading]"));
        assert!(toml.contains("jupiter"));
        assert!(toml.contains("mean_reversion"));
        assert!(toml.contains("momentum"));
    }
}
