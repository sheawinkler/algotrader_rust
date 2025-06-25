//! Utility functions and types for the trading system.

pub mod atr_cache;
pub mod binance_stream;
pub mod coinbase_stream;
#[cfg(feature = "legacy_config")]
mod config;
pub mod error;
mod fs;
pub mod helius_stream;
pub mod indicators;
pub mod kraken_stream;
mod logging;
pub mod market_stream;
pub mod serum_stream;
pub mod triton_stream;
pub mod types;
pub mod websocket;

#[cfg(feature = "legacy_config")]
pub use config::Config;
pub use error::Error;
pub use fs::*;
pub use logging::init_logging;
pub use types::*;

/// Re-export of commonly used types
pub mod prelude {
    #[cfg(feature = "legacy_config")]
    pub use super::config::Config;
    pub use super::{
        error::{Error, Result},
        fs::*,
        indicators::*,
        logging::init_logging,
        types::*,
    };
}

/// Common result type for utility functions
pub type Result<T> = std::result::Result<T, Error>;
