use anyhow::Context;
use serde::Deserialize;
use std::fmt;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub grpc: GrpcConfig,
    pub db: DatabaseConfig,
    pub redis: RedisConfig,
    pub keycloak: KeycloakConfig,
    pub session: SessionConfig,
    pub runtime: RuntimeConfig,
    pub observability: ObservabilityConfig,
    pub bootstrap: BootstrapConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub keep_alive_seconds: u64,
    pub client_request_timeout_seconds: u64,
    pub workers: Option<usize>,
    pub cors_allow_credentials: bool,
}

impl ServerConfig {
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct GrpcConfig {
    pub host: String,
    pub port: u16,
}

impl GrpcConfig {
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub connect_timeout_seconds: u64,
    pub max_connections: u32,
    pub min_connections: u32,
}

impl fmt::Debug for DatabaseConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DatabaseConfig")
            .field("url", &"[REDACTED]")
            .field("connect_timeout_seconds", &self.connect_timeout_seconds)
            .field("max_connections", &self.max_connections)
            .field("min_connections", &self.min_connections)
            .finish()
    }
}

#[derive(Clone, Deserialize)]
pub struct RedisConfig {
    pub url: Option<String>,
    pub connect_timeout_seconds: u64,
}

impl fmt::Debug for RedisConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let redacted_url = self.url.as_ref().map(|_| "[REDACTED]");
        f.debug_struct("RedisConfig")
            .field("url", &redacted_url)
            .field("connect_timeout_seconds", &self.connect_timeout_seconds)
            .finish()
    }
}

#[derive(Clone, Deserialize)]
pub struct KeycloakConfig {
    pub issuer_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_url: String,
}

impl fmt::Debug for KeycloakConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeycloakConfig")
            .field("issuer_url", &self.issuer_url)
            .field("client_id", &self.client_id)
            .field("client_secret", &"[REDACTED]")
            .field("redirect_url", &self.redirect_url)
            .finish()
    }
}

#[derive(Clone, Deserialize)]
pub struct SessionConfig {
    pub cookie_secure: bool,
    pub encryption_key_b64: Option<String>,
    pub ttl_seconds: i64,
}

impl fmt::Debug for SessionConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let redacted_key = self.encryption_key_b64.as_ref().map(|_| "[REDACTED]");
        f.debug_struct("SessionConfig")
            .field("cookie_secure", &self.cookie_secure)
            .field("encryption_key_b64", &redacted_key)
            .field("ttl_seconds", &self.ttl_seconds)
            .finish()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeConfig {
    pub ws_log_buffer_size: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ObservabilityConfig {
    pub rust_log: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BootstrapConfig {
    pub app_env: String,
    pub default_tenant_id: String,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let _ = dotenvy::dotenv();

        let mut builder = ::config::Config::builder()
            .set_default("server.host", "127.0.0.1")?
            .set_default("server.port", 8000)?
            .set_default("server.keep_alive_seconds", 75)?
            .set_default("server.client_request_timeout_seconds", 10)?
            .set_default("server.cors_allow_credentials", false)?
            .set_default("grpc.host", "127.0.0.1")?
            .set_default("grpc.port", 50051)?
            .set_default("db.connect_timeout_seconds", 5)?
            .set_default("db.max_connections", 20)?
            .set_default("db.min_connections", 5)?
            .set_default("redis.connect_timeout_seconds", 5)?
            .set_default(
                "keycloak.issuer_url",
                "http://localhost:8080/realms/middleware",
            )?
            .set_default("keycloak.client_id", "middleware-app")?
            .set_default("keycloak.client_secret", "")?
            .set_default(
                "keycloak.redirect_url",
                "http://localhost:8000/api/callback",
            )?
            .set_default("session.cookie_secure", false)?
            .set_default("session.ttl_seconds", 600)?
            .set_default("runtime.ws_log_buffer_size", 1000)?
            .set_default("observability.rust_log", "info")?
            .set_default("bootstrap.app_env", "dev")?
            .set_default("bootstrap.default_tenant_id", "tenant1")?
            .add_source(::config::Environment::default().separator("__"));

        // Canonical overrides use the nested form consumed by config::Environment
        // (for example SERVER__HOST and DB__URL). The block below intentionally
        // keeps these flat legacy names as compatibility shims.
        let legacy_env_overrides = [
            ("APP_HOST", "server.host"),
            ("APP_PORT", "server.port"),
            ("CORS_ALLOW_CREDENTIALS", "server.cors_allow_credentials"),
            ("APP_GRPC_HOST", "grpc.host"),
            ("APP_GRPC_PORT", "grpc.port"),
            ("DATABASE_URL", "db.url"),
            ("REDIS_URL", "redis.url"),
            ("KEYCLOAK_ISSUER_URL", "keycloak.issuer_url"),
            ("KEYCLOAK_CLIENT_ID", "keycloak.client_id"),
            ("KEYCLOAK_MIDDLEWARE_APP_SECRET", "keycloak.client_secret"),
            ("KEYCLOAK_REDIRECT_URL", "keycloak.redirect_url"),
            ("SESSION_COOKIE_SECURE", "session.cookie_secure"),
            ("SESSION_ENCRYPTION_KEY", "session.encryption_key_b64"),
            ("WS_LOG_BUFFER_SIZE", "runtime.ws_log_buffer_size"),
            ("RUST_LOG", "observability.rust_log"),
            ("APP_ENV", "bootstrap.app_env"),
            ("DEFAULT_TENANT_ID", "bootstrap.default_tenant_id"),
        ];

        for (env_name, config_key) in legacy_env_overrides {
            if let Some(v) = env_opt(env_name) {
                builder = builder.set_override(config_key, v)?;
            }
        }

        let cfg = builder
            .build()
            .context("failed to build runtime config from env")?
            .try_deserialize::<AppConfig>()
            .context("failed to deserialize runtime config")?;

        if cfg.db.url.trim().is_empty() {
            anyhow::bail!("DATABASE_URL (or DB__URL) must be configured");
        }

        if cfg.bootstrap.app_env != "dev" && cfg.keycloak.client_secret.trim().is_empty() {
            anyhow::bail!("KEYCLOAK_MIDDLEWARE_APP_SECRET is required when APP_ENV is not dev");
        }

        Ok(cfg)
    }
}

fn env_opt(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.trim().is_empty())
}
