//! Utility functions and types for the trading system.

#[cfg(feature = "legacy_config")]
mod config;
pub mod error;
mod fs;
mod logging;
pub mod types;
pub mod websocket;
pub mod market_stream;
pub mod binance_stream;
pub mod coinbase_stream;
pub mod kraken_stream;
pub mod serum_stream;
pub mod helius_stream;
pub mod triton_stream;
pub mod indicators;
pub mod atr_cache;

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
        logging::init_logging,
        types::*,
        indicators::*,

    };
}

/// Common result type for utility functions
pub type Result<T> = std::result::Result<T, Error>;
