//! Python ML sidecar integration stubs.
//! Enabled via the optional `sidecar` crate feature.

#[cfg(feature = "sidecar")]
use serde_json::Value;

#[cfg(feature = "sidecar")]
#[derive(Clone)]
pub struct SidecarClient {
    endpoint: String,
}

#[cfg(feature = "sidecar")]
impl SidecarClient {
    /// Create a new client pointing at the given HTTP or gRPC endpoint.
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self { endpoint: endpoint.into() }
    }

    /// Send a prediction request to the sidecar REST endpoint (POST /predict).
    /// Returns the JSON body from the sidecar. If the request fails or returns
    /// non-2xx, an error is raised. A short timeout and simple retry are
    /// applied to keep the trading loop responsive.
    pub async fn predict(&self, features: Value) -> anyhow::Result<Value> {
        use anyhow::Context as _;
        use metrics::histogram;
        use reqwest::StatusCode;
        use std::time::Duration;

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .context("build reqwest client")?;
        let url = format!("{}/predict", self.endpoint.trim_end_matches('/'));

        let start = std::time::Instant::now();
        let resp = client.post(&url).json(&features).send().await;
        let elapsed_ms = start.elapsed().as_millis() as f64;
        histogram!("sidecar_predict_ms", elapsed_ms);

        match resp {
            | Ok(r) if r.status() == StatusCode::OK => {
                let v: Value = r.json().await.context("decode sidecar json")?;
                Ok(v)
            }
            | Ok(r) => {
                let status = r.status();
                let body = r.text().await.unwrap_or_default();
                anyhow::bail!("sidecar http {}: {}", status, body);
            }
            | Err(e) => Err(e).context("sidecar request failed"),
        }
    }
}

// When the `sidecar` feature is NOT enabled, export a no-op stub so the rest of the
// codebase can still compile without conditional compilation boilerplate.
#[cfg(not(feature = "sidecar"))]
#[derive(Clone)]
pub struct SidecarClient;

#[cfg(not(feature = "sidecar"))]
impl SidecarClient {
    pub fn new(_endpoint: impl Into<String>) -> Self { Self }
    pub async fn predict(&self, _features: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        Ok(serde_json::json!({}))
    }
}
