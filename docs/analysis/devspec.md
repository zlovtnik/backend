# RCS Nexus — Developer Specification

Version derived from codebase analysis (2026-04-02).
Package: `rcs` v0.2.0 · Edition 2021 · MSRV 1.86

---

## 1. System Overview

RCS Nexus is a multi-tenant REST API built on Actix-Web 4 backed by PostgreSQL (Diesel ORM). Its primary domain is Brazilian fiscal document management (NF-e / SEFAZ integration). It exposes a JSON API secured by Keycloak OAuth 2.0 / JWT, with WebSocket-based live log streaming and an optional functional programming subsystem (`rcs-functional`).

```
┌────────────────────────────────────────────────────────────┐
│  Client (HTTP/WS)                                          │
└───────────────────────────┬────────────────────────────────┘
                            │
┌───────────────────────────▼────────────────────────────────┐
│  Actix-Web HttpServer                                      │
│  Middleware stack (outermost→innermost):                   │
│    CORS → FunctionalAuthentication → TracingLogger → Session│
└───────────────────────────┬────────────────────────────────┘
                            │
┌───────────────────────────▼────────────────────────────────┐
│  Controllers (api/)                                        │
│  user · tenant · account · address_book                    │
│  nfag · nfe_document · eventos · cons_sit · cons_stat      │
│  ping · health · ws · openapi · functional_operations      │
└────────────┬──────────────────────────┬────────────────────┘
             │                          │
┌────────────▼────────┐    ┌────────────▼────────────────────┐
│  Services (services/)│    │  rcs-functional (optional)      │
│  user · tenant       │    │  PureFunctionRegistry           │
│  account · nfe_doc   │    │  IteratorEngine · LazyPipeline  │
│  address_book        │    │  ValidationEngine               │
│  functional_patterns │    │  UnifiedPagination              │
└────────────┬────────┘    └─────────────────────────────────┘
             │
┌────────────▼────────────────────────────────────────────────┐
│  Database (Diesel / PostgreSQL)                             │
│  TenantPoolManager → per-tenant r2d2 Pool                  │
│  schema.rs auto-generated via build.rs + diesel CLI        │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. Workspace Layout

```
nexus/
├── src/                        Main crate (rcs)
│   ├── main.rs                 Server startup, env validation, middleware config
│   ├── lib.rs                  Module tree; feature-gated functional stubs
│   ├── error.rs                ServiceError, ErrorContext, error_pipeline, error_logging
│   ├── schema.rs               Auto-generated Diesel schema (do not edit manually)
│   ├── pagination.rs           Legacy index-based pagination (DEPRECATED in favour of unified_pagination)
│   ├── types.rs                Shared newtype aliases
│   ├── constants.rs            App-wide constants
│   ├── api/                    HTTP handlers (controllers)
│   ├── config/                 db pool, cache (Redis), app routing
│   ├── middleware/             auth_middleware, functional_middleware, ws_security
│   ├── models/                 Diesel models + domain structs
│   ├── services/               Business logic layer
│   └── utils/                  keycloak, token_utils, ws_logger
│
├── functional_lib/             Sub-crate (rcs-functional)
│   └── src/
│       ├── lib.rs              Module root; re-exports IteratorEngine, ChainBuilder, PureFunctionRegistry
│       ├── iterator_engine.rs  SafeIterator + IteratorChain processing
│       ├── chain_builder.rs    Fluent builder for iterator chains
│       ├── pure_function_registry.rs  Thread-safe function registry
│       ├── lazy_pipeline.rs    Deferred computation steps
│       ├── immutable_state.rs  Persistent state via `im` crate
│       ├── state_transitions.rs Higher-level state machine
│       ├── query_builder.rs    Column<T,C> type-safe refs
│       ├── query_builders.rs   Predicate combinators
│       ├── query_composition.rs Diesel query composition helpers
│       ├── validation_engine.rs Iterator-based validators
│       ├── validation_rules.rs  Built-in rules (Custom, Required, etc.)
│       ├── validation_integration.rs Integration with Diesel models
│       ├── unified_pagination.rs AES-GCM cursor pagination (RECOMMENDED)
│       ├── pagination.rs        Legacy iterator pagination
│       ├── concurrent_processing.rs Rayon-based parallel ops
│       ├── parallel_iterators.rs par_iter helpers
│       ├── response_transformers.rs Composable HttpResponse builders
│       ├── performance_monitoring.rs Pipeline metrics singleton
│       ├── math_functions.rs   Pure math helpers
│       ├── models.rs           Shared data types for functional lib
│       ├── constants.rs        Functional lib constants
│       ├── prelude.rs          Re-exports for convenience
│       └── schema.rs           Diesel schema mirror for functional queries
│
├── tests/                      Integration tests (testcontainers)
├── benches/                    Criterion benchmarks
├── examples/                   pipeline_metrics_demo
├── build.rs                    Calls diesel print-schema on debug builds
└── Cargo.toml                  Workspace root; feature flags
```

---

## 3. Environment Variables

> **SECURITY — Production Checklist**
>
> Several variables have insecure dev-only defaults that **must** be overridden before deploying to production.
> Startup **must** fail when `APP_ENV != dev` and any of the following conditions hold:
>
> | Variable | Insecure default | Required production value |
> |----------|-----------------|--------------------------|
> | `SESSION_COOKIE_SECURE` | `false` | Must be `true` (HTTPS only) |
> | `KEYCLOAK_ISSUER_URL` | `http://localhost:8080/realms/middleware` | Your real Keycloak realm URL |
> | `KEYCLOAK_MIDDLEWARE_APP_SECRET` | `middleware-app-secret-dev` | A strong, randomly generated secret |
> | `SESSION_ENCRYPTION_KEY` | auto-generated (new key on every restart) | A stable base64-encoded 64-byte key stored in a secrets manager |
>
> Enforce this by adding startup guards analogous to the existing `KEYCLOAK_MIDDLEWARE_APP_SECRET` check:
> return an `io::Error` when `APP_ENV != "dev"` and any insecure default would be used.

| Variable | Required | Default | Notes |
|----------|----------|---------|-------|
| `APP_HOST` | yes | — | Bind address (e.g. `0.0.0.0`) |
| `APP_PORT` | yes | — | Bind port (e.g. `8000`) |
| `APP_ENV` | no | — | Set to `dev` to enable dev-only fallbacks; any other value activates prod guards |
| `DATABASE_URL` | yes | — | PostgreSQL DSN |
| `REDIS_URL` | no | — | Redis DSN; cache disabled if absent |
| `KEYCLOAK_ISSUER_URL` | **yes** | `http://localhost:8080/realms/middleware` ⚠️ | Required in production; dev fallback is localhost |
| `KEYCLOAK_CLIENT_ID` | no | `middleware-app` | |
| `KEYCLOAK_MIDDLEWARE_APP_SECRET` | **yes** | `middleware-app-secret-dev` ⚠️ (dev only) | Required unless `APP_ENV=dev` |
| `KEYCLOAK_REDIRECT_URL` | no | `http://localhost:8000/api/callback` | |
| `SESSION_ENCRYPTION_KEY` | **yes** | auto-generated ⚠️ (dev only) | Base64-encoded 64-byte key; auto-generation invalidates sessions on restart |
| `SESSION_COOKIE_SECURE` | no | `false` ⚠️ | **Must be `true` in production** |
| `CURSOR_ENCRYPTION_KEY` | yes (functional) | — | Base64-encoded 32-byte AES-GCM key |
| `CORS_ALLOW_CREDENTIALS` | no | `false` | Set `true` only if cookies required cross-origin |
| `WS_LOG_BUFFER_SIZE` | no | `1000` | Ring buffer for WebSocket log broadcast |
| `RUST_LOG` | no | `info` | Tracing filter directives |

---

## 4. Feature Flags

```toml
[features]
default = ["functional", "performance_monitoring", "datetime"]

functional          # links rcs-functional; enables registry, cursor pagination, iterators
performance_monitoring  # enables PerformanceMonitor instrumentation
datetime            # enables chrono + diesel/chrono
hybrid-db           # links oracle crate (non-default; experimental)
```

Build without functional subsystem:
```bash
cargo build --no-default-features --features datetime
```

---

## 5. Error Model

All domain errors are `ServiceError` variants. Services return `ServiceResult<T> = Result<T, ServiceError>`.

### Creating errors

```rust
// Simple
ServiceError::not_found("User not found")

// With context
ServiceError::bad_request("Invalid email")
    .with_detail("email must contain @")
    .with_correlation_id(request_id)
    .with_tag("validation")
    .with_metadata("field", "email")
```

### Propagating errors (service layer)

```rust
// Use ? via ServiceResultExt
let user = repo.find(id)?;

// Attach context to upstream errors
repo.find(id)
    .attach_context(|ctx| ctx.with_tag("user_lookup"))?;

// Guard post-condition
repo.find(id)
    .ensure(|u| u.is_active, || ServiceError::unauthorized("Account disabled"))?;
```

### HTTP response

`ServiceError` implements `actix_web::ResponseError`. The response body shape:

```json
{
  "message": "User not found",
  "data": {
    "code": "REQ-404",
    "message": "User not found",
    "timestamp": 1712000000,
    "status": 404,
    "detail": "user id abc was not found",
    "correlation_id": "corr-xyz",
    "tags": ["user_lookup"],
    "metadata": { "id": "abc" }
  }
}
```

---

## 6. Service Layer Pattern

Services receive a `Pool` and interact with the DB via `ServicePipeline` or `FunctionalQueryService`.

```rust
// Preferred: functional pipeline
FunctionalQueryService::new(pool.clone())
    .query_optional(|conn| {
        users::table
            .filter(users::id.eq(id))
            .first::<User>(conn)
            .optional()
            .map_err(|e| ServiceError::internal_server_error(e.to_string()))
    })

// With validation + transformation
FunctionalQueryService::new(pool.clone())
    .query_with_pipeline(
        input,
        |data, conn| { /* db op */ },
        NonEmptyStringValidation,
        |data| Ok(data.trim().to_string()),
    )
```

Controllers receive `web::Data<Pool>` (or `web::Data<TenantPoolManager>`) and delegate to services.

---

## 7. Pagination

### Unified (recommended) — cursor-based, AES-GCM encrypted

```rust
// Decode incoming cursor from request
let cursor = UnifiedPaginationCursor::decode(&encrypted_string, &key)?;

// Encode next cursor for response
let next_cursor = cursor.next_page().encode(&key)?;
```

Requires `CURSOR_ENCRYPTION_KEY` env var (32 bytes, base64). Validated at startup.

### Legacy — index-based (avoid for new features)

```rust
let pagination = Pagination::new(page, page_size);
let offset = pagination.offset(); // page × page_size
```

---

## 8. Functional Library Usage

### PureFunctionRegistry

```rust
let registry = PureFunctionRegistry::shared(); // Arc<PureFunctionRegistry>

// Register
registry.register(FunctionWrapper::new(
    |x: i32| x * 2,
    "double",
    FunctionCategory::Mathematical,
))?;

// Execute
let result: Option<i32> = registry.execute(
    FunctionCategory::Mathematical, "double", 5
)?; // Some(10)

// NOTE: compose_functions() is currently unimplemented — do not use
```

### ValidationEngine

```rust
let engine = ValidationEngine::<User>::new();
let engine = engine.validate_field(
    &user,
    "email",
    vec![Custom::new(|v| v.contains('@'), "INVALID_EMAIL", "must contain @")],
);
if !engine.errors.is_empty() {
    return Err(ServiceError::bad_request("Validation failed"));
}
```

---

## 9. Multi-Tenancy

> **SECURITY — Do not derive tenant identity from a client-supplied header.**
> The `x-tenant-id` header is attacker-controlled and allows tenant impersonation if trusted directly.
> Resolve tenant from a server-controlled source instead (in order of preference):
> 1. **JWT claim** — extract `tenant_id` from the validated Keycloak JWT (e.g. a custom claim set by Keycloak mappers)
> 2. **Request subdomain** — parse the `Host` header server-side (e.g. `acme.example.com` → tenant `acme`)
> 3. **mTLS certificate** — extract from the client certificate's Subject CN or SAN when mTLS is enforced

The `TenantPoolManager` resolves a tenant identifier to a dedicated `Pool`. The controller context extracts the pool from a **server-controlled** source:

```rust
/// Extract tenant_id from the validated JWT claims (preferred).
/// The JWT has already been verified by FunctionalAuthentication middleware.
fn extract_tenant_from_token(req: &HttpRequest) -> Option<String> {
    // Claims are injected into request extensions by auth middleware
    req.extensions().get::<ValidatedClaims>()
        .and_then(|claims| claims.tenant_id.clone())
}

/// Fallback: parse tenant from the request subdomain.
fn extract_tenant_from_host(req: &HttpRequest) -> Option<String> {
    req.connection_info()
        .host()
        .split('.')
        .next()
        .filter(|s| !s.is_empty() && *s != "www")
        .map(str::to_owned)
}

async fn handler(
    manager: web::Data<TenantPoolManager>,
    req: HttpRequest,
) -> Result<HttpResponse, ServiceError> {
    // Try JWT claim first, then subdomain — never trust a client-supplied header
    let tenant_id = extract_tenant_from_token(&req)
        .or_else(|| extract_tenant_from_host(&req))
        .ok_or_else(|| ServiceError::unauthorized("tenant context could not be determined"))?;

    let pool = manager.get_tenant_pool(&tenant_id)
        .ok_or_else(|| ServiceError::not_found("tenant not found"))?;
    // ...
}
```

---

## 10. Testing

```bash
# Unit tests (no Docker required)
cargo test --lib

# Integration tests (require Docker for testcontainers)
cargo test --test functional_tests
cargo test --test test_pagination_performance

# Benchmarks
cargo bench

# Full suite
cargo test
```

Integration tests use `testcontainers` to spin up a real PostgreSQL instance. Tests that fail to connect to Docker skip gracefully rather than failing.

---

## 11. Build & Run

```bash
# Dev
cargo run

# Release
cargo build --release
./target/release/rcs

# Schema regeneration (requires diesel CLI)
diesel print-schema > src/schema.rs
# or triggered automatically on cargo build (debug only)

# Linting
cargo clippy -- -D warnings
cargo fmt --check
```

---

## 12. Known Limitations & Debt

| Area | Issue | Tracking |
|------|-------|---------|
| `PureFunctionRegistry::compose_functions` | Public API always returns error ("not yet implemented") | Pack 3 |
| `SESSION_COOKIE_SECURE` | Defaults to `false` — must be explicitly set in prod | Pack 5 |
| `src/lib.rs` functional stubs | Dual maintenance with `rcs-functional` crate | Pack 2 |
| `src/pagination.rs` | Legacy system with no deprecation annotation | Pack 2 |
| `SafeIterator::next` | Catches panics via `catch_unwind` — unsound for `!UnwindSafe` iterators | Pack 4 |
| Hebrew comments | Non-English inline comments in `main.rs` and `functional_service_base.rs` | Pack 5 |
| Dual password hashers | Both `bcrypt` and `argon2` are dependencies; active one is unclear | — |
