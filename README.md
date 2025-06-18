# AlgoTraderV2 Rust

A high-performance algorithmic trading bot for Solana, built in Rust. This bot is designed for automated trading with support for various strategies, risk management, and performance monitoring.

## ✨ Features

- **Advanced Trading**: Support for arbitrage, meme coin trading, and major crypto swings
- **Risk Management**: Built-in risk assessment and position sizing
- **Wallet Analysis**: Track and analyze profitable wallets and tokens
- **High Performance**: Built with Rust for maximum performance and safety
- **Configurable**: Flexible configuration system with environment variable support
- **Paper Trading**: Test strategies risk-free with paper trading mode
- **Performance Metrics**: Track trading performance with detailed metrics

## 🚀 Quick Start

### Prerequisites

- Rust (latest stable version)
- Cargo (Rust's package manager)
- Solana CLI (for Solana wallet management)

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/algotraderv2_rust.git
   cd algotraderv2_rust
   ```

2. Build the project:
   ```bash
   cargo build --release
   ```

3. Generate a default configuration file:
   ```bash
   ./target/release/algotraderv2 init
   ```

4. Edit the generated `config.toml` file with your settings.

## 🛠️ Configuration

### Generate a New Config

Generate a new configuration file with detailed comments:

```bash
./target/release/algotraderv2 init --commented -c config.toml
```

### Configuration Structure

The configuration file is divided into several sections:

```toml
[wallet]
private_key = ""  # Base58-encoded private key (optional)
keypair_path = "wallet.json"  # Path to keypair file
min_sol_balance = 0.1  # Minimum SOL balance to maintain
max_fee_sol = 0.001    # Maximum SOL to use for transaction fees

[solana]
rpc_url = "https://api.mainnet-beta.solana.com"  # RPC endpoint
ws_url = "wss://api.mainnet-beta.solana.com"      # WebSocket endpoint (optional)
commitment = "confirmed"  # Commitment level

[trading]
default_pair = "SOL/USDC"  # Default trading pair
default_order_size = 0.1    # Default order size in quote currency
max_open_positions = 5      # Maximum number of open positions
max_position_size_pct = 20.0 # Maximum position size as % of portfolio
trading_enabled = false     # Enable/disable trading
paper_trading = true        # Enable paper trading mode

[risk]
max_drawdown_pct = 10.0       # Maximum allowed drawdown %
max_position_risk_pct = 2.0    # Maximum risk per position %
daily_loss_limit_pct = 5.0    # Daily loss limit %
max_leverage = 1.0             # Maximum leverage (1.0 = no leverage)
stop_loss_enabled = true       # Enable stop losses
default_stop_loss_pct = 5.0    # Default stop loss %
default_take_profit_pct = 10.0 # Default take profit %

[performance]
enabled = true                # Enable performance tracking
collection_interval_secs = 60  # Metrics collection interval
max_data_points = 10000        # Maximum data points to keep
detailed_logging = true       # Enable detailed logging
```

## 💻 Usage

### Start Trading

Start the trading bot with default configuration:

```bash
./target/release/algotraderv2 start
```

Start with debug logging:

```bash
./target/release/algotraderv2 start --debug
```

### Wallet Management

Generate a new wallet:

```bash
./target/release/algotraderv2 wallet new
```

Show wallet information:

```bash
./target/release/algotraderv2 wallet info
```

### Check Configuration

Validate your configuration file:

```bash
./target/release/algotraderv2 check-config -c config.toml
```

## 🖥️ Running as a systemd Service

You can keep the trading bot running in the background and automatically restart it on failure by running it as a systemd unit on Linux.

1. Build the release binary (or copy a pre-built one):
   ```bash
   cargo build --release
   ```
2. Copy the provided unit file and adjust paths:
   ```bash
   sudo cp deployment/algotrader.service /etc/systemd/system/
   sudo nano /etc/systemd/system/algotrader.service   # edit WorkingDirectory / ExecStart
   ```
3. Reload systemd and enable the service:
   ```bash
   sudo systemctl daemon-reload
   sudo systemctl enable algotrader.service
   sudo systemctl start algotrader.service
   ```
4. Check status & logs:
   ```bash
   sudo systemctl status algotrader.service
   journalctl -u algotrader.service -f
   ```

The service file already applies some hardening flags (`ProtectSystem=strict`, `PrivateTmp=true`, etc.). Feel free to tweak these as needed.

The bot exposes a `/healthz` endpoint on `127.0.0.1:8888` that you can probe from a local monitor:
```bash
curl http://127.0.0.1:8888/healthz  # returns "OK"
```

---

## 🔧 Development

### Build for Development

```bash
cargo build
```

### Run Tests

```bash
cargo test
```

### Run with Logging

```bash
RUST_LOG=debug cargo run -- start --debug
```

## 📊 Performance Monitoring

The bot includes built-in performance monitoring that tracks:

- Win rate and profit factor
- Drawdown and risk metrics
- Equity curve tracking
- Trade history and analysis

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 📧 Contact

Your Name - [@yourtwitter](https://twitter.com/yourtwitter) - your.email@example.com

Project Link: [https://github.com/yourusername/algotraderv2_rust](https://github.com/yourusername/algotraderv2_rust)
   ```bash
   git clone https://github.com/yourusername/algotraderv2_rust.git
   cd algotraderv2_rust
   ```

2. Build the project:
   ```bash
   cargo build --release
   ```

## Configuration

1. Generate a default configuration file:
   ```bash
   cargo run -- config
   ```

2. Edit the generated `config.toml` file to configure your trading parameters, API keys, and strategies.

## Usage

### Start the trading bot in live mode:
```bash
cargo run -- start
```

### Run in backtest mode:
```bash
cargo run -- start --backtest
```

### Test connections to exchanges:
```bash
cargo run -- test
```

## Available Strategies

### Mean Reversion
A strategy that identifies when the price of an asset has deviated significantly from its historical average and trades in the expectation that it will revert to the mean.

### Momentum
A strategy that identifies trends in asset prices and takes positions in the direction of the trend.

## Project Structure

```
├── Cargo.toml          # Project dependencies and metadata
├── config.toml         # Configuration file (generated)
├── src/
│   ├── main.rs        # Main application entry point
│   ├── lib.rs         # Library root
│   ├── dex/           # DEX implementations
│   │   ├── mod.rs
│   │   ├── jupiter.rs
│   │   ├── raydium.rs
│   │   └── photon.rs
│   ├── strategy/      # Trading strategies
│   │   ├── mod.rs
│   │   ├── mean_reversion.rs
│   │   └── momentum.rs
│   └── utils/         # Utility modules
│       ├── mod.rs
│       ├── config.rs
│       ├── error.rs
│       ├── logging.rs
│       └── types.rs
└── tests/             # Integration tests
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Disclaimer

This software is for educational purposes only. Use at your own risk. The authors are not responsible for any financial losses incurred while using this software. Always test thoroughly with small amounts before trading with real funds.
