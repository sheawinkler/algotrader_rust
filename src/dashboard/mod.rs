//! Minimal monitoring dashboard (opt-in via `dashboard` feature).
//! Currently serves two JSON endpoints and a basic root page.
//! Run automatically by calling `dashboard::run()` from your application.

#[cfg(feature = "dashboard")]
use axum::{extract::State, response::Html, routing::get, Json, Router};
#[cfg(feature = "dashboard")]
use serde::Serialize;
#[cfg(feature = "dashboard")]
use std::net::SocketAddr;

#[cfg(feature = "dashboard")]
#[derive(Default, Serialize, Clone)]
pub struct DashboardSnapshot {
    pub equity_usd: f64,
    pub equity_sol: f64,
    pub pnl_usd: f64,
    pub open_positions: usize,
}

#[cfg(feature = "dashboard")]
pub type SharedSnapshot = std::sync::Arc<tokio::sync::RwLock<DashboardSnapshot>>;
#[cfg(not(feature = "dashboard"))]
pub type SharedSnapshot = ();

// --- Handlers ---
#[derive(serde::Serialize)]
struct PortfolioSummary {
    equity_usd: f64,
    equity_sol: f64,
}

#[cfg(feature = "dashboard")]
async fn portfolio_handler(State(state): State<SharedSnapshot>) -> Json<PortfolioSummary> {
    let snap = state.read().await;
    Json(PortfolioSummary { equity_usd: snap.equity_usd, equity_sol: snap.equity_sol })
}

#[cfg(feature = "dashboard")]
#[derive(serde::Serialize)]
struct MetricsSummary {
    pnl_usd: f64,
    open_positions: usize,
}

#[cfg(feature = "dashboard")]
async fn metrics_handler(State(state): State<SharedSnapshot>) -> Json<MetricsSummary> {
    let snap = state.read().await;
    Json(MetricsSummary { pnl_usd: snap.pnl_usd, open_positions: snap.open_positions })
}

#[cfg(feature = "dashboard")]
async fn root_handler() -> Html<&'static str> {
    Html(include_str!("static/index.html"))
}

/// Run the dashboard web server on `127.0.0.1:8080`.
///
/// This spawns an Axum HTTP server; call inside a Tokio runtime.
#[cfg(feature = "dashboard")]
pub async fn run(state: SharedSnapshot) {
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/api/portfolio", get(portfolio_handler))
        .route("/api/metrics", get(metrics_handler))
        .with_state(state.clone());

    // Try 8080 first; if in use, bind to a random port.
    let primary_addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    let listener = match std::net::TcpListener::bind(primary_addr) {
        | Ok(l) => l,
        | Err(e) => {
            eprintln!("Port 8080 unavailable: {} – binding to random port", e);
            std::net::TcpListener::bind("127.0.0.1:0").expect("failed to bind random port")
        }
    };
    let local_addr = listener.local_addr().expect("listener has no local_addr");
    println!("Dashboard running at http://{}", local_addr);

    if let Err(e) = axum::Server::from_tcp(listener)
        .expect("failed to create server from listener")
        .serve(app.into_make_service())
        .await
    {
        eprintln!("Dashboard server error: {}", e);
    }
}

// When the feature is disabled, provide a no-op stub so callers don’t need cfg guards.
#[cfg(not(feature = "dashboard"))]
pub async fn run(_state: ()) {
    eprintln!("Dashboard feature is disabled; skipping web server startup.");
}
