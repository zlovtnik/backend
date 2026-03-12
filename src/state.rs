use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use actix_web::cookie::Key;
use anyhow::Context;
use base64::Engine;
use rand::rngs::OsRng;
use rand::RngCore;
use secrecy::SecretString;
use sqlx::postgres::PgPoolOptions;

use crate::config::{cache, db, runtime::AppConfig};
use crate::utils::{keycloak, ws_logger};

#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    cfg: AppConfig,
    main_pool: db::Pool,
    tenant_manager: db::TenantPoolManager,
    redis_pool: Option<cache::Pool>,
    keycloak_client: keycloak::KeycloakClient,
    log_broadcaster: ws_logger::LogBroadcaster,
    sqlx_pools: RwLock<HashMap<String, sqlx::PgPool>>,
    session_key: Key,
}

impl AppState {
    pub async fn new(cfg: AppConfig) -> anyhow::Result<Self> {
        let log_broadcaster = ws_logger::LogBroadcaster::new(cfg.runtime.ws_log_buffer_size);
        ws_logger::init_websocket_logging_with_filter(
            log_broadcaster.clone(),
            Some(&cfg.observability.rust_log),
        )
        .map_err(|e| anyhow::anyhow!("failed to initialize websocket logging: {e}"))?;

        #[cfg(feature = "functional")]
        crate::unified_pagination::validate_cursor_encryption_key().context(
            "invalid CURSOR_ENCRYPTION_KEY configuration: expected Base64-encoded 32-byte key",
        )?;

        let main_pool = db::init_db_pool(&cfg.db.url);
        {
            let mut conn = main_pool
                .get()
                .context("failed to get db connection for migrations")?;
            db::run_migration(&mut conn).context("database migration failed")?;
        }

        let tenant_manager = db::TenantPoolManager::new(main_pool.clone());
        tenant_manager
            .add_tenant_pool(cfg.bootstrap.default_tenant_id.clone(), main_pool.clone())
            .context("failed to register default tenant pool")?;

        let redis_pool = match cfg.redis.url.as_deref() {
            Some(redis_url) => match cache::try_init_redis_client(redis_url) {
                Some(pool) => Some(pool),
                None => {
                    tracing::warn!(
                        redis_url = %redact_connection_url(redis_url),
                        "redis initialization failed; cache adapter disabled"
                    );
                    None
                }
            },
            None => None,
        };

        let keycloak_secret = if cfg.keycloak.client_secret.trim().is_empty()
            && cfg.bootstrap.app_env == "dev"
        {
            let mut bytes = [0u8; 32];
            OsRng.fill_bytes(&mut bytes);
            tracing::warn!(
                "KEYCLOAK_MIDDLEWARE_APP_SECRET missing in development; generated ephemeral random secret"
            );
            base64::engine::general_purpose::STANDARD.encode(bytes)
        } else {
            cfg.keycloak.client_secret.clone()
        };

        let keycloak_client = keycloak::KeycloakClient::new(keycloak::KeycloakConfig {
            issuer_url: cfg.keycloak.issuer_url.clone(),
            client_id: cfg.keycloak.client_id.clone(),
            client_secret: SecretString::new(keycloak_secret.into_boxed_str()),
            redirect_url: cfg.keycloak.redirect_url.clone(),
        })
        .await
        .map_err(|e| anyhow::anyhow!("failed to initialize Keycloak client: {e}"))?;

        let session_key = load_or_generate_session_key(&cfg)?;

        let sqlx_pool = PgPoolOptions::new()
            .max_connections(cfg.db.max_connections)
            .min_connections(cfg.db.min_connections)
            .acquire_timeout(std::time::Duration::from_secs(
                cfg.db.connect_timeout_seconds,
            ))
            .connect(&cfg.db.url)
            .await
            .context("failed to establish primary sqlx pool")?;

        let mut sqlx_pools = HashMap::new();
        sqlx_pools.insert(cfg.bootstrap.default_tenant_id.clone(), sqlx_pool);

        Ok(Self {
            inner: Arc::new(AppStateInner {
                cfg,
                main_pool,
                tenant_manager,
                redis_pool,
                keycloak_client,
                log_broadcaster,
                sqlx_pools: RwLock::new(sqlx_pools),
                session_key,
            }),
        })
    }

    pub fn config(&self) -> &AppConfig {
        &self.inner.cfg
    }

    pub fn main_pool(&self) -> db::Pool {
        self.inner.main_pool.clone()
    }

    pub fn tenant_manager(&self) -> db::TenantPoolManager {
        self.inner.tenant_manager.clone()
    }

    pub fn redis_pool(&self) -> Option<cache::Pool> {
        self.inner.redis_pool.clone()
    }

    pub fn keycloak_client(&self) -> keycloak::KeycloakClient {
        self.inner.keycloak_client.clone()
    }

    pub fn log_broadcaster(&self) -> ws_logger::LogBroadcaster {
        self.inner.log_broadcaster.clone()
    }

    pub fn session_key(&self) -> Key {
        self.inner.session_key.clone()
    }

    pub fn sqlx_pool_for_tenant(
        &self,
        tenant_id: &str,
    ) -> Result<Option<sqlx::PgPool>, std::sync::PoisonError<()>> {
        self.inner
            .sqlx_pools
            .read()
            .map(|pools| pools.get(tenant_id).cloned())
            .map_err(|_| std::sync::PoisonError::new(()))
    }

    pub fn insert_sqlx_pool(
        &self,
        tenant_id: impl Into<String>,
        pool: sqlx::PgPool,
    ) -> Result<(), std::sync::PoisonError<()>> {
        let tenant_id = tenant_id.into();
        self.inner
            .sqlx_pools
            .write()
            .map(|mut pools| {
                pools.insert(tenant_id, pool);
            })
            .map_err(|_| std::sync::PoisonError::new(()))
    }
}

fn load_or_generate_session_key(cfg: &AppConfig) -> anyhow::Result<Key> {
    match cfg.session.encryption_key_b64.as_ref() {
        Some(key_b64) => {
            let decoded = base64::engine::general_purpose::STANDARD
                .decode(key_b64)
                .context("failed to decode SESSION_ENCRYPTION_KEY")?;
            let key_bytes: [u8; 64] = decoded
                .try_into()
                .map_err(|_| anyhow::anyhow!("SESSION_ENCRYPTION_KEY must decode to 64 bytes"))?;
            Ok(Key::from(&key_bytes))
        }
        None if cfg.bootstrap.app_env == "dev" => Ok(Key::generate()),
        None => anyhow::bail!("SESSION_ENCRYPTION_KEY must be configured when APP_ENV is not dev"),
    }
}

fn redact_connection_url(url: &str) -> String {
    match url::Url::parse(url) {
        Ok(mut parsed_url) => {
            let _ = parsed_url.set_password(Some("<redacted>"));
            parsed_url.to_string()
        }
        Err(_) => "<unparseable-url>".to_string(),
    }
}
