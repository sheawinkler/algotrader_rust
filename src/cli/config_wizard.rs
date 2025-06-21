//! Interactive configuration wizard for AlgoTraderV2 `deploy` sub-command.
//!
//! This helper is split into its own module so that the bulk of the user-
//! prompting code is isolated from the CLI glue in `deploy.rs`.

use anyhow::Result;
use dialoguer::{Confirm, Input};
use shellexpand::tilde;
use std::path::Path;

use crate::config::Config;

/// Launch an interactive prompt to fill in critical configuration values.
///
/// The updated config is persisted to `path` if the user confirms.
pub fn interactive_fill_config(path: &Path) -> Result<()> {
    // Load existing config or fallback to defaults.
    let mut cfg = if path.exists() {
        Config::from_file(path).unwrap_or_default()
    } else {
        Config::default()
    };

    println!("\n🔧 Interactive configuration wizard (leave blank to keep defaults)\n");

    // ---------- Solana RPC URL ----------
    let rpc_url: String = Input::new()
        .with_prompt("Solana RPC URL")
        .default(cfg.solana.rpc_url.clone())
        .interact_text()?;
    if !rpc_url.trim().is_empty() {
        cfg.solana.rpc_url = rpc_url.trim().to_string();
    }

    // ---------- Helius API key ----------
    let helius_key_default = cfg.solana.helius_api_key.clone().unwrap_or_default();
    let helius_key: String = Input::new()
        .with_prompt("Helius API key (optional)")
        .default(helius_key_default.clone())
        .allow_empty(true)
        .interact_text()?;
    cfg.solana.helius_api_key = if helius_key.trim().is_empty() {
        None
    } else {
        Some(helius_key.trim().to_string())
    };

    // ---------- Default trading pair ----------
    let pair: String = Input::new()
        .with_prompt("Default trading pair (e.g. SOL/USDC)")
        .default(cfg.trading.default_pair.clone())
        .interact_text()?;
    if !pair.trim().is_empty() {
        cfg.trading.default_pair = pair.trim().to_string();
    }

    // ---------- Default order size ----------
    let order_size: f64 = Input::new()
        .with_prompt("Default order size (in SOL)")
        .default(cfg.trading.default_order_size)
        .validate_with(|v: &f64| {
            if *v > 0.0 {
                Ok(())
            } else {
                Err("Must be greater than 0")
            }
        })
        .interact_text()?;
    cfg.trading.default_order_size = order_size;

    // ---------- Daily loss limit ----------
    let daily_loss: f64 = Input::new()
        .with_prompt("Daily loss limit %")
        .default(cfg.risk.daily_loss_limit_pct)
        .validate_with(|v: &f64| {
            if *v >= 0.0 && *v <= 100.0 {
                Ok(())
            } else {
                Err("Must be between 0 and 100")
            }
        })
        .interact_text()?;
    cfg.risk.daily_loss_limit_pct = daily_loss;

    // ---------- Wallet keypair path ----------
    if cfg.wallet.keypair_path.is_none() {
        let default_wallet = tilde("~/.config/solana/id.json").to_string();
        let kp: String = Input::new()
            .with_prompt("Keypair JSON path")
            .default(default_wallet)
            .interact_text()?;
        if !kp.trim().is_empty() {
            cfg.wallet.keypair_path = Some(kp.trim().to_string());
        }
    }

    // ---------- Summary ----------
    println!("\nConfiguration summary:\n{}", toml::to_string_pretty(&cfg)?);

    if Confirm::new()
        .with_prompt("Save configuration?")
        .default(true)
        .interact()? {
        cfg.save(path)?;
        println!("\n💾 Configuration saved to {}", path.display());
    } else {
        println!("\n⚠️  Configuration NOT saved – run deploy again if you wish to rerun the wizard.");
    }

    Ok(())
}
