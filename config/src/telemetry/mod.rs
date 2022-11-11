//! providers opentelemetry configuration, as well as a wrapper around the tracer
//! with helper functions for settings values such as keyed tx hashes, errors, etc..

pub mod tulip_tracer;

use anyhow::Result;
use opentelemetry::global::shutdown_tracer_provider;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tulip_tracer::*;

#[remain::sorted]
#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
/// provides configuration option for application telemetry
pub struct Telemetry {
    pub agent_endpoint: String,
    pub enabled: bool,
}

impl Telemetry {
    pub fn new_tracer(&self, service_name: &str) -> Result<Arc<TulipTracer>> {
        tulip_tracer::TulipTracer::new(service_name.to_string(), self.agent_endpoint.clone())
    }
    /// use to invoke the global shutdown_tracer_provider function
    /// todo(bonedaddy): probably need to refactor this
    pub fn shutdown_tracer(&self) {
        shutdown_tracer_provider();
    }
}
