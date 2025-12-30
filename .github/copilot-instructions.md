<!-- Repository-specific guidance for AI coding agents. Keep concise and actionable. -->
# Copilot instructions for dispo-rusty (backend)

Purpose
- Help AI agents be immediately productive in this multi-tenant Actix Web + Diesel Rust backend.

Quick architecture summary
- Multi-tenant, single PostgreSQL database using per-tenant schemas. Tenant context is always derived server-side; clients' tenant IDs must be ignored.
- HTTP server: Actix Web. Authentication: JWT tokens carrying tenant context. DB access via Diesel with a tenant-aware pool manager.
- Real-time logs are streamed over WebSocket at `/api/ws/logs` using `rcs::utils::ws_logger`.
- Functional programming: Leverages `rcs-functional` library for pure functions, iterator chains, and immutable state management.

Key files to inspect (examples)
- README: [README.md](README.md#L1)
- Application entry: [src/main.rs](src/main.rs#L1)
- Library modules: [src/lib.rs](src/lib.rs#L1)
- API controllers: [src/api/mod.rs](src/api/mod.rs#L1) and specific controllers under `src/api/` (e.g. `address_book_controller.rs`).
- DB config and tenant pool manager: [src/config/db.rs](src/config/db.rs#L1)
- Middleware and auth: [src/middleware/auth_middleware.rs](src/middleware/auth_middleware.rs#L1) and `functional_auth` in that folder.
- WebSocket logger: [src/utils/ws_logger.rs](src/utils/ws_logger.rs#L1)
- Pagination: [src/unified_pagination.rs](src/unified_pagination.rs#L1) (contains cursor encryption validation used at startup)
- Functional lib: [functional_lib/src/lib.rs](functional_lib/src/lib.rs#L1) for advanced FP utilities.

Concrete conventions and patterns
- Tenant context: derive server-side (host/subdomain, mTLS, or lookup). Never trust client-supplied tenant identifiers; check `controller_context` and `auth_middleware`.
- DB routing: use the `TenantPoolManager` (see `config::db::TenantPoolManager`) to obtain the per-tenant connection pool.
- Middleware order matters: tracing, auth middleware (functional_auth with `PureFunctionRegistry`), then route handlers. See `src/main.rs` for the exact stack.
- Functional helpers: the project uses a `PureFunctionRegistry` and functional-style middleware — prefer using provided helper functions and registry patterns rather than ad-hoc globals.
- Schema migrations: migrations live in `migrations/`. Add migration SQL and follow existing timestamped naming.
- Pagination: Use `unified_pagination` with encrypted cursors (AES-256-GCM via `CURSOR_ENCRYPTION_KEY` env var, validated at startup).
- Logging: WebSocket-based real-time streaming with configurable format (text/JSON) via `WS_LOG_FORMAT` and buffer size via `WS_LOG_BUFFER_SIZE`.

Developer workflows and commands
- Local dev: `cargo run` (reads `.env`).
- Build for production: `cargo run --release` or containerize via `docker-compose -f docker-compose.local.yml up --build`.
- Migrations: `diesel migration generate <name>` then `diesel migration run`. Schema file `src/schema.rs` is auto-generated during build; manual `diesel print-schema > src/schema.rs` is rarely needed.
- Tests:
  - Unit: `cargo test`
  - Integration requiring DB/Redis: use docker or `docker compose --profile test up -d`, then set `DATABASE_URL` and `REDIS_URL` and run `cargo test --test integration_tests`.
  - CI uses `cargo test` and tarpaulin for coverage (`cargo tarpaulin --out Html`).
- Code quality: `cargo clippy` for linting, `cargo fmt` for formatting.

Integration points and external services
- PostgreSQL (one DB, multiple schemas) — connection string via `DATABASE_URL`.
- Redis for caching/sessions — `REDIS_URL`.
- WebSocket for live logs — client tokens for browsers are passed as query parameter `?token=`; server uses header `Authorization: Bearer` where supported.

Guidance for code edits by AI agents
- Preserve tenant isolation: any change touching request routing, DB access, or auth must preserve server-derived tenant context checks.
- When modifying models or DB schema:
  - Add a SQL migration under `migrations/` (timestamped folder).
  - Avoid changing `src/schema.rs` manually unless necessary; let the build regenerate it.
- When touching middleware order or wrapping services, mirror the stack in `src/main.rs` (tracing -> auth -> app handlers).
- If adding new endpoints that return logs or streaming data, prefer existing WebSocket logger utilities in `src/utils/ws_logger.rs` to keep a single source of truth.
- Use functional patterns: Leverage `rcs-functional` for iterator chains, pure functions, and state transitions where appropriate.

Examples (copyable patterns)
- Obtain tenant pool: `let pool = manager.get_pool(&tenant_id)?;` (see `src/config/db.rs`).
- Use functional auth middleware: `FunctionalAuthentication::with_registry(pure_registry.clone())` (see `src/main.rs`).
- Create encrypted cursor: `let cursor = IdCursor::new(id).encode()?;` (see `src/unified_pagination.rs`).
- Broadcast log: `broadcaster.send("Log message".to_string());` (see `src/utils/ws_logger.rs`).

What to avoid
- Do not accept tenant identifiers from request payloads/headers without server-side verification.
- Do not bypass the `TenantPoolManager` for DB connections.
- Do not expose `CURSOR_ENCRYPTION_KEY` in logs or error messages.

If you change runtime defaults or env usage
- Document new environment variables in `README.md` and update `.env.sample`.

If you are unsure
- Run `cargo test` and, for integration tests, bring up services with `docker compose --profile test up -d`. Check `src/main.rs` and `src/api/*` for request flow examples.

Ask for clarification
- If any behavior or test fails due to DB schema changes, ask for the intended tenant isolation semantics and which migration to add.

End.
