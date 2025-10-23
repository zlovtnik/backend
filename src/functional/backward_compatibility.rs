//! Backward Compatibility Validation
//!
//! This module provides comprehensive validation of backward compatibility
//! for the Actix Web REST API with multi-tenancy and JWT authentication.
//! It ensures that functional programming enhancements do not break existing
//! API contracts, authentication flows, or data isolation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for backward compatibility tests
#[derive(Debug, Clone)]
pub struct CompatibilityTestConfig {
    /// Test tenant ID for multi-tenant validation
    pub test_tenant_id: String,
    /// Test username for authentication validation
    pub test_username: String,
    /// Test password for authentication validation
    pub test_password: String,
    /// JWT secret for token validation
    pub jwt_secret: String,
    /// Base URL for API endpoint testing
    pub base_url: String,
    /// Performance baseline thresholds (endpoint -> max_ms)
    pub performance_baselines: HashMap<String, u64>,
}

impl Default for CompatibilityTestConfig {
    fn default() -> Self {
        let mut performance_baselines = HashMap::new();
        performance_baselines.insert("/api/ping".to_string(), 50);
        performance_baselines.insert("/api/auth/login".to_string(), 200);
        performance_baselines.insert("/api/auth/signup".to_string(), 300);
        performance_baselines.insert("/api/auth/me".to_string(), 100);
        performance_baselines.insert("/api/auth/refresh".to_string(), 150);
        performance_baselines.insert("/api/address-book".to_string(), 200);
        performance_baselines.insert("/api/health/status".to_string(), 100);

        Self {
            test_tenant_id: "tenant1".to_string(),
            test_username: "testuser".to_string(),
            test_password: "testpass123".to_string(),
            jwt_secret: "test_secret_key_for_compatibility_testing_only".to_string(),
            base_url: "http://localhost:8080".to_string(),
            performance_baselines,
        }
    }
}

/// Overall compatibility status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompatibilityStatus {
    /// All tests pass, fully backward compatible
    FullyCompatible,
    /// Some issues but mostly compatible
    PartiallyCompatible,
    /// Major compatibility issues
    Incompatible,
}

/// Performance regression data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceRegression {
    /// Endpoint that regressed
    pub endpoint: String,
    /// Expected maximum response time in milliseconds
    pub expected_max_ms: u64,
    /// Actual response time in milliseconds
    pub actual_ms: u64,
    /// Regression percentage
    pub regression_percentage: f64,
}

/// Results of compatibility test suite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityTestResults {
    /// API endpoint test results
    pub api_endpoints_passed: u32,
    pub api_endpoints_failed: u32,
    /// Authentication test results
    pub auth_tests_passed: u32,
    pub auth_tests_failed: u32,
    /// Tenant isolation test results
    pub tenant_isolation_passed: u32,
    pub tenant_isolation_failed: u32,
    /// Database operation test results
    pub database_tests_passed: u32,
    pub database_tests_failed: u32,
    /// Frontend compatibility test results
    pub frontend_compatibility_passed: u32,
    pub frontend_compatibility_failed: u32,
    /// Overall compatibility status
    pub overall_compatibility: CompatibilityStatus,
    /// List of failed test descriptions
    pub failed_tests: Vec<String>,
    /// Performance regressions detected
    pub performance_regressions: Vec<PerformanceRegression>,
}

impl Default for CompatibilityTestResults {
    fn default() -> Self {
        Self {
            api_endpoints_passed: 0,
            api_endpoints_failed: 0,
            auth_tests_passed: 0,
            auth_tests_failed: 0,
            tenant_isolation_passed: 0,
            tenant_isolation_failed: 0,
            database_tests_passed: 0,
            database_tests_failed: 0,
            frontend_compatibility_passed: 0,
            frontend_compatibility_failed: 0,
            overall_compatibility: CompatibilityStatus::FullyCompatible,
            failed_tests: Vec::new(),
            performance_regressions: Vec::new(),
        }
    }
}

/// Backward compatibility validator
pub struct BackwardCompatibilityValidator {
    config: CompatibilityTestConfig,
}

impl BackwardCompatibilityValidator {
    /// Create a new validator with the given configuration
    pub fn new(config: CompatibilityTestConfig) -> Self {
        Self { config }
    }

    /// Run the full compatibility test suite
    pub async fn run_full_compatibility_suite(&self) -> CompatibilityTestResults {
        let mut results = CompatibilityTestResults::default();

        // Test API endpoints
        match self.test_api_endpoints().await {
            Ok(_) => results.api_endpoints_passed += 5,
            Err(e) => {
                results.api_endpoints_failed += 5;
                results.failed_tests.push(format!("API endpoints: {}", e));
            }
        }

        // Test JWT authentication
        match self.test_jwt_authentication().await {
            Ok(_) => results.auth_tests_passed += 3,
            Err(e) => {
                results.auth_tests_failed += 3;
                results
                    .failed_tests
                    .push(format!("JWT authentication: {}", e));
            }
        }

        // Test multi-tenant isolation
        match self.test_multi_tenant_isolation().await {
            Ok(_) => results.tenant_isolation_passed += 2,
            Err(e) => {
                results.tenant_isolation_failed += 2;
                results
                    .failed_tests
                    .push(format!("Multi-tenant isolation: {}", e));
            }
        }

        // Test database operations
        match self.test_database_operations().await {
            Ok(_) => results.database_tests_passed += 3,
            Err(e) => {
                results.database_tests_failed += 3;
                results
                    .failed_tests
                    .push(format!("Database operations: {}", e));
            }
        }

        // Test frontend integration
        match self.test_frontend_integration().await {
            Ok(_) => results.frontend_compatibility_passed += 3,
            Err(e) => {
                results.frontend_compatibility_failed += 3;
                results
                    .failed_tests
                    .push(format!("Frontend integration: {}", e));
            }
        }

        // Test performance regression
        match self.test_performance_regression().await {
            Ok(regressions) => results.performance_regressions = regressions,
            Err(e) => {
                results
                    .failed_tests
                    .push(format!("Performance regression: {}", e));
            }
        }

        // Calculate overall status
        results.overall_compatibility = self.calculate_overall_status(&results);

        results
    }

    /// Test all existing API endpoints for backward compatibility
    pub async fn test_api_endpoints(&self) -> Result<(), String> {
        use awc::Client;

        let client = Client::default();

        // Test /api/ping endpoint
        let ping_url = format!("{}/api/ping", self.config.base_url);
        let mut response = client
            .get(&ping_url)
            .send()
            .await
            .map_err(|e| format!("Failed to reach /api/ping: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("/api/ping returned status: {}", response.status()));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse /api/ping response: {}", e))?;

        if !body.get("message").is_some() {
            return Err("/api/ping response missing 'message' field".to_string());
        }

        // Test /api/health/status endpoint
        let health_url = format!("{}/api/health/status", self.config.base_url);
        let mut response = client
            .get(&health_url)
            .send()
            .await
            .map_err(|e| format!("Failed to reach /api/health/status: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "/api/health/status returned status: {}",
                response.status()
            ));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse /api/health/status response: {}", e))?;

        if !body.get("status").is_some() {
            return Err("/api/health/status response missing 'status' field".to_string());
        }

        // Test signup endpoint structure (without actually creating user)
        let signup_url = format!("{}/api/auth/signup", self.config.base_url);
        let signup_payload = serde_json::json!({
            "username": "testuser",
            "email": "test@example.com",
            "password": "testpass123",
            "tenant_id": self.config.test_tenant_id
        });

        let mut response = client
            .post(&signup_url)
            .insert_header(("Content-Type", "application/json"))
            .send_json(&signup_payload)
            .await
            .map_err(|e| format!("Failed to reach /api/auth/signup: {}", e))?;

        // Should return success or validation error (not 404 or 500)
        if response.status().is_server_error() {
            return Err(format!(
                "/api/auth/signup returned server error: {}",
                response.status()
            ));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse /api/auth/signup response: {}", e))?;

        if !body.get("message").is_some() {
            return Err("/api/auth/signup response missing 'message' field".to_string());
        }

        // Test login endpoint structure
        let login_url = format!("{}/api/auth/login", self.config.base_url);
        let login_payload = serde_json::json!({
            "username": "testuser",
            "password": "testpass123",
            "tenant_id": self.config.test_tenant_id
        });

        let mut response = client
            .post(&login_url)
            .insert_header(("Content-Type", "application/json"))
            .send_json(&login_payload)
            .await
            .map_err(|e| format!("Failed to reach /api/auth/login: {}", e))?;

        if response.status().is_server_error() {
            return Err(format!(
                "/api/auth/login returned server error: {}",
                response.status()
            ));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse /api/auth/login response: {}", e))?;

        if !body.get("message").is_some() {
            return Err("/api/auth/login response missing 'message' field".to_string());
        }

        Ok(())
    }

    /// Test JWT authentication flow for backward compatibility
    pub async fn test_jwt_authentication(&self) -> Result<(), String> {
        use awc::Client;

        let client = Client::default();

        // First, try to signup a test user
        let signup_url = format!("{}/api/auth/signup", self.config.base_url);
        let signup_payload = serde_json::json!({
            "username": self.config.test_username,
            "email": format!("{}@test.com", self.config.test_username),
            "password": self.config.test_password,
            "tenant_id": self.config.test_tenant_id
        });

        let response = client
            .post(&signup_url)
            .insert_header(("Content-Type", "application/json"))
            .send_json(&signup_payload)
            .await
            .map_err(|e| format!("Failed to signup test user: {}", e))?;

        // Signup should succeed or user already exists (not a server error)
        if response.status().is_server_error() {
            return Err(format!(
                "Signup returned server error: {}",
                response.status()
            ));
        }

        // Now try to login
        let login_url = format!("{}/api/auth/login", self.config.base_url);
        let login_payload = serde_json::json!({
            "username": self.config.test_username,
            "password": self.config.test_password,
            "tenant_id": self.config.test_tenant_id
        });

        let mut response = client
            .post(&login_url)
            .insert_header(("Content-Type", "application/json"))
            .send_json(&login_payload)
            .await
            .map_err(|e| format!("Failed to login: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Login failed with status: {}", response.status()));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse login response: {}", e))?;

        let token = body
            .get("data")
            .and_then(|d| d.get("token"))
            .and_then(|t| t.as_str())
            .ok_or_else(|| "Login response missing token".to_string())?;

        // Test authenticated endpoint with the token
        let me_url = format!("{}/api/auth/me", self.config.base_url);
        let mut response = client
            .get(&me_url)
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .insert_header(("x-tenant-id", self.config.test_tenant_id.clone()))
            .send()
            .await
            .map_err(|e| format!("Failed to access /api/auth/me: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "/api/auth/me failed with status: {}",
                response.status()
            ));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse /api/auth/me response: {}", e))?;

        if !body.get("data").and_then(|d| d.get("username")).is_some() {
            return Err("/api/auth/me response missing user data".to_string());
        }

        // Test token refresh
        let refresh_url = format!("{}/api/auth/refresh", self.config.base_url);
        let response = client
            .post(&refresh_url)
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .insert_header(("x-tenant-id", self.config.test_tenant_id.clone()))
            .send()
            .await
            .map_err(|e| format!("Failed to refresh token: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Token refresh failed with status: {}",
                response.status()
            ));
        }

        Ok(())
    }

    /// Verifies that users and authentication tokens are isolated between tenants by creating test users in two tenants, ensuring their tokens differ, and confirming that tenant-scoped requests succeed for the correct tenant and are rejected for cross-tenant access.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Run inside an async test or runtime
    /// let validator = BackwardCompatibilityValidator::new(config);
    /// validator.test_multi_tenant_isolation().await.unwrap();
    /// ```
    pub async fn test_multi_tenant_isolation(&self) -> Result<(), String> {
        use awc::Client;

        let client = Client::default();

        // Create users in different tenants
        let tenant1 = "tenant1";
        let tenant2 = "tenant2";

        // Signup user in tenant1
        let signup_url = format!("{}/api/auth/signup", self.config.base_url);
        let signup_payload1 = serde_json::json!({
            "username": "tenant1user",
            "email": "tenant1user@test.com",
            "password": "testpass123",
            "tenant_id": tenant1
        });

        let response = client
            .post(&signup_url)
            .insert_header(("Content-Type", "application/json"))
            .send_json(&signup_payload1)
            .await
            .map_err(|e| format!("Failed to signup tenant1 user: {}", e))?;

        if response.status().is_server_error() {
            return Err(format!(
                "Tenant1 signup server error: {}",
                response.status()
            ));
        }

        // Signup user in tenant2
        let signup_payload2 = serde_json::json!({
            "username": "tenant2user",
            "email": "tenant2user@test.com",
            "password": "testpass123",
            "tenant_id": tenant2
        });

        let response = client
            .post(&signup_url)
            .insert_header(("Content-Type", "application/json"))
            .send_json(&signup_payload2)
            .await
            .map_err(|e| format!("Failed to signup tenant2 user: {}", e))?;

        if response.status().is_server_error() {
            return Err(format!(
                "Tenant2 signup server error: {}",
                response.status()
            ));
        }

        // Login with tenant1 user
        let login_url = format!("{}/api/auth/login", self.config.base_url);
        let login_payload1 = serde_json::json!({
            "username": "tenant1user",
            "password": "testpass123",
            "tenant_id": tenant1
        });

        let mut response = client
            .post(&login_url)
            .insert_header(("Content-Type", "application/json"))
            .send_json(&login_payload1)
            .await
            .map_err(|e| format!("Failed to login tenant1 user: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Tenant1 login failed: {}", response.status()));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse tenant1 login response: {}", e))?;

        let token1 = body
            .get("data")
            .and_then(|d| d.get("token"))
            .and_then(|t| t.as_str())
            .ok_or_else(|| "Tenant1 login response missing token".to_string())?;

        // Login with tenant2 user
        let login_payload2 = serde_json::json!({
            "username": "tenant2user",
            "password": "testpass123",
            "tenant_id": tenant2
        });

        let mut response = client
            .post(&login_url)
            .insert_header(("Content-Type", "application/json"))
            .send_json(&login_payload2)
            .await
            .map_err(|e| format!("Failed to login tenant2 user: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Tenant2 login failed: {}", response.status()));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse tenant2 login response: {}", e))?;

        let token2 = body
            .get("data")
            .and_then(|d| d.get("token"))
            .and_then(|t| t.as_str())
            .ok_or_else(|| "Tenant2 login response missing token".to_string())?;

        // Verify tenant1 user cannot access tenant2 data (if address book exists)
        // For now, just verify that tokens are different and tenant-specific
        if token1 == token2 {
            return Err("Tokens should be different for different tenants".to_string());
        }

        // Test that tenant1 token works with tenant1 header
        let me_url = format!("{}/api/auth/me", self.config.base_url);
        let response = client
            .get(&me_url)
            .insert_header(("Authorization", format!("Bearer {}", token1)))
            .insert_header(("x-tenant-id", tenant1))
            .send()
            .await
            .map_err(|e| format!("Failed to access tenant1 data: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Tenant1 data access failed: {}", response.status()));
        }

        // Test that tenant1 token fails with tenant2 header (should be unauthorized)
        let response = client
            .get(&me_url)
            .insert_header(("Authorization", format!("Bearer {}", token1)))
            .insert_header(("x-tenant-id", tenant2))
            .send()
            .await
            .map_err(|e| format!("Failed to test cross-tenant access: {}", e))?;

        // Should fail with unauthorized (401) or forbidden (403)
        if response.status().is_success() {
            return Err("Cross-tenant access should be blocked".to_string());
        }

        Ok(())
    }

    /// Verifies database-backed user flows and tenant-scoped data operations against the API.
    ///
    /// Performs a signup and login to create a unique test user, exercises tenant-scoped address-book
    /// endpoints (create and retrieve a contact when available), and logs out to allow backend session
    /// cleanup. Returns an error if any step encounters a server error, an unexpected status, or if
    /// required response fields (like an auth token) are missing.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tokio::runtime::Runtime;
    /// # use your_crate::{BackwardCompatibilityValidator, CompatibilityTestConfig};
    /// # let rt = Runtime::new().unwrap();
    /// rt.block_on(async {
    ///     let config = CompatibilityTestConfig::default();
    ///     let validator = BackwardCompatibilityValidator::new(config);
    ///     let result = validator.test_database_operations().await;
    ///     // This example assumes an available test server; in CI this would be an integration test.
    ///     let _ = result; // Inspect or assert in real tests
    /// });
    /// ```
    ///
    /// # Returns
    ///
    /// `Ok(())` if all tested database-related endpoints behave as expected; `Err(String)` with a
    /// descriptive message if any request fails, returns a server error status, or required response
    /// data (such as an authentication token) is missing.
    pub async fn test_database_operations(&self) -> Result<(), String> {
        use awc::Client;

        let client = Client::default();

        // Test user creation and retrieval
        let signup_url = format!("{}/api/auth/signup", self.config.base_url);
        let unique_username = format!("dbtest_{}", chrono::Utc::now().timestamp());
        let signup_payload = serde_json::json!({
            "username": unique_username,
            "email": format!("{}@test.com", unique_username),
            "password": "testpass123",
            "tenant_id": self.config.test_tenant_id
        });

        let response = client
            .post(&signup_url)
            .insert_header(("Content-Type", "application/json"))
            .send_json(&signup_payload)
            .await
            .map_err(|e| format!("Failed to create test user: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("User creation failed: {}", response.status()));
        }

        // Login to get token
        let login_url = format!("{}/api/auth/login", self.config.base_url);
        let login_payload = serde_json::json!({
            "username": unique_username,
            "password": "testpass123",
            "tenant_id": self.config.test_tenant_id
        });

        let mut response = client
            .post(&login_url)
            .insert_header(("Content-Type", "application/json"))
            .send_json(&login_payload)
            .await
            .map_err(|e| format!("Failed to login test user: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("User login failed: {}", response.status()));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse login response: {}", e))?;

        let token = body
            .get("data")
            .and_then(|d| d.get("token"))
            .and_then(|t| t.as_str())
            .ok_or_else(|| "Login response missing token".to_string())?;

        // Test address book operations if available
        let address_book_url = format!("{}/api/address-book", self.config.base_url);

        // Try to get address book (may be empty)
        let response = client
            .get(&address_book_url)
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .insert_header(("x-tenant-id", self.config.test_tenant_id.clone()))
            .send()
            .await
            .map_err(|e| format!("Failed to access address book: {}", e))?;

        if response.status().is_server_error() {
            return Err(format!(
                "Address book access server error: {}",
                response.status()
            ));
        }

        // If address book is available, test creating a contact
        if response.status().is_success() {
            let create_contact_payload = serde_json::json!({
                "name": "Test Contact",
                "email": "contact@test.com",
                "phone": "123-456-7890"
            });

            let response = client
                .post(&address_book_url)
                .insert_header(("Authorization", format!("Bearer {}", token)))
                .insert_header(("x-tenant-id", self.config.test_tenant_id.clone()))
                .insert_header(("Content-Type", "application/json"))
                .send_json(&create_contact_payload)
                .await
                .map_err(|e| format!("Failed to create contact: {}", e))?;

            if response.status().is_server_error() {
                return Err(format!(
                    "Contact creation server error: {}",
                    response.status()
                ));
            }

            // Test retrieval after creation
            let response = client
                .get(&address_book_url)
                .insert_header(("Authorization", format!("Bearer {}", token)))
                .insert_header(("x-tenant-id", self.config.test_tenant_id.clone()))
                .send()
                .await
                .map_err(|e| format!("Failed to retrieve contacts: {}", e))?;

            if !response.status().is_success() {
                return Err(format!("Contact retrieval failed: {}", response.status()));
            }
        }

        // Test logout (database session cleanup)
        let logout_url = format!("{}/api/auth/logout", self.config.base_url);
        let response = client
            .post(&logout_url)
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .insert_header(("x-tenant-id", self.config.test_tenant_id.clone()))
            .send()
            .await
            .map_err(|e| format!("Failed to logout: {}", e))?;

        if response.status().is_server_error() {
            return Err(format!("Logout server error: {}", response.status()));
        }

        Ok(())
    }

    /// Verifies frontend integration concerns: CORS behavior and standard JSON response formats.
    ///
    /// Performs a CORS preflight check on `/api/ping`, validates presence of CORS response headers
    /// on both the preflight and a subsequent GET, ensures the `/api/ping` response contains
    /// a `message` field and either `status` or `data`, and verifies that an invalid login returns
    /// a client error with a `message` field instead of a server error.
    ///
    /// # Examples
    ///
    /// ```
    /// # use your_crate::{BackwardCompatibilityValidator, CompatibilityTestConfig};
    /// # #[tokio::main]
    /// # async fn main() {
    /// let cfg = CompatibilityTestConfig::default();
    /// let validator = BackwardCompatibilityValidator::new(cfg);
    /// // This example requires the target API to be running and reachable at `base_url`.
    /// let _ = validator.test_frontend_integration().await;
    /// # }
    /// ```
    ///
    /// Returns `Ok(())` when all checks pass; returns `Err(String)` with a descriptive message on failure.
    pub async fn test_frontend_integration(&self) -> Result<(), String> {
        use awc::Client;

        let client = Client::default();

        // Test CORS preflight request
        let ping_url = format!("{}/api/ping", self.config.base_url);
        let response = client
            .request(awc::http::Method::OPTIONS, &ping_url)
            .insert_header(("Origin", "http://localhost:3000"))
            .insert_header(("Access-Control-Request-Method", "GET"))
            .insert_header(("Access-Control-Request-Headers", "x-tenant-id"))
            .send()
            .await
            .map_err(|e| format!("Failed CORS preflight: {}", e))?;

        if response.status() != awc::http::StatusCode::OK {
            return Err(format!("CORS preflight failed: {}", response.status()));
        }

        // Check CORS headers
        let cors_allow_origin = response.headers().get("access-control-allow-origin");
        let cors_allow_methods = response.headers().get("access-control-allow-methods");
        let cors_allow_headers = response.headers().get("access-control-allow-headers");

        if cors_allow_origin.is_none() {
            return Err("Missing CORS Access-Control-Allow-Origin header".to_string());
        }

        if cors_allow_methods.is_none() {
            return Err("Missing CORS Access-Control-Allow-Methods header".to_string());
        }

        if cors_allow_headers.is_none() {
            return Err("Missing CORS Access-Control-Allow-Headers header".to_string());
        }

        // Test actual request with CORS headers
        let mut response = client
            .get(&ping_url)
            .insert_header(("Origin", "http://localhost:3000"))
            .send()
            .await
            .map_err(|e| format!("Failed ping with CORS: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Ping with CORS failed: {}", response.status()));
        }

        // Verify CORS headers on actual response
        let cors_allow_origin = response.headers().get("access-control-allow-origin");
        if cors_allow_origin.is_none() {
            return Err("Missing CORS headers on ping response".to_string());
        }

        // Test response format consistency
        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse ping response: {}", e))?;

        // Check standard response structure
        if !body.get("message").is_some() {
            return Err("Ping response missing 'message' field".to_string());
        }

        if !body.get("status").is_some() && !body.get("data").is_some() {
            return Err("Ping response missing standard fields".to_string());
        }

        // Test error response format
        let invalid_login_url = format!("{}/api/auth/login", self.config.base_url);
        let invalid_payload = serde_json::json!({
            "username": "",
            "password": "",
            "tenant_id": ""
        });

        let mut response = client
            .post(&invalid_login_url)
            .insert_header(("Content-Type", "application/json"))
            .send_json(&invalid_payload)
            .await
            .map_err(|e| format!("Failed invalid login test: {}", e))?;

        // Should return client error, not server error
        if response.status().is_server_error() {
            return Err(format!(
                "Invalid login caused server error: {}",
                response.status()
            ));
        }

        // Check error response format
        if response.status().is_client_error() {
            let body: serde_json::Value = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse error response: {}", e))?;

            if !body.get("message").is_some() {
                return Err("Error response missing 'message' field".to_string());
            }
        }

        Ok(())
    }

    /// Measures configured endpoints against latency baselines and reports any regressions.
    ///
    /// Iterates over `self.config.performance_baselines`, performs a GET to each endpoint, and records a
    /// `PerformanceRegression` entry when an endpoint's observed latency exceeds its configured
    /// `expected_max_ms`. If a login token can be obtained via `test_login_flow`, the function also
    /// checks the authenticated endpoints `/api/auth/me` and `/api/auth/refresh` with their own
    /// thresholds. Returns an `Err` if any request fails to send or if an endpoint responds with a
    /// non-success status.
    ///
    /// # Returns
    ///
    /// A `Vec<PerformanceRegression>` containing one entry per endpoint that exceeded its baseline.
    ///
    /// # Examples
    ///
    /// ```
    /// # async fn example(mut validator: crate::BackwardCompatibilityValidator) {
    /// let regressions = validator.test_performance_regression().await.unwrap();
    /// // regressions contains any endpoints that violated their latency baselines
    /// # }
    /// ```
    pub async fn test_performance_regression(&self) -> Result<Vec<PerformanceRegression>, String> {
        use awc::Client;
        use std::time::Instant;

        let client = Client::default();
        let mut regressions = Vec::new();

        // Test each endpoint in the performance baselines
        for (endpoint, max_ms) in &self.config.performance_baselines {
            let url = format!("{}{}", self.config.base_url, endpoint);

            let start = Instant::now();
            let response = client
                .get(&url)
                .send()
                .await
                .map_err(|e| format!("Failed to test {}: {}", endpoint, e))?;

            let elapsed = start.elapsed();
            let actual_ms = elapsed.as_millis() as u64;

            if !response.status().is_success() {
                return Err(format!(
                    "{} returned error status: {}",
                    endpoint,
                    response.status()
                ));
            }

            if actual_ms > *max_ms {
                regressions.push(PerformanceRegression {
                    endpoint: endpoint.clone(),
                    expected_max_ms: *max_ms,
                    actual_ms,
                    regression_percentage: ((actual_ms as f64 - *max_ms as f64) / *max_ms as f64)
                        * 100.0,
                });
            }
        }

        // Test authenticated endpoints if we can get a token
        if let Ok(token) = self.test_login_flow().await {
            let auth_endpoints = vec![("/api/auth/me", 100u64), ("/api/auth/refresh", 150u64)];

            for (endpoint, max_ms) in auth_endpoints {
                let url = format!("{}{}", self.config.base_url, endpoint);

                let start = Instant::now();
                let response = client
                    .get(&url)
                    .insert_header(("Authorization", format!("Bearer {}", token)))
                    .insert_header(("x-tenant-id", self.config.test_tenant_id.clone()))
                    .send()
                    .await
                    .map_err(|e| format!("Failed to test {}: {}", endpoint, e))?;

                let elapsed = start.elapsed();
                let actual_ms = elapsed.as_millis() as u64;

                if !response.status().is_success() {
                    return Err(format!(
                        "{} returned error status: {}",
                        endpoint,
                        response.status()
                    ));
                }

                if actual_ms > max_ms {
                    regressions.push(PerformanceRegression {
                        endpoint: endpoint.to_string(),
                        expected_max_ms: max_ms,
                        actual_ms,
                        regression_percentage: ((actual_ms as f64 - max_ms as f64) / max_ms as f64)
                            * 100.0,
                    });
                }
            }
        }

        Ok(regressions)
    }

    /// Helper method to get a test token for authenticated requests
    pub async fn test_login_flow(&self) -> Result<String, String> {
        use awc::Client;

        let client = Client::default();

        // Try to login with test credentials
        let login_url = format!("{}/api/auth/login", self.config.base_url);
        let login_payload = serde_json::json!({
            "username": self.config.test_username,
            "password": self.config.test_password,
            "tenant_id": self.config.test_tenant_id
        });

        let mut response = client
            .post(&login_url)
            .insert_header(("Content-Type", "application/json"))
            .send_json(&login_payload)
            .await
            .map_err(|e| format!("Failed to get test token: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Test login failed: {}", response.status()));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse test login response: {}", e))?;

        body.get("data")
            .and_then(|d| d.get("token"))
            .and_then(|t| t.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| "Test login response missing token".to_string())
    }

    pub fn calculate_overall_status(
        &self,
        results: &CompatibilityTestResults,
    ) -> CompatibilityStatus {
        let total_tests = results.api_endpoints_passed
            + results.api_endpoints_failed
            + results.auth_tests_passed
            + results.auth_tests_failed
            + results.tenant_isolation_passed
            + results.tenant_isolation_failed
            + results.database_tests_passed
            + results.database_tests_failed
            + results.frontend_compatibility_passed
            + results.frontend_compatibility_failed;

        let total_passed = results.api_endpoints_passed
            + results.auth_tests_passed
            + results.tenant_isolation_passed
            + results.database_tests_passed
            + results.frontend_compatibility_passed;

        let pass_rate = if total_tests > 0 {
            (total_passed as f64) / (total_tests as f64)
        } else {
            0.0
        };

        if pass_rate >= 1.0 && results.performance_regressions.is_empty() {
            CompatibilityStatus::FullyCompatible
        } else if pass_rate >= 0.8 {
            CompatibilityStatus::PartiallyCompatible
        } else {
            CompatibilityStatus::Incompatible
        }
    }
}

/// Generate a compatibility test report
pub fn generate_compatibility_report(results: &CompatibilityTestResults) -> String {
    let mut report = String::new();

    report.push_str("# Backward Compatibility Validation Report\n\n");

    report.push_str(&format!(
        "**Overall Status:** {:?}\n\n",
        results.overall_compatibility
    ));

    report.push_str("## Test Results Summary\n\n");
    report.push_str(&format!(
        "- **API Endpoints:** {} passed, {} failed\n",
        results.api_endpoints_passed, results.api_endpoints_failed
    ));
    report.push_str(&format!(
        "- **Authentication:** {} passed, {} failed\n",
        results.auth_tests_passed, results.auth_tests_failed
    ));
    report.push_str(&format!(
        "- **Tenant Isolation:** {} passed, {} failed\n",
        results.tenant_isolation_passed, results.tenant_isolation_failed
    ));
    report.push_str(&format!(
        "- **Database Operations:** {} passed, {} failed\n",
        results.database_tests_passed, results.database_tests_failed
    ));
    report.push_str(&format!(
        "- **Frontend Compatibility:** {} passed, {} failed\n",
        results.frontend_compatibility_passed, results.frontend_compatibility_failed
    ));

    if !results.failed_tests.is_empty() {
        report.push_str("\n## Failed Tests\n\n");
        for test in &results.failed_tests {
            report.push_str(&format!("- {}\n", test));
        }
    }

    if !results.performance_regressions.is_empty() {
        report.push_str("\n## Performance Regressions\n\n");
        for regression in &results.performance_regressions {
            report.push_str(&format!(
                "- **{}:** Expected ≤{}ms, Actual {}ms ({:.1}% regression)\n",
                regression.endpoint,
                regression.expected_max_ms,
                regression.actual_ms,
                regression.regression_percentage
            ));
        }
    }

    report.push_str("\n## Recommendations\n\n");

    match results.overall_compatibility {
        CompatibilityStatus::FullyCompatible => {
            report.push_str("✅ **All systems are fully backward compatible.** The functional programming enhancements have been successfully integrated without breaking existing functionality.\n");
        }
        CompatibilityStatus::PartiallyCompatible => {
            report.push_str("⚠️ **Partial compatibility detected.** Some minor issues need to be addressed before full deployment.\n");
        }
        CompatibilityStatus::Incompatible => {
            report.push_str("❌ **Critical compatibility issues detected.** Major problems need immediate attention before deployment.\n");
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_compatibility_validator_creation() {
        let config = CompatibilityTestConfig::default();
        let validator = BackwardCompatibilityValidator::new(config);

        // Test that validator is created successfully
        assert_eq!(validator.config.test_tenant_id, "tenant1");
        assert_eq!(validator.config.test_username, "testuser");
        assert_eq!(
            validator.config.jwt_secret,
            "test_secret_key_for_compatibility_testing_only"
        );
    }

    #[tokio::test]
    #[ignore] // Requires running server
    async fn test_mock_login_flow() {
        let config = CompatibilityTestConfig::default();
        let validator = BackwardCompatibilityValidator::new(config);

        let result = validator.test_login_flow().await;
        assert!(
            result.is_ok(),
            "Login flow should succeed with valid config"
        );
    }

    #[tokio::test]
    async fn test_compatibility_status_calculation() {
        let config = CompatibilityTestConfig::default();
        let validator = BackwardCompatibilityValidator::new(config);

        let results = CompatibilityTestResults {
            api_endpoints_passed: 5,
            api_endpoints_failed: 0,
            auth_tests_passed: 3,
            auth_tests_failed: 0,
            tenant_isolation_passed: 2,
            tenant_isolation_failed: 0,
            database_tests_passed: 3,
            database_tests_failed: 0,
            frontend_compatibility_passed: 3,
            frontend_compatibility_failed: 0,
            overall_compatibility: CompatibilityStatus::FullyCompatible,
            failed_tests: Vec::new(),
            performance_regressions: Vec::new(),
        };

        let status = validator.calculate_overall_status(&results);
        assert_eq!(status, CompatibilityStatus::FullyCompatible);
    }

    #[tokio::test]
    async fn test_report_generation() {
        let results = CompatibilityTestResults {
            api_endpoints_passed: 5,
            api_endpoints_failed: 0,
            auth_tests_passed: 3,
            auth_tests_failed: 0,
            tenant_isolation_passed: 2,
            tenant_isolation_failed: 0,
            database_tests_passed: 3,
            database_tests_failed: 0,
            frontend_compatibility_passed: 3,
            frontend_compatibility_failed: 0,
            overall_compatibility: CompatibilityStatus::FullyCompatible,
            failed_tests: Vec::new(),
            performance_regressions: Vec::new(),
        };

        let report = generate_compatibility_report(&results);
        eprintln!("{}", report);
        assert!(report.contains("Backward Compatibility Validation Report"));
        assert!(report.contains("FullyCompatible"));
        assert!(report.contains("0 failed"));
    }

    #[tokio::test]
    async fn test_calculate_overall_status_with_regressions() {
        let config = CompatibilityTestConfig::default();
        let validator = BackwardCompatibilityValidator::new(config);

        let results = CompatibilityTestResults {
            api_endpoints_passed: 5,
            api_endpoints_failed: 0,
            auth_tests_passed: 3,
            auth_tests_failed: 0,
            tenant_isolation_passed: 2,
            tenant_isolation_failed: 0,
            database_tests_passed: 3,
            database_tests_failed: 0,
            frontend_compatibility_passed: 3,
            frontend_compatibility_failed: 0,
            overall_compatibility: CompatibilityStatus::FullyCompatible,
            failed_tests: Vec::new(),
            performance_regressions: vec![PerformanceRegression {
                endpoint: "/api/ping".into(),
                expected_max_ms: 10,
                actual_ms: 30,
                regression_percentage: 200.0,
            }],
        };

        let status = validator.calculate_overall_status(&results);
        assert_eq!(status, CompatibilityStatus::PartiallyCompatible);
    }

    #[tokio::test]
    async fn test_calculate_overall_status_incompatible() {
        let config = CompatibilityTestConfig::default();
        let validator = BackwardCompatibilityValidator::new(config);

        let results = CompatibilityTestResults {
            api_endpoints_passed: 1,
            api_endpoints_failed: 4,
            auth_tests_passed: 0,
            auth_tests_failed: 3,
            tenant_isolation_passed: 0,
            tenant_isolation_failed: 2,
            database_tests_passed: 0,
            database_tests_failed: 3,
            frontend_compatibility_passed: 0,
            frontend_compatibility_failed: 3,
            overall_compatibility: CompatibilityStatus::FullyCompatible,
            failed_tests: vec!["Sample failure".into()],
            performance_regressions: Vec::new(),
        };

        let status = validator.calculate_overall_status(&results);
        assert_eq!(status, CompatibilityStatus::Incompatible);
    }

    #[tokio::test]
    async fn test_generate_report_with_failures_and_regressions() {
        let results = CompatibilityTestResults {
            api_endpoints_passed: 3,
            api_endpoints_failed: 2,
            auth_tests_passed: 1,
            auth_tests_failed: 2,
            tenant_isolation_passed: 1,
            tenant_isolation_failed: 1,
            database_tests_passed: 2,
            database_tests_failed: 1,
            frontend_compatibility_passed: 1,
            frontend_compatibility_failed: 2,
            overall_compatibility: CompatibilityStatus::Incompatible,
            failed_tests: vec!["Endpoint failure".into(), "Auth failure".into()],
            performance_regressions: vec![PerformanceRegression {
                endpoint: "/api/address-book".into(),
                expected_max_ms: 200,
                actual_ms: 400,
                regression_percentage: 100.0,
            }],
        };

        let report = generate_compatibility_report(&results);

        assert!(report.contains("Endpoint failure"));
        assert!(report.contains("Auth failure"));
        assert!(report.contains("Performance Regressions"));
        assert!(report.contains("Incompatible"));
    }

    #[tokio::test]
    #[ignore] // Requires running server
    async fn test_run_full_suite_has_expected_counts() {
        let config = CompatibilityTestConfig::default();
        let validator = BackwardCompatibilityValidator::new(config);

        let results = validator.run_full_compatibility_suite().await;

        assert!(results.api_endpoints_passed >= 5);
        assert_eq!(results.api_endpoints_failed, 0);
        assert!(results.auth_tests_passed >= 3);
        assert_eq!(results.auth_tests_failed, 0);
        assert_eq!(
            results.overall_compatibility,
            CompatibilityStatus::FullyCompatible
        );
    }
}
