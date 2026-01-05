use openidconnect::{
    core::{CoreClient, CoreProviderMetadata, CoreResponseType},
    AuthenticationFlow, ClientId, ClientSecret, CsrfToken, Nonce, OAuth2TokenResponse,
    PkceCodeChallenge, RedirectUrl, Scope,
};
use reqwest::Client as ReqwestClient;
use reqwest::blocking::Client as BlockingClient;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use url::Url;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};

#[derive(Deserialize)]
struct Jwk {
    kid: String,
    kty: String,
    n: String,
    e: String,
}

#[derive(Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

#[derive(Clone, Deserialize)]
pub struct KeycloakConfig {
    pub issuer_url: String,
    pub client_id: String,
    pub client_secret: SecretString,
    pub redirect_url: String,
}

impl std::fmt::Debug for KeycloakConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeycloakConfig")
            .field("issuer_url", &self.issuer_url)
            .field("client_id", &self.client_id)
            .field("client_secret", &"***REDACTED***")
            .field("redirect_url", &self.redirect_url)
            .finish()
    }
}

/// OAuth session state for PKCE, CSRF, and nonce validation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OAuthSessionState {
    /// PKCE code verifier (never sent over the network)
    pub pkce_verifier: String,
    /// CSRF token / state parameter
    pub csrf_token: String,
    /// OpenID Connect nonce for replay attack prevention
    pub nonce: String,
    /// Unix timestamp when this state was created (for expiration check)
    pub created_at: i64,
}

/// Token response from Keycloak after code exchange
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    /// OAuth 2.0 access token (Bearer token for API access)
    pub access_token: String,
    /// OpenID Connect ID token (contains user claims/identity)
    pub id_token: Option<String>,
    /// OAuth 2.0 refresh token (for obtaining new access tokens)
    pub refresh_token: Option<String>,
}

/// Validates the nonce claim in an ID token JWT without signature verification.
///
/// **SECURITY WARNING**: This function decodes the JWT and extracts claims WITHOUT validating
/// the cryptographic signature. It is suitable ONLY for:
/// - Development/testing environments
/// - As a preliminary check before full signature validation
/// - When the ID token is immediately validated with full signature verification via JWKS
///
/// In production with untrusted ID tokens, always validate the signature using Keycloak's JWKS endpoint.
///
/// # Arguments
/// * `id_token_jwt` - The JWT string from the ID token (format: header.claims.signature)
/// * `expected_nonce` - The nonce value stored during the authorization request
///
/// # Returns
/// * `Ok(())` if nonce claim in token matches expected_nonce
/// * `Err(String)` if token cannot be decoded, nonce claim is missing, or values don't match
///
/// # Examples
///
/// ```no_run
/// let token = "eyJhbGc...eyJub25j...signature";
/// let nonce = "abc123xyz";
/// validate_id_token_nonce(token, nonce)?; // Returns Ok if nonce matches
/// ```
fn validate_id_token_nonce(id_token_jwt: &str, expected_nonce: &str) -> Result<(), String> {
    use base64::Engine;

    // Split JWT into header.claims.signature
    let parts: Vec<&str> = id_token_jwt.split('.').collect();
    if parts.len() != 3 {
        return Err("Invalid JWT format: expected 3 parts (header.claims.signature)".to_string());
    }

    // Decode the claims part (second part) from base64
    let claims_json = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|e| format!("Failed to decode JWT claims from base64: {}", e))?;

    // Parse claims JSON
    let claims: Claims = serde_json::from_slice(&claims_json)
        .map_err(|e| format!("Failed to parse JWT claims as JSON: {}", e))?;

    // Extract and validate nonce claim
    let token_nonce = claims
        .nonce
        .ok_or_else(|| "Nonce claim not present in ID token".to_string())?;

    // Compare nonces for equality
    if token_nonce != expected_nonce {
        return Err(format!(
            "Nonce mismatch: token contains '{}' but expected '{}' (possible replay attack)",
            token_nonce, expected_nonce
        ));
    }

    Ok(())
}

#[derive(Clone)]
pub struct KeycloakClient {
    issuer_url: String,
    client_id: String,
    client_secret: SecretString,
    redirect_url: String,
}

const KEYCLOAK_TIMEOUT: Duration = Duration::from_secs(10);

/// Helper function to construct metadata URL robustly from issuer URL using URL::join
///
/// This ensures proper URL path joining and validates the resulting URL.
/// The issuer URL should be the full realm issuer URL (e.g., https://keycloak.example.com/realms/master).
///
/// # Errors
/// Returns an error if the issuer URL is invalid or URL joining fails.
fn build_metadata_url(
    issuer_url: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // Ensure issuer URL ends with / so join() doesn't replace the last path segment
    let issuer_with_slash = if issuer_url.ends_with('/') {
        issuer_url.to_string()
    } else {
        format!("{}/", issuer_url)
    };

    // Parse issuer URL and join with the well-known path
    let base_url = Url::parse(&issuer_with_slash)
        .map_err(|e| {
            log::error!("Invalid issuer URL: {}", e);
            Box::new(e) as Box<dyn std::error::Error + Send + Sync>
        })?
        .join(".well-known/openid-configuration")
        .map_err(|e| {
            log::error!("Failed to construct metadata URL: {}", e);
            Box::new(e) as Box<dyn std::error::Error + Send + Sync>
        })?
        .to_string();

    Ok(base_url)
}

impl KeycloakClient {
    pub async fn new(
        config: KeycloakConfig,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Validate configuration by attempting discovery
        let http_client = ReqwestClient::builder()
            .timeout(KEYCLOAK_TIMEOUT)
            .connect_timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        // Construct metadata URL robustly using URL::join
        let metadata_url = build_metadata_url(&config.issuer_url)?;

        log::debug!("Keycloak: Fetching metadata from {}", metadata_url);
        let response = http_client.get(&metadata_url).send().await.map_err(|e| {
            log::error!("Keycloak: Failed to fetch metadata: {}", e);
            Box::new(e) as Box<dyn std::error::Error + Send + Sync>
        })?;

        let status = response.status();
        log::debug!("Keycloak: Metadata response status: {}", status);

        let body_text = response.text().await.map_err(|e| {
            log::error!("Keycloak: Failed to read metadata response body: {}", e);
            Box::new(e) as Box<dyn std::error::Error + Send + Sync>
        })?;

        // Check HTTP status before attempting JSON parse
        if !status.is_success() {
            let error_msg = if body_text.len() > 200 {
                format!("{}...", &body_text[..200])
            } else {
                body_text.clone()
            };
            log::error!(
                "Keycloak: Metadata request failed with status {}: {}",
                status,
                error_msg
            );
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Keycloak metadata request failed: {}", status),
            )) as Box<dyn std::error::Error + Send + Sync>);
        }

        // Log only a truncated preview for security (avoid exposing internal URLs/config)
        log::trace!(
            "Keycloak: Metadata response (truncated): {}",
            if body_text.len() > 200 {
                format!("{}...", &body_text[..200])
            } else {
                body_text.clone()
            }
        );

        let _provider_metadata: CoreProviderMetadata =
            serde_json::from_str(&body_text).map_err(|e| {
                log::error!("Keycloak: Failed to parse metadata response as JSON: {}", e);
                Box::new(e) as Box<dyn std::error::Error + Send + Sync>
            })?;

        log::info!(
            "Keycloak: Successfully initialized with issuer: {}",
            config.issuer_url
        );

        Ok(KeycloakClient {
            issuer_url: config.issuer_url,
            client_id: config.client_id,
            client_secret: config.client_secret,
            redirect_url: config.redirect_url,
        })
    }

    #[allow(dead_code)]
    async fn validate_configuration(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Validate configuration by attempting discovery
        let http_client = ReqwestClient::builder()
            .timeout(KEYCLOAK_TIMEOUT)
            .connect_timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        let metadata_url = build_metadata_url(&self.issuer_url)?;

        let response = http_client
            .get(metadata_url)
            .send()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let _provider_metadata: CoreProviderMetadata = response
            .json()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        Ok(())
    }

    /// Generate authorization URL and return session state for secure storage.
    ///
    /// The returned `OAuthSessionState` contains:
    /// - `pkce_verifier`: Used during token exchange (must not be sent to browser)
    /// - `csrf_token`: Returned as `state` parameter in callback (for CSRF protection)
    /// - `nonce`: Validated in ID token (for replay attack prevention)
    /// - `created_at`: Timestamp for session expiration validation
    ///
    /// **IMPORTANT**: Store the returned `OAuthSessionState` securely in an HttpOnly,
    /// Secure, SameSite=Strict cookie and DO NOT expose to frontend code.
    pub async fn get_authorization_url(
        &self,
    ) -> Result<(String, OAuthSessionState), Box<dyn std::error::Error + Send + Sync>> {
        // Fetch provider metadata
        let http_client = ReqwestClient::builder()
            .timeout(KEYCLOAK_TIMEOUT)
            .connect_timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        // Construct metadata URL robustly using URL::join (same as initialization)
        let metadata_url = build_metadata_url(&self.issuer_url)?;

        let response = http_client
            .get(metadata_url)
            .send()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();

        // Check HTTP status before attempting JSON parse
        if !status.is_success() {
            let error_msg = if body_text.len() > 200 {
                format!("{}...", &body_text[..200])
            } else {
                body_text.clone()
            };
            log::error!(
                "get_authorization_url: Metadata request failed with status {}: {}",
                status,
                error_msg
            );
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Keycloak metadata request failed: {}", status),
            )) as Box<dyn std::error::Error + Send + Sync>);
        }

        // Log only a truncated preview for security (avoid exposing internal URLs/config)
        log::trace!(
            "get_authorization_url: Metadata response (truncated): {}",
            if body_text.len() > 200 {
                format!("{}...", &body_text[..200])
            } else {
                body_text.clone()
            }
        );

        let provider_metadata: CoreProviderMetadata =
            serde_json::from_str(&body_text).map_err(|e| {
                log::error!(
                    "get_authorization_url: Failed to parse metadata as JSON: {}",
                    e
                );
                Box::new(e) as Box<dyn std::error::Error + Send + Sync>
            })?;

        // Parse the redirect_url into RedirectUrl type required by OpenID Connect
        let redirect_url = RedirectUrl::new(self.redirect_url.clone())
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        // Create a fully configured client from the metadata with redirect_uri set
        let client = CoreClient::from_provider_metadata(
            provider_metadata,
            ClientId::new(self.client_id.clone()),
            Some(ClientSecret::new(
                self.client_secret.expose_secret().to_string(),
            )),
        )
        .set_redirect_uri(redirect_url);

        // Generate PKCE and authorization URL
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let (auth_url, csrf_token, nonce) = client
            .authorize_url(
                AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .add_scopes(vec![Scope::new("openid".to_string())])
            .set_pkce_challenge(pkce_challenge)
            .url();

        let session_state = OAuthSessionState {
            pkce_verifier: pkce_verifier.secret().to_string(),
            csrf_token: csrf_token.secret().to_string(),
            nonce: nonce.secret().to_string(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
        };

        Ok((auth_url.to_string(), session_state))
    }

    /// Exchange authorization code for tokens (access token, ID token, refresh token).
    ///
    /// This function:
    /// 1. Fetches Keycloak provider metadata
    /// 2. Exchanges the authorization code for tokens using PKCE verifier
    /// 3. Returns the token response containing ID token (with claims) and access token
    ///
    /// # Arguments
    /// * `code` - Authorization code from OAuth callback
    /// * `pkce_verifier` - PKCE code verifier from OAuthSessionState
    /// * `nonce` - OpenID Connect nonce from OAuthSessionState (for later validation)
    ///
    /// # Returns
    /// TokenResponse containing id_token (with claims), access_token, and refresh_token
    ///
    /// # Errors
    /// Returns an error if metadata fetch, token exchange, or ID token parsing fails
    pub async fn exchange_code_for_token(
        &self,
        code: &str,
        pkce_verifier: String,
        nonce: String,
    ) -> Result<TokenResponse, Box<dyn std::error::Error + Send + Sync>> {
        use openidconnect::{AuthorizationCode, PkceCodeVerifier};

        // Fetch provider metadata
        let http_client = ReqwestClient::builder()
            .timeout(KEYCLOAK_TIMEOUT)
            .connect_timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        // Construct metadata URL robustly using URL::join
        let metadata_url = build_metadata_url(&self.issuer_url)?;

        let response = http_client.get(&metadata_url).send().await.map_err(|e| {
            log::error!("exchange_code_for_token: Failed to fetch metadata: {}", e);
            Box::new(e) as Box<dyn std::error::Error + Send + Sync>
        })?;

        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            log::error!(
                "exchange_code_for_token: Metadata request failed with status {}: {}",
                status,
                body_text
            );
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Keycloak metadata request failed: {}", status),
            )) as Box<dyn std::error::Error + Send + Sync>);
        }

        let provider_metadata: CoreProviderMetadata =
            serde_json::from_str(&body_text).map_err(|e| {
                log::error!(
                    "exchange_code_for_token: Failed to parse metadata as JSON: {}",
                    e
                );
                Box::new(e) as Box<dyn std::error::Error + Send + Sync>
            })?;

        // Parse the redirect_url into RedirectUrl type required by OpenID Connect
        let redirect_url = RedirectUrl::new(self.redirect_url.clone())
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        // Create a fully configured client from the metadata with redirect_uri set
        let client = CoreClient::from_provider_metadata(
            provider_metadata,
            ClientId::new(self.client_id.clone()),
            Some(ClientSecret::new(
                self.client_secret.expose_secret().to_string(),
            )),
        )
        .set_redirect_uri(redirect_url);

        // Exchange authorization code for tokens using PKCE verifier
        let token_response = client
            .exchange_code(AuthorizationCode::new(code.to_string()))?
            .set_pkce_verifier(PkceCodeVerifier::new(pkce_verifier))
            .request_async(&http_client)
            .await
            .map_err(|e| {
                log::error!("exchange_code_for_token: Token exchange failed: {}", e);
                Box::new(e) as Box<dyn std::error::Error + Send + Sync>
            })?;

        log::debug!(
            "exchange_code_for_token: Successfully exchanged authorization code for tokens"
        );

        // Extract ID token from extra fields and validate nonce
        // The ID token is required for OpenID Connect and contains user claims including nonce
        let id_token_jwt = token_response
            .extra_fields()
            .id_token()
            .ok_or_else(|| {
                log::error!("exchange_code_for_token: ID token missing from token response");
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "ID token not present in token response",
                )) as Box<dyn std::error::Error + Send + Sync>
            })?
            .to_string();

        // Validate nonce in ID token claims without signature verification
        // **IMPORTANT**: This decodes the JWT claims WITHOUT validating the signature.
        // In production, implement full signature validation using Keycloak's JWKS endpoint
        // (see validate_token_sync method for structure).
        validate_id_token_nonce(&id_token_jwt, &nonce).map_err(|e| {
            log::warn!(
                "exchange_code_for_token: ID token nonce validation failed: {}",
                e
            );
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("ID token nonce validation failed: {}", e),
            )) as Box<dyn std::error::Error + Send + Sync>
        })?;

        log::debug!("exchange_code_for_token: ID token nonce validation successful");

        // Extract access token as a String
        // TODO: For full production readiness, also validate access token scope and lifetime
        let access_token_str = token_response.access_token().secret().to_string();

        // Extract refresh token if present
        let refresh_token_opt = token_response
            .refresh_token()
            .map(|token| token.secret().to_string());

        Ok(TokenResponse {
            access_token: access_token_str,
            id_token: Some(id_token_jwt),
            refresh_token: refresh_token_opt,
        })
    }

    /// Validate JWT token signature and claims using RS256 and JWKS.
    ///
    /// This function:
    /// 1. Decodes the JWT header to extract the key ID (kid)
    /// 2. Fetches or uses cached Keycloak JWKS endpoint
    /// 3. Locates the JWK matching the token's kid
    /// 4. Converts RSA public key to DecodingKey
    /// 5. Validates RS256 signature and standard claims (iss, aud, exp, nbf)
    ///
    /// **TODO**: Currently a stub that returns an error.
    /// **TODO**: Implement JWKS fetching and caching (consider TTL-based cache)
    /// **TODO**: Convert JWK to DecodingKey using RSA modulus (n) and exponent (e)
    /// **TODO**: Validate nonce claim matches stored value from OAuthSessionState
    pub fn validate_token_sync(
        &self,
        _token: &str,
    ) -> Result<Claims, Box<dyn std::error::Error + Send + Sync>> {
        // Temporary: Return explicit error rather than using dummy key
        // In production, uncomment and implement JWKS-based RS256 validation:
        //
        // use jsonwebtoken::{decode, DecodingKey, Validation};
        // 1. Parse JWT header to extract 'kid'
        // 2. Fetch JWKS from Keycloak realm's JWKS endpoint
        // 3. Find JWK entry matching kid
        // 4. Convert RSA public key (n, e) to PEM and create DecodingKey::from_rsa_components(n, e)
        // 5. Validate: decode::<Claims>(token, &key, &Validation::new(Algorithm::RS256))
        // 6. Check standard claims: iss, aud, exp, nbf, and validate iss matches realm issuer URL and aud contains client_id
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Token validation stub: RS256/JWKS validation not yet implemented. See code comments for implementation.",
        )))
    }

    /// Validate ID token signature and claims using Keycloak's JWKS endpoint.
    ///
    /// This method:
    /// 1. Decodes the JWT header to extract the key ID (kid)
    /// 2. Fetches JWKS from Keycloak's /protocol/openid-connect/certs endpoint
    /// 3. Locates the JWK matching the token's kid
    /// 4. Converts RSA public key to DecodingKey
    /// 5. Validates RS256 signature and standard claims (iss, aud, exp, nbf)
    /// 6. Validates issuer and audience claims
    ///
    /// # Arguments
    /// * `id_token` - The JWT ID token string
    ///
    /// # Returns
    /// * `Ok(Claims)` if the token is valid and signature verification succeeds
    /// * `Err(Box<dyn std::error::Error + Send + Sync>)` if validation fails
    ///
    /// # Errors
    /// Returns an error if:
    /// - JWT header cannot be decoded
    /// - JWKS cannot be fetched
    /// - No matching key found in JWKS
    /// - RSA key conversion fails
    /// - Signature verification fails
    /// - Standard claims validation fails (iss, aud, exp, nbf)
    pub async fn validate_id_token(
        &self,
        id_token: &str,
    ) -> Result<Claims, Box<dyn std::error::Error + Send + Sync>> {
        // Decode JWT header to get key ID
        let header = decode_header(id_token).map_err(|e| {
            log::error!("Failed to decode JWT header: {}", e);
            Box::new(e) as Box<dyn std::error::Error + Send + Sync>
        })?;

        let kid = header.kid.ok_or_else(|| {
            log::error!("JWT header missing 'kid' (key ID) claim");
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "JWT header missing 'kid' claim",
            )) as Box<dyn std::error::Error + Send + Sync>
        })?;

        // Fetch JWKS from Keycloak
        let jwks_url = format!("{}/protocol/openid-connect/certs", self.issuer_url.trim_end_matches('/'));
        let http_client = ReqwestClient::builder()
            .timeout(KEYCLOAK_TIMEOUT)
            .build()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let jwks_response = http_client
            .get(&jwks_url)
            .send()
            .await
            .map_err(|e| {
                log::error!("Failed to fetch JWKS from {}: {}", jwks_url, e);
                Box::new(e) as Box<dyn std::error::Error + Send + Sync>
            })?;

        let jwks: Jwks = jwks_response.json().await.map_err(|e| {
            log::error!("Failed to parse JWKS response: {}", e);
            Box::new(e) as Box<dyn std::error::Error + Send + Sync>
        })?;

        // Find the matching key
        let jwk = jwks.keys.into_iter().find(|k| k.kid == kid).ok_or_else(|| {
            log::error!("No JWK found matching kid: {}", kid);
            Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("No JWK found for key ID: {}", kid),
            )) as Box<dyn std::error::Error + Send + Sync>
        })?;

        // Validate JWK is RSA type and has required components
        if jwk.kty != "RSA" {
            let error_msg = format!("JWK with kid {} has unsupported kty {}, expected RSA", kid, jwk.kty);
            log::error!("{}", error_msg);
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                error_msg,
            )) as Box<dyn std::error::Error + Send + Sync>);
        }
        if jwk.n.is_empty() || jwk.e.is_empty() {
            let error_msg = format!("JWK with kid {} is missing required RSA components (n or e)", kid);
            log::error!("{}", error_msg);
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                error_msg,
            )) as Box<dyn std::error::Error + Send + Sync>);
        }

        // Convert JWK to DecodingKey
        let decoding_key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e).map_err(|e| {
            log::error!("Failed to create decoding key from JWK: {}", e);
            Box::new(e) as Box<dyn std::error::Error + Send + Sync>
        })?;

        // Create validation settings
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.issuer_url]);
        validation.set_audience(&[&self.client_id]);

        // Decode and validate the token
        let token_data = decode::<Claims>(id_token, &decoding_key, &validation).map_err(|e| {
            log::error!("JWT signature validation failed: {}", e);
            Box::new(e) as Box<dyn std::error::Error + Send + Sync>
        })?;

        // Additional claim validation (defense-in-depth: validate claims again after decode
        // to ensure library validation was applied correctly and catch any edge cases)
        let claims = token_data.claims;
        claims.validate_issuer(&self.issuer_url)?;
        claims.validate_audience(&self.client_id)?;

        log::debug!("ID token signature validation successful");
        Ok(claims)
    }

    /// Get the issuer URL for this Keycloak client
    pub fn issuer_url(&self) -> &str {
        &self.issuer_url
    }

    /// Get the client ID for this Keycloak client
    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    /// Validate ID token signature and claims using Keycloak's JWKS endpoint (synchronous version).
    ///
    /// This method performs the same validation as validate_id_token but synchronously.
    pub fn validate_id_token_sync(&self, id_token: &str) -> Result<Claims, Box<dyn std::error::Error + Send + Sync>> {
        let header = decode_header(id_token).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let kid = header.kid.ok_or_else(|| Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "No kid in header")) as Box<dyn std::error::Error + Send + Sync>)?;

        let jwks_url = format!("{}/protocol/openid-connect/certs", self.issuer_url);

        let client = BlockingClient::new();
        let jwks: Jwks = client.get(&jwks_url).send().map_err(|e| Box::new(e))?.json().map_err(|e| Box::new(e))?;

        let jwk = jwks.keys.iter().find(|j| j.kid == kid).ok_or_else(|| Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, format!("No JWK found for key ID: {}", kid))) as Box<dyn std::error::Error + Send + Sync>)?;

        let decoding_key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e).map_err(|e| Box::new(e))?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.issuer_url]);
        validation.set_audience(&[&self.client_id]);

        let token_data = decode::<Claims>(id_token, &decoding_key, &validation).map_err(|e| Box::new(e))?;

        let claims = token_data.claims;
        claims.validate_issuer(&self.issuer_url)?;
        claims.validate_audience(&self.client_id)?;

        Ok(claims)
    }
}

fn deserialize_aud<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    use serde_json::Value;

    let value = Value::deserialize(deserializer)?;
    match value {
        Value::String(s) => Ok(Some(vec![s])),
        Value::Array(arr) => {
            let mut vec = Vec::new();
            for v in arr {
                if let Value::String(s) = v {
                    vec.push(s);
                } else {
                    return Err(serde::de::Error::custom("aud array must contain strings"));
                }
            }
            Ok(Some(vec))
        }
        Value::Null => Ok(None),
        _ => Err(serde::de::Error::custom("aud must be a string or array of strings")),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub email: Option<String>,
    pub preferred_username: Option<String>,
    pub tenant_id: Option<String>,
    pub exp: usize,
    pub iat: usize,
    pub nonce: Option<String>,
    /// OpenID Connect issuer URL (iss claim) - typically the Keycloak realm URL
    #[serde(default)]
    pub iss: Option<String>,
    /// OpenID Connect audience (aud claim) - can be a single string or array of strings
    /// Serde will attempt to deserialize as Vec<String>, falling back to a single string wrapped in a Vec
    #[serde(default, deserialize_with = "deserialize_aud")]
    pub aud: Option<Vec<String>>,
}

impl Claims {
    /// Validates that the issuer claim matches the expected Keycloak realm issuer URL.
    ///
    /// # Arguments
    /// * `expected_issuer` - The configured Keycloak realm issuer URL (e.g., "https://keycloak.example.com/realms/myrealm")
    ///
    /// # Returns
    /// * `Ok(())` if the issuer matches or if the iss claim is not present (for backward compatibility)
    /// * `Err(String)` if the issuer doesn't match
    pub fn validate_issuer(
        &self,
        expected_issuer: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(iss) = &self.iss {
            if iss != expected_issuer {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "Token issuer '{}' does not match expected issuer '{}'",
                        iss, expected_issuer
                    ),
                )));
            }
        }
        Ok(())
    }

    /// Validates that the audience claim contains the expected client ID.
    ///
    /// # Arguments
    /// * `expected_client_id` - The expected Keycloak client ID
    ///
    /// # Returns
    /// * `Ok(())` if the audience contains the client ID or if the aud claim is not present (for backward compatibility)
    /// * `Err(String)` if the audience doesn't contain the client ID
    pub fn validate_audience(
        &self,
        expected_client_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(aud) = &self.aud {
            if !aud.contains(&expected_client_id.to_string()) {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "Token audience '{}' does not contain expected client ID '{}'",
                        aud.join(", "),
                        expected_client_id
                    ),
                )));
            }
        }
        Ok(())
    }
}
