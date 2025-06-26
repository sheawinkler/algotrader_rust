//! Logging configuration for the trading system.

use chrono::Local;
use env_logger::{Builder, Env, Target};
use log::{info, LevelFilter};
use std::io::Write;

/// Initialize the logging system
pub fn init_logging(level: &str) {
    let env = Env::default()
        .filter_or("ALGOTRADER_LOG", level)
        .write_style_or("ALGOTRADER_LOG_STYLE", "auto");

    Builder::from_env(env)
        .format(|buf, record| {
            let level = record.level();
            let level_color = match level {
                | log::Level::Error => "\x1b[31m", // Red
                | log::Level::Warn => "\x1b[33m",  // Yellow
                | log::Level::Info => "\x1b[32m",  // Green
                | log::Level::Debug => "\x1b[36m", // Cyan
                | log::Level::Trace => "\x1b[35m", // Magenta
            };
            let reset = "\x1b[0m";

            writeln!(
                buf,
                "{} {}{:5}{} [{}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                level_color,
                level,
                reset,
                record.target(),
                record.args()
            )
        })
        .target(Target::Stdout)
        .try_init().ok();

    info!("Logging initialized at level: {}", level);
}

/// Initialize test logging (for use in tests)
#[cfg(test)]
pub fn init_test_logging() {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(LevelFilter::Debug)
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::{debug, error, info, warn};

    #[test]
    fn test_logging() {
        // This is a visual test - run with `cargo test -- --nocapture` to see the output
        init_logging("debug");

        error!("This is an error message");
        warn!("This is a warning message");
        info!("This is an info message");
        debug!("This is a debug message");
    }

    #[test]
    fn test_test_logging() {
        init_test_logging();
        debug!("This debug message should only appear in test output with --nocapture");
    }
}
