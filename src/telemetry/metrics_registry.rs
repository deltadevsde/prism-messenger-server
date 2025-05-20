use lazy_static::lazy_static;
use opentelemetry::{global, metrics::{Gauge, Meter}};
use parking_lot::Mutex;
use tracing::info;
use std::sync::Arc;

use prism_telemetry::telemetry::build_attributes;

// Struct to hold all metrics
#[derive(Clone)]
pub struct PrismMessengerMetrics {
    #[allow(dead_code)]
    meter: Meter,
    // Node info metric
    pub node_info: Gauge<u64>,
}

impl Default for PrismMessengerMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl PrismMessengerMetrics {
    pub fn new() -> Self {
        info!("Initializing Prism metrics registry");
        let meter = global::meter("prism");

        let prefix = "prism_messenger_server_";
        
        let node_info = meter
            .u64_gauge(format!("{}node_info", prefix))
            .with_description("Prism node info")
            .build();

        PrismMessengerMetrics {
            meter,
            node_info,
        }
    }

    /// Records basic node information with the given attributes.
    ///
    /// # Parameters
    /// * `attributes` - Vector of key-value pairs to attach to the metric
    pub fn record_node_info(&self, attributes: Vec<(String, String)>) {
        self.node_info.record(1, build_attributes(attributes).as_slice());
    }
}

// Global instance of PrismMetrics
lazy_static! {
    static ref METRICS: Mutex<Option<Arc<PrismMessengerMetrics>>> = Mutex::new(None);
}

// Initialize the global metrics instance
pub fn init_metrics_registry() {
    let mut metrics = METRICS.lock();
    if metrics.is_none() {
        *metrics = Some(Arc::new(PrismMessengerMetrics::new()));
        info!("Prism metrics registry initialized");
    }
}

// Get a reference to the metrics registry
pub fn get_metrics() -> Option<Arc<PrismMessengerMetrics>> {
    let guard = METRICS.lock();
    guard.clone()
}
