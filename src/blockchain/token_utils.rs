use anyhow::{anyhow, Result};
use solana_sdk::program_pack::Pack;
use solana_sdk::pubkey::Pubkey;
use spl_associated_token_account::get_associated_token_address;
use spl_token::state::Account as TokenAccount;
use std::str::FromStr;

/// Token utility functions
pub struct TokenUtils;

impl TokenUtils {
    /// Get the associated token account for a wallet and mint
    pub fn get_associated_token_address(wallet: &str, mint: &str) -> Result<Pubkey> {
        let wallet_pubkey =
            Pubkey::from_str(wallet).map_err(|e| anyhow!("Invalid wallet address: {}", e))?;
        let mint_pubkey =
            Pubkey::from_str(mint).map_err(|e| anyhow!("Invalid mint address: {}", e))?;

        Ok(get_associated_token_address(&wallet_pubkey, &mint_pubkey))
    }

    /// Format token amount with decimals
    pub fn format_token_amount(amount: u64, decimals: u8) -> f64 {
        amount as f64 / 10_f64.powi(decimals as i32)
    }

    /// Parse token amount from string with decimals
    pub fn parse_token_amount(amount: &str, decimals: u8) -> Result<u64> {
        let amount: f64 = amount
            .parse()
            .map_err(|_| anyhow!("Invalid token amount"))?;

        let factor = 10_f64.powi(decimals as i32);
        let raw_amount = (amount * factor).round() as u64;

        Ok(raw_amount)
    }

    /// Check if a token account is empty
    pub fn is_token_account_empty(account: &TokenAccount) -> bool {
        account.amount == 0
    }

    /// Get the token balance from a token account
    ///
    /// # Arguments
    /// * `account` - The token account
    /// * `decimals` - The number of decimals for the token
    pub fn get_token_balance(account: &TokenAccount, decimals: u8) -> f64 {
        account.amount as f64 / 10_f64.powi(decimals as i32)
    }

    /// Get the token balance in UI format (string with proper decimal places)
    ///
    /// # Arguments
    /// * `account` - The token account
    /// * `decimals` - The number of decimals for the token
    pub fn get_token_balance_ui(account: &TokenAccount, decimals: u8) -> String {
        format!("{:.1$}", account.amount as f64 / 10_f64.powi(decimals as i32), decimals as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_token_amount() {
        // Test with 9 decimals (like SOL)
        assert_eq!(TokenUtils::format_token_amount(1_000_000_000, 9), 1.0);
        assert_eq!(TokenUtils::format_token_amount(1_500_000_000, 9), 1.5);

        // Test with 6 decimals (like USDC)
        assert_eq!(TokenUtils::format_token_amount(1_000_000, 6), 1.0);
        assert_eq!(TokenUtils::format_token_amount(1_500_000, 6), 1.5);
    }

    #[test]
    fn test_parse_token_amount() {
        // Test with 9 decimals (like SOL)
        assert_eq!(TokenUtils::parse_token_amount("1.0", 9).unwrap(), 1_000_000_000);
        assert_eq!(TokenUtils::parse_token_amount("1.5", 9).unwrap(), 1_500_000_000);

        // Test with 6 decimals (like USDC)
        assert_eq!(TokenUtils::parse_token_amount("1.0", 6).unwrap(), 1_000_000);
        assert_eq!(TokenUtils::parse_token_amount("1.5", 6).unwrap(), 1_500_000);
    }

    #[test]
    fn test_get_token_balance() {
        let mut data = vec![0u8; 165];
        let mut account = TokenAccount::unpack_unchecked(&data).unwrap();
        account.amount = 1_500_000_000; // 1.5 SOL (9 decimals)
                                        // decimals stored separately, pass as param below

        assert_eq!(TokenUtils::get_token_balance(&account, 9), 1.5);
        assert_eq!(TokenUtils::get_token_balance_ui(&account, 9), "1.500000000");
    }
}
