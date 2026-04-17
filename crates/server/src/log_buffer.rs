use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use tracing::{Event, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;

use shared::models::ServerLogEntry;

const MAX_ENTRIES: usize = 1000;

pub type LogBuffer = Arc<RwLock<VecDeque<ServerLogEntry>>>;

pub fn new_buffer() -> LogBuffer {
    Arc::new(RwLock::new(VecDeque::with_capacity(MAX_ENTRIES)))
}

pub struct BufferLayer {
    buffer: LogBuffer,
}

impl BufferLayer {
    pub fn new(buffer: LogBuffer) -> Self {
        Self { buffer }
    }
}

struct MessageVisitor<'a> {
    message: &'a mut String,
}

impl<'a> tracing::field::Visit for MessageVisitor<'a> {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message.push_str(value);
        } else {
            use std::fmt::Write;
            let _ = write!(self.message, " {}={}", field.name(), value);
        }
    }
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        use std::fmt::Write;
        if field.name() == "message" {
            let _ = write!(self.message, "{:?}", value);
        } else {
            let _ = write!(self.message, " {}={:?}", field.name(), value);
        }
    }
}

impl<S> Layer<S> for BufferLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let level = metadata.level().as_str().to_string();

        // Only keep WARN and ERROR in the buffer (keeps it small + relevant)
        if metadata.level() > &tracing::Level::WARN {
            return;
        }

        let target = metadata.target().to_string();

        let mut message = String::new();
        let mut visitor = MessageVisitor { message: &mut message };
        event.record(&mut visitor);

        let timestamp = chrono::Utc::now().to_rfc3339();

        let entry = ServerLogEntry { timestamp, level, target, message };

        let mut buf = self.buffer.write().unwrap();
        if buf.len() >= MAX_ENTRIES {
            buf.pop_front();
        }
        buf.push_back(entry);
    }
}
