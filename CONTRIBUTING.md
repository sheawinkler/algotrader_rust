# Contributing to AlgoTraderV2 Rust

Thank you for your interest in contributing to AlgoTraderV2! We appreciate your time and effort. This document outlines the process for contributing to the project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Coding Standards](#coding-standards)
- [Testing](#testing)
- [Documentation](#documentation)
- [Pull Requests](#pull-requests)
- [Bug Reports](#bug-reports)
- [Feature Requests](#feature-requests)
- [Security Issues](#security-issues)
- [License](#license)

## Code of Conduct

This project adheres to the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct). By participating, you are expected to uphold this code.

## Getting Started

### Prerequisites

- Rust (latest stable version)
- Cargo (Rust's package manager)
- Solana CLI (for wallet management)
- Git (for version control)

### Setup Instructions

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/yourusername/algotraderv2_rust.git
   cd algotraderv2_rust
   ```
3. **Set up the development environment**:
   ```bash
   # Install Rust if you haven't already
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   
   # Install development tools
   rustup component add rustfmt clippy
   cargo install cargo-edit cargo-watch cargo-tarpaulin cargo-audit
   
   # Install Solana CLI (for wallet management)
   sh -c "$(curl -sSfL https://release.solana.com/v1.17.0/install)"
   ```
4. **Build the project**:
   ```bash
   cargo build
   ```
5. **Run tests**:
   ```bash
   cargo test
   ```
6. **Generate documentation**:
   ```bash
   cargo doc --open
   ```

## Development Workflow

1. **Create a new branch** for your changes:
   ```bash
   git checkout -b feature/your-feature-name
   # or
   git checkout -b fix/issue-number-description
   ```

2. **Make your changes** following the coding standards below

3. **Run tests and checks**:
   ```bash
   # Format code
   cargo fmt --check
   
   # Run linter
   cargo clippy -- -D warnings
   
   # Run tests
   cargo test
   
   # Check for security vulnerabilities
   cargo audit
   ```

4. **Commit your changes** with a descriptive message:
   ```bash
   git commit -m "Add feature/fix: brief description"
   ```

5. **Push your changes** to your fork:
   ```bash
   git push origin your-branch-name
   ```

6. **Open a Pull Request** against the `main` branch

## Coding Standards

### Code Style

- Follow the official [Rust Style Guide](https://doc.rust-lang.org/1.0.0/style/)
- Use `rustfmt` for consistent formatting
- Maximum line length: 100 characters
- Use 4 spaces for indentation (not tabs)
- Document all public APIs with `///` doc comments

### Naming Conventions

- `snake_case` for variables and functions
- `PascalCase` for types and traits
- `SCREAMING_SNAKE_CASE` for constants
- `'a` for lifetime parameters
- `T`, `U`, `V` for generic type parameters

### Error Handling

- Use `anyhow` for application errors
- Use `thiserror` for library errors
- Include context with errors using `.context()`
- Use `bail!` macro for early returns with errors

### Logging

- Use the `log` crate for logging
- Follow these log levels:
  - `error!`: Errors that prevent normal operation
  - `warn!`: Potentially problematic situations
  - `info!`: General operational information
  - `debug!`: Detailed debugging information
  - `trace!`: Very detailed debugging information

## Testing

### Unit Tests

- Place unit tests in the same file as the code they test
- Use `#[cfg(test)]` to conditionally compile test code
- Follow the pattern:
  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;
      
      #[test]
      fn test_function_name() {
          // Test code here
      }
  }
  ```

### Integration Tests

- Place integration tests in the `tests/` directory
- Each file is compiled as a separate crate
- Test public API only

### Property Testing

- Use `proptest` for property-based testing
- Test properties, not just examples
- Generate random inputs to test edge cases

## Documentation

### Code Documentation

- Document all public APIs with `///` doc comments
- Include examples in documentation
- Use markdown in doc comments
- Document error conditions and panics

### Project Documentation

- Keep `README.md` up to date
- Update `CHANGELOG.md` for all user-visible changes
- Add new documentation to the `docs/` directory

## Pull Requests

1. **Fork** the repository
2. **Create a feature branch** from `main`
3. **Make your changes**
4. **Add tests** for new functionality
5. **Update documentation** as needed
6. **Run all tests** and fix any issues
7. **Push** to your fork
8. **Open a Pull Request**

### PR Guidelines

- Keep PRs focused on a single feature/fix
- Include tests for new functionality
- Update documentation as needed
- Reference any related issues
- Follow the PR template

## Bug Reports

When reporting a bug, please include:

1. A clear, descriptive title
2. Steps to reproduce the issue
3. Expected vs. actual behavior
4. Environment details (OS, Rust version, etc.)
5. Any relevant logs or error messages

## Feature Requests

For feature requests, please:

1. Check if the feature already exists
2. Explain why this feature would be valuable
3. Provide examples of how it would be used
4. Consider contributing a pull request

## Security Issues

Please report security issues to security@example.com. Do not create a public issue.

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
   git checkout -b feature/your-feature-name
   # or
   git checkout -b bugfix/description-of-fix
   ```
2. Make your changes following the coding standards
3. Add tests for your changes
4. Run the test suite
5. Commit your changes with a descriptive message:
   ```bash
   git commit -m "Add feature/fix: brief description of changes"
   ```
6. Push to your fork and open a pull request

## Coding Standards

### General

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `rustfmt` to format your code
- Run `cargo clippy` and fix all warnings before committing
- Document all public APIs with Rustdoc

### Code Style

- Use 4 spaces for indentation
- Maximum line length: 100 characters
- Use `snake_case` for variables and functions, `PascalCase` for types and traits
- Prefer `impl Trait` over generic parameters where possible
- Use `thiserror` for error types and `anyhow` for application errors

### Async Code

- Use `#[async_trait]` for trait methods that are async
- Prefer `?` operator over `.expect()` or `.unwrap()`
- Use `tokio::select!` for concurrent operations
- Document potential cancellation points

## Testing

- Write unit tests in the same file as the code being tested
- Put integration tests in the `tests/` directory
- Use `#[test]` for synchronous tests and `#[tokio::test]` for async tests
- Use `#[serial_test::serial]` for tests that can't run in parallel
- Aim for at least 80% test coverage

To run tests with coverage:
```bash
cargo tarpaulin --ignore-tests --out Html
```

## Documentation

- Document all public APIs with Rustdoc
- Include examples in documentation when appropriate
- Update the README.md for user-facing changes
- Add or update CHANGELOG.md for significant changes

## Pull Requests

1. Fork the repository and create your branch from `main`
2. Make sure all tests pass and there are no linting errors
3. Update the documentation as needed
4. Add tests that demonstrate your changes work as intended
5. Ensure the test suite passes
6. Submit the PR with a clear description of the changes

## Bug Reports

When reporting a bug, please include:

1. A clear, descriptive title
2. Steps to reproduce the issue
3. Expected behavior
4. Actual behavior
5. Environment details (Rust version, OS, etc.)
6. Any relevant logs or error messages

## Feature Requests

For feature requests, please:

1. Check if the feature has already been requested
2. Explain why this feature is needed
3. Describe how it should work
4. Include any relevant examples or references

## License

By contributing to this project, you agree that your contributions will be licensed under the MIT License.
