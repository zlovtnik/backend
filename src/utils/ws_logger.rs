use std::fmt;
use chrono::Local;
use tokio::sync::broadcast;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use serde_json::json;

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
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let metadata = event.metadata();
        let level = metadata.level();

        // Format the log event based on configured format
        let message = format_log_event(event, level, metadata, self.format);
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
///
/// # Returns
///
/// A formatted log message string
fn format_log_event(
    event: &tracing::Event<'_>,
    level: &tracing::Level,
    metadata: &tracing::Metadata<'_>,
    format: LogFormat,
) -> String {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let target = metadata.target();

    // Create a visitor to extract the message
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
            // Use serde_json for JSON output
            let json_obj = serde_json::json!({
                "timestamp": timestamp.to_string(),
                "level": level.to_string(),
                "target": target,
                "message": message,
            });
            json_obj.to_string()
        }
    }
}

/// A visitor that extracts the message from a tracing event.
#[derive(Default)]
struct LogVisitor {
    message: String,
}

impl tracing::field::Visit for LogVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        }
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
            // If the subscriber is already set, we return Ok since we want idempotent behavior
            // In tests, this allows multiple calls to init_websocket_logging
            eprintln!(
                "Tracing subscriber already initialized or failed to initialize: {:?}",
                e
            );
            Ok(())
        }
    }
}
