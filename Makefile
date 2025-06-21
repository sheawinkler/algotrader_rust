.PHONY: all build run test fmt lint clean doc check update help

# Variables
CARGO = cargo
RUSTFLAGS = -D warnings
PROJECT = algotrader
TARGET = target/release/$(PROJECT)

# Default target
all: build

# Build the project in release mode
build:
	@echo "Building $(PROJECT) in release mode..."
	@$(CARGO) build --release

# Run the application
run: build
	@echo "Running $(PROJECT)..."
	@$(CARGO) run --release -- $(ARGS)

# Run tests
test:
	@echo "Running tests..."
	@$(CARGO) test -- --nocapture

# Format code
fmt:
	@echo "Formatting code..."
	@$(CARGO) fmt --all

# Lint code
lint:
	@echo "Linting code..."
	@$(CARGO) clippy --all-targets --all-features -- -D warnings

# Clean build artifacts
clean:
	@echo "Cleaning..."
	@$(CARGO) clean

# Generate documentation
doc:
	@echo "Generating documentation..."
	@$(CARGO) doc --no-deps --open

# Run all checks
check: fmt lint test

# Update dependencies
update:
	@echo "Updating dependencies..."
	@$(CARGO) update

# Install development tools
setup-dev:
	@echo "Installing development tools..."
	@rustup component add rustfmt clippy
	@$(CARGO) install cargo-edit cargo-watch cargo-tarpaulin

# Generate a release build
release: build
	@echo "Release build complete. Binary is at $(TARGET)"

# Help message
help:
	@echo "AlgoTraderV2 Rust - Makefile Commands"
	@echo ""
	@echo "  Available targets:"
	@echo "    build     - Build in release mode"
	@echo "    run       - Run the application (ARGS= for arguments)"
	@echo "    test      - Run tests"
	@echo "    fmt       - Format code"
	@echo "    lint      - Lint code"
	@echo "    clean     - Clean build artifacts"
	@echo "    doc       - Generate and open documentation"
	@echo "    check     - Run all checks (fmt, lint, test)"
	@echo "    update    - Update dependencies"
	@echo "    setup-dev - Install development tools"
	@echo "    release   - Create a release build"
	@echo "    help      - Show this help message"

# Print help by default
.DEFAULT_GOAL := help
