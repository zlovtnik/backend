//! Functional State Transitions
//!
//! This module provides high-level functional state transition operations
//! for the immutable state management system. It defines common state
//! transformation patterns while maintaining immutability and tenant isolation.
//!
//! Key features:
//! - User session management transitions
//! - Application data manipulation transitions
//! - Query cache management transitions
//! - Composite transition builders
//! - Transition validation and composition

#![allow(dead_code)]

use super::immutable_state::{PersistentVector, QueryResult, SessionData, TenantApplicationState};
use chrono::{DateTime, Duration, Utc};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Result type for state transitions
pub type TransitionResult<T> = Result<T, TransitionError>;

/// Errors that can occur during state transitions
#[derive(Debug, thiserror::Error)]
pub enum TransitionError {
    #[error("Invalid transition parameters: {message}")]
    InvalidParameters { message: String },

    #[error("State transition validation failed: {field} - {reason}")]
    ValidationFailed { field: String, reason: String },

    #[error("Resource not found: {resource_type} '{resource_id}'")]
    NotFound {
        resource_type: String,
        resource_id: String,
    },

    #[error("Concurrency conflict: {details}")]
    ConcurrencyConflict { details: String },

    #[error("Serialization error: {message}")]
    SerializationError { message: String },
}

/// Transition context for carrying metadata through transition chains
#[derive(Clone, Debug)]
pub struct TransitionContext {
    /// Initiating user ID (for audit trails)
    pub user_id: Option<String>,
    /// Transition timestamp
    pub timestamp: DateTime<Utc>,
    /// Additional metadata
    pub metadata: HashMap<String, JsonValue>,
}

/// Creates a transition that inserts a new user session into a tenant's application state.
///
/// The produced closure clones the given state, adds a SessionData entry for `session_id` with
/// the provided `user_data` and an expiry computed as now + `ttl_seconds`, updates `last_updated`,
/// and returns the new state.
///
/// # Examples
///
/// ```
/// /// let state = TenantApplicationState::default();
/// /// let transition = create_user_session("sess-123".to_string(), "user-42".to_string(), 3600);
/// /// let new_state = transition(&state);
/// assert!(new_state.user_sessions.contains_key(&"sess-123".to_string()));
/// ```
pub fn create_user_session(
    session_id: String,
    user_data: String,
    ttl_seconds: u64,
) -> impl FnOnce(&TenantApplicationState) -> TenantApplicationState {
    move |state| {
        let mut new_state = state.clone();
        new_state.user_sessions = state.user_sessions.insert(
            session_id,
            SessionData {
                user_data,
                expires_at: Utc::now() + Duration::seconds(ttl_seconds as i64),
            },
        );
        new_state.last_updated = Utc::now();

        new_state
    }
}

/// Create a transition that updates an existing user session.
///
/// The returned transition, when applied to a tenant's application state, will:
/// - leave the state unchanged if the session ID does not exist;
/// - otherwise replace the session's `user_data` with `new_user_data` and, if
///   `extend_ttl_seconds` is provided, extend the session expiry by that many seconds;
/// - update the state's `last_updated` timestamp to the current time.
///
/// # Parameters
///
/// - `session_id` — session identifier to update; must not be empty.
/// - `new_user_data` — new data to store on the session.
/// - `extend_ttl_seconds` — optional number of seconds to extend the session expiry.
///
/// # Returns
///
/// A closure which, when invoked with a `&TenantApplicationState`, produces a new
/// `TenantApplicationState` reflecting the described update.
///
/// # Examples
///
/// ```
/// // Create a transition for session "s1" that updates its user data without extending TTL.
/// /// let transition = update_user_session("s1", "new-data".to_string(), None).unwrap();
/// // Applying to a state with no such session leaves it unchanged.
/// /// let state = TenantApplicationState::default();
/// /// let new_state = transition(&state);
/// assert_eq!(new_state.user_sessions.contains_key("s1"), false);
/// ```
pub fn update_user_session(
    session_id: impl Into<String>,
    new_user_data: String,
    extend_ttl_seconds: Option<u64>,
) -> Result<impl FnOnce(&TenantApplicationState) -> TenantApplicationState, TransitionError> {
    let session_id = session_id.into();

    if session_id.trim().is_empty() {
        return Err(TransitionError::InvalidParameters {
            message: "Session ID cannot be empty".to_string(),
        });
    }

    Ok(move |state: &TenantApplicationState| {
        if !state.user_sessions.contains_key(&session_id) {
            // Session doesn't exist - this would be an error, but for the transition
            // function itself, we'll just return the original state unchanged
            return state.clone();
        }

        let mut new_state = state.clone();

        // Get the existing session to preserve/update its data
        if let Some(existing_session) = state.user_sessions.get(&session_id) {
            let updated_session = SessionData {
                user_data: new_user_data,
                expires_at: if let Some(ttl) = extend_ttl_seconds {
                    Utc::now() + Duration::seconds(ttl as i64)
                } else {
                    existing_session.expires_at
                },
            };
            new_state.user_sessions = state.user_sessions.insert(session_id, updated_session);
        }

        new_state.last_updated = Utc::now();

        new_state
    })
}

/// Removes a user session from the tenant state.
///
/// The returned transition produces a new `TenantApplicationState` with the specified session removed
/// (if present) and `last_updated` set to the current time.
///
/// # Examples
///
/// ```
/// // Assuming `state` is a `TenantApplicationState` containing a session `"sess1"`.
/// /// let transition = remove_user_session("sess1");
/// /// let new_state = transition(&state);
/// assert!(!new_state.user_sessions.contains_key("sess1"));
/// ```
pub fn remove_user_session(
    session_id: impl Into<String>,
) -> impl FnOnce(&TenantApplicationState) -> TenantApplicationState {
    let session_id = session_id.into();

    move |state| {
        let mut new_state = state.clone();
        new_state.user_sessions = state.user_sessions.remove(&session_id);
        new_state.last_updated = Utc::now();

        new_state
    }
}

/// Creates a transition that sets or updates an application configuration entry.
///
/// The returned transition, when applied to a `TenantApplicationState`, inserts or replaces
/// `app_data[key]` with the provided JSON `value` and updates the state's `last_updated` timestamp.
///
/// # Returns
///
/// - `Ok(transition)` — a closure that applies the configuration change and updates `last_updated`.
/// - `Err(TransitionError::InvalidParameters)` — if `key` is empty or whitespace.
/// - `Err(TransitionError::ValidationFailed)` — if a provided `validate` function returns `false`.
///
/// # Examples
///
/// ```
/// use serde_json::json;
///
/// // prepare a transition
/// /// let transition = set_app_config("theme", json!("dark"), None).unwrap();
///
/// // apply it to an existing state (assumes `state` is a TenantApplicationState)
/// /// let new_state = transition(&state);
/// assert_eq!(new_state.app_data.get("theme").unwrap(), &json!("dark"));
/// ```
pub fn set_app_config<F>(
    key: impl Into<String>,
    value: JsonValue,
    validate: Option<F>,
) -> Result<impl FnOnce(&TenantApplicationState) -> TenantApplicationState, TransitionError>
where
    F: Fn(&JsonValue) -> bool,
{
    let key = key.into();

    if key.trim().is_empty() {
        return Err(TransitionError::InvalidParameters {
            message: "Configuration key cannot be empty".to_string(),
        });
    }

    // Validate the value if validator provided
    if let Some(ref validator) = validate {
        if !validator(&value) {
            return Err(TransitionError::ValidationFailed {
                field: key.clone(),
                reason: "Configuration value failed validation".to_string(),
            });
        }
    }

    Ok(move |state: &TenantApplicationState| {
        let mut new_state = state.clone();
        new_state.app_data = state.app_data.insert(key, value);
        new_state.last_updated = Utc::now();

        new_state
    })
}

/// Removes a configuration entry identified by `key` from the application's data and updates `last_updated`.
///
/// # Examples
///
/// ```
/// /// let state = TenantApplicationState::default();
/// /// let new_state = remove_app_config("theme")(&state);
/// assert!(!new_state.app_data.contains_key("theme"));
/// ```
pub fn remove_app_config(
    key: impl Into<String>,
) -> impl FnOnce(&TenantApplicationState) -> TenantApplicationState {
    let key = key.into();

    move |state| {
        let mut new_state = state.clone();
        new_state.app_data = state.app_data.remove(&key);
        new_state.last_updated = Utc::now();

        new_state
    }
}

/// Updates an application data entry identified by `key` by applying `transform`.
///
/// If `key` is absent, `default_value` is provided to `transform`. When `transform` returns `Ok(new_value)`,
/// the produced transition sets `app_data[key]` to `new_value` and updates `last_updated` to the current time.
/// If `transform` returns `Err(_)`, the transition returns the original state unchanged.
///
/// `key` must not be empty or contain only whitespace; otherwise this function returns
/// `Err(TransitionError::InvalidParameters)`.
///
/// # Examples
///
/// ```no_run
/// use chrono::Utc;
/// use serde_json::json;
/// use crate::{TenantApplicationState, transform_app_data};
///
/// // Create a transition that increments an integer stored under "counter"
/// let tr = transform_app_data(
///     "counter",
///     |v| {
///         let n = v.as_i64().unwrap_or(0);
///         Ok(json!(n + 1))
///     },
///     json!(0),
/// ).unwrap();
///
/// /// let state = TenantApplicationState::default();
/// /// let new_state = tr(&state);
/// assert!(new_state.app_data.get("counter").unwrap().as_i64().unwrap() >= 1);
/// ```
pub fn transform_app_data<F>(
    key: impl Into<String>,
    transform: F,
    default_value: JsonValue,
) -> Result<impl FnOnce(&TenantApplicationState) -> TenantApplicationState, TransitionError>
where
    F: Fn(&JsonValue) -> Result<JsonValue, String>,
{
    let key = key.into();

    if key.trim().is_empty() {
        return Err(TransitionError::InvalidParameters {
            message: "Data key cannot be empty".to_string(),
        });
    }

    Ok(move |state: &TenantApplicationState| {
        let current_value = state.app_data.get(&key).unwrap_or(&default_value);
        let new_value = match transform(current_value) {
            Ok(val) => val,
            Err(_) => return state.clone(), // Transform failed, return unchanged state
        };

        let mut new_state = state.clone();
        new_state.app_data = state.app_data.insert(key, new_value);
        new_state.last_updated = Utc::now();

        new_state
    })
}

/// Appends a query result with a computed expiration to a tenant's query cache.
///
/// Validates that `query_id` and `data` are not empty and, on success, returns a transition
/// closure that, when applied to a `TenantApplicationState`, clones the state, appends a
/// `QueryResult { query_id, data, expires_at }` (with `expires_at = now + ttl_seconds`),
/// updates `last_updated`, and returns the new state.
///
/// # Errors
///
/// Returns `TransitionError::InvalidParameters` if `query_id` is empty or `data` is empty.
///
/// # Examples
///
/// ```
/// /// let state = TenantApplicationState::default();
/// let transition = cache_query_result("search:users?page=1", vec![1, 2, 3], 60).unwrap();
/// /// let new_state = transition(&state);
/// assert!(new_state
///     .query_cache
///     .iter()
///     .any(|entry| entry.query_id == "search:users?page=1"));
/// ```
pub fn cache_query_result(
    query_id: impl Into<String>,
    data: Vec<u8>,
    ttl_seconds: u64,
) -> Result<impl FnOnce(&TenantApplicationState) -> TenantApplicationState, TransitionError> {
    let query_id = query_id.into();

    if query_id.trim().is_empty() {
        return Err(TransitionError::InvalidParameters {
            message: "Query ID cannot be empty".to_string(),
        });
    }

    if data.is_empty() {
        return Err(TransitionError::InvalidParameters {
            message: "Query data cannot be empty".to_string(),
        });
    }

    let expires_at = Utc::now() + Duration::seconds(ttl_seconds as i64);
    let query_result = QueryResult {
        query_id,
        data,
        expires_at,
    };

    Ok(move |state: &TenantApplicationState| {
        let mut new_state = state.clone();
        new_state.query_cache = state.query_cache.append(query_result);
        new_state.last_updated = Utc::now();

        new_state
    })
}

/// Removes expired entries from a tenant's query cache.
///
/// The produced transition returns a new `TenantApplicationState` containing only
/// cache entries whose `expires_at` is greater than the current time and updates
/// the state's `last_updated` timestamp to the current time.
///
/// # Examples
///
/// ```
/// let transition = clean_expired_cache();
/// /// let new_state = transition(&old_state);
/// // `new_state.query_cache` contains only entries with `expires_at > Utc::now()`.
/// ```
pub fn clean_expired_cache() -> impl FnOnce(&TenantApplicationState) -> TenantApplicationState {
    move |state| {
        let now = Utc::now();
        let mut valid_entries = Vec::new();

        // Filter out expired entries
        for i in 0..state.query_cache.len() {
            if let Some(entry) = state.query_cache.get(i) {
                if entry.expires_at > now {
                    valid_entries.push(entry.clone());
                }
            }
        }

        // Rebuild cache with only valid entries
        let mut new_cache = PersistentVector::new();
        for entry in valid_entries {
            new_cache = new_cache.append(entry);
        }

        let mut new_state = state.clone();
        new_state.query_cache = new_cache;
        new_state.last_updated = Utc::now();

        new_state
    }
}

/// Create a sequence of state transitions that perform a user login.
///
/// The returned transitions, when applied in order to a tenant state, remove expired query-cache entries, create a new user session with a generated session ID, and record the user's last-login timestamp in app data.
///
/// # Returns
///
/// A vector of boxed transition functions that each take a `TenantApplicationState` and return an updated `TenantApplicationState`; applying them sequentially performs the login-related updates.
///
/// # Examples
///
/// ```
/// let transitions = build_login_transitions("alice", "{\"roles\": [\"user\"]}".to_string(), 3600);
/// assert!(transitions.is_ok());
/// let transitions = transitions.unwrap();
/// assert_eq!(transitions.len(), 3);
/// ```
pub fn build_login_transitions(
    user_id: impl Into<String>,
    session_data: String,
    session_ttl_seconds: u64,
) -> Result<Vec<Box<dyn FnOnce(&TenantApplicationState) -> TenantApplicationState>>, TransitionError>
{
    let user_id = user_id.into();
    let session_id = format!("session_{}_{}", user_id, Utc::now().timestamp());

    if user_id.trim().is_empty() {
        return Err(TransitionError::InvalidParameters {
            message: "User ID cannot be empty".to_string(),
        });
    }

    let transitions: Vec<Box<dyn FnOnce(&TenantApplicationState) -> TenantApplicationState>> = vec![
        // Clean expired sessions
        Box::new(clean_expired_cache()),
        // Create new session
        Box::new(create_user_session(
            session_id.clone(),
            session_data,
            session_ttl_seconds,
        )),
        // Update user's last login timestamp in app data
        Box::new(
            transform_app_data(
                format!("user_{}_last_login", user_id),
                |_| Ok(JsonValue::String(Utc::now().to_rfc3339())),
                JsonValue::Null,
            )
            .map_err(|_| TransitionError::InvalidParameters {
                message: "Failed to create last login update".to_string(),
            })?,
        ),
    ];

    Ok(transitions)
}

/// Builds a logout transition sequence that removes a session and purges expired query cache.
///
/// The returned transitions, when applied in order, first remove the specified user session and
/// then remove expired entries from the query cache. Returns `TransitionError::InvalidParameters`
/// if `session_id` is empty or contains only whitespace.
///
/// # Examples
///
/// ```
/// let transitions = build_logout_transitions("session-123").unwrap();
/// assert_eq!(transitions.len(), 2);
/// ```
pub fn build_logout_transitions(
    session_id: impl Into<String>,
) -> Result<Vec<Box<dyn FnOnce(&TenantApplicationState) -> TenantApplicationState>>, TransitionError>
{
    let session_id = session_id.into();

    if session_id.trim().is_empty() {
        return Err(TransitionError::InvalidParameters {
            message: "Session ID cannot be empty".to_string(),
        });
    }

    let transitions: Vec<Box<dyn FnOnce(&TenantApplicationState) -> TenantApplicationState>> = vec![
        // Remove the session
        Box::new(remove_user_session(session_id)),
        // Clean expired cache
        Box::new(clean_expired_cache()),
    ];

    Ok(transitions)
}

/// Builds a sequence of transitions that atomically apply multiple app configuration updates.
///
/// Each returned transition, when applied to a `TenantApplicationState`, sets the corresponding
/// key in `app_data` to the provided JSON value and updates `last_updated`.
///
/// # Errors
///
/// Returns `Err(TransitionError::InvalidParameters)` if any configuration key is empty.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use serde_json::json;
///
/// let mut updates = HashMap::new();
/// updates.insert("theme".to_string(), json!("dark"));
/// updates.insert("items_per_page".to_string(), json!(20));
///
/// let transitions = build_config_updates(updates).expect("valid config updates");
/// // transitions contains one boxed transition per entry in the map
/// assert_eq!(transitions.len(), 2);
/// ```
pub fn build_config_updates(
    config_updates: HashMap<String, JsonValue>,
) -> Result<
    Vec<Box<dyn FnOnce(&TenantApplicationState) -> TenantApplicationState + Send + Sync>>,
    TransitionError,
> {
    let mut transitions = Vec::new();

    for (key, value) in config_updates {
        // Use set_app_config to centralize validation and state mutation
        let transition_fn = set_app_config(key, value, None::<fn(&JsonValue) -> bool>)?;

        // Box the transition function to match the return type
        let boxed_transition: Box<
            dyn FnOnce(&TenantApplicationState) -> TenantApplicationState + Send + Sync,
        > = Box::new(transition_fn);

        transitions.push(boxed_transition);
    }

    Ok(transitions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::functional::immutable_state::ImmutableStateManager;
    use crate::models::tenant::Tenant;
    use std::collections::HashMap;

    /// Creates a Tenant populated with deterministic test values for use in unit tests.
    ///
    /// The returned Tenant uses fixed identifiers and local test database URL and sets
    /// `created_at` and `updated_at` to the current UTC timestamp.
    ///
    /// # Examples
    ///
    /// ```
    /// let t = create_test_tenant();
    /// assert_eq!(t.id, "test_tenant");
    /// assert!(t.name.contains("Test"));
    /// ```
    fn create_test_tenant() -> Tenant {
        Tenant {
            id: "test_tenant".to_string(),
            name: "Test Tenant".to_string(),
            db_url: "postgres://test:test@localhost/test".to_string(),
            created_at: Some(chrono::Utc::now().naive_utc()),
            updated_at: Some(chrono::Utc::now().naive_utc()),
        }
    }

    #[test]
    fn test_create_user_session_transition() {
        let manager = ImmutableStateManager::new(100);
        let tenant = create_test_tenant();
        manager.initialize_tenant(tenant).unwrap();

        // Apply create session transition
        let create_fn = create_user_session(
            "session123".to_string(),
            "user_data_here".to_string(),
            3600, // 1 hour TTL
        );

        manager
            .apply_transition("test_tenant", |state| Ok(create_fn(state)))
            .unwrap();

        let state = manager.get_tenant_state("test_tenant").unwrap();
        let session = state.user_sessions.get(&"session123".to_string());
        assert!(session.is_some());
        assert_eq!(session.unwrap().user_data, "user_data_here");
    }

    #[test]
    fn test_update_user_session_transition() {
        let manager = ImmutableStateManager::new(100);
        let tenant = create_test_tenant();
        manager.initialize_tenant(tenant).unwrap();

        // First create a session
        let create_fn = create_user_session("session123".to_string(), "old_data".to_string(), 3600);
        manager
            .apply_transition("test_tenant", |state| Ok(create_fn(state)))
            .unwrap();

        // Then update it
        let update_fn = update_user_session("session123", "new_data".to_string(), None).unwrap();

        manager
            .apply_transition("test_tenant", |state| Ok(update_fn(state)))
            .unwrap();

        let state = manager.get_tenant_state("test_tenant").unwrap();
        let session = state.user_sessions.get(&"session123".to_string());
        assert!(session.is_some());
        assert_eq!(session.unwrap().user_data, "new_data");
    }

    #[test]
    fn test_set_app_config_transition() {
        let manager = ImmutableStateManager::new(100);
        let tenant = create_test_tenant();
        manager.initialize_tenant(tenant).unwrap();

        // Set some configuration
        let config_fn = set_app_config(
            "app.theme",
            serde_json::json!("dark"),
            None::<fn(&serde_json::Value) -> bool>,
        )
        .unwrap();

        manager
            .apply_transition("test_tenant", |state| Ok(config_fn(state)))
            .unwrap();

        let state = manager.get_tenant_state("test_tenant").unwrap();
        assert_eq!(
            state.app_data.get(&"app.theme".to_string()),
            Some(&serde_json::json!("dark"))
        );
    }

    #[test]
    fn test_composite_login_transitions() {
        let manager = ImmutableStateManager::new(100);
        let tenant = create_test_tenant();
        manager.initialize_tenant(tenant).unwrap();

        // Build login transitions
        let transitions = build_login_transitions(
            "user123",
            "session_data".to_string(),
            1800, // 30 minutes
        )
        .unwrap();

        // Apply all transitions atomically
        manager
            .apply_transitions("test_tenant", transitions)
            .unwrap();

        let state = manager.get_tenant_state("test_tenant").unwrap();

        // Check that a session was created (session ID contains timestamp, so we check it exists)
        assert!(!state.user_sessions.is_empty());

        // Check that user last login was recorded
        assert!(state
            .app_data
            .get(&"user_user123_last_login".to_string())
            .is_some());
    }

    #[test]
    fn test_logout_transitions() {
        let manager = ImmutableStateManager::new(100);
        let tenant = create_test_tenant();
        manager.initialize_tenant(tenant).unwrap();

        // Login first
        let login_transitions =
            build_login_transitions("user123", "session_data".to_string(), 1800).unwrap();
        manager
            .apply_transitions("test_tenant", login_transitions)
            .unwrap();

        // Capture the session ID (this is a bit hacky for testing)
        let temp_state = manager.get_tenant_state("test_tenant").unwrap();
        let session_id = temp_state.user_sessions.iter().next().unwrap().0.clone();

        // Logout
        let logout_transitions = build_logout_transitions(&session_id).unwrap();
        manager
            .apply_transitions("test_tenant", logout_transitions)
            .unwrap();

        let final_state = manager.get_tenant_state("test_tenant").unwrap();

        // Session should be removed
        assert!(final_state.user_sessions.get(&session_id).is_none());
    }

    #[test]
    fn test_config_updates_transitions() {
        let manager = ImmutableStateManager::new(100);
        let tenant = create_test_tenant();
        manager.initialize_tenant(tenant).unwrap();

        // Prepare multiple config updates
        let mut updates = HashMap::new();
        updates.insert("app.theme".to_string(), serde_json::json!("dark"));
        updates.insert("app.language".to_string(), serde_json::json!("en"));
        updates.insert("app.debug".to_string(), serde_json::json!(true));

        // Build config update transitions
        let transitions = build_config_updates(updates).unwrap();

        // Apply all config updates atomically
        manager
            .apply_transitions("test_tenant", transitions)
            .unwrap();

        let state = manager.get_tenant_state("test_tenant").unwrap();

        // Verify all config values were set
        assert_eq!(
            state.app_data.get(&"app.theme".to_string()),
            Some(&serde_json::json!("dark"))
        );
        assert_eq!(
            state.app_data.get(&"app.language".to_string()),
            Some(&serde_json::json!("en"))
        );
        assert_eq!(
            state.app_data.get(&"app.debug".to_string()),
            Some(&serde_json::json!(true))
        );
    }

    #[test]
    fn test_validation_errors() {
        // Test empty session ID validation
        assert!(build_logout_transitions("").is_err());

        // Test empty config key validation
        assert!(set_app_config(
            "",
            serde_json::json!("value"),
            None::<fn(&serde_json::Value) -> bool>
        )
        .is_err());
    }
}

// ==================== Snapshot-Aware Transition Builders ====================

/// Creates a transition context with metadata for audit trails
pub fn create_transition_context(user_id: Option<String>) -> TransitionContext {
    TransitionContext {
        user_id,
        timestamp: Utc::now(),
        metadata: HashMap::new(),
    }
}

/// Builds a batch of transitions with automatic rollback points
///
/// This creates named snapshots at strategic points in a multi-step operation,
/// allowing fine-grained rollback if later steps fail.
pub struct TransactionBuilder {
    transitions: Vec<Box<dyn FnOnce(&TenantApplicationState) -> TenantApplicationState>>,
    checkpoint_names: Vec<Option<String>>,
}

impl TransactionBuilder {
    /// Creates a new transaction builder
    pub fn new() -> Self {
        Self {
            transitions: Vec::new(),
            checkpoint_names: Vec::new(),
        }
    }

    /// Adds a transition to the transaction
    pub fn add_transition<F>(mut self, transition: F) -> Self
    where
        F: FnOnce(&TenantApplicationState) -> TenantApplicationState + 'static,
    {
        self.transitions.push(Box::new(transition));
        self.checkpoint_names.push(None);
        self
    }

    /// Adds a transition with a named checkpoint for rollback
    pub fn add_transition_with_checkpoint<F>(mut self, name: String, transition: F) -> Self
    where
        F: FnOnce(&TenantApplicationState) -> TenantApplicationState + 'static,
    {
        self.transitions.push(Box::new(transition));
        self.checkpoint_names.push(Some(name));
        self
    }

    /// Returns the transitions and checkpoint names
    pub fn build(
        self,
    ) -> (
        Vec<Box<dyn FnOnce(&TenantApplicationState) -> TenantApplicationState>>,
        Vec<Option<String>>,
    ) {
        (self.transitions, self.checkpoint_names)
    }
}

impl Default for TransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Creates a complex multi-step user onboarding transaction with checkpoints
///
/// This demonstrates how to build complex state transitions with automatic
/// rollback points for debugging and recovery.
pub fn build_user_onboarding_transaction(
    user_id: String,
    session_ttl: u64,
    initial_config: HashMap<String, JsonValue>,
) -> TransactionBuilder {
    let mut builder = TransactionBuilder::new();

    // Step 1: Create user session (checkpoint: "session_created")
    let session_id = format!("session_{}", user_id);
    builder = builder.add_transition_with_checkpoint(
        "session_created".to_string(),
        create_user_session(session_id.clone(), user_id.clone(), session_ttl),
    );

    // Step 2: Set initial configuration (checkpoint: "config_initialized")
    for (key, value) in initial_config {
        builder = builder.add_transition_with_checkpoint(format!("config_{}", key), move |state| {
            let mut new_state = state.clone();
            new_state.app_data = state.app_data.insert(key, value);
            new_state.last_updated = Utc::now();
            new_state
        });
    }

    // Step 3: Mark onboarding complete
    builder =
        builder.add_transition_with_checkpoint("onboarding_complete".to_string(), move |state| {
            let mut new_state = state.clone();
            new_state.app_data = state
                .app_data
                .insert(format!("user_{}_onboarded", user_id), JsonValue::Bool(true));
            new_state.last_updated = Utc::now();
            new_state
        });

    builder
}

/// Creates a state diff transition that compares two states
///
/// This is useful for debugging and understanding what changed between snapshots
pub fn create_state_diff_summary(
    old_state: &TenantApplicationState,
    new_state: &TenantApplicationState,
) -> HashMap<String, String> {
    let mut diff = HashMap::new();

    // Compare session counts
    let old_sessions = old_state.user_sessions.iter().count();
    let new_sessions = new_state.user_sessions.iter().count();
    if old_sessions != new_sessions {
        diff.insert(
            "sessions".to_string(),
            format!("{} -> {}", old_sessions, new_sessions),
        );
    }

    // Compare app data keys
    let old_keys: std::collections::HashSet<_> =
        old_state.app_data.iter().map(|(k, _)| k).collect();
    let new_keys: std::collections::HashSet<_> =
        new_state.app_data.iter().map(|(k, _)| k).collect();

    let added_keys: Vec<_> = new_keys.difference(&old_keys).collect();
    let removed_keys: Vec<_> = old_keys.difference(&new_keys).collect();

    if !added_keys.is_empty() {
        diff.insert("added_keys".to_string(), format!("{:?}", added_keys));
    }
    if !removed_keys.is_empty() {
        diff.insert("removed_keys".to_string(), format!("{:?}", removed_keys));
    }

    // Compare cache sizes
    let old_cache = old_state.query_cache.iter().count();
    let new_cache = new_state.query_cache.iter().count();
    if old_cache != new_cache {
        diff.insert(
            "cache_entries".to_string(),
            format!("{} -> {}", old_cache, new_cache),
        );
    }

    diff
}

/// Creates a transition that cleans up expired sessions
///
/// This is useful for maintenance operations and can be combined with snapshots
/// to ensure you can rollback if cleanup goes wrong
pub fn cleanup_expired_sessions() -> impl FnOnce(&TenantApplicationState) -> TenantApplicationState
{
    move |state| {
        let mut new_state = state.clone();
        let now = Utc::now();

        // Filter out expired sessions
        let mut updated_sessions = state.user_sessions.clone();
        for (session_id, session_data) in state.user_sessions.iter() {
            if session_data.expires_at < now {
                updated_sessions = updated_sessions.remove(session_id);
            }
        }

        new_state.user_sessions = updated_sessions;
        new_state.last_updated = now;
        new_state
    }
}

/// Creates a transition that prunes old cache entries
pub fn prune_cache(
    max_entries: usize,
) -> impl FnOnce(&TenantApplicationState) -> TenantApplicationState {
    move |state| {
        let mut new_state = state.clone();
        let cache_len = state.query_cache.iter().count();

        if cache_len > max_entries {
            // Keep only the most recent entries (assuming they're ordered)
            let to_skip = cache_len - max_entries;
            let mut new_cache = PersistentVector::new();

            for (idx, entry) in state.query_cache.iter().enumerate() {
                if idx >= to_skip {
                    new_cache = new_cache.append(entry.clone());
                }
            }

            new_state.query_cache = new_cache;
        }

        new_state.last_updated = Utc::now();
        new_state
    }
}
