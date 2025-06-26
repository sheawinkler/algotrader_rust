# Algotrader v0.3 Road Map

_Last updated: 2025-06-26_

## Recently Completed (v0.2.0)
* Cargo‐wide formatting pass (`cargo fmt`).
* All `clippy -D warnings` issues resolved.
* Full test suite passes (unit, integration, e2e) with network-sensitive tests gated.
* Logging initialisation made idempotent to avoid panics during repeated test runs.
* Release branch `release/v0.2.0` merged into `main` (tag pending).

## Immediate Next Steps (v0.3)
1. **Python ML Sidecar**
   * Implement FastAPI micro-service to host proprietary models.
   * Expose endpoints: `/ping`, `/predict`, `/feature`.
   * Containerise with Docker, add CI build.
2. **Rust ↔︎ Sidecar Wiring**
   * Finish `SidecarClient` request/response handling.
   * Define protobuf/JSON schema for `PredictRequest` & `PredictResponse`.
   * Add circuit-breaker & retry logic.
3. **Meta-Strategy Integration**
   * Update `MetaStrategyManager` to blend local & sidecar signals (weighted by `confidence`).
   * Expand unit tests under `strategy_tests` to cover blended decisions.
4. **Back-testing with Sidecar Signals**
   * Extend backtester to call the mocked sidecar during simulations (feature-flag `sidecar_tests`).
   * Run walk-forward validation; optimise Sharpe, max draw-down, hit-rate.
5. **Persistence Layer Enhancements**
   * Store sidecar features, predictions & realised PnL for offline analysis.
6. **Deployment Prep**
   * Helm/K8s manifests or Docker-Compose for orchestrated trading stack.
   * Secret management for API keys & private keys (Hashicorp Vault or age-encrypted files).

## Stretch Goals
* Reinforcement-learning agent for dynamic parameter tuning.
* Live tick-level back-tester with exchange-simulated order book.
* Real-time dashboard (React + WebSockets) for trade monitoring.

---

> _“Edges decay quickly – iterate faster.”_  
> **Next milestone:** functional sidecar round-trip & integrated back-test.
