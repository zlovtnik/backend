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
- **Logging**: Structured logging via `tracing` with real-time WebSocket streaming and optional JSON output

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

- **Backend**: Rust API with JWT auth, database isolation, and health checks
- **Database**: Multi-tenant PostgreSQL setup with proper migrations
- **Security**: CORS, input validation, password hashing, and secure token handling

### 🔄 What's Next

- **Testing**: [Issue #2](https://github.com/zlovtnik/dispo-rusty/issues/2) @qa-team — Add comprehensive test coverage for critical paths; verified by 85%+ code coverage and all integration tests passing
- **Performance**: [Issue #3](https://github.com/zlovtnik/dispo-rusty/issues/3) @devops-team — Implement caching improvements and query optimization; verified by <100ms avg response times
- **Features**: [Issue #4](https://github.com/zlovtnik/dispo-rusty/issues/4) @product-team — Add advanced search, data export, and analytics; verified by user acceptance testing

## Quick Start

### Prerequisites

- Rust stable 1.90.0 (MSRV: 1.86.0+) with Diesel CLI
- PostgreSQL 13+
- Redis 6+

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

# Copy environment file
cp .env.example .env
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

## API Usage

### Authentication

**Security Note**: Tenant IDs supplied by clients are ignored—tenant context is derived and validated server-side only.

```bash
# Register a new user (tenant context derived server-side)
curl -X POST http://localhost:8000/api/auth/signup \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "email": "admin@tenant1.com",
    "password": "MyS3cur3P@ssw0rd!"
  }'

# Login and capture JWT token (tenant context derived server-side)
TOKEN=$(curl -X POST http://localhost:8000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username_or_email": "admin",
    "password": "MyS3cur3P@ssw0rd!"
  }' | jq -r '.token')

# Use captured token in subsequent requests

# Note: "MyS3cur3P@ssw0rd!" is only an example and should not be reused in production.
# Always use strong, unique passwords for each account.
curl -X GET http://localhost:8000/api/address-book \
  -H "Authorization: Bearer $TOKEN"
```

### Address Book Operations

```bash
# Create a new contact (using captured JWT token)
curl -X POST http://localhost:8000/api/address-book \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "John Doe",
    "email": "john@example.com",
    "phone": "+1234567890"
  }'

# Unauthorized responses
curl -X GET http://localhost:8000/api/address-book  # 401: Missing JWT
# Response: {"status":401,"error":"Unauthorized","message":"Missing authorization header"}

curl -X GET http://localhost:8000/api/address-book \
  -H "Authorization: Bearer invalid_token"  # 401: Invalid JWT
# Response: {"status":401,"error":"Unauthorized","message":"Invalid token"}

curl -X GET http://localhost:8000/api/address-book \
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
```

#### Coverage Reports

**Rust:** `cargo tarpaulin --out Html` (reports in `tarpaulin-report.html`)

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
    steps:
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: ${{ matrix.rust }}
      - run: cargo test
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
# Build and run with Docker Compose
docker-compose -f docker-compose.local.yml up --build
```

### Docker Compose Environments

The project uses separate Docker Compose files for different environments:

- **docker-compose.local.yml**: For local development with hot-reload and developer tools
  - Runs backend service with development configuration
  - Exposes port 8000 for local testing

See `docker-compose.local.yml` and `docker-compose.prod.yml` for full configuration details.

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

## Real-Time Logging via WebSocket

The application now provides real-time log streaming through a WebSocket endpoint, replacing the previous file-based SSE streaming. This allows multiple clients to subscribe to live application logs without file I/O overhead.

### WebSocket Log Streaming Endpoint

**Endpoint**: `GET /api/ws/logs`

**Protocol**: WebSocket (ws:// or wss://)

**Description**: Connects to this endpoint to receive real-time application logs as they happen. The WebSocket connection remains open, streaming log messages to the client as the application generates them.

### JavaScript Examples

#### Node.js Backend Example (Using `ws` Package)

For server-side Node.js applications, use the `ws` package to set custom headers with Bearer tokens:

```javascript
const WebSocket = require('ws');

const token = 'your-jwt-token-here';
const url = 'ws://localhost:9000/logs';

const ws = new WebSocket(url, {
    headers: {
        'Authorization': `Bearer ${token}`
    }
});

ws.on('open', () => {
    console.log('Connected to log stream');
});

ws.on('message', (data) => {
    console.log('Log:', data);
    // data contains formatted log messages:
    // Text format: [2024-10-28 14:48:32.123] INFO [module::path] User logged in successfully
    // JSON format: {"timestamp":"2024-10-28 14:48:32.123","level":"INFO","target":"module::path","message":"User logged in successfully"}
});

ws.on('error', (error) => {
    console.error('WebSocket error:', error);
});

ws.on('close', () => {
    console.log('Disconnected from log stream');
});
```

**Installation:**
```bash
npm install ws
```

#### Browser Example (Using Native WebSocket API)

Browser environments don't allow custom `Authorization` headers due to CORS and WebSocket specification restrictions. Instead, authenticate via query parameter or cookie:

**Option 1: Token via Query Parameter** (Recommended for demos)
```javascript
// Browser-based client sending token via query parameter
const token = 'your-jwt-token-here';
const url = `ws://localhost:9000/logs?token=${encodeURIComponent(token)}`;

const ws = new WebSocket(url);

ws.onopen = () => {
    console.log('Connected to log stream');
};

ws.onmessage = (event) => {
    console.log('Log:', event.data);
};

ws.onerror = (error) => {
    console.error('WebSocket error:', error);
};

ws.onclose = () => {
    console.log('Disconnected from log stream');
};
```

**Option 2: Token via Cookie** (Recommended for production)
```javascript
// Browser-based client using cookie authentication
// (Ensure the server sets a secure HttpOnly cookie after login)

const url = 'wss://yourdomain.com:9000/logs';  // Use WSS in production!

const ws = new WebSocket(url);

ws.onopen = () => {
    console.log('Connected to log stream');
};

ws.onmessage = (event) => {
    console.log('Log:', event.data);
};

ws.onerror = (error) => {
    console.error('WebSocket error:', error);
};

ws.onclose = () => {
    console.log('Disconnected from log stream');
};
```

**Security Notes:**
- ⚠️ Use `wss://` (WebSocket Secure) in production environments to encrypt the connection
- ⚠️ Query parameters may be logged in server/proxy logs—avoid sensitive data in URLs for production
- ✅ Cookies with `HttpOnly` flag are more secure for browser-based clients
- ✅ Configure CORS appropriately to restrict WebSocket connections to your domain

**Server Requirements:**
To support browser-based clients, your server must implement one or both of these authentication methods:
- Query parameter parsing: Check `?token=...` in the connection request
- Cookie parsing: Extract authentication token from `Cookie` header (automatically sent by browsers)

Please verify which authentication method your server supports and adjust the client code accordingly.

### Rust Example (Using `tokio-tungstenite`)

```rust
use tokio_tungstenite::connect_async;
use futures::StreamExt;

#[tokio::main]
async fn main() {
    let token = "your-jwt-token-here";
    let url = "ws://127.0.0.1:9000/logs";
    
    // Build request with authorization header
    let req = http::Request::builder()
        .uri(url)
        .header("Authorization", format!("Bearer {}", token))
        .body(())
        .unwrap();
    
    // Connect using the request (sends the Authorization header)
    match connect_async(req).await {
        Ok((ws_stream, _)) => {
            let (_, mut read) = ws_stream.split();
            
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(msg) => println!("Log: {}", msg.to_text().unwrap_or_default()),
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}
```

### Log Message Format

Log messages are formatted with the following structure:

```text
[TIMESTAMP] LEVEL [MODULE::PATH] MESSAGE
```

Example:

```text
[2024-10-28 14:48:32.123] INFO [rcs::services::auth] User admin logged in
[2024-10-28 14:48:33.456] ERROR [rcs::services::db] Database connection pool exhausted
[2024-10-28 14:48:34.789] WARN [rcs::api::health_controller] High memory usage detected
```

### Configuration

Log level filtering is controlled via the `RUST_LOG` environment variable:

```bash
# Set log level to debug
export RUST_LOG=debug

# Filter by module
export RUST_LOG=rcs::api=debug,rcs::services=info

# Multiple modules
export RUST_LOG=rcs::services::auth=debug,rcs::api::health_controller=info
```

Available log levels: `trace`, `debug`, `info`, `warn`, `error`

### WebSocket Logging Configuration

Configure WebSocket logging with the following environment variables:

**`APP_WS_PORT`** (default: `9000`)
- Dedicated port for WebSocket logging endpoint
- Example: `APP_WS_PORT=9000`

**`WS_LOG_BUFFER_SIZE`** (default: `1000`)
- Number of log messages to keep in the broadcast buffer
- Higher values use more memory but tolerate slower clients better
- Example: `WS_LOG_BUFFER_SIZE=5000`

**`WS_LOG_FORMAT`** (default: `text`)
- Output format for log messages
- Options: `text` (human-readable) or `json` (structured)
- Example: `WS_LOG_FORMAT=json`

**`WS_LOGS_ADMIN_USER`** (optional)
- Comma-separated list of user IDs authorized to access WebSocket logs
- If not set, any valid JWT token holder can access logs
- **RECOMMENDED**: Set this in production to restrict access to authorized admins only
- Example: `WS_LOGS_ADMIN_USER=admin@example.com,user1,user2`

**`CORS_ALLOWED_ORIGINS`** (production-specific)
- Comma-separated list of allowed origin domains for WebSocket and HTTP requests
- In production, explicitly list only trusted origins
- **SECURITY**: Do NOT use wildcards; each origin must be explicit
- Example: `CORS_ALLOWED_ORIGINS=https://app.example.com,https://admin.example.com`

### WebSocket Security

The WebSocket logging endpoint implements multiple security layers:

#### 1. **Authentication**
- All WebSocket connections require a valid JWT token in the `Authorization: Bearer <token>` header
- Invalid or missing tokens receive HTTP 403 Forbidden
- Tokens are validated and not logged to prevent credential leakage

#### 2. **Authorization**
- Optional authorization check via `WS_LOGS_ADMIN_USER` environment variable
- Restrict WebSocket log access to specific admin users
- Without this setting, any valid JWT token holder can access logs (use for development only)

#### 3. **Origin Validation (CSWSH Prevention)**
- WebSocket connections validate the `Origin` header against the configured `CORS_ALLOWED_ORIGINS` list
- Prevents cross-site WebSocket hijacking (CSWSH) attacks
- In production (APP_ENV=production), origin validation is enforced
- Mismatched origins receive HTTP 403 Forbidden with sanitized error messages

#### 4. **Network-Level Firewall Rules**
- **CRITICAL for Production**: The WebSocket port (APP_WS_PORT, default 9000) must be firewalled to only allow trusted sources
- Without firewall rules, any network-accessible system can attempt connections

**Example firewall configuration using iptables:**
```bash
# Allow WebSocket connections only from internal networks
iptables -A INPUT -p tcp --dport 9000 -s 10.0.0.0/8 -j ACCEPT
iptables -A INPUT -p tcp --dport 9000 -s 172.16.0.0/12 -j ACCEPT
iptables -A INPUT -p tcp --dport 9000 -j DROP
```

**Example using firewalld:**
```bash
# Create a service for WebSocket logs (allow internal network only)
firewall-cmd --permanent --add-rich-rule='rule family="ipv4" source address="10.0.0.0/8" port port="9000" protocol="tcp" accept'
firewall-cmd --permanent --add-rich-rule='rule family="ipv4" port port="9000" protocol="tcp" reject'
firewall-cmd --reload
```

**Example using AWS Security Groups:**
```
Inbound Rules:
- Type: Custom TCP
- Port: 9000
- Source: 10.0.0.0/8 (your private network CIDR)
```

#### 5. **Log Sanitization**
- Authentication tokens are NOT logged even on errors
- Sensitive error messages are omitted from logs; details available only at DEBUG level
- Connection metadata (but not credentials) is logged for troubleshooting

### Deprecated: SSE Endpoint

The previous Server-Sent Events (SSE) endpoint at `GET /api/logs` is deprecated as of v0.2.0. This endpoint returned HTTP 410 (Gone) with a deprecation notice. WebSocket at `/api/ws/logs` is the recommended replacement.

**Migration from SSE to WebSocket:**

- **Old**: `curl http://localhost:8000/api/logs` (SSE)
- **New**: `wscat -c ws://localhost:9000/logs` (WebSocket, dedicated port)

### Buffer Management

The WebSocket log broadcast channel has a capacity of 1000 messages. If messages are produced faster than consumed:

- Slow clients will receive a `[WARNING]` message notifying them that messages were dropped
- This prevents memory buildup from accumulating log messages

### Architecture

The logging system uses:

- **`tracing` crate**: Structured logging framework
- **`tracing-log` bridge**: Compatibility with existing `log` crate macros throughout the codebase
- **`tokio::sync::broadcast`**: Efficient multi-client message distribution
- **`tracing-actix-web`**: Structured HTTP request logging

Applications logs are captured via the `log::*!` macros:

```rust
log::info!("User logged in");
log::error!("Database error: {}", err);
log::debug!("Processing request...");
```

These are automatically broadcasted to all connected WebSocket clients.

## Contributing

We welcome contributions! Here's how to get started:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Add tests for new functionality
5. Run the test suite (`cargo test`)
6. Commit your changes (`git commit -m 'Add amazing feature'`)
7. Push to your branch (`git push origin feature/amazing-feature`)
8. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

Thanks to the amazing open-source community:

- **Actix Web** - High-performance Rust web framework
- **Diesel** - Type-safe ORM for Rust
- **PostgreSQL** - The world's most advanced open-source database
- **Redis** - In-memory data structure store

---

## Built with ❤️ using [Rust](https://www.rust-lang.org), [Actix Web](https://actix.rs), and modern DevOps practices

*Solving real-world multi-tenant challenges with production-ready architecture and security-first design.*
