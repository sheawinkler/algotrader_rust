# Configuration Guide

This directory contains configuration files for the AlgoTraderV2 trading bot.

## Files

- `default.toml`: The default configuration template. Copy this to `config.toml` and customize as needed.
- `config.toml`: The main configuration file (not included in version control).

## Configuration Structure

The configuration is organized into several sections:

### General

- `bot_name`: Name of this bot instance
- `log_level`: Logging level (trace, debug, info, warn, error, off)
- `debug`: Enable debug mode

### Wallet

- `keypair_path`: Path to the wallet keypair file
- `min_sol_balance`: Minimum SOL balance to maintain (in SOL)
- `max_fee_sol`: Maximum SOL to use for transaction fees (in SOL)

### Solana

- `rpc_url`: Solana RPC endpoint
- `ws_url`: WebSocket endpoint (optional)
- `commitment`: Commitment level (processed, confirmed, finalized)
- `timeout_secs`: Request timeout in seconds
- `retries`: Number of retries for failed requests
- `rate_limit`: Rate limit in requests per second

### Trading

- `default_pair`: Default trading pair (e.g., SOL/USDC)
- `default_order_size`: Default order size in quote currency
- `max_open_positions`: Maximum number of open positions
- `max_position_size_pct`: Maximum position size as percentage of portfolio
- `trading_enabled`: Enable/disable trading
- `paper_trading`: Enable paper trading mode (no real trades)
- `slippage_tolerance_pct`: Slippage tolerance percentage
- `min_price_movement_pct`: Minimum price movement to trigger a trade

### Risk

- `max_drawdown_pct`: Maximum allowed drawdown percentage
- `max_position_risk_pct`: Maximum risk per position as percentage of portfolio
- `daily_loss_limit_pct`: Daily loss limit percentage
- `max_leverage`: Maximum leverage (1.0 = no leverage)
- `stop_loss_enabled`: Enable/disable stop losses
- `default_stop_loss_pct`: Default stop loss percentage
- `default_take_profit_pct`: Default take profit percentage
- `trailing_stop_enabled`: Enable/disable trailing stops
- `trailing_stop_distance_pct`: Trailing stop distance percentage

### Performance

- `enabled`: Enable/disable performance tracking
- `collection_interval_secs`: Metrics collection interval in seconds
- `max_data_points`: Maximum number of data points to keep
- `detailed_logging`: Enable detailed logging

## DEX Configuration

Example DEX configurations are provided in the default config file. Uncomment and customize as needed:

```toml
[dex.jup]
enabled = true
api_url = "https://quote-api.jup.ag/v6"
program_id = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4"
max_retries = 3
retry_delay_ms = 1000
```

## Strategies

Example strategy configurations are provided in the default config file. Uncomment and customize as needed:

```toml
[strategies.mean_reversion]
enabled = true
lookback_periods = 20
std_dev_threshold = 2.0
rsi_period = 14
rsi_oversold = 30
rsi_overbought = 70
take_profit_pct = 5.0
stop_loss_pct = 3.0
position_size_pct = 5.0
cool_down_period_secs = 300
```

## Initializing Configuration

To create a new configuration file:

```bash
# Create a default config file
cargo run --bin init_config -- --config config.toml

# Or force overwrite an existing file
cargo run --bin init_config -- --config config.toml --force
```

## Environment Variables

You can override any configuration setting using environment variables. For example:

```bash
# Set RPC URL
export ALGOTRADER_SOLANA_RPC_URL="https://api.mainnet-beta.solana.com"

# Set wallet path
export ALGOTRADER_WALLET_KEYPAIR_PATH="wallet.json"

# Enable debug logging
export ALGOTRADER_DEBUG=true
```

Environment variables follow this pattern:

```
ALGOTRADER_<SECTION>_<KEY>=<VALUE>
```

For nested keys, use double underscores:

```
ALGOTRADER_STRATEGIES__MEAN_REVERSION__ENABLED=true
```

## Best Practices

1. Never commit sensitive information (private keys, API keys) to version control
2. Use environment variables for sensitive data in production
3. Keep a backup of your configuration
4. Test configuration changes in paper trading mode first
5. Document any custom configurations for your deployment
