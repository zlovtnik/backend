---
description: Repository Information Overview
alwaysApply: true
---

# dispo-rusty Information

## Summary
dispo-rusty is an enterprise multi-tenant REST API starter built with Rust and Actix Web. It provides a production-ready foundation for secure, high-performance, tenant-isolated REST API services with strong data isolation, JWT authentication, and modern frontend integration.

## Structure
- **src/**: Core application code (API controllers, models, services, middleware)
- **migrations/**: Database migration files for PostgreSQL
- **tests/**: Functional and integration tests
- **benches/**: Performance benchmarks
- **docs/**: Documentation and architecture guides
- **frontend/**: React + TypeScript frontend application

## Language & Runtime
**Language**: Rust
**Version**: 1.86.0 (minimum supported version)
**Build System**: Cargo
**Package Manager**: Cargo

## Dependencies
**Main Dependencies**:
- actix-web: 4.3.1 (Web framework)
- diesel: 2.2.0 (ORM with PostgreSQL support)
- jsonwebtoken: 8.3.0 (JWT authentication)
- redis: 0.32.7 (Caching)
- tokio: 1.35.0 (Async runtime)
- rayon: 1.11 (Parallel processing)
- serde: 1.0.163 (Serialization)

**Development Dependencies**:
- testcontainers: 0.14.0 (Integration testing)
- criterion: 0.5 (Benchmarking)
- reqwest: 0.11 (HTTP client for tests)

## Build & Installation
```bash
# Install dependencies
cargo install diesel_cli --no-default-features --features postgres

# Setup database
cp .env.example .env
diesel migration run

# Build and run
cargo build
cargo run --release
```

## Docker
**Dockerfiles**: 
- Dockerfile.local (Development)
- Dockerfile.github-action (Production)

**Configuration**: 
- docker-compose.local.yml (Development)
- docker-compose.prod.yml (Production)

**Run Command**:
```bash
# Development
docker compose --profile dev up --build

# Production
docker compose --profile prod up --build
```

## Testing
**Framework**: Built-in Rust test framework with testcontainers
**Test Location**: tests/ directory and src/ unit tests
**Run Command**:
```bash
# Run all tests
cargo test

# Run specific test
cargo test --test functional_tests

# Benchmarks
cargo bench
```

## Database
**Type**: PostgreSQL with multi-tenant isolation
**ORM**: Diesel with r2d2 connection pooling
**Migrations**: Diesel migrations in migrations/ directory
**Schema**: Auto-generated during build via diesel print-schema

## Authentication
**Method**: JWT tokens with tenant context
**Storage**: Redis for sessions and token management
**Security**: bcrypt password hashing, CORS protection
**Middleware**: Custom authentication middleware in src/middleware/

## Features
- Multi-tenant architecture with database isolation
- JWT authentication with tenant context
- CORS protection with configurable origins
- Functional programming patterns with performance monitoring
- Comprehensive validation engine
- Parallel processing capabilities
- React frontend with TypeScript