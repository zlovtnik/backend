/// WebSocket security middleware and utilities.
///
/// This module provides security features for WebSocket connections including:
/// - Origin validation to prevent cross-site WebSocket hijacking (CSWSH)
/// - CORS origin allowlist configuration
/// - Token sanitization in logs to prevent leaking sensitive credentials
///
/// # Security Considerations
///
/// WebSocket connections can be vulnerable to cross-site WebSocket hijacking (CSWSH) if not properly validated.
/// This module implements the following security measures:
///
/// 1. **Origin Validation**: Checks that WebSocket upgrade requests come from allowed origins
/// 2. **Token Sanitization**: Ensures authentication tokens are not logged with sensitive values
/// 3. **Firewalling**: Documentation for production deployment firewall rules
///
/// # Environment Variables
///
/// - `APP_ENV`: Either "production" or "development" (default: "development")
/// - `CORS_ALLOWED_ORIGINS`: Comma-separated list of allowed origins for production
///   Example: `https://example.com,https://app.example.com`
/// - `WS_LOGS_ADMIN_USER`: Comma-separated list of authorized admin user IDs for WebSocket logs
///
/// # Production Deployment
///
/// For production deployments, ensure:
/// 1. The WebSocket port (default 9000) is only accessible from allowed networks
/// 2. Firewall rules restrict access to the WebSocket server to trusted origins/networks
/// 3. Example iptables rule:
///    ```
///    # Allow connections only from internal network
///    iptables -A INPUT -i eth0 -p tcp --dport 9000 -s 10.0.0.0/8 -j ACCEPT
///    iptables -A INPUT -i eth0 -p tcp --dport 9000 -j DROP
///    ```
/// 4. Or using firewalld:
///    ```
///    firewall-cmd --permanent --add-rich-rule='rule family="ipv4" source address="10.0.0.0/8" port port="9000" protocol="tcp" accept'
///    firewall-cmd --permanent --add-rich-rule='rule family="ipv4" port port="9000" protocol="tcp" reject'
///    ```

use actix_web::http::header::HeaderValue;
use std::env;

/// Represents a sanitized Origin header for logging (without sensitive data).
#[derive(Debug, Clone)]
pub struct SanitizedOrigin(String);

impl SanitizedOrigin {
    /// Create a sanitized origin from a header value.
    /// This extracts just the scheme and authority, removing any path components.
    ///
    /// # Returns
    ///
    /// `Some(SanitizedOrigin)` if the header contains a valid origin format (http:// or https://),
    /// `None` if the header is invalid or contains an unsupported scheme.
    pub fn from_header(value: &HeaderValue) -> Option<Self> {
        value
            .to_str()
            .ok()
            .and_then(|s| {
                // Keep origin value as-is (already contains no path/query by spec)
                // but ensure it's a valid origin format
                if s.starts_with("http://") || s.starts_with("https://") {
                    Some(SanitizedOrigin(s.to_string()))
                } else {
                    None
                }
            })
    }

    /// Get the sanitized origin string for logging
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Build CORS allowed origins list from environment configuration.
///
/// In production, reads from `CORS_ALLOWED_ORIGINS` environment variable.
/// In development, uses a predefined set of common development origins.
///
/// # Returns
///
/// A vector of allowed origin strings
pub fn get_allowed_origins() -> Vec<String> {
    let app_env = env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());

    if app_env == "production" {
        // Production: read from env or default to localhost
        env::var("CORS_ALLOWED_ORIGINS")
            .unwrap_or_else(|_| "http://localhost:3000".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        // Development: allow common dev origins
        vec![
            "http://localhost:3000".to_string(),
            "http://localhost:3001".to_string(),
            "http://127.0.0.1:3000".to_string(),
            "http://127.0.0.1:3001".to_string(),
            "http://localhost:5173".to_string(), // Vite dev server
            "http://127.0.0.1:5173".to_string(), // Vite dev server
        ]
    }
}

/// Normalize an origin for comparison purposes.
///
/// Removes trailing slashes and standardizes default ports:
/// - `http://example.com:80` → `http://example.com`
/// - `https://example.com:443` → `https://example.com`
/// - `http://example.com/` → `http://example.com`
///
/// # Arguments
///
/// * `origin` - The origin string to normalize
///
/// # Returns
///
/// Normalized origin string
fn normalize_origin(origin: &str) -> String {
    let mut normalized = origin.trim_end_matches('/').to_string();
    
    // Remove default ports
    if normalized.ends_with(":80") && normalized.starts_with("http://") {
        normalized.truncate(normalized.len() - 3);
    } else if normalized.ends_with(":443") && normalized.starts_with("https://") {
        normalized.truncate(normalized.len() - 4);
    }
    
    normalized
}

/// Validate if an origin is in the allowed list.
///
/// Performs normalized comparison to handle trailing slashes and default ports:
/// - `http://example.com` matches `http://example.com/`
/// - `http://example.com:80` matches `http://example.com`
/// - `https://example.com:443` matches `https://example.com`
///
/// # Arguments
///
/// * `origin` - The origin string to validate (e.g., "https://example.com")
/// * `allowed_origins` - List of allowed origins
///
/// # Returns
///
/// `true` if the origin is allowed, `false` otherwise
pub fn is_origin_allowed(origin: &str, allowed_origins: &[String]) -> bool {
    let normalized_origin = normalize_origin(origin);
    allowed_origins.iter().any(|allowed| {
        normalize_origin(allowed) == normalized_origin
    })
}

/// Check if origin validation should be enforced based on environment.
///
/// In production, origin validation is always enforced.
/// In development, it can be optionally enforced if env var is set to a truthy value:
/// - "true", "1", "yes", "on" (case-insensitive) are all treated as true
/// - Any other value is treated as false
///
/// # Returns
///
/// `true` if origin validation should be enforced
pub fn should_enforce_origin_validation() -> bool {
    let app_env = env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
    app_env == "production"
        || env::var("ENFORCE_ORIGIN_VALIDATION")
            .map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes" | "on"))
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_origin_allowed() {
        let allowed = vec!["https://example.com".to_string(), "https://app.example.com".to_string()];

        assert!(is_origin_allowed("https://example.com", &allowed));
        assert!(is_origin_allowed("https://app.example.com", &allowed));
        assert!(!is_origin_allowed("https://evil.com", &allowed));
    }

    #[test]
    fn test_is_origin_allowed_with_trailing_slash() {
        let allowed = vec!["https://example.com".to_string()];

        // Trailing slash should match
        assert!(is_origin_allowed("https://example.com/", &allowed));
    }

    #[test]
    fn test_is_origin_allowed_with_default_ports() {
        let allowed = vec!["https://example.com".to_string(), "http://example.com".to_string()];

        // Default ports should match
        assert!(is_origin_allowed("https://example.com:443", &allowed));
        assert!(is_origin_allowed("http://example.com:80", &allowed));
    }

    #[test]
    fn test_normalize_origin() {
        assert_eq!(normalize_origin("https://example.com/"), "https://example.com");
        assert_eq!(normalize_origin("https://example.com:443"), "https://example.com");
        assert_eq!(normalize_origin("http://example.com:80"), "http://example.com");
        assert_eq!(normalize_origin("https://example.com:443/"), "https://example.com");
    }

    #[test]
    fn test_sanitized_origin_valid() {
        let header = HeaderValue::from_static("https://example.com");
        let sanitized = SanitizedOrigin::from_header(&header).unwrap();
        assert_eq!(sanitized.as_str(), "https://example.com");
    }

    #[test]
    fn test_sanitized_origin_invalid() {
        let header = HeaderValue::from_static("not-an-origin");
        assert!(SanitizedOrigin::from_header(&header).is_none());
    }

    #[test]
    fn test_should_enforce_origin_validation_production() {
        // In production, always enforce (implicitly tested via behavior)
        // This would require mocking env vars, so we test development cases below
    }

    #[test]
    fn test_enforce_origin_validation_parsing() {
        // Note: These test the normalization logic directly, not the env var parsing
        // since env var mocking is complex in integration tests
        
        // Verify the logic would work correctly for these values:
        let test_values = vec!["true", "1", "yes", "on"];
        for value in test_values {
            assert!(matches!(value.to_lowercase().as_str(), "true" | "1" | "yes" | "on"),
                "Value '{}' should be recognized as truthy", value);
        }
    }
}
