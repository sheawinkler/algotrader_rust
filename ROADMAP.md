# AlgoTraderV2 Rust - Development Roadmap

This document outlines the development roadmap for AlgoTraderV2 Rust, including planned features, improvements, and milestones.

## Version 0.1.0 - Initial Release (MVP)
- [x] Project setup and basic structure
- [x] Core trading engine implementation
- [x] Basic DEX integrations (Jupiter, Raydium, Photon)
- [x] Implement basic trading strategies
- [x] Configuration management
- [x] Logging and error handling
- [x] Basic command-line interface
- [x] Unit tests and CI/CD pipeline
- [x] Advanced execution engine (slippage limits, fee caps, chunked order splitting with jittered delays, wallet rotation)

## Version 0.2.0 - Enhanced Features (Next)
- [ ] Deployment & self-configuration feature
- [ ] Advanced order types (limit, stop-loss, take-profit)
- [ ] Portfolio management and tracking
- [ ] Performance metrics and analytics
- [ ] Backtesting framework
- [ ] More DEX integrations (Orca, Serum, etc.)
- [ ] WebSocket support for real-time data
- [ ] Improved error handling and recovery

## Version 0.3.0 - Advanced Trading
- [ ] Machine learning integration for strategy optimization
- [ ] Paper trading mode
- [ ] Risk management system
- [ ] More sophisticated indicators and strategies
- [ ] Multi-asset portfolio optimization
- [ ] Advanced charting capabilities

## Version 0.4.0 - Scaling & Performance
- [ ] Performance optimization
- [ ] Distributed execution
- [ ] High-frequency trading capabilities
- [ ] Advanced caching mechanisms
- [ ] Load testing and optimization

## Version 0.5.0 - User Experience
- [ ] Web-based dashboard
- [ ] Mobile app
- [ ] Advanced configuration UI
- [ ] Strategy builder (visual programming)
- [ ] Comprehensive documentation

## Version 1.0.0 - Production Ready
- [ ] Comprehensive test coverage
- [ ] Security audit
- [ ] Performance benchmarking
- [ ] Production deployment guides
- [ ] Enterprise support options

## Future Possibilities
- [ ] Decentralized exchange (DEX) aggregation
- [ ] Cross-chain trading
- [ ] Social trading features
- [ ] AI-powered trading signals
- [ ] Institutional-grade features

## How to Contribute

We welcome contributions to help us achieve these goals! Here's how you can help:

1. **Code Contributions**: Pick an issue from the [issue tracker](https://github.com/yourusername/algotraderv2_rust/issues) or propose a new feature.
2. **Testing**: Help us test the software and report any bugs you find.
3. **Documentation**: Improve our documentation, including code comments, README, and guides.
4. **Community**: Help answer questions from other users and share your experience.

## Versioning

We use [Semantic Versioning](https://semver.org/) for version numbers. Given a version number MAJOR.MINOR.PATCH:

- **MAJOR** version for incompatible API changes
- **MINOR** version for added functionality in a backward-compatible manner
- **PATCH** version for backward-compatible bug fixes

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for a detailed list of changes in each version.

## Detailed Implementation Plan

Below is the full implementation plan (mirrored from `PLAN.md`) to keep roadmap and execution details in one place.

---

# AlgoTraderV2 Rust Implementation Plan

## Notes
- The user wants to implement Version 0.2.0 of the `algotraderv2_rust` project.
- The primary goal is to get a trading-capable version running as quickly as possible by re-evaluating the existing roadmap.
- User has approved the re-prioritized plan focusing on a fast path to live trading.
- All code changes should be staged in git, and pushed after completing major tasks.
- The user has a specific persona they want the assistant to adopt: "ANARCHIST REBEL GENIUS MAD SCIENTIST".
- `OrderType::Market` is being used as the default for now during refactoring.
- There are two main `Signal` structs: `trading::Signal` (produced by strategies) and `utils::types::Signal` (used by the engine). Refactoring needs to happen in stages: update one, fix build, update the other, connect them.
- Codebase was reset to `HEAD` (v0.1.0) due to extensive errors from automated patching. Will re-implement advanced order types step-by-step.
- The `strategy_tests` feature flag has been added to `Cargo.toml` to silence `cfg` warnings.
- A dummy module `mod _removed_brace {}` was added to `src/strategies/config_impls.rs` to resolve a syntax error after removing the `tf_or_default` macro.
- The Jupiter price API endpoint `price.jup.ag` was failing. Switched to user-suggested `lite-api.jup.ag`. This is the v2 API, which requires token mint addresses (not symbols), returns prices as strings, and does not use the `vsToken` parameter.
- Initial attempts to pre-fetch prices in `lib.rs` caused compilation errors (`await` in non-async function) and were reverted.
- User has specified that hardcoded default/fallback values for prices are unacceptable; these have been removed.
- "Address already in use" errors occur if the previous process is not terminated before a new one is started. Processes can be killed with `pkill -f algotraderv2_rust`.

## Task List
- [x] Read and analyze the `ROADMAP.md` file to understand the full scope of Version 0.2.0.
- [x] Propose a re-ordered plan to the user that prioritizes the fastest path to a functional trading bot.
- [x] Get user approval for the new, prioritized plan.
- [x] **Phase 1 – “Can Trade Right Now”**
  - [x] Implement self-configuring runtime (detect/prompt for missing config, persist answers).
  - [x] Implement advanced order types (limit, stop-loss, take-profit) for Jupiter & Raydium.
    - [x] **Sub-Phase: `Order` struct and `execute_trade` signature updates**
        - [x] Add `pub order_type: OrderType` field to `Order` struct in `src/utils/types.rs` and ensure `pub timestamp: i64`.
        - [x] Update `Order` instantiations across the codebase to include `order_type` (default: `OrderType::Market`) and handle `timestamp`.
            - [x] Update `Order` instantiations in `src/lib.rs`.
            - [x] Update `Order` instantiations in `src/performance/monitor.rs` (test).
        - [x] Run `cargo check` to confirm `Order` related changes are correct and build is clean.
        - [x] Update `execute_trade` signatures in `DexClient` trait, implementations, and call sites (`src/lib.rs`) to pass `order_type: OrderType`, `limit_price: Option<f64>`, `stop_price: Option<f64>`, `take_profit_price: Option<f64>`.
        - [x] Run `cargo check` to ensure build remains clean after `execute_trade` signature changes.
    - [x] **Sub-Phase: `Signal` struct updates and propagation**
        - [x] Add `order_type: OrderType` and `limit_price: Option<f64>` fields to `utils::types::Signal` struct in `src/utils/types.rs`.
        - [x] Add `order_type: OrderType` and `limit_price: Option<f64>` fields to `trading::Signal` struct in `src/trading/mod.rs`.
        - [x] Update `trading::Signal` instantiations in `src/lib.rs` to include new fields (defaults: `OrderType::Market`, `None`).
        - [x] Run `cargo check` to confirm all `Signal` and `Order` struct changes and instantiations are correct.
        - [x] Update `convert_strategy_signal` in `lib.rs` to map `trading::Signal::{order_type, limit_price}` to the corresponding fields in `utils::types::Signal`.
        - [x] Update `handle_signals` in `src/lib.rs` to pass new fields from `utils::types::Signal` to `dex_client.execute_trade`.
        - [x] Run `cargo check` to confirm core logic updates are correct.
    - [x] **Sub-Phase: DEX client implementation for advanced orders**
        - [x] Update `JupiterClient::execute_trade` to handle `Market` and `Limit` order types.
        - [x] Update `RaydiumClient::execute_trade` to handle `Market` and `Limit` order types.
        - [x] Update `PhotonClient::execute_trade` to handle `Market` and `Limit` order types.
        - [x] Add placeholder error handling for `Stop`, `StopLimit` order types in all DEX clients.
  - [x] Integrate WebSocket market-data feed (e.g., Jupiter).
    - [x] Create `market_data::ws` module.
    - [x] Implement WebSocket connection to Jupiter (`wss://quote-api.jup.ag/v6/ws`).
    - [x] Implement `price:update` subscription logic for relevant trading pairs.
    - [x] Create a shared price cache (e.g., `Arc<RwLock<HashMap<TradingPair, f64>>>`).
    - [x] Extend `TradingEngine` to spawn a background task for the WebSocket feed.
    - [x] Add `get_live_price(pair)` helper in `TradingEngine` to read from the cache.
    - [x] Update DEX clients' `execute_trade` for `Stop`/`StopLimit` to use live prices.
    - [x] **Enhance `Signal` structs for `stop_price` (Part 1 - Definition & Instantiation):**
      - [x] Add `stop_price: Option<f64>` field to `trading::Signal` struct in `src/trading/mod.rs`.
      - [x] Add `stop_price: Option<f64>` field to `utils::types::Signal` struct in `src/utils/types.rs` (struct definition updated).
        - [x] Clean up `test_signal` in `src/utils/types.rs` to remove duplicated fields and correctly include all necessary fields (e.g. `order_type`, `limit_price`, `stop_price`).
      - [x] Update `trading::Signal` instantiations in `src/lib.rs` to include `stop_price` (default: `None`).
      - [x] Run `cargo check` to confirm changes.
    - [x] **Enhance `Signal` structs for `stop_price` (Part 2 - Propagation):**
      - [x] Update `convert_strategy_signal` in `lib.rs` to map `trading::Signal::stop_price` to `utils::types::Signal::stop_price`.
      - [x] Update `handle_signals` in `src/lib.rs` to pass `sig.stop_price` to `dex_client.execute_trade`.
      - [x] Run `cargo check` to confirm changes.
    - [x] Implement a simple scheduler in `TradingEngine` to retry pending stop/trigger orders.
      - [x] Add `PendingOrder` struct to `utils/types.rs`.
      - [x] Add `pending_orders`, `scheduler_handle`, `retry_tx`, and `retry_rx` to `TradingEngine`.
      - [x] In `TradingEngine::with_config`, initialize scheduler components and spawn the scheduler task.
      - [x] In `handle_signals`, queue `PendingOrder`s for non-triggered stop orders.
      - [x] Update `TradingEngine` event loop (`start_with_market_router`):
          - [x] Use `retry_rx` in `tokio::select!` to receive triggered orders.
          - [x] Create `process_pending_order` to handle execution of triggered orders.
      - [x] Run `cargo check` to confirm changes.
- [x] **Phase 1 – “Can Trade Right Now” (COMPLETED)**
- [x] **Phase 2 – “Measure & Improve”**
  - [x] Implement portfolio tracking module.
    - [x] Create `src/portfolio/mod.rs`.
    - [x] Define `Position` struct (e.g., `symbol`, `size`, `average_entry_price`, `realized_pnl`).
    - [x] Define `Portfolio` struct (e.g., `cash_usd`, `positions: HashMap<String, Position>`, `total_realized_pnl`, methods for `total_usd_value` and `total_sol_value`).
    - [x] Add `portfolio: portfolio::Portfolio` field to `TradingEngine`.
    - [x] **Finish `TradingEngine` integration and cleanup:**
        - [x] Correctly implement `apply_trade_effects` in `src/lib.rs`:
            - [x] Ensure the core logic uses `self.portfolio.update_on_buy` and `self.portfolio.update_on_sell`.
            - [x] Sync legacy `TradingEngine` fields:
                - [x] `self.current_balance = self.portfolio.cash_usd;`
                - [x] Reconstruct `self.open_positions` from `self.portfolio.positions`.
            - [x] Update `self.daily_loss` using `self.portfolio.cash_usd` (or `self.current_balance` after sync).
        - [x] Remove all other legacy position/balance calculation code from `src/lib.rs` and ensure no stray code/comments cause compilation errors.
        - [x] Add `starting_balance_usd: f64` field to `TradingConfig` in `src/config/mod.rs` and provide a default value.
        - [x] Update `TradingEngine::with_config` to initialize `portfolio` with the `starting_balance_usd` from the config.
        - [x] Ensure `equity_usd()` and `equity_sol()` methods are correctly implemented in `TradingEngine`.
        - [x] Run `cargo check` and `cargo test` to ensure a clean build and all tests (including `tests/portfolio.rs`) pass.
        - [x] Address compilation errors from `cargo test` output in strategy files (`order_flow.rs`, `performance_aware.rs`, `mean_reversion.rs`, `trend_following.rs`) and `MarketData`/`Order` instantiations.
            - [x] Implement `Default` for `MarketData` in `src/utils/types.rs`.
- [x] **Phase 3 – “Make it Robust & Smart”**
  - [x] Add `strategy_tests` feature flag to `Cargo.toml`.
  - [x] Address `unreachable_pattern` warnings in `src/dex/jupiter.rs`, `src/dex/raydium.rs`, and `src/dex/photon.rs`.
  - [x] Address `unused_macro` warning for `tf_or_default`.
  - [x] Implement strategy tests using the `strategy_tests` feature flag.
    - [x] Create `tests/strategies_compile.rs` for factory instantiation tests.
    - [x] Run `cargo test` to confirm all strategy compile-check tests pass.
  - [x] Implement basic UI/dashboard for monitoring.
    - [x] Add `dashboard` feature flag in `Cargo.toml`.
    - [x] Create `src/dashboard/mod.rs` with a stubbed Axum server.
    - [x] Add `pub mod dashboard;` to `src/lib.rs`.
    - [x] In `TradingEngine`, add an optional `dashboard_handle: JoinHandle<()>` and `dashboard_state: Option<SharedSnapshot>` fields.
    - [x] In `TradingEngine` startup, initialize `DashboardSnapshot` and spawn the dashboard server in a separate task, passing the shared state.
    - [x] Update Axum handlers in `src/dashboard/mod.rs` to accept and use `State<SharedSnapshot>`.
    - [x] Wire up `/api/portfolio` and `/api/metrics` to read from the shared state (initial read, not yet updating).
    - [x] Create `dashboard/static/index.html` with basic JS to fetch and display data.
    - [x] Update `root_handler` in `src/dashboard/mod.rs` to serve the static HTML file.
    - [x] Periodically update the `DashboardSnapshot` in `TradingEngine` with live portfolio and metrics data.
      - [x] Add a method in `TradingEngine` like `update_dashboard_snapshot(&self) async`.
      - [x] This method should populate `DashboardSnapshot` fields:
        - [x] `equity_usd` (from `self.portfolio.total_usd_value(&self.price_cache).await` or similar)
        - [x] `equity_sol` (from `self.portfolio.total_sol_value(&self.price_cache).await` or similar)
        - [x] `pnl_usd` (from `self.portfolio.total_realized_pnl_usd()`)
        - [x] `open_positions` (from `self.portfolio.positions.len()`)
      - [x] Call `update_dashboard_snapshot` regularly from `TradingEngine` (e.g., in `start_with_market_router` loop or a new dedicated tokio task).
    - [x] Verify dashboard displays live, updating data.
      - [x] Fix runtime configuration errors in `config.toml` to allow the application to start.
      - [x] Fix compilation errors and successfully start the application.
      - [x] Update Jupiter API endpoint to `lite-api.jup.ag/v2` and adapt to its requirements (mint addresses, string prices).
      - [x] Implement automatic wallet-to-portfolio sync to reflect wallet balance in the dashboard.
      - [x] Access the dashboard in a browser and confirm live data is displayed, reflecting the synced wallet balance.
- [ ] **Phase 4 – “Enhancements & New Features”**
  - [ ] Propose and get user approval for the next set of features.
  - [ ] Implement a robust backtesting engine.
  - [ ] Add support for more complex trading strategies.
  - [ ] Persist portfolio and trade history to a database.
  - [ ] Improve error handling and system resilience.

## Current Goal
Propose next development phase.

---

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
