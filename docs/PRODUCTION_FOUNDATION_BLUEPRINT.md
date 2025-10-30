# Production Readiness Blueprint

_Date: October 29, 2025_

## Overview

This document distills the previously defined multi-phase plan into actionable work streams, grounded in the current state of the repository at `dev`. Each section outlines:

- **Current Baseline** – what exists today, with direct code references.
- **Target Additions** – the concrete deliverables required to fulfill the plan.
- **Considerations** – sequencing, dependencies, or risks to track while executing the work.

The goal is to make the scope implementable in tractable increments while preserving the strategic intent of the original roadmap.

---

## Phase 1 · Production Foundation (Weeks 1–4)

### 1. Testing Expansion

#### Testing · Current Baseline

- `tests/` contains only three files focused on functional behaviours (`tests/functional_tests.rs`, `tests/test_pagination_performance.rs`, `tests/ws_logger_zero_capacity_test.rs`). There are no unit suites covering controllers, services, middleware, validators, or token utilities.
- Service modules such as `src/services/account_service.rs` (729 lines) and `src/services/tenant_service.rs` lack direct unit coverage despite complex functional pipelines.

#### Testing · Target Additions

- Introduce the unit-test hierarchy under `tests/unit/` (`services/`, `controllers/`, `middleware/`, `validators/`, `utils/`).
- Author focused suites for:
  - `account_service` (signup/login/logout/refresh flows, retry logic, refresh-token rotation).
  - `tenant_service` (statistics aggregation, pagination, CRUD readers).
  - `auth_middleware` and supporting token utilities.
  - Validation modules (user, person, NFe, tenant validators).
- Stand up `tests/integration/{api,common}/` harness with reusable server/bootstrap utilities (testcontainers-backed Postgres/Redis, fixture helpers, authenticated client helpers).
- Implement end-to-end flows (`auth_flow_test.rs`, `tenant_isolation_test.rs`) covering signup → login → authz → refresh → logout and multi-tenant data isolation.

#### Testing · Considerations

- Reuse the functional `QueryReader` patterns by injecting mock pools or leveraging an in-memory database strategy for deterministic unit tests.
- Integration harness should mirror production config (`src/config/db.rs`, Redis pool) to exercise tenant selection logic end-to-end.

### 2. Security Hardening

#### Security · Current Baseline

- `src/models/user_token.rs` still falls back to reading `src/secret.key`; the binary file `src/secret.key` is committed to the repo.
- `.gitignore` does not exclude `*.key`, allowing secrets to be checked in.
- `UserToken::generate_token` unwraps `jsonwebtoken::encode`, risking panics on encoding errors.
- `token_utils::decode_token` uses `Validation::default()` without explicit algorithm/claim validation, and `verify_token` returns string errors rather than structured context.
- `account_service::logout` only calls `user_ops::logout_user` and does **not** revoke outstanding refresh tokens (`RefreshToken` operations exist but are unused during logout).
- No middleware enforces rate limiting, security headers, or correlation IDs (`src/middleware` currently provides `auth_middleware.rs`, `functional_middleware.rs`, `ws_security.rs`).

#### Security · Target Additions

- Delete `src/secret.key` and add secret/credential patterns to `.gitignore`.
- Harden secret loading: require `JWT_SECRET` in production, validate length (`>= 32 bytes`), log masked metadata, downgrade file fallback to debug builds only.
- Make `UserToken::generate_token` return `Result` instead of panicking; propagate errors to callers.
- Enhance `token_utils` with explicit HS256 validation, strict claim checks (`tenant_id`, `user`, `login_session`), structured error types, and retry-aware logging.
- Update `account_service::logout` to revoke refresh tokens atomically alongside session invalidation.
- Introduce new middleware modules (`request_id`, `security_headers`, `rate_limit`), exporting them via `src/middleware/mod.rs`.

#### Security · Considerations

- Coordinate secret rotation for all environments when removing the committed key; document mitigation steps in deployment guides.
- Rate limiting middleware should degrade gracefully (fail-open) if Redis is unavailable, and expose counters for observability.

### 3. Observability Foundation

#### Observability · Current Baseline

- Logging relies on `tracing` but lacks enforced JSON formatting or correlation IDs in the current middleware chain (`main.rs` initializes tracing primarily for WebSocket logging).
- No Prometheus metrics exporter exists (`src/observability/` directory absent).
- Health controller (`src/api/health_controller.rs`) offers a single readiness endpoint without liveness/readiness separation or structured payloads.

#### Observability · Target Additions

- Create `src/observability/{mod.rs,metrics.rs,tracing_config.rs}` to initialize Prometheus registry, HTTP metrics middleware, and production-grade logging.
- Register `/metrics` route returning Prometheus text format; capture HTTP/auth/database/cache metrics listed in the plan.
- Add request ID middleware and ensure `ServiceError` surfaces correlation IDs in responses.
- Expand health endpoints: `GET /health/live` (process heartbeat) and enhanced readiness with dependency checks, standardized JSON structure, tighter timeouts.
- Update `main.rs` middleware stack order: `RequestId` → `SecurityHeaders` → `TracingLogger` → `RateLimit` → `Authentication`.
- Validate critical env vars on startup (JWT secret, DATABASE_URL, REDIS_URL, CURSOR_ENCRYPTION_KEY) and log sanitized configuration summaries.

#### Observability · Considerations

- Ensure Prometheus metrics do not expose tenant identifiers directly; use labels carefully.
- Document operational guidance (scrape interval, alerting thresholds) in the new observability docs.

### 4. Local & CI Tooling Enhancements

#### Tooling · Current Baseline

- `docker-compose.local.yml` only runs the API container; Postgres/Redis services absent.
- No `.github/` directory or CI workflows exist.
- Dockerfile is a single multi-stage build without explicit builder target reuse or security-hardening steps.

#### Tooling · Target Additions

- Expand `docker-compose.local.yml` with health-checked Postgres (13+) and Redis (6+) services, wire environment variables for the API container, and persist volumes for local data.
- Introduce `docker-compose.test.yml` for CI integration tests (Postgres/Redis/test-runner pattern).
- Add `.github/workflows/ci.yml` with lint, test (Rust MSRV/stable matrix), build, and security audit jobs; configure test services and coverage upload.
- Refine `Dockerfile` for cache efficiency (optional `cargo chef`), non-root runtime, metadata labels, healthcheck, and container registry readiness.

#### Tooling · Considerations

- Align CI workflow secrets (Codecov, registry credentials) with organization practices; document prerequisites in README.
- Keep MSRV (1.86) enforcement in CI to match `Cargo.toml` `rust-version`.

---

## Phase 2 · Deployment Infrastructure (Weeks 5–6)

### 5. Documentation & Operational Playbooks

#### Documentation · Current Baseline

- Existing docs (`docs/`) focus on functional refactoring history and performance reports; deployment guidance is minimal and spread across `README.md`.
- No consolidated API reference or deployment guide.

#### Documentation · Target Additions

- Create `docs/DEPLOYMENT_GUIDE.md` detailing environment setup, secret management, migrations, Docker/Docker Compose usage, CI/CD flow, and troubleshooting.
- Produce `docs/API_DOCUMENTATION.md` summarizing current endpoints (auth, tenants, address book, health, metrics) with request/response schemas, authentication requirements, and error envelopes.
- Update `README.md` to reference new docs, describe testing commands (unit/integration/coverage), outline observability endpoints, and summarize security improvements.

#### Documentation · Considerations

- Keep documentation synchronized with configuration defaults introduced in Phase 1 (e.g., rate-limiting env vars, metrics endpoint).
- Use docs as interim step toward eventual OpenAPI automation planned for Phase 3.

---

## Phase 3 · API Maturity (Weeks 7–8)

### 6. Specification & Standardization

#### Specification · Current Baseline

- Controllers (`src/api/*`) return JSON payloads but lack centralized response envelopes, versioning strategy, or machine-readable API spec.

#### Specification · Target Additions

- Generate OpenAPI documentation (manual or via tooling like `utoipa`) based on `docs/API_DOCUMENTATION.md`.
- Formalize response envelope and error format across controllers; evaluate introducing versioned routes (`/api/v1/…`).
- Ensure new validation tests from Phase 1 inform schema constraints in documentation.

#### Specification · Considerations

- Coordinate with client consumers before altering response shapes or introducing version headers.

---

## Phase 4 · Performance & Scale (Weeks 9–10)

### 7. Data Layer Optimization & Caching

#### Caching · Current Baseline

- Redis is wired for health checks but no tenant-aware caching module exists.
- `src/config/cache.rs` manages Redis pools but higher-level cache abstractions are absent.

#### Caching · Target Additions

- Add `src/cache/{mod.rs,tenant_cache.rs,cache_keys.rs,invalidation.rs}` implementing cache-aside helpers with tenant-scoped keys, TTL strategies, and invalidation utilities.
- Instrument cache operations with Prometheus metrics (hit/miss counters, latency histograms).
- Evaluate query-level optimizations and connection pool metrics once observability groundwork is in place.

#### Caching · Considerations

- Define cache invalidation policies upfront to avoid stale data across tenants; tie invalidation calls into service write paths.

### 8. Performance Profiling & Query Tuning

#### Performance · Target Additions

- Use the new metrics and logging infrastructure to identify hot paths.
- Apply database optimizations (Diesel query tuning, index verification) and document findings in existing performance ADRs.

---

## Cross-Cutting Implementation Notes

- **Sequencing**: Prioritize Phase 1 security fixes (secret removal, JWT hardening) before expanding test coverage to avoid rework.
- **Testing Infrastructure**: Plan to wire new integration tests into CI once the `docker-compose.test.yml` and GitHub workflow land.
- **Env Management**: Introduce `.env.example` updates reflecting new variables (rate limiting, security headers, metrics) alongside documentation changes.
- **Risk Mitigation**: Secret rotation and token handling changes require coordinated release notes; ensure rollback playbooks cover shifted behaviours (e.g., stricter JWT validation).

---

## Open Questions

1. **Secret Distribution** – Which secret management solution (e.g., Vault, AWS Secrets Manager) should back the `JWT_SECRET` once file fallback is removed?
2. **Rate Limit Policies** – What per-tenant/per-user thresholds align with product requirements, and should there be override configurations?
3. **Metrics Exposure** – Are there compliance constraints around exposing tenant identifiers in metrics labels? Masking strategy may be required.
4. **CI Resources** – Is the organization prepared to provision Codecov tokens and container registry credentials for the new workflow?
5. **API Versioning** – Does the consumer ecosystem require a compatibility window before enforcing response envelope changes?

Documenting and resolving these questions ahead of implementation will keep the execution path predictable and aligned with stakeholders.
