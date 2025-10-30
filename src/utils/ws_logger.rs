use std::fmt;
use tokio::sync::broadcast;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

/// Log output format types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// Human-readable text format with timestamp, level, target, and message
    Text,
    /// JSON format with structured fields: timestamp, level, target, message
    Json,
}

impl LogFormat {
    /// Parses a format string from environment or returns the default Text format
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rcs::utils::ws_logger::LogFormat;
    ///
    /// assert_eq!(LogFormat::from_env_or_default(), LogFormat::Text); // default
    /// std::env::set_var("WS_LOG_FORMAT", "json");
    /// assert_eq!(LogFormat::from_env_or_default(), LogFormat::Json);
    /// ```
    pub fn from_env_or_default() -> Self {
        use std::env;
        env::var("WS_LOG_FORMAT")
            .ok()
            .and_then(|s| match s.to_lowercase().as_str() {
                "json" => Some(LogFormat::Json),
                "text" => Some(LogFormat::Text),
                _ => None,
            })
            .unwrap_or(LogFormat::Text)
    }
}

/// Broadcasts log messages to multiple WebSocket subscribers.
///
/// This struct wraps a `tokio::sync::broadcast::Sender<String>` to allow
/// real-time distribution of log messages to multiple connected WebSocket clients.
#[derive(Clone)]
pub struct LogBroadcaster {
    sender: broadcast::Sender<String>,
}

impl LogBroadcaster {
    /// Creates a new `LogBroadcaster` with the specified buffer capacity.
    ///
    /// The capacity will be clamped to at least 1 to avoid panics from
    /// misconfigured zero capacity values. If capacity is 0, it will be
    /// automatically increased to 1.
    ///
    /// # Arguments
    ///
    /// * `capacity` - The maximum number of log messages to buffer in the broadcast channel
    ///   (will be clamped to at least 1 if 0)
    ///
    /// # Examples
    ///
    /// ```
    /// use rcs::utils::ws_logger::LogBroadcaster;
    ///
    /// let broadcaster = LogBroadcaster::new(1000);
    /// 
    /// // Zero capacity is automatically clamped to 1
    /// let broadcaster_min = LogBroadcaster::new(0);
    /// ```
    pub fn new(capacity: usize) -> Self {
        // Guard against zero capacity which would panic in broadcast::channel
        let cap = capacity.max(1);
        let (sender, _) = broadcast::channel(cap);
        LogBroadcaster { sender }
    }

    /// Broadcasts a log message to all subscribed receivers.
    ///
    /// Ignores errors when there are no active receivers.
    ///
    /// # Arguments
    ///
    /// * `message` - The log message to broadcast
    ///
    /// # Examples
    ///
    /// ```
    /// use rcs::utils::ws_logger::LogBroadcaster;
    ///
    /// let broadcaster = LogBroadcaster::new(100);
    /// broadcaster.send("Application started".to_string());
    /// ```
    pub fn send(&self, message: String) {
        let _ = self.sender.send(message);
    }

    /// Creates a new receiver subscribed to the log broadcast channel.
    ///
    /// # Returns
    ///
    /// A `tokio::sync::broadcast::Receiver<String>` that will receive broadcasted log messages
    ///
    /// # Examples
    ///
    /// ```
    /// use rcs::utils::ws_logger::LogBroadcaster;
    ///
    /// let broadcaster = LogBroadcaster::new(100);
    /// let mut receiver = broadcaster.subscribe();
    /// ```
    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.sender.subscribe()
    }
}

/// Custom tracing layer that broadcasts log events to WebSocket clients.
///
/// This layer implements the `tracing_subscriber::Layer` trait to capture
/// tracing events and format them into log messages that are sent to the
/// `LogBroadcaster`. Supports both text and JSON output formats.
pub struct WebSocketLogLayer {
    broadcaster: LogBroadcaster,
    format: LogFormat,
}

impl WebSocketLogLayer {
    /// Creates a new `WebSocketLogLayer` with the given broadcaster and format.
    ///
    /// # Arguments
    ///
    /// * `broadcaster` - The `LogBroadcaster` to send formatted events to
    /// * `format` - The format to use for log messages (Text or Json)
    pub fn new(broadcaster: LogBroadcaster, format: LogFormat) -> Self {
        WebSocketLogLayer {
            broadcaster,
            format,
        }
    }
}

impl<S> tracing_subscriber::Layer<S> for WebSocketLogLayer
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let metadata = event.metadata();
        let level = metadata.level();
        let span_name = metadata.name().to_string();
        let message = format_log_event(event, level, metadata, self.format, span_name);
        self.broadcaster.send(message);
    }
}

/// Formats a tracing event into a log message using the specified format.
///
/// # Arguments
///
/// * `event` - The tracing event to format
/// * `level` - The log level
/// * `metadata` - The event metadata
/// * `format` - The output format (Text or Json)
/// * `span_name` - The name of the current span
///
/// # Returns
///
/// A formatted log message string
fn format_log_event(
    event: &tracing::Event<'_>,
    level: &tracing::Level,
    metadata: &tracing::Metadata<'_>,
    format: LogFormat,
    span_name: String,
) -> String {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let target = metadata.target();

    // Create a visitor to extract the message and all fields
    let mut visitor = LogVisitor::default();
    event.record(&mut visitor);

    let message = if visitor.message.is_empty() {
        "[no message]".to_string()
    } else {
        visitor.message
    };

    match format {
        LogFormat::Text => {
            format!("[{}] {} [{}] {}", timestamp, level, target, message)
        }
        LogFormat::Json => {
            // Build JSON object with structured fields
            let mut json_obj = serde_json::json!({
                "timestamp": timestamp.to_string(),
                "level": level.to_string(),
                "target": target,
                "message": message,
                "span": span_name,
            });
            
            // Add all captured fields (if any besides message)
            if let Some(obj) = json_obj.as_object_mut() {
                // Include fields object with all recorded field values
                if !visitor.fields.is_empty() {
                    let fields: serde_json::Map<String, serde_json::Value> = visitor.fields
                        .into_iter()
                        .filter(|(k, _)| k != "message") // Message already at root level
                        .map(|(k, v)| (k, serde_json::Value::String(v)))
                        .collect();
                    if !fields.is_empty() {
                        obj.insert("fields".to_string(), serde_json::Value::Object(fields));
                    }
                }
            }
            
            json_obj.to_string()
        }
    }
}

/// A visitor that captures all fields from a tracing event into a structured map.
#[derive(Default)]
struct LogVisitor {
    message: String,
    fields: std::collections::BTreeMap<String, String>,
}

impl tracing::field::Visit for LogVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        let field_name = field.name();
        let value_str = format!("{:?}", value);
        
        // Always capture the message field
        if field_name == "message" {
            self.message = value_str.clone();
        }
        
        // Capture all fields (including message) in the structured map for JSON output
        self.fields.insert(field_name.to_string(), value_str);
    }
}

/// Initializes the tracing subscriber with WebSocket logging, console output, and env filter.
///
/// Sets up the tracing infrastructure with:
/// - Custom `WebSocketLogLayer` for broadcasting log events to WebSocket clients
/// - `fmt::layer()` for console output with timestamps and targets
/// - Environment filter for log level control
/// - Log-to-tracing bridge for compatibility with existing `log` crate macros
///
/// This function is safe to call multiple times. If called when a global subscriber
/// is already set, it returns `Ok(())` without error (idempotent).
///
/// # Arguments
///
/// * `broadcaster` - The `LogBroadcaster` to use for broadcasting log events
///
/// # Returns
///
/// `Ok(())` if initialization succeeds or if already initialized,
/// or an error if setup fails for other reasons
///
/// # Examples
///
/// ```
/// use rcs::utils::ws_logger::{LogBroadcaster, init_websocket_logging};
///
/// let broadcaster = LogBroadcaster::new(1000);
/// let _ = init_websocket_logging(broadcaster);
/// ```
pub fn init_websocket_logging(
    broadcaster: LogBroadcaster,
) -> Result<(), Box<dyn std::error::Error>> {
    use tracing_log::LogTracer;
    use tracing_subscriber::fmt;

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // Determine output format from environment
    let format = LogFormat::from_env_or_default();

    let ws_layer = WebSocketLogLayer::new(broadcaster, format);

    // Create fmt layer with timestamps and targets
    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true);

    // Initialize LogTracer bridge first (idempotent - errors are ignored)
    let _ = LogTracer::init();

    // Try to initialize the global subscriber with both WebSocket and fmt layers
    // If already initialized, ignore the error and return Ok
    let result = tracing_subscriber::registry()
        .with(env_filter)
        .with(ws_layer)
        .with(fmt_layer)
        .try_init();

    // Only return error if it's not due to already being initialized
    match result {
        Ok(()) => Ok(()),
        Err(e) => {
            // Treat already-initialized scenarios as success (idempotent)
            let err_text = e.to_string().to_lowercase();
            if err_text.contains("already been initialized")
                || err_text.contains("already initialized")
                || err_text.contains("global default subscriber set")
            {
                log::debug!(
                    "Tracing subscriber already initialized (expected in tests or multi-starts): {:?}",
                    e
                );
                Ok(())
            } else {
                log::warn!("Failed to initialize tracing subscriber: {:?}", e);
                Err(Box::new(e))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_log_broadcaster_sends_and_receives() {
        let broadcaster = LogBroadcaster::new(100);
        let mut receiver = broadcaster.subscribe();

        broadcaster.send("Test message 1".to_string());
        broadcaster.send("Test message 2".to_string());

        // Verify messages are received
        let msg1 = timeout(Duration::from_millis(100), receiver.recv())
            .await
            .expect("should receive message 1")
            .expect("message should be valid");
        assert_eq!(msg1, "Test message 1");

        let msg2 = timeout(Duration::from_millis(100), receiver.recv())
            .await
            .expect("should receive message 2")
            .expect("message should be valid");
        assert_eq!(msg2, "Test message 2");
    }

    #[tokio::test]
    async fn test_log_broadcaster_multiple_receivers() {
        let broadcaster = LogBroadcaster::new(100);
        let mut receiver1 = broadcaster.subscribe();
        let mut receiver2 = broadcaster.subscribe();

        broadcaster.send("Broadcast message".to_string());

        // Both receivers should get the message
        let msg1 = timeout(Duration::from_millis(100), receiver1.recv())
            .await
            .expect("receiver1 should receive message")
            .expect("message should be valid");
        assert_eq!(msg1, "Broadcast message");

        let msg2 = timeout(Duration::from_millis(100), receiver2.recv())
            .await
            .expect("receiver2 should receive message")
            .expect("message should be valid");
        assert_eq!(msg2, "Broadcast message");
    }

    #[test]
    fn test_log_broadcaster_ignores_no_receivers() {
        let broadcaster = LogBroadcaster::new(100);
        // Send message with no receivers - should not panic
        broadcaster.send("Message with no receivers".to_string());
        // If we reach here, test passes
    }

    #[test]
    fn test_log_format_from_env_default() {
        // When WS_LOG_FORMAT is not set, should default to Text
        let format = LogFormat::from_env_or_default();
        assert_eq!(format, LogFormat::Text);
    }

    #[test]
    fn test_log_format_debug_display() {
        assert_eq!(format!("{:?}", LogFormat::Text), "Text");
        assert_eq!(format!("{:?}", LogFormat::Json), "Json");
    }
}
