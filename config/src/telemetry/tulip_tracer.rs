//! provides TulipTracer, a wrapper around the opentelemetry tracer library
use anyhow::Result;
use opentelemetry::trace::StatusCode::Error as TraceError;
use opentelemetry::KeyValue;
use opentelemetry::{sdk::trace::Tracer, trace::SpanRef};
use std::sync::Arc;
use std::time::SystemTime;

/// TulipTracer is a wrapper around the opentelemetry tracer library
/// providing facilities for ease of use
pub struct TulipTracer {
    pub tracer: Arc<Tracer>,
}

pub struct TulipSpan<'a> {
    pub span: SpanRef<'a>,
}

impl TulipTracer {
    pub fn new(service_name: String, agent_endpoint: String) -> Result<Arc<TulipTracer>> {
        let tracer = opentelemetry_datadog::new_pipeline()
            .with_service_name(service_name)
            .with_agent_endpoint(agent_endpoint)
            .install_batch(opentelemetry::runtime::Tokio)?;
        Ok(Arc::new(TulipTracer {
            tracer: Arc::new(tracer),
        }))
    }
    pub fn new_tulip_span<'a>(&self, span: SpanRef<'a>) -> TulipSpan<'a> {
        TulipSpan { span }
    }
}

impl<'a> TulipSpan<'a> {
    pub fn set_name(&mut self, name: String) {
        self.span.set_attribute(KeyValue::new("name", name))
    }
    /// set an arbitrary key-value pair in the span
    pub fn set_kv(&mut self, key: String, value: String) {
        self.span.set_attribute(KeyValue::new(key, value))
    }
    /// sets the `skipped` attribute, along with a reason
    pub fn set_skipped(&mut self, reason: String) {
        self.span
            .set_attribute(KeyValue::new("skipped", format!("reason:{}", reason)));
    }
    /// sets a tx_hash attribute
    pub fn set_txhash(&mut self, hash: String) {
        self.set_keyed_txhash("".to_string(), hash)
    }
    /// sets a name tx_hash attribute
    pub fn set_keyed_txhash(&mut self, key: String, hash: String) {
        if key.is_empty() {
            self.span.set_attribute(KeyValue::new("tx_hash", hash))
        } else {
            self.span
                .set_attribute(KeyValue::new(format!("{}.tx_hash", key), hash));
        }
    }
    pub fn set_event(&mut self, name: String, key: String, value: String) {
        self.set_events(name, vec![KeyValue::new(key, value)]);
    }
    pub fn set_events(&mut self, name: String, events: Vec<KeyValue>) {
        self.span
            .add_event_with_timestamp(name, SystemTime::now(), events);
    }
    /// sets a keyed error attribute, but does not set the eror status
    /// this should be used to set errors that may be received during processing
    /// which are recoverable, or may not be critical enough to warrant aborting
    /// further processing
    pub fn new_keyed_error(&mut self, name: String, error_msg: String) {
        self.set_error(name, error_msg)
    }
    /// used to set a fatal error status, indicating the received error
    /// is an unrecoverable failure and that the current request must be aborted
    pub fn new_error(&mut self, error_msg: String) {
        self.set_error("".to_string(), error_msg);
    }
    /// helper function to end the span with a timestamp
    /// if this span is ending without an error, then the Ok status code is set
    pub fn end(&mut self) {
        self.span.end_with_timestamp(SystemTime::now())
    }
    // helper function to set error messages.
    fn set_error(&mut self, name: String, error_msg: String) {
        self.span.set_status(TraceError, error_msg.clone());
        if name.is_empty() {
            self.span.set_attribute(KeyValue::new("error", error_msg));
        } else {
            self.span
                .set_attribute(KeyValue::new(format!("{}.error", name), error_msg));
        }
    }
}
