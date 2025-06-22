//! Error handling for the trading system.

use thiserror::Error;

/// Main error type for the trading system
#[derive(Debug, Error)]
pub enum Error {
    /// Wallet-related errors
    #[error("Wallet error: {0}")]
    WalletError(String),
    /// Configuration errors
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    /// Data-related errors (e.g. missing or malformed market data)
    #[error("Data error: {0}")]
    DataError(String),

    /// Connection / network errors
    #[error("Connection error: {0}")]
    ConnectionError(String),

    /// DEX-related errors
    #[error("DEX error: {0}")]
    DexError(String),
    
    /// Strategy-related errors
    #[error("Strategy error: {0}")]
    StrategyError(String),
    
    /// I/O errors
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    
    /// JSON serialization/deserialization errors
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    
    /// TOML serialization/deserialization errors
    #[error("TOML error: {0}")]
    TomlError(#[from] toml::de::Error),
    
    /// TOML serialization errors
    #[error("TOML serialization error: {0}")]
    TomlSerializeError(#[from] toml::ser::Error),
    
    /// Request errors
    #[error("Request error: {0}")]
    ReqwestError(#[from] reqwest::Error),
    
    /// Invalid argument errors
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    
    /// Other errors
    #[error("Error: {0}")]
    Other(String),
}

/// Result type for the trading system
pub type Result<T> = std::result::Result<T, Error>;


// Add From conversion for bs58::decode::Error
impl From<bs58::decode::Error> for Error {
    fn from(err: bs58::decode::Error) -> Self {
        Error::WalletError(format!("bs58 decode error: {}", err))
    }
}

impl From<&str> for Error {
    fn from(err: &str) -> Self {
        Error::Other(err.to_string())
    }
}

impl From<String> for Error {
    fn from(err: String) -> Self {
        Error::Other(err)
    }
}

// Allow automatic conversion from anyhow::Error to our Error type
impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Error::Other(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_display() {
        let config_error = Error::ConfigError("missing field".to_string());
        assert_eq!(
            config_error.to_string(),
            "Configuration error: missing field"
        );
        
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let wrapped_io_error = Error::from(io_error);
        assert!(wrapped_io_error.to_string().contains("I/O error"));
        
        let string_error = Error::from("custom error");
        assert_eq!(string_error.to_string(), "Error: custom error");
        
        let str_error = Error::from("custom error");
        assert_eq!(str_error.to_string(), "Error: custom error");
    }
    
    #[test]
    fn test_result_type() {
        fn might_fail() -> Result<()> {
            if true {
                Ok(())
            } else {
                Err(Error::Other("error".to_string()))
            }
        }
        
        assert!(might_fail().is_ok());
    }
}
