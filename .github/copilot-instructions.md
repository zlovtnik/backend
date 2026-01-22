<!-- Repository-specific guidance for AI coding agents. Keep concise and actionable. -->
# Copilot instructions for dispo-rusty (backend)

## Purpose
Help AI agents be immediately productive in this multi-tenant Actix Web + Diesel Rust backend with advanced functional programming patterns and Keycloak OAuth2 integration.

## Architecture & Core Concepts
- **Multi-tenant Isolation**: Single PostgreSQL database using per-tenant schemas. Tenant context is ALWAYS derived server-side (host/subdomain, mTLS, or lookup). NEVER trust client-supplied tenant IDs.
- **Authentication**: Dual auth system - JWT tokens (direct login) and optional Keycloak OAuth2 flows. Keycloak is not mandatory and can be disabled for pure JWT deployments. Three supported modes:
  - **Keycloak-required mode** (`AUTH_MODE=keycloak`): SSO deployments requiring Keycloak OAuth2. Server fails fast if `KEYCLOAK_ISSUER_URL`, `KEYCLOAK_CLIENT_ID`, or `KEYCLOAK_CLIENT_SECRET` are missing or invalid.
  - **JWT-only fallback mode** (`AUTH_MODE=jwt`): Pure JWT authentication without Keycloak. Server starts normally even if Keycloak variables are missing.
  - **Hybrid mode** (`AUTH_MODE=hybrid`): Supports both JWT and Keycloak OAuth2 flows. Server validates Keycloak settings at startup when Keycloak config is present.
  Server startup validates Keycloak settings only when `AUTH_MODE` includes keycloak, logging clear errors on misconfiguration. In JWT-only mode, Keycloak variables are ignored.
- **Functional Programming**: Leverages `rcs-functional` library for pure functions, iterator chains (`ChainBuilder`), and immutable state management.
- **NFAg Domain**: Implements Nota Fiscal Eletrônica da Agua (NFAg) with complex XSD-based validation and multi-tenant CRUD operations.
- **Real-time Observability**: Logs streamed over WebSocket at `/api/ws/logs` using `rcs::utils::ws_logger`.
- **WebSocket Logging**: Configurable buffer size (`WS_LOG_BUFFER_SIZE`) for real-time log streaming.

## Key Files & Directories
- **Entry Points**: [src/main.rs](src/main.rs), [src/lib.rs](src/lib.rs).
- **Functional Core**: [functional_lib/src/lib.rs](functional_lib/src/lib.rs) and `src/functional/` (re-exported as `functional`).
- **API Controllers**: [src/api/](src/api/) (e.g., [nfag_controller.rs](src/api/nfag_controller.rs)).
- **Configuration**: [src/config/app.rs](src/config/app.rs) (route registration), [src/config/db.rs](src/config/db.rs) (tenant pool manager).
- **Middleware**: [src/middleware/auth_middleware.rs](src/middleware/auth_middleware.rs).
- **Keycloak Integration**: [src/utils/keycloak.rs](src/utils/keycloak.rs) for OAuth2 flows.
- **Docker Setups**: `docker-compose.local.yml` (dev), `docker-compose.prod.yml` (production), `docker-compose.kafka.yml` (with Kafka).

## Conventions & Patterns
- **Route Building**: Use `RouteBuilder` in [src/config/app.rs](src/config/app.rs) for composable route registration.
- **Database Access**: Use `TenantPoolManager` to obtain per-tenant connection pools: `manager.get_pool(&tenant_id)?`.
- **Functional Logic**: Prefer `PureFunctionRegistry` and `ChainBuilder` over ad-hoc imperative logic.
- **Validation**: Use `ValidationEngine` for iterator-based validation pipelines, especially for NFAg XSD compliance.
- **Backward Compatibility**: Use `BackwardCompatibilityValidator` in [src/api/health_controller.rs](src/api/health_controller.rs) to ensure functional enhancements don't break existing APIs.
- **Pagination**: Use `unified_pagination` with encrypted cursors (AES-256-GCM).
- **Error Handling**: ServiceError with `.with_detail()` and `.with_tag()` for structured error responses.
- **OAuth2 Flow**: Keycloak redirect at `/api/auth/login/keycloak`, callback at `/api/callback`.

## Developer Workflows
- **Local Dev**: `cargo run` (reads `.env`).
- **Migrations**: `diesel migration generate <name>` then `diesel migration run`.
- **Testing**:
  - Unit: `cargo test`.
  - Integration: Requires Docker. Run with `cargo test --test integration_tests`.
  - Log Streaming: Run with `cargo test -- --test-threads=1` to avoid race conditions.
  - Functional Tests: Comprehensive tests in `tests/functional_tests.rs` covering iterator engines, pure functions, validation pipelines.
- **Code Quality**: `cargo clippy` and `cargo fmt`.
- **Docker**: `make docker-up-local` (dev stack), `make docker-up-prod` (prod stack), `make docker-up-kafka` (with Kafka).
- **Keycloak Setup**: Configure `KEYCLOAK_ISSUER_URL`, `KEYCLOAK_CLIENT_ID`, `KEYCLOAK_CLIENT_SECRET` in `.env`.

## Code Edit Guidelines
- **Preserve Tenant Isolation**: Any change to routing, DB, or auth must preserve server-derived tenant context.
- **Functional Style**: Leverage `rcs-functional` for iterator chains and state transitions.
- **Migrations**: Add timestamped SQL migrations in `migrations/`. Do not edit `src/schema.rs` manually.
- **Middleware Order**: Tracing -> Auth -> App Handlers. See [src/main.rs](src/main.rs).
- **Keycloak Integration**: Use `KeycloakClient` for OAuth2 flows, handle redirects and callbacks properly.
- **WebSocket**: Use `LogBroadcaster` for real-time logging, configure buffer size appropriately.

## Examples
- **Obtain Tenant Pool**: `let pool = manager.get_pool(&tenant_id)?;`
- **Functional Route**: `RouteBuilder::new().add_route(|cfg| { cfg.service(health); }).build(cfg);`
- **Encrypted Cursor**: `let cursor = IdCursor::new(id).encode()?;`
- **Compatibility Test**: `let validator = BackwardCompatibilityValidator::new(config); validator.run_full_compatibility_suite().await;`
- **Keycloak Client Init**: `let keycloak_client = KeycloakClient::new(keycloak_config).await?;`
- **WebSocket Logging**: `let log_broadcaster = LogBroadcaster::new(env::var("WS_LOG_BUFFER_SIZE").unwrap_or_else(|_| "1000".to_string()).parse().unwrap_or(1000)); init_websocket_logging(log_broadcaster)?;`

## Environment Variables
- **Required**: `DATABASE_URL` - PostgreSQL connection string for database-backed runs
- **Optional**: `APP_HOST` - Server host binding (default: 0.0.0.0)
- **Optional**: `APP_PORT` - Server port binding (default: 3000)
- **Optional**: `REDIS_URL` - Redis connection for sessions/caching (required only if sessions/caching features are enabled)
- **Optional**: `AUTH_MODE` - Authentication mode (keycloak|jwt|hybrid, default: jwt)
- **Optional**: `KEYCLOAK_ISSUER_URL` - Keycloak realm issuer URL (required only when `AUTH_MODE` includes keycloak)
- **Optional**: `KEYCLOAK_CLIENT_ID` - Keycloak OAuth2 client ID (required only when `AUTH_MODE` includes keycloak)
- **Optional**: `KEYCLOAK_CLIENT_SECRET` - Keycloak OAuth2 client secret (required only when `AUTH_MODE` includes keycloak)
- **Optional**: `SESSION_ENCRYPTION_KEY` - 64-byte base64-encoded key for session cookies (auto-generated in development)
- **Optional**: `WS_LOG_BUFFER_SIZE` - WebSocket log buffer size (default: 1000)
- **Optional**: `CURSOR_ENCRYPTION_KEY` - 32-byte base64-encoded key for pagination cursors (auto-generated in development)

## Key Generation
Secure cryptographic keys must be generated using cryptographically secure methods. Never commit keys to version control or use weak generation methods.

- **Session Encryption Key** (64 bytes): `openssl rand -base64 64`
- **Cursor Encryption Key** (32 bytes): `openssl rand -base64 32`

Store keys securely using a secrets management system (e.g., HashiCorp Vault, AWS Secrets Manager, or Azure Key Vault). In development, keys are auto-generated but should never be used in production.
