# RCS Nexus — Wordmap

Vocabulary of the codebase organized by semantic cluster. Each term maps to where it lives, what it means in this project's context, and how it relates to adjacent terms.

---

## Cluster A · Runtime & HTTP Layer

```
actix-web
  └─ HttpServer          entry point in main.rs; binds to APP_HOST:APP_PORT
  └─ App                 per-worker application factory; receives middleware stack
  └─ web::Data<T>        dependency injection mechanism for shared state
  └─ ResponseError       trait impl on ServiceError → converts domain errors to HTTP responses
  └─ TracingLogger       tracing-actix-web middleware; injects request-id into spans

Middleware stack (applied outermost-last, executes outermost-first):
  CORS                   ← outermost (added last via .wrap(cors))
  FunctionalAuthentication ← JWT validation via PureFunctionRegistry
  TracingLogger          ← request span/trace injection
  SessionMiddleware      ← cookie-based OAuth state storage (10-min TTL)
```

---

## Cluster B · Domain Language (NFe / Brazilian Fiscal)

| Term | Module | Meaning |
|------|--------|---------|
| `NF-e` / `NFe` | `models/nfag*`, `models/nfe_*` | Nota Fiscal Eletrônica — Brazilian electronic invoice |
| `nfag` | `models/nfag.rs` | NF-e aggregate root model |
| `nfag_ide` | `models/nfag_ide.rs` | NF-e identification block (IDE) |
| `nfag_emit` | `models/nfag_emit.rs` | NF-e emitter (issuer) block |
| `nfag_dest` | `models/nfag_dest.rs` | NF-e recipient/destination block |
| `nfag_total` | `models/nfag_total.rs` | NF-e totals block |
| `nfe_document` | `models/nfe_document/` | Full NF-e document with operations and validators |
| `nfe_item` | `models/nfe_item.rs` | Line item on an NF-e |
| `nfe_product` | `models/nfe_product.rs` | Product master data |
| `nfe_emitter` | `models/nfe_emitter.rs` | Emitter entity (CNPJ, address, etc.) |
| `nfe_recipient` | `models/nfe_recipient.rs` | Recipient entity |
| `nfe_icms` | `models/nfe_icms.rs` | ICMS tax (state VAT) |
| `nfe_pis` | `models/nfe_pis.rs` | PIS tax (social contribution) |
| `nfe_cofins` | `models/nfe_cofins.rs` | COFINS tax (social contribution) |
| `nfe_ipi` | `models/nfe_ipi.rs` | IPI tax (industrialized products) |
| `evento_nfag` | `models/evento_nfag.rs` | NF-e lifecycle event (cancel, correct) |
| `cons_sit_nfag` | `models/cons_sit_nfag.rs` | Consult NF-e situation (SEFAZ query) |
| `cons_stat_serv_nfag` | `models/cons_stat_serv_nfag.rs` | Consult SEFAZ service status |
| `ret_nfag` | `models/ret_nfag.rs` | SEFAZ return/response envelope |
| `SEFAZ` | (implicit) | Brazilian tax authority — the external system |

---

## Cluster C · Multi-Tenancy

| Term | Location | Meaning |
|------|----------|---------|
| `tenant` | `models/tenant.rs`, `services/tenant_service.rs` | Isolated business entity; owns its DB pool |
| `TenantPoolManager` | `config/db.rs` | Registry mapping `tenant_id → Pool`; cloned per worker |
| `x-tenant-id` | `main.rs` CORS headers | HTTP header that routes requests to the correct pool |
| `tenant1` | `main.rs:217` | Hardcoded demo tenant; in production, loaded from DB |
| `Pool` | `config/db.rs` | `r2d2::Pool<diesel::r2d2::ConnectionManager<PgConnection>>` |
| `main_pool` | `main.rs` | The root DB pool shared across all tenants as default |

---

## Cluster D · Authentication & Security

| Term | Location | Meaning |
|------|----------|---------|
| `Keycloak` | `utils/keycloak.rs` | External OAuth 2.0 / OIDC identity provider |
| `KeycloakConfig` | `utils/keycloak.rs` | Issuer URL, client ID, client secret, redirect URL |
| `KeycloakClient` | `utils/keycloak.rs` | HTTP client wrapping the OIDC flow |
| `jwt` / `jsonwebtoken` | deps | Used for JWT validation in auth middleware |
| `SESSION_ENCRYPTION_KEY` | env var | 64-byte base64 key for cookie session encryption |
| `SESSION_COOKIE_SECURE` | env var | Whether session cookie requires HTTPS |
| `oauth_session` | `main.rs` | Cookie name for OAuth state (10-min TTL, HttpOnly, SameSite=Strict) |
| `CURSOR_ENCRYPTION_KEY` | env var | Base64 32-byte AES-GCM key for pagination cursors |
| `FunctionalAuthentication` | `middleware/auth_middleware.rs` | Actix middleware wrapping JWT validation with PureFunctionRegistry |
| `refresh_token` | `models/refresh_token.rs` | Stored refresh tokens for session renewal |
| `login_history` | `models/login_history.rs` | Audit log of authentication events |
| `bcrypt` / `argon2` | deps | Password hashing algorithms (both present; unclear which is active) |
| `secrecy::SecretString` | `main.rs:139` | Zero-on-drop wrapper for the Keycloak client secret |

---

## Cluster E · Error System

```
ServiceError (enum)
  ├─ Unauthorized        HTTP 401 / code AUTH-401
  ├─ InternalServerError HTTP 500 / code SRV-500
  ├─ BadRequest          HTTP 400 / code REQ-400
  ├─ NotFound            HTTP 404 / code REQ-404
  └─ Conflict            HTTP 409 / code REQ-409

Each variant carries:
  error_message: String      → human-readable, forwarded to HTTP response
  context: ErrorContext      → structured metadata (detail, tags, correlation_id, metadata)

ErrorContext (builder pattern)
  .with_detail()             → free-text additional info
  .with_tag()                → categorical tag (e.g. "db", "validation")
  .with_correlation_id()     → request-trace linkage
  .with_metadata(k, v)       → arbitrary key-value pairs

ErrorEnvelope               → the JSON body sent to the caller (includes timestamp, status, code)

ServiceResult<T>            = Result<T, ServiceError>
ServiceResultExt<T>         → extension trait: .attach_context(), .ensure(), .tap_error(), .log_on_error()

error_pipeline::             → module: process_sequence, collect_successes, build_error_reporter, Pipeline
error_logging::              → module: log_errors, log_errors_if, compose_transformers, chain_log_and_transform
monadic::                    → module: option_to_result, flatten_option
```

---

## Cluster F · Functional Library (`rcs-functional`)

```
IteratorEngine             iterator chain processing; SafeIterator wraps items with panic-catch
ChainBuilder               fluent builder for iterator chains
PureFunctionRegistry       thread-safe HashMap<FunctionCategory, HashMap<sig, FunctionContainer>>
  └─ FunctionCategory      Transformation | Mathematical | StringProcessing | ...
  └─ FunctionWrapper       newtype around a closure implementing PureFunction<I,O>
  └─ FunctionContainer     type-erased wrapper holding Any + call fn
  └─ RegistryMetrics       avg_lookup_time_ns, lookup_count, composition_count
  └─ SharedRegistry        Arc<PureFunctionRegistry>

LazyPipeline               deferred computation chains (Box<dyn Fn> steps)
ImmutableState             functional state with structural sharing (im crate)
StateTransitions           higher-level state machine helpers

QueryBuilder               Column<T,C> type-safe column refs
QueryComposition           predicate combinators for Diesel queries

ValidationEngine           iterator-based validation pipeline
ValidationRules            Custom, Required, Length, Regex validators
ValidationError            { field, code, message }

UnifiedPagination          AES-GCM encrypted opaque cursor
  └─ CursorPayload         { offset, limit, sort_key, direction, checksum }
  └─ CURSOR_ENCRYPTION_KEY 32-byte key from env, validated at startup
  └─ validate_cursor_encryption_key() called in main before server starts

Pagination (legacy)         index-based cursor; offset = cursor × page_size
  └─ FusedIterator impl    prevents re-polling after exhaustion

PerformanceMonitor          singleton via get_performance_monitor(); tracks pipeline metrics
OperationType               enum: Custom(String) + built-in variants
Measurable                  trait: record(op_type, duration)

ConcurrentProcessing        Rayon-based parallel map/reduce
ParallelIterators           par_iter helpers over functional collections

ResponseTransformer<T>      Cow<'static, str> message + StatusCode + metadata → HttpResponse
```

---

## Cluster G · Database Layer

| Term | Location | Meaning |
|------|----------|---------|
| `diesel` | dep | ORM + query builder; PostgreSQL via `r2d2` |
| `schema.rs` | `src/schema.rs` | Auto-generated by `build.rs` via `diesel print-schema` |
| `build.rs` | root | Runs `diesel print-schema` on debug builds; skips on release |
| `migrations/` | (implied) | Diesel migration SQL files; run at startup via `run_migration` |
| `PgConnection` | diesel | Sync PostgreSQL connection |
| `r2d2::Pool` | config/db | Connection pool; max size configured via env |
| `BigDecimal` | dep | Used for monetary/tax values in NF-e models |
| `RustDecimal` | dep | Alternative decimal type (also present; feature `db-diesel-postgres`) |

---

## Cluster H · Crate Features & Build

| Feature | Default | Effect |
|---------|---------|--------|
| `functional` | yes | Links `rcs-functional` crate; enables iterator engine, registry, pagination |
| `performance_monitoring` | yes | Enables `PerformanceMonitor` instrumentation hooks |
| `datetime` | yes | Enables `chrono` and `diesel/chrono` integration |
| `hybrid-db` | no | Links `oracle` crate for Oracle DB support |

```
Profiles:
  dev     → opt-level 1, codegen-units 16, incremental=true
  release → opt-level 3, LTO fat, codegen-units 1, strip=true, panic=abort
```

---

## Cluster I · Supporting Infrastructure

| Term | Location | Meaning |
|------|----------|---------|
| `LogBroadcaster` | `utils/ws_logger.rs` | Broadcasts log lines to WebSocket subscribers |
| `WS_LOG_BUFFER_SIZE` | env var | Ring-buffer capacity for log lines (default 1000) |
| `ws_controller` | `api/ws_controller.rs` | WebSocket endpoint serving live log stream |
| `ws_security` | `middleware/ws_security.rs` | CORS origin allowlist for WebSocket connections |
| `openapi` | `api/openapi.rs` | utoipa-generated OpenAPI spec + Swagger UI |
| `crud_engine` | `api/crud_engine.rs` | Generic CRUD handler factory |
| `controller_context` | `api/controller_context.rs` | Request-scoped context (pool, tenant, auth) |
| `person` | `models/person.rs` | CPF/CNPJ person entity with validators |
| `address_book` | `services/address_book_service.rs` | CRUD for contact/address entities |
| `quick-xml` | dep | XML serialization for NF-e SEFAZ payloads |
| `im` crate | dep | Persistent immutable collections (used in ImmutableState) |
| `rayon` | dep (optional) | Data-parallelism; gated by `rayon` feature in main / required in functional_lib |
| `notify` | dep | File system watching (used for config hot-reload?) |
