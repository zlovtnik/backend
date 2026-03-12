# Rust Backend Refactor Audit (Wave 1)

Date: 2026-03-09
Scope: `/src` (backend crate), pre-refactor baseline and migration scaffolding.

## 1) Module Inventory (src/)

### Runtime + Wiring
- `src/main.rs`
  - Responsibility: Tokio runtime bootstrap, concurrent HTTP + gRPC startup, graceful shutdown signal fanout.
  - Imports: `tokio`, `anyhow`, runtime config/state + transport runners.
  - Layer fit: Composition root.

- `src/lib.rs`
  - Responsibility: module graph/export surface.
  - Imports: crate-local modules.
  - Layer fit: crate boundary.

### Config + State
- `src/config/mod.rs`, `src/config/runtime.rs`
  - Responsibility: typed app configuration loading (`config` + `dotenvy`).
  - Imports: `config`, `serde`, `anyhow`.
  - Layer fit: infrastructure bootstrap.

- `src/config/app.rs`
  - Responsibility: route registration and OpenAPI wiring for Actix.
  - Imports: `actix_web`, API modules.
  - Layer fit: HTTP adapter routing.

- `src/config/db.rs`
  - Responsibility: Diesel/r2d2 pool management and tenant pool registry.
  - Imports: `diesel`, `r2d2`, `RwLock<HashMap<...>>`.
  - Layer fit: DB adapter (legacy).

- `src/config/cache.rs`
  - Responsibility: Redis pool manager with optional initialization.
  - Imports: `redis`, `r2d2`.
  - Layer fit: outbound adapter.

- `src/state.rs`
  - Responsibility: unified `AppState` (`Arc`), shared dependencies, session key, sqlx pool map.
  - Imports: runtime config, keycloak, db/cache pools, `sqlx`.
  - Layer fit: composition/infrastructure.

### Error Boundaries
- `src/error.rs`
  - Responsibility: legacy `ServiceError` + response envelope + response integration.
  - Imports: `actix_web`, `serde`, `tracing`.
  - Layer fit: shared error boundary (legacy).

- `src/app_error.rs`
  - Responsibility: unified transport-agnostic `AppError` + Actix/Tonic mapping.
  - Imports: `thiserror`, `actix_web::ResponseError`, `tonic::Status`, `sqlx`.
  - Layer fit: shared error boundary (new).

### Domain / Ports / Services (new)
- `src/domain/mod.rs`, `src/domain/health.rs`
  - Responsibility: pure domain type(s).
  - Imports: `serde` only.
  - Layer fit: domain.

- `src/ports/mod.rs`, `src/ports/repository.rs`, `src/ports/outbound.rs`
  - Responsibility: repository/outbound abstractions (`async_trait`, Send+Sync).
  - Imports: `async_trait`.
  - Layer fit: ports.

- `src/services/mod.rs`, `src/services/core/mod.rs`, `src/services/core/health_service.rs`
  - Responsibility: framework-agnostic orchestration for core health use case.
  - Imports: domain + ports only.
  - Layer fit: application services.

### Adapters (new)
- `src/adapters/mod.rs`
  - Responsibility: adapter namespace.

- `src/adapters/http/mod.rs`, `src/adapters/http/server.rs`
  - Responsibility: Actix server factory/run loop + middleware/data wiring through `AppState`.
  - Imports: `actix-web`, `actix-session`, `tracing-actix-web`.
  - Layer fit: HTTP adapter.

- `src/adapters/grpc/mod.rs`, `src/adapters/grpc/server.rs`, `src/adapters/grpc/health_impl.rs`
  - Responsibility: Tonic server setup and gRPC handler(s) delegating to services.
  - Imports: `tonic`, `tower-http`.
  - Layer fit: gRPC adapter.

- `src/adapters/db/mod.rs`, `src/adapters/db/tenant_sqlx_repository.rs`
  - Responsibility: sqlx repository implementation for tenant-scoped DB checks.
  - Imports: `sqlx`, `async_trait`.
  - Layer fit: DB adapter.

### Existing HTTP API Surface (legacy)
- Controllers in `src/api/*.rs` (account, tenant, user, nfag, health, ws, etc.) remain in place for strict compatibility.
  - Responsibility: transport handlers; some still contain DB/business orchestration (layering violation to migrate incrementally).
  - Imports: `actix_web`, model/service modules, Diesel pool extraction.

### Existing Legacy Services / Models / Middleware / Utils
- `src/services/*.rs` (legacy), `src/models/**/*.rs`, `src/middleware/*.rs`, `src/utils/*.rs`
  - Responsibility: current production logic (auth/session, tenant mgmt, domain entities, keycloak, websockets).
  - Status: retained for non-breaking compatibility; migration boundary introduced via new modules.

## 2) Async Boundary Audit

### Blocking / sync pressure points
- Sync DB acquisition in async handlers (`pool.get()` appears broadly in `src/api/*`).
- `web::block` present in NFAg/event controllers (`src/api/ret_nfag_controller.rs`, `src/api/evento_nfag_controller.rs`).
- `spawn_blocking` already used in health/tenant checks (`src/api/health_controller.rs`, `src/api/tenant_controller.rs`).
- Blocking HTTP client still present for token validation path: `reqwest::blocking::Client` (`src/utils/keycloak.rs`).
- `std::thread::sleep` present in functional utility paths (`src/services/functional_patterns.rs`).

### Sequential-await parallelization candidates
- Health and tenant aggregation paths have independent checks that can be consolidated under bounded concurrent patterns.
- Token/JWKS flows should stay fully async in request paths; sync fallback retained only for compatibility.

## 3) Shared State Audit

### Current state distribution (legacy)
- Multiple separate `web::Data<T>` injections in startup (manager, DB pool, redis option, broadcaster, keycloak client).
- Global lazy cache in Keycloak module (`once_cell::sync::Lazy` + `RwLock<HashMap<...>>`).

### New target state
- `AppState` introduced as single `Arc` state root with explicit getters.
- Contains tenant manager, legacy pools/clients, session key, and sqlx pool map (per-tenant ready).

## 4) gRPC Surface Audit

- Current baseline includes additive gRPC scaffolding only:
  - `proto/nexus/core/core.proto` with `HealthService.Check` (unary).
  - generated module wiring in `build.rs`.
  - Tonic server bootstrap in `src/adapters/grpc/server.rs`.
- No existing auth/user/tenant/nfag parity RPCs are implemented yet; HTTP remains the only business transport.

### Target RPC Matrix (wave 1 additive scope)

| Domain | Existing HTTP surface | Target gRPC service | Notes |
| --- | --- | --- | --- |
| Health | `/health`, `/health/detailed` | `HealthService.Check` | Already partially implemented; can stay coarse-grained. |
| Auth | `/api/auth/login`, `/logout`, `/refresh`, `/refresh-token`, `/me` | `AuthService` | Preserve HTTP envelope/status mapping; gRPC remains additive. |
| User | `/api/users/*` | `UserService` | Reuse same service layer as HTTP. |
| Tenant | `/api/admin/*`, `/api/tenants/*` | `TenantService` | Preserve per-tenant pool resolution semantics. |
| NFAg | `/api/nfag/*` | `NfagService` | Core CRUD parity first, no functional module spillover. |

## 5) Hot Path Markers Added

- `src/middleware/auth_middleware.rs`: authentication middleware `call` path.
- `src/api/account_controller.rs`: `login` and `refresh`.
- `src/api/health_controller.rs`: `health_detailed`.
- `src/api/tenant_controller.rs`: `get_system_stats`.
- `src/api/nfag_controller.rs`: `create` and `find_all`.
- `src/utils/keycloak.rs`: `validate_id_token_internal`.

## Baseline Gates (current workspace state)

- `cargo check` (with `RUSTC_WRAPPER=`) passes before strict refactor gates.
- Workspace-wide strict clippy currently fails due existing `functional_lib` warnings; backend refactor quality gates remain scoped to backend crate in this wave.
