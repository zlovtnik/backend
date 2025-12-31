<!-- Repository-specific guidance for AI coding agents. Keep concise and actionable. -->
# Copilot instructions for dispo-rusty (backend)

## Purpose
Help AI agents be immediately productive in this multi-tenant Actix Web + Diesel Rust backend with advanced functional programming patterns.

## Architecture & Core Concepts
- **Multi-tenant Isolation**: Single PostgreSQL database using per-tenant schemas. Tenant context is ALWAYS derived server-side (host/subdomain, mTLS, or lookup). NEVER trust client-supplied tenant IDs.
- **Functional Programming**: Leverages `rcs-functional` library for pure functions, iterator chains (`ChainBuilder`), and immutable state management.
- **NFAg Domain**: Implements Nota Fiscal Eletrônica da Água (NFAg) with complex XSD-based validation and multi-tenant CRUD operations.
- **Real-time Observability**: Logs streamed over WebSocket at `/api/ws/logs` using `rcs::utils::ws_logger`.

## Key Files & Directories
- **Entry Points**: [src/main.rs](src/main.rs), [src/lib.rs](src/lib.rs).
- **Functional Core**: [functional_lib/src/lib.rs](functional_lib/src/lib.rs) and `src/functional/` (re-exported as `functional`).
- **API Controllers**: [src/api/](src/api/) (e.g., [nfag_controller.rs](src/api/nfag_controller.rs)).
- **Configuration**: [src/config/app.rs](src/config/app.rs) (route registration), [src/config/db.rs](src/config/db.rs) (tenant pool manager).
- **Middleware**: [src/middleware/auth_middleware.rs](src/middleware/auth_middleware.rs).

## Conventions & Patterns
- **Route Building**: Use `RouteBuilder` in [src/config/app.rs](src/config/app.rs) for composable route registration.
- **Database Access**: Use `TenantPoolManager` to obtain per-tenant connection pools: `manager.get_pool(&tenant_id)?`.
- **Functional Logic**: Prefer `PureFunctionRegistry` and `ChainBuilder` over ad-hoc imperative logic.
- **Validation**: Use `ValidationEngine` for iterator-based validation pipelines, especially for NFAg XSD compliance.
- **Backward Compatibility**: Use `BackwardCompatibilityValidator` in [src/api/health_controller.rs](src/api/health_controller.rs) to ensure functional enhancements don't break existing APIs.
- **Pagination**: Use `unified_pagination` with encrypted cursors (AES-256-GCM).

## Developer Workflows
- **Local Dev**: `cargo run` (reads `.env`).
- **Migrations**: `diesel migration generate <name>` then `diesel migration run`.
- **Testing**:
  - Unit: `cargo test`.
  - Integration: Requires Docker. Run with `cargo test --test integration_tests`.
  - Log Streaming: Run with `cargo test -- --test-threads=1` to avoid race conditions.
- **Code Quality**: `cargo clippy` and `cargo fmt`.

## Code Edit Guidelines
- **Preserve Tenant Isolation**: Any change to routing, DB, or auth must preserve server-derived tenant context.
- **Functional Style**: Leverage `rcs-functional` for iterator chains and state transitions.
- **Migrations**: Add timestamped SQL migrations in `migrations/`. Do not edit `src/schema.rs` manually.
- **Middleware Order**: Tracing -> Auth -> App Handlers. See [src/main.rs](src/main.rs).

## Examples
- **Obtain Tenant Pool**: `let pool = manager.get_pool(&tenant_id)?;`
- **Functional Route**: `RouteBuilder::new().add_route(|cfg| { cfg.service(health); }).build(cfg);`
- **Encrypted Cursor**: `let cursor = IdCursor::new(id).encode()?;`
- **Compatibility Test**: `let validator = BackwardCompatibilityValidator::new(config); validator.run_full_compatibility_suite().await;`

