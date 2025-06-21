# Development Guide

Welcome to the AlgoTraderV2 Rust project! This guide will help you set up your development environment and understand the project structure.

## 🛠️ Development Setup

### Prerequisites

- Rust (latest stable version)
- Cargo (Rust's package manager)
- Solana CLI (for wallet management)
- Git (for version control)

### Cloning the Repository

```bash
git clone https://github.com/yourusername/algotraderv2_rust.git
cd algotraderv2_rust
```

### Building the Project

```bash
# Debug build
cargo build

# Release build (recommended for testing performance)
cargo build --release
```

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with detailed output
cargo test -- --nocapture

# Run a specific test
cargo test test_name
```

## 📁 Project Structure

```
algotraderv2_rust/
├── src/
│   ├── lib.rs           # Library root
│   ├── main.rs          # Binary entry point
│   ├── cli/             # Command-line interface
│   ├── config/          # Configuration management
│   ├── dex/             # DEX integrations
│   ├── strategy/        # Trading strategies
│   ├── analysis/        # Market and wallet analysis
│   ├── blockchain/      # Blockchain interactions
│   └── utils/           # Utility functions
├── tests/               # Integration tests
├── benches/             # Benchmark tests
├── examples/            # Example code
├── docs/                # Documentation
└── Cargo.toml           # Project manifest
```

## 🧪 Testing

### Unit Tests

Unit tests are located in the same file as the code they test, within a `mod tests` block.

### Integration Tests

Integration tests are located in the `tests/` directory. Each file in this directory is compiled as a separate crate.

### Running Benchmarks

```bash
cargo bench
```

## 📝 Code Style

We follow the official [Rust Style Guide](https://doc.rust-lang.org/1.0.0/style/).

Key points:
- 4 spaces for indentation
- Maximum line width of 100 characters
- Use `rustfmt` for consistent formatting
- Document all public APIs with `///` doc comments

### Formatting Code

```bash
# Format all code
cargo fmt

# Check formatting without making changes
cargo fmt -- --check
```

### Linting

```bash
# Run clippy for additional lints
cargo clippy -- -D warnings
```

## 🔄 Git Workflow

1. Create a new branch for your feature or bugfix:
   ```bash
   git checkout -b feature/your-feature-name
   # or
   git checkout -b fix/issue-number-description
   ```

2. Make your changes and commit them with a descriptive message:
   ```bash
   git add .
   git commit -m "Add feature/fix: brief description"
   ```

3. Push your changes to your fork:
   ```bash
   git push origin your-branch-name
   ```

4. Open a Pull Request against the `main` branch.

## 🚀 Release Process

1. Update the version in `Cargo.toml` following [Semantic Versioning](https://semver.org/)
2. Update `CHANGELOG.md` with the changes in the new version
3. Create a new release on GitHub with a tag (e.g., `v1.0.0`)
4. Publish the crate to crates.io (maintainers only):
   ```bash
   cargo publish
   ```

## 🐛 Debugging

### Enable Debug Logging

```bash
RUST_LOG=debug cargo run -- start --debug
```

### Using a Debugger

You can use `rust-lldb` or `rust-gdb` for debugging:

```bash
# Build with debug symbols
cargo build

# Start debugging
rust-lldb target/debug/algotraderv2 -- start
```

## 🤝 Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](../LICENSE) file for details.
