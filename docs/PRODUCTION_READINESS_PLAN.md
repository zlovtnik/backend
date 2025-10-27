# Production Readiness Plan

This document captures the hardening tasks required before promoting the service to production. The checklist is grouped by capability area so work can be parallelized across the team. Mark each task as it is completed and cross link to the implementation PR for traceability.

## Testing Strategy

- [ ] **Unit coverage**: add tests for business logic modules (`account_service`, `address_book_service`, validator modules, token utilities).
- [ ] **Integration coverage**: exercise primary Actix handlers (`/api/auth/*`, `/api/address-book/*`) using in-memory or containerized Postgres.
- [ ] **End-to-end scenario**: script a user flow (tenant bootstrap → signup → login → refresh token → CRUD address book entry) using the test harness.
- [ ] **Regression gate**: ensure `cargo fmt`, `cargo clippy --all-targets --all-features`, and `cargo test` run inside CI for every PR.
- [ ] **Performance smoke tests**: reuse existing benchmarks (if any) or add a lightweight load step in CI/CD for critical endpoints.

## Security Hardening

- [ ] Rotate all credentials committed previously; store replacements in a secrets manager (Vault, AWS Secrets Manager, SSM, etc.).
- [ ] Implement secrets retrieval in the runtime (e.g., environment injection from the secrets store) and document local overrides.
- [ ] Add rate limiting middleware for authentication endpoints with configurable thresholds per tenant.
- [ ] Extend the refresh token module to support explicit revocation/blacklisting and audit logging.
- [ ] Configure dependency and vulnerability scanning (Dependabot, cargo audit) as part of CI.

## Deployment & Delivery

- [ ] Build a production-grade multi-stage Dockerfile (non-root runtime, minimal base image, health endpoints exposed).
- [ ] Publish a GitHub Actions workflow that runs lint/test/build, pushes the container image, and triggers deployment on main.
- [ ] Provide infrastructure-as-code manifests: Kubernetes Deploy/Service/Ingress + ConfigMap/Secret templates (or the chosen platform equivalent).
- [ ] Document environment-specific configuration (dev/stage/prod) and how to promote artifacts across them.
- [ ] Add automated migration invocation to the deployment pipeline (pre-deploy step with safe rollback on failure).

## Documentation

- [ ] Generate OpenAPI/Swagger documentation for all public API routes; host it (e.g., `/docs` endpoint or published artifact).
- [ ] Author a deployment runbook covering: bootstrap, scaling, rollback, and tenant onboarding.
- [ ] Create an operations playbook with SOPs for common incidents (token compromise, tenant isolation, high latency).
- [ ] Update README with quick start, dev workflow, and links to the new docs.
- [ ] Ensure ADRs capture architectural decisions made during hardening (metrics stack, secrets approach, etc.).

## Observability & Operations

- [ ] Convert logging to structured JSON with correlation and tenant IDs; ensure log level is configurable.
- [ ] Expose Prometheus-compatible metrics (request latency, error rates, auth successes/failures, DB pool stats).
- [ ] Provide `/healthz` (liveness) and `/readyz` (readiness) endpoints that integrate with orchestration probes.
- [ ] Define alert rules for critical signals (auth failure surge, DB connection exhaustion, latency SLO breach).
- [ ] Integrate the service with centralized tracing (OpenTelemetry exporters, tracing IDs).

## Data Management

- [ ] Establish zero-downtime migration practices (expand/contract, feature flags, background migrations where needed).
- [ ] Automate database backups with tested restore procedures; specify RPO/RTO targets.
- [ ] Implement tenant-aware data retention policies and tooling for export or deletion requests.
- [ ] Monitor database health (connection usage, slow queries) with dashboards and alerts.
- [ ] Review indexes and query plans for hot paths; document tuning steps.

## Resilience & Recovery

- [ ] Add retry with jitter for transient external dependencies (email providers, caches, third-party APIs).
- [ ] Introduce circuit breakers or fallback responses for integrations prone to failure.
- [ ] Ensure graceful shutdown drains in-flight requests and releases resources cleanly.
- [ ] Simulate failure scenarios (token store outage, DB failover) and document recovery steps.
- [ ] Capture post-incident review template and checklist.

## Governance & Tracking

- [ ] Assign owners for each section above; record status in this document weekly.
- [ ] Link supporting tickets/PRs under each bullet as they progress.
- [ ] Schedule a final production readiness review once all critical items are complete.

---

## Next Steps

1. Review and prioritize the checklist with stakeholders; tag items as Blocker, High, or Nice-to-have.
2. Create GitHub issues or project board cards referencing each task for visibility.
3. Begin implementation in parallel streams (Testing, Security, Deployment, Observability) with regular syncs.
