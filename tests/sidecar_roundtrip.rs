//! Integration test: ensure SidecarClient can round-trip to a stub FastAPI-compatible endpoint.
// Run with: `cargo test --features sidecar -- --nocapture`

#[cfg(feature = "sidecar")]
mod tests {
    use algotraderv2_rust::sidecar::SidecarClient;
    use axum::{routing::post, Json, Router};
    use serde_json::json;
    use std::net::SocketAddr;

    // Stub `/predict` handler: echoes back a dummy signal list
    async fn predict_handler(Json(_payload): Json<serde_json::Value>) -> Json<serde_json::Value> {
        Json(json!([
            {
                "strategy_id": "sidecar",
                "pair": { "base": "SOL", "quote": "USDC" },
                "action": "Buy",
                "price": 100.0,
                "size": 0.1,
                "order_type": "Market",
                "limit_price": null,
                "stop_price": null,
                "stop_loss": null,
                "take_profit": null,
                "timestamp": 0,
                "metadata": {}
            }
        ]))
    }

    #[tokio::test]
    async fn sidecar_round_trip() {
        // Spin up stub server on an ephemeral port
        let app = Router::new().route("/predict", post(predict_handler));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr: SocketAddr = listener.local_addr().unwrap();
        let server = axum::Server::from_tcp(listener)
            .unwrap()
            .serve(app.into_make_service());
        // Run server in background
        tokio::spawn(server);

        // Create client pointing to stub
        let client = SidecarClient::new(format!("http://{}", addr));
        let resp = client
            .predict(json!({ "features": [1, 2, 3] }))
            .await
            .expect("sidecar predict");

        assert!(resp.is_array());
        assert_eq!(resp[0]["strategy_id"], "sidecar");
    }
}
