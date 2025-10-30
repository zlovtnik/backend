use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_ws::Message;
use futures::stream::StreamExt;
use log::{debug, error, info};
use tokio::sync::broadcast;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::utils::ws_logger::LogBroadcaster;
use crate::utils::token_utils;
use crate::middleware::ws_security::{
    get_allowed_origins, is_origin_allowed, should_enforce_origin_validation, SanitizedOrigin,
};
use std::env;

/// Global connection counter for operational safeguards.
/// Used to track active WebSocket connections and enforce global limits.
/// This is initialized at module load time and shared across all handler instances.
static ACTIVE_WS_CONNECTIONS: AtomicUsize = AtomicUsize::new(0);

/// RAII guard for WebSocket connections.
/// Automatically decrements the connection counter when dropped,
/// guaranteeing cleanup on all exit paths (success, error, or panic).
/// This prevents connection count leaks when operations fail after incrementing.
struct ConnectionGuard;

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        ACTIVE_WS_CONNECTIONS.fetch_sub(1, Ordering::Relaxed);
    }
}

/// Maximum concurrent WebSocket connections allowed globally.
/// Set via WS_MAX_GLOBAL_CONNECTIONS environment variable (default: 1000).
/// This prevents resource exhaustion from too many simultaneous connections.
const DEFAULT_MAX_GLOBAL_CONNECTIONS: usize = 1000;

/// Per-client idle timeout in seconds.
/// If no data is received from either broadcaster or client for this duration,
/// the connection is closed. Set via WS_IDLE_TIMEOUT_SECS environment variable (default: 300).
const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 300;

/// Maximum number of consecutive send errors before forcing disconnect.
/// Helps prevent resource leaks from clients that can't receive data.
const MAX_SEND_ERRORS: usize = 5;

/// WebSocket handler for streaming real-time application logs.
///
/// This handler requires a valid JWT token in the Authorization header.
/// The token must be from a user in the authorized admin list (configured via WS_LOGS_ADMIN_USER env var).
/// If authorization fails, returns HTTP 403 Forbidden.
///
/// This handler upgrades an HTTP connection to WebSocket and streams
/// log messages from the application's broadcast channel to the connected client.
/// The handler maintains the connection until the client disconnects or an error occurs.
///
/// # Arguments
///
/// * `req` - The HTTP request (must contain Authorization header with Bearer token)
/// * `stream` - The payload stream for WebSocket upgrade
/// * `broadcaster` - The log broadcaster shared via app data
///
/// # Returns
///
/// `Ok(HttpResponse)` with a WebSocket connection on success,
/// `Err` if authorization fails or WebSocket upgrade fails
///
/// # Authorization
///
/// This endpoint checks:
/// 1. Authorization header is present and contains a Bearer token
/// 2. Token is valid and not expired
/// 3. User ID from token matches the authorized admin user list
///
/// Set `WS_LOGS_ADMIN_USER` environment variable to a comma-separated list of user IDs allowed to access WebSocket logs.
/// Example: `WS_LOGS_ADMIN_USER=user1,user2,admin@example.com`
///
/// # Examples
///
/// ```no_run
/// use actix_web::{web, App, HttpServer};
/// use rcs::api::ws_controller;
/// use rcs::utils::ws_logger::LogBroadcaster;
///
/// #[actix_web::main]
/// async fn main() -> std::io::Result<()> {
///     let broadcaster = LogBroadcaster::new(1000);
///
///     HttpServer::new(move || {
///         App::new()
///             .app_data(web::Data::new(broadcaster.clone()))
///             .service(web::resource("/logs").route(web::get().to(ws_controller::ws_logs)))
///     })
///     .bind("127.0.0.1:9000")?
///     .run()
///     .await
/// }
/// ```
pub async fn ws_logs(
    req: HttpRequest,
    stream: web::Payload,
    broadcaster: web::Data<LogBroadcaster>,
) -> Result<HttpResponse, Error> {
    // Get max global connection limit from environment or use default
    let max_global_connections = env::var("WS_MAX_GLOBAL_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(DEFAULT_MAX_GLOBAL_CONNECTIONS);

    // Validate Origin header during WebSocket handshake to prevent cross-site WebSocket hijacking (CSWSH)
    if should_enforce_origin_validation() {
        let allowed_origins = get_allowed_origins();

        let origin = req
            .headers()
            .get("Origin")
            .and_then(|h| h.to_str().ok());

        match origin {
            Some(origin_str) => {
                if !is_origin_allowed(origin_str, &allowed_origins) {
                    let sanitized = req.headers()
                        .get("Origin")
                        .and_then(SanitizedOrigin::from_header)
                        .map(|s| s.as_str().to_string())
                        .unwrap_or_else(|| "[invalid]".to_string());
                    error!("WebSocket logs: Rejected connection from disallowed origin: {}", sanitized);
                    return Err(actix_web::error::ErrorForbidden(
                        "Origin not allowed for WebSocket logs",
                    ));
                }
            }
            None => {
                error!("WebSocket logs: Missing Origin header (required for CORS validation)");
                return Err(actix_web::error::ErrorForbidden(
                    "Origin header required",
                ));
            }
        }
    }

    // Extract and validate JWT token
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            error!("WebSocket logs: Missing Authorization header");
            actix_web::error::ErrorForbidden("Authorization header required")
        })?;

    // Extract bearer token (do not log the token itself to avoid leaking credentials)
    let token = auth_header
        .strip_prefix("Bearer ")
        .or_else(|| auth_header.strip_prefix("bearer "))
        .ok_or_else(|| {
            error!("WebSocket logs: Invalid Authorization header format");
            actix_web::error::ErrorForbidden("Invalid Authorization header format")
        })?;

    // Decode and validate token
    let token_data = token_utils::decode_token(token.to_string()).map_err(|e| {
        error!("WebSocket logs: Token validation failed (details omitted for security)");
        debug!("WebSocket logs: Token decode error: {} (debug only)", e);
        actix_web::error::ErrorForbidden("Invalid token")
    })?;

    // Get list of authorized admin users from environment (optional - use empty string to allow any valid token)
    let authorized_users = env::var("WS_LOGS_ADMIN_USER").unwrap_or_default();

    // Check if current user is in the authorized list (only enforce if list is non-empty)
    if !authorized_users.is_empty() {
        let user_id = token_data.claims.user.to_string();
        let is_authorized = authorized_users
            .split(',')
            .map(|s| s.trim())
            .any(|auth_user| auth_user == user_id);

        if !is_authorized {
            error!("WebSocket logs: unauthorized access attempt");
            debug!("WebSocket logs: User {} not in authorized list", user_id);
            return Err(actix_web::error::ErrorForbidden(
                "User not authorized for WebSocket logs",
            ));
        }
    }

    // Upgrade the HTTP connection to WebSocket.
    // This must succeed before we reserve a connection slot.
    let (res, session, stream) = actix_ws::handle(&req, stream)?;

    // Atomically reserve a connection slot using compare-and-swap after successful handshake.
    // This eliminates the TOCTOU race condition by making the check-and-increment operation
    // indivisible: only one thread can successfully reserve each slot.
    let mut current = ACTIVE_WS_CONNECTIONS.load(Ordering::Relaxed);
    loop {
        if current >= max_global_connections {
            error!(
                "WebSocket logs: Connection rejected - global limit ({}) reached ({})",
                max_global_connections, current
            );
            // Connection is established but we couldn't reserve a slot; gracefully close it
            let _ = session.close(None).await;
            return Err(actix_web::error::ErrorServiceUnavailable(
                "WebSocket service at capacity - too many active connections",
            ));
        }

        // Attempt to atomically increment from current to current+1.
        // This succeeds only if no other thread incremented the counter between our load and this compare_exchange.
        match ACTIVE_WS_CONNECTIONS.compare_exchange(
            current,
            current + 1,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => {
                // Successfully reserved a slot; create ConnectionGuard and move it into the spawned task
                let guard = ConnectionGuard;
                let broadcaster = broadcaster.get_ref().clone();
                actix_web::rt::spawn(async move {
                    // Bind the guard to a local variable to keep it alive for the duration of this task
                    let _guard = guard;
                    if let Err(e) = handle_ws_session(session, stream, broadcaster).await {
                        debug!("WebSocket session error: {}", e);
                    }
                    // _guard is dropped here when the task completes, decrementing the counter
                });
                break;
            }
            Err(actual) => {
                // Another thread changed the counter; retry with the current value
                current = actual;
            }
        }
    }

    Ok(res)
}

/// Handles the WebSocket session by forwarding log messages to the connected client.
///
/// This function subscribes to the log broadcaster and continuously forwards
/// incoming log messages to the WebSocket client. It handles keep-alive pings,
/// client messages, and gracefully closes the connection on error.
///
/// **Operational Safeguards:**
/// - **Idle Timeout:** Closes connections idle for longer than WS_IDLE_TIMEOUT_SECS (default: 300s)
/// - **Error Threshold:** Closes connection after MAX_SEND_ERRORS consecutive send failures
/// - **Backpressure Handling:** Notifies client when messages are dropped due to buffer overflow
/// - **Resource Cleanup:** Ensures connection is properly closed and resources released
///
/// # Arguments
///
/// * `session` - The WebSocket session
/// * `stream` - The message stream from the client
/// * `broadcaster` - The log broadcaster to subscribe to
///
/// # Returns
///
/// `Ok(())` on successful session completion,
/// or an error if the session fails
async fn handle_ws_session(
    mut session: actix_ws::Session,
    mut stream: actix_ws::MessageStream,
    broadcaster: LogBroadcaster,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get idle timeout from environment or use default
    let idle_timeout_secs = env::var("WS_IDLE_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(DEFAULT_IDLE_TIMEOUT_SECS);

    // Subscribe to the log broadcaster
    let mut rx = broadcaster.subscribe();
    let mut last_activity = std::time::Instant::now();
    let mut consecutive_send_errors = 0usize;

    info!("WebSocket log client connected (authentication and origin validated)");

    // Send initial message
    session
        .text("Log streaming started. Receiving real-time application logs.")
        .await?;
    last_activity = std::time::Instant::now();

    loop {
        // Calculate remaining idle timeout
        let elapsed = last_activity.elapsed();
        let timeout_duration = if elapsed.as_secs() < idle_timeout_secs {
            std::time::Duration::from_secs(idle_timeout_secs - elapsed.as_secs())
        } else {
            // Idle timeout exceeded
            info!(
                "WebSocket log client disconnected (idle timeout after {}s)",
                idle_timeout_secs
            );
            let _ = session.text("[INFO] Connection closed due to inactivity.").await;
            let _ = session.close(None).await;
            break;
        };

        // Use tokio::select! with timeout for idle connection detection
        tokio::select! {
            // Handle incoming log messages
            msg = tokio::time::timeout(timeout_duration, rx.recv()) => {
                match msg {
                    Ok(Ok(log_msg)) => {
                        // Send the log message to the WebSocket client
                        match session.text(log_msg).await {
                            Ok(_) => {
                                // Reset error counter on successful send
                                consecutive_send_errors = 0;
                                last_activity = std::time::Instant::now();
                            }
                            Err(e) => {
                                consecutive_send_errors += 1;
                                debug!("Failed to send log message to WebSocket client (attempt {}): {}", 
                                    consecutive_send_errors, e);

                                // Force disconnect after too many send errors
                                if consecutive_send_errors >= MAX_SEND_ERRORS {
                                    error!(
                                        "WebSocket client disconnected after {} consecutive send errors",
                                        MAX_SEND_ERRORS
                                    );
                                    let _ = session.close(None).await;
                                    break;
                                }
                            }
                        }
                    }
                    Ok(Err(broadcast::error::RecvError::Lagged(skipped))) => {
                        // Notify the client that messages were dropped (backpressure signal)
                        let lag_msg = format!(
                            "[WARNING] {} log messages were dropped due to buffer overflow - subscriber too slow",
                            skipped
                        );
                        if let Err(e) = session.text(lag_msg).await {
                            debug!("Failed to send lag warning: {}", e);
                            consecutive_send_errors += 1;
                        } else {
                            consecutive_send_errors = 0;
                            last_activity = std::time::Instant::now();
                        }
                    }
                    Ok(Err(broadcast::error::RecvError::Closed)) => {
                        // The broadcaster was dropped, close the connection
                        debug!("Log broadcaster closed");
                        break;
                    }
                    Err(_) => {
                        // Timeout: No message received within idle timeout window
                        info!(
                            "WebSocket log client disconnected (idle timeout after {}s)",
                            idle_timeout_secs
                        );
                        let _ = session.text("[INFO] Connection closed due to inactivity.").await;
                        let _ = session.close(None).await;
                        break;
                    }
                }
            }

            // Handle incoming WebSocket messages from client (ping/pong, etc.)
            msg = stream.next() => {
                match msg {
                    Some(Ok(msg)) => {
                        match msg {
                            Message::Ping(bytes) => {
                                // Respond to ping with pong (keep-alive)
                                if let Err(e) = session.pong(&bytes).await {
                                    debug!("Failed to send pong: {}", e);
                                    break;
                                }
                                last_activity = std::time::Instant::now();
                            }
                            Message::Pong(_) => {
                                // Ignore pong messages, update activity timer
                                last_activity = std::time::Instant::now();
                            }
                            Message::Text(_) => {
                                // Ignore text messages from client (one-way log stream)
                                // but update activity timer to prevent idle timeout
                                last_activity = std::time::Instant::now();
                            }
                            Message::Binary(_) => {
                                // Ignore binary messages from client
                                last_activity = std::time::Instant::now();
                            }
                            Message::Close(reason) => {
                                // Client requested close
                                let _ = session.close(reason).await;
                                info!("WebSocket log client disconnected (client-initiated)");
                                break;
                            }
                            _ => {
                                // Handle other message types if needed
                                last_activity = std::time::Instant::now();
                            }
                        }
                    }
                    Some(Err(e)) => {
                        error!("WebSocket message error: {}", e);
                        break;
                    }
                    None => {
                        // Connection closed by client
                        info!("WebSocket log client disconnected");
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}
