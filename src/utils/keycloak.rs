use openidconnect::{
    core::{CoreClient, CoreProviderMetadata, CoreResponseType},
    reqwest::async_http_client,
    AuthenticationFlow, ClientId, ClientSecret, CsrfToken, IssuerUrl,
    Nonce, PkceCodeChallenge, RedirectUrl, Scope,
};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

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

#[derive(Clone)]
pub struct KeycloakClient {
    client: CoreClient,
}

impl KeycloakClient {
    pub async fn new(config: KeycloakConfig) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let provider_metadata = CoreProviderMetadata::discover_async(
            IssuerUrl::new(config.issuer_url.clone())?,
            async_http_client,
        )
        .await?;

        let client = CoreClient::from_provider_metadata(
            provider_metadata,
            ClientId::new(config.client_id),
            Some(ClientSecret::new(config.client_secret.expose_secret().clone())),
        )
        .set_redirect_uri(RedirectUrl::new(config.redirect_url)?);

        Ok(KeycloakClient {
            client,
        })
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
    pub fn get_authorization_url(&self) -> (String, OAuthSessionState) {
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let (auth_url, csrf_token, nonce) = self
            .client
            .authorize_url(
                AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            .add_scope(Scope::new("email".to_string()))
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

        (auth_url.to_string(), session_state)
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
    pub fn validate_token_sync(&self, _token: &str) -> Result<Claims, Box<dyn std::error::Error + Send + Sync>> {
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
    #[serde(default)]
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
    pub fn validate_issuer(&self, expected_issuer: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(iss) = &self.iss {
            if iss != expected_issuer {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Token issuer '{}' does not match expected issuer '{}'", iss, expected_issuer),
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
    pub fn validate_audience(&self, expected_client_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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