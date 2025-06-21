# Development Guide

This document provides guidance for developers working on the AlgoTraderV2 Rust project.

## Table of Contents
- [Prerequisites](#prerequisites)
- [Getting Started](#getting-started)
- [Project Structure](#project-structure)
- [Development Workflow](#development-workflow)
- [Testing](#testing)
- [Code Style](#code-style)
- [Documentation](#documentation)
- [Debugging](#debugging)
- [Performance Optimization](#performance-optimization)
- [CI/CD](#cicd)
- [Troubleshooting](#troubleshooting)

## Prerequisites

- Rust (latest stable version)
- Cargo (Rust's package manager)
- Git
- Solana CLI (for Solana development)
- Docker (optional, for containerized development)

## Getting Started

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/algotraderv2_rust.git
   cd algotraderv2_rust
   ```

2. Install development tools:
   ```bash
   rustup component add rustfmt clippy
   cargo install cargo-edit cargo-watch cargo-tarpaulin cargo-audit
   ```

3. Build the project:
   ```bash
   cargo build
   ```

4. Run tests:
   ```bash
   cargo test
   ```

## Project Structure

```
.
├── Cargo.toml           # Project metadata and dependencies
├── Cargo.lock           # Lock file for reproducible builds
├── src/                 # Source code
│   ├── lib.rs           # Library root
│   ├── main.rs          # Binary entry point
│   ├── dex/             # DEX implementations
│   ├── strategy/        # Trading strategies
│   └── utils/           # Utility modules
├── tests/               # Integration tests
├── benches/             # Benchmarks
├── examples/            # Example code
├── config.toml.example  # Example configuration
├── .env.example        # Example environment variables
├── .gitignore          # Git ignore rules
├── rust-toolchain.toml  # Rust toolchain configuration
├── justfile            # Just commands
├── Makefile            # Make commands
└── README.md           # Project documentation
```

## Development Workflow

1. Create a new branch for your feature or bugfix:
   ```bash
   git checkout -b feature/your-feature-name
   # or
   git checkout -b bugfix/description-of-fix
   ```

2. Make your changes following the coding standards

3. Run tests and checks:
   ```bash
   cargo check
   cargo test
   cargo clippy --all-targets --all-features -- -D warnings
   cargo fmt --all -- --check
   ```

4. Commit your changes with a descriptive message:
   ```bash
   git add .
   git commit -m "Add feature/fix: brief description of changes"
   ```

5. Push your changes and open a pull request

## Testing

### Running Tests

- Run all tests:
  ```bash
  cargo test
  ```

- Run a specific test:
  ```bash
  cargo test test_name
  ```

- Run tests with logging:
  ```bash
  RUST_LOG=debug cargo test -- --nocapture
  ```

### Test Coverage

To generate a test coverage report:

```bash
cargo tarpaulin --ignore-tests --out Html
```

## Code Style

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `rustfmt` for consistent formatting:
  ```bash
  cargo fmt
  ```
- Use `clippy` for linting:
  ```bash
  cargo clippy --all-targets --all-features -- -D warnings
  ```

## Documentation

- Document all public APIs with Rustdoc
- Build documentation locally:
  ```bash
  cargo doc --no-deps --open
  ```
- Check documentation for broken links:
  ```bash
  cargo doc --no-deps --document-private-items
  ```

## Debugging

### Logging

Use the `log` crate for logging:

```rust
use log::{info, debug, error};

info!("This is an info message");
debug!("Debug information: {:?}", some_value);
error!("An error occurred: {}", err);
```

Set log level with `RUST_LOG` environment variable:

```bash
RUST_LOG=debug cargo run
```

### Debugging with LLDB

1. Install LLDB:
   ```bash
   rustup component add lldb
   ```

2. Run with LLDB:
   ```bash
   rust-lldb target/debug/algotrader -- args
   ```

## Performance Optimization

### Profiling

1. Install `flamegraph`:
   ```bash
   cargo install flamegraph
   ```

2. Generate a flamegraph:
   ```bash
   cargo flamegraph --bin algotrader -- --your-arguments
   ```

### Benchmarking

1. Add benchmarks to the `benches/` directory
2. Run benchmarks:
   ```bash
   cargo bench
   ```

## CI/CD

The project uses GitHub Actions for CI/CD. The workflow includes:

- Format checking with `rustfmt`
- Linting with `clippy`
- Running tests
- Generating code coverage
- Security audit with `cargo-audit`
- Building for multiple platforms

## Troubleshooting

### Common Issues

- **Build fails with linking errors**: Try `cargo clean && cargo build`
- **Dependency resolution issues**: Delete `Cargo.lock` and run `cargo update`
- **Rustfmt/clippy not found**: Install them with `rustup component add rustfmt clippy`

### Getting Help

If you encounter any issues:

1. Check the [issues](https://github.com/yourusername/algotraderv2_rust/issues) page
2. Search the [Rust user forum](https://users.rust-lang.org/)
3. Open a new issue with details about your problem

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
