use once_cell::sync::OnceCell;
use std::error::Error;

use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

static PROM_HANDLE: OnceCell<PrometheusHandle> = OnceCell::new();

/// Initialise the global Prometheus recorder and expose a handle that allows
/// rendering metrics in the Prometheus exposition format.
///
/// This should be called once at programme start, before any metrics are
/// emitted. Calling it multiple times is a no-op after the first success.
pub fn init() -> Result<(), Box<dyn Error + Send + Sync>> {
    if PROM_HANDLE.get().is_some() {
        return Ok(()); // already initialised
    }

    let builder = PrometheusBuilder::new();
    let recorder = builder.build();
    let handle = recorder.handle();

    // ignore error if recorder already set by tests
    let _ = metrics::set_boxed_recorder(Box::new(recorder));

    let _ = PROM_HANDLE.set(handle);
    Ok(())
}

/// Return the global Prometheus handle. Panics if `init` has not been called.
pub fn handle() -> &'static PrometheusHandle {
    PROM_HANDLE
        .get()
        .expect("metrics::init() must be called first")
}
