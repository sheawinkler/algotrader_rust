use anyhow::{anyhow, Result};
use serde::Deserialize;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_request::TokenAccountsFilter;
use solana_client::rpc_response::RpcKeyedAccount;
use solana_sdk::program_pack::Pack;
use solana_sdk::pubkey::Pubkey;
use spl_token::state::Account as TokenAccount;

/// Scan a wallet and return the list of token symbols (excluding SOL) with non-zero balance.
/// The function queries the RPC for SPL token accounts held by `owner` and then resolves each
/// token mint to a human-readable symbol via the Birdeye public API. If API lookup fails the
/// raw mint address is returned instead.
pub fn get_wallet_token_symbols(owner: &str, rpc_url: &str) -> Result<Vec<String>> {
    let owner_pk: Pubkey = owner
        .parse()
        .map_err(|e| anyhow!("invalid wallet pubkey: {e}"))?;
    let client = RpcClient::new(rpc_url.to_string());

    let accounts = client
        .get_token_accounts_by_owner(&owner_pk, TokenAccountsFilter::ProgramId(spl_token::id()))?;

    let mut symbols = Vec::new();
    for RpcKeyedAccount { account, .. } in accounts {
        // account.data is UiAccountData; extract raw bytes from Base64
        if let solana_account_decoder::UiAccountData::Binary(data_b64, _) = &account.data {
            if let Ok(raw) = base64::decode(data_b64) {
                if let Ok(ta) = TokenAccount::unpack(&raw) {
                    if ta.amount == 0 {
                        continue;
                    }
                    let mint_str = ta.mint.to_string();
                    if let Ok(sym) = symbol_for_mint(&mint_str) {
                        symbols.push(sym);
                    } else {
                        symbols.push(mint_str);
                    }
                }
            }
        }
    }
    symbols.sort();
    symbols.dedup();
    Ok(symbols)
}

#[derive(Deserialize)]
struct BirdeyeTokenResp {
    data: Option<BirdeyeTokenData>,
}

#[derive(Deserialize)]
struct BirdeyeTokenData {
    symbol: String,
}

/// Resolve a mint address to symbol via Birdeye public API.
fn symbol_for_mint(mint: &str) -> Result<String> {
    let url = format!("https://public-api.birdeye.so/public/token/{}", mint);
    let resp: BirdeyeTokenResp = reqwest::blocking::get(&url)?.json()?;
    if let Some(data) = resp.data {
        Ok(data.symbol)
    } else {
        Err(anyhow!("symbol not found for mint"))
    }
}
