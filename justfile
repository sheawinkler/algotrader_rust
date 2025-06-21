# Justfile for AlgoTraderV2 Rust development
# Install Just: cargo install just

# Default target (run with `just`)
default:
    #!/usr/bin/env bash
    echo "Available commands:"
    just -l

# Build the project in release mode
build:
    cargo build --release

# Run the application
run:
    cargo run --release -- {{...}}

# Run tests
@test:
    cargo test {{...}}

# Run tests with coverage
@coverage:
    cargo tarpaulin --ignore-tests --out Html

# Format code
fmt:
    cargo fmt --all -- --check

# Lint code
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Check for outdated dependencies
outdated:
    cargo outdated

# Update dependencies
update:
    cargo update

# Clean build artifacts
clean:
    cargo clean

# Generate documentation
docs:
    cargo doc --no-deps --open

# Run all checks (fmt, clippy, test)
check:
    cargo check --all-targets --all-features
    cargo fmt --all -- --check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo test --all-features

# Install development tools
setup-dev:
    rustup component add rustfmt clippy
    cargo install cargo-edit cargo-watch cargo-tarpaulin

# Watch for changes and run tests
@watch:
    cargo watch -x "test -- --nocapture"

# Generate a flamegraph
@flamegraph:
    cargo flamegraph -- {{...}}

# Run benchmarks
@bench:
    cargo bench

# Check for security vulnerabilities
audit:
    cargo audit

# Run with logging
@log:
    RUST_LOG=debug cargo run --release -- {{...}}

# Generate a release build
release:
    cargo build --release
    @echo "Release build complete. Binary is at target/release/algotrader"

# Cross-compile for different targets
@cross:
    # Install target: rustup target add x86_64-unknown-linux-musl
    # Then: cargo build --release --target x86_64-unknown-linux-musl
    @echo "Cross-compilation not configured. See comments in justfile."

# Clean and rebuild everything
rebuild: clean build

# Help message
@help:
    #!/usr/bin/env bash
    echo "AlgoTraderV2 Rust Development Commands:"
    echo "  just build     - Build in release mode"
    echo "  just run       - Run the application"
    echo "  just test      - Run tests"
    echo "  just fmt       - Format code"
    echo "  just lint      - Lint code"
    echo "  just check     - Run all checks (fmt, clippy, test)"
    echo "  just docs      - Generate and open documentation"
    echo "  just coverage  - Generate test coverage report"
    echo "  just clean     - Clean build artifacts"
    echo "  just rebuild   - Clean and rebuild"
    echo "  just release   - Create a release build"
    echo "  just help      - Show this help message"
