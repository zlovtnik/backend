# dispo-rusty: The Enterprise Multi-Tenant API Starter

Built to solve real-world SaaS and managed platform pain points—compliance, scale, and speed—dispo-rusty is an open-source foundation for secure, high-performance, tenant-isolated REST API services. Whether you're a fast-moving founder, a platform CTO, or an enterprise engineering team, dispo-rusty helps you reduce costs, onboard clients instantly, and meet demanding security requirements from day zero.

🚀 **Database Isolation** | ⚡ **Enterprise Security** | 🤝 **Rapid Onboarding** | 🏗️ **Scale-Ready**

## Why dispo-rusty?

**The Problem**: Building multi-tenant SaaS platforms is hard. You need database isolation, security compliance, and the ability to scale—all while keeping development velocity high.

**The Solution**: dispo-rusty gives you a production-ready foundation that handles the complex stuff so you can focus on your business logic.

### What You Get Out of the Box

- **🔒 Strong Data Isolation**: One PostgreSQL database per tenant (not per-schema) to minimize cross-tenant risk
- **⚡ High Performance**: Rust backend designed for low-latency APIs; see benchmarks for your workload.
- **🛡️ Security First**: JWT authentication, CORS protection, input validation
- **🎨 Modern Frontend**: React + TypeScript with Ant Design components
- **🐳 Production Ready**: Docker containers, health checks, monitoring
- **📈 Built to Scale**: Connection pooling, caching, and tenant-aware routing

## The Tech Stack

### Backend (Rust + Actix Web)

- **Database**: PostgreSQL with Diesel ORM for type-safe queries
- **Authentication**: JWT tokens with tenant context built-in
- **Caching**: Redis for sessions and performance
- **Connection Pooling**: r2d2 for efficient database connections
- **Logging**: Structured logging with file rotation

### Frontend (React + TypeScript)

- **Framework**: React 18 with TypeScript for type safety
- **Build Tool**: Vite + Bun for lightning-fast development
- **UI Library**: Ant Design for professional components
- **Forms**: React Hook Form with real-time validation
- **State**: Context API for global state management

## How It Works

### Multi-Tenant Architecture

Each tenant gets their own database. Tenant context is derived server-side from trusted sources (host/subdomain, mTLS client certificates, organization slug lookup, or other server-controlled mappings). APIs must validate that any tenant identifier in requests matches the server-derived context before routing to databases or minting tokens.

```text
┌─────────────────────┐    ┌─────────────────────┐
│   Main Database     │    │ Tenant Database     │
│  (Configuration)    │    │  (Isolated Data)    │
│  ┌────────────────┐ │    │  ┌────────────────┐ │
│  │ Tenants Config │ │    │  │ User Data      │ │
│  │ Database URLs  │ │    │  │ Business Logic │ │
│  │ Security Keys  │ │    │  │ Application    │ │
│  └────────────────┘ │    │  │ State          │ │
└─────────────────────┘    │  └────────────────┘ │
         │                 └─────────────────────┘
         │                           │
         └─────────JWT Token─────────┘
              (includes tenant_id)
```

**Why This Matters:**

- **Strong Data Isolation**: Designed to prevent cross-tenant data access through strict isolation and access controls
- **Compliance Ready**: Meets strict data isolation requirements
- **Performance**: Each tenant gets optimized database connections
- **Simple**: JWT tokens handle routing automatically

## Current Status

### ✅ What's Working Now

- **Frontend**: Complete React app with authentication, tenant switching, and contact management
- **Backend**: Rust API with JWT auth, database isolation, and health checks
- **Database**: Multi-tenant PostgreSQL setup with proper migrations
- **Security**: CORS, input validation, password hashing, and secure token handling

### 🔄 What's Next

- **API Integration**: [Issue #1](https://github.com/zlovtnik/dispo-rusty/issues/1) @frontend-team — Switch baseURL in frontend/.env to backend and remove mock data; verified by end-to-end login/profile flows passing
- **Testing**: [Issue #2](https://github.com/zlovtnik/dispo-rusty/issues/2) @qa-team — Add comprehensive test coverage for critical paths; verified by 85%+ code coverage and all integration tests passing
- **Performance**: [Issue #3](https://github.com/zlovtnik/dispo-rusty/issues/3) @devops-team — Implement bundle optimization and caching improvements; verified by Lighthouse scores >90 and <2s page load times
- **Features**: [Issue #4](https://github.com/zlovtnik/dispo-rusty/issues/4) @product-team — Add advanced search, data export, and internationalization; verified by user acceptance testing with 10+ beta users

## Quick Start

### Prerequisites

- Rust stable 1.90.0 (MSRV: 1.86.0+) with Diesel CLI
- PostgreSQL 13+
- Redis 6+
- Bun 1.0+ (for frontend)

#### Installing Diesel CLI

**Debian/Ubuntu:**

```bash
sudo apt-get install libpq-dev pkg-config libssl-dev
cargo install diesel_cli --no-default-features --features postgres
```

**macOS:**

```bash
brew install postgresql pkg-config openssl
cargo install diesel_cli --no-default-features --features postgres
```

*Note: Requires working PostgreSQL client libraries to build.*

### 1. Clone and Setup

```bash
git clone https://github.com/zlovtnik/dispo-rusty.git
cd dispo-rusty

# Copy environment files
cp .env.example .env
cp frontend/.env.example frontend/.env
```

### 2. Database Setup

```bash
# Run migrations
diesel migration run

# Schema is automatically generated during cargo build
# To manually regenerate schema (if needed):
diesel print-schema > src/schema.rs

# Optional: Seed tenant data
psql -d rust_rest_api_db -f scripts/seed_tenants.sql
```

**Note**: Schema generation is now automated during the build process. You no longer need to manually run `diesel print-schema` in most cases.

### 3. Start the Backend

```bash
# Backend loads environment variables from .env file (dotenv)
# Ensure .env is created/populated before running
cargo run                    # Development mode
cargo run --release          # Production mode (recommended for performance)
```

### 4. Start the Frontend

```bash
cd frontend
bun install
bun run dev
```

Visit `http://localhost:3000` to see the app in action.

## API Usage

### Authentication

**Security Note**: Tenant IDs supplied by clients are ignored—tenant context is derived and validated server-side only.

```bash
# Register a new user (tenant context derived server-side)
curl -X POST http://localhost:8080/api/auth/signup \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "email": "admin@tenant1.com",
    "password": "MyS3cur3P@ssw0rd!"
  }'

# Login and capture JWT token (tenant context derived server-side)
TOKEN=$(curl -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username_or_email": "admin",
    "password": "MyS3cur3P@ssw0rd!"
  }' | jq -r '.token')

# Use captured token in subsequent requests

# Note: "MyS3cur3P@ssw0rd!" is only an example and should not be reused in production.
# Always use strong, unique passwords for each account.
curl -X GET http://localhost:8080/api/address-book \
  -H "Authorization: Bearer $TOKEN"
```

### Address Book Operations

```bash
# Get all contacts (using captured JWT token)
curl -X GET http://localhost:8080/api/address-book \
  -H "Authorization: Bearer $TOKEN"

# Create a new contact (using captured JWT token)
curl -X POST http://localhost:8080/api/address-book \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "John Doe",
    "email": "john@example.com",
    "phone": "+1234567890"
  }'

# Unauthorized responses
curl -X GET http://localhost:8080/api/address-book  # 401: Missing JWT
# Response: {"status":401,"error":"Unauthorized","message":"Missing authorization header"}

curl -X GET http://localhost:8080/api/address-book \
  -H "Authorization: Bearer invalid_token"  # 401: Invalid JWT
# Response: {"status":401,"error":"Unauthorized","message":"Invalid token"}

curl -X GET http://localhost:8080/api/address-book \
  -H "Authorization: Bearer $OTHER_TENANT_TOKEN"  # 403: Tenant mismatch
# Response: {"status":403,"error":"Forbidden","message":"Tenant access denied"}
```

## Security Features

- **JWT Authentication**: Secure token-based auth with tenant context
- **Database Isolation**: Each tenant has their own database
- **CORS Protection**: Configurable origin validation
- **Input Validation**: Comprehensive request validation
- **Password Security**: bcrypt hashing with configurable cost
- **SQL Injection Prevention**: Diesel ORM with parameterized queries

## Development

### Running Tests

```bash
# Backend tests
cargo test

# Frontend tests
cd frontend
bun test
```

#### Coverage Reports

**Rust:** `cargo tarpaulin --out Html` (reports in `tarpaulin-report.html`)  
**Bun:** `bun test --coverage` (reports in `coverage/` directory)

#### Integration Tests

For tests requiring PostgreSQL/Redis, use testcontainers or docker-compose:

```bash
# Start services with docker-compose
docker compose --profile test up -d

# Set environment variables
export DATABASE_URL=postgres://user:password@localhost:5432/test_db
export REDIS_URL=redis://localhost:6379

# Run integration tests
cargo test --test integration_tests
```

#### CI Setup

Example GitHub Actions matrix:

```yaml
jobs:
  test:
    strategy:
      matrix:
        rust: [1.86.0, stable]
        bun: [1.0.0, latest]
    steps:
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: ${{ matrix.rust }}
      - uses: oven-sh/setup-bun@v1
        with:
          bun-version: ${{ matrix.bun }}
      - run: cargo test
      - run: cd frontend && bun test --coverage
      - name: Upload coverage reports to Codecov
        uses: codecov/codecov-action@v3
        with:
          file: ./frontend/coverage/lcov.info
          flags: frontend
          name: frontend-coverage
      - name: Upload coverage reports to Codecov
        uses: codecov/codecov-action@v3
        with:
          file: ./coverage/tarpaulin-report.xml
          flags: backend
          name: backend-coverage
```

### Database Migrations

```bash
# Create a new migration
diesel migration generate add_new_table

# Run migrations
diesel migration run

# Revert last migration
diesel migration revert
```

## Deployment

### Docker

```bash
# Build and run with Docker Compose (dev profile)
docker compose --profile dev up --build
```

### Docker Compose Profiles

The project uses Docker Compose profiles to separate different environments:

- **dev**: For local development with hot-reload and developer tools
  - Enables backend service with development configuration
  - Mounts source code for live reloading
  - Exposes debugging ports

- **test**: For CI/test-only services
  - Runs minimal services needed for testing
  - Uses test-specific configurations
  - Optimized for fast startup

See `docker-compose.local.yml` and `docker-compose.prod.yml` for full details on services per profile.

### Environment Variables

```env
# Database
DATABASE_URL=postgres://user:password@localhost/dbname
REDIS_URL=redis://127.0.0.1:6379

# Security
JWT_SECRET=your-secret-key-here
MAX_AGE=604800

# Server
APP_HOST=0.0.0.0
APP_PORT=8080
LOG_FILE=logs/app.log

# CORS
CORS_ORIGINS=http://localhost:3000,http://localhost:4321
CORS_CREDENTIALS=true
```

## Contributing

We welcome contributions! Here's how to get started:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Add tests for new functionality
5. Run the test suite (`cargo test` and `bun test`)
6. Commit your changes (`git commit -m 'Add amazing feature'`)
7. Push to your branch (`git push origin feature/amazing-feature`)
8. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

Thanks to the amazing open-source community:

- **Actix Web** - High-performance Rust web framework
- **Diesel** - Type-safe ORM for Rust
- **React** - The UI library that powers the frontend
- **Ant Design** - Enterprise UI component library
- **PostgreSQL** - The world's most advanced open-source database
- **Redis** - In-memory data structure store

---

## Built with ❤️ using [Rust](https://www.rust-lang.org), [Actix Web](https://actix.rs), [React](https://react.dev), and modern DevOps practices

*Solving real-world multi-tenant challenges with production-ready architecture and security-first design.*
