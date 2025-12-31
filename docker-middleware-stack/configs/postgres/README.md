# PostgreSQL Configuration

This directory contains configuration and initialization scripts for PostgreSQL in the middleware stack.

## Files

### `init.sql` - Database Initialization Script

Executed automatically on PostgreSQL container startup via Docker entrypoint.

**Creates**:
- Databases: `keycloak`, `app`, `analytics`, `warehouse`, `middleware` (implicit)
- Users: `keycloak`, `app_user`, `analytics_user`, `rcl`, `middleware`, `middleware_admin`
- Extensions: UUID, pgcrypto, hstore, pg_stat_statements, pg_buffercache
- Tables: `staging_example`, `heartbeat`

**Critical Table**: `heartbeat`
- **Purpose**: Debezium CDC connector health monitoring
- **Schema**: `CREATE TABLE heartbeat (ts TIMESTAMP NOT NULL DEFAULT NOW());`
- **Required by**: Debezium PostgreSQL connector (see [`kafka/README.md`](../kafka/README.md))
- **Usage**: Connector periodically inserts records to detect database connectivity

### `postgresql.conf` - Server Configuration

PostgreSQL server configuration file.

**Key Settings for CDC**:
```
wal_level = logical              # Enable logical replication for CDC
max_wal_senders = 4              # Allow concurrent replication senders
max_replication_slots = 4        # Allow replication slots for connectors
```

## Initialization Flow

```
Docker container start
  ↓
PostgreSQL server initialization
  ↓
Execute /docker-entrypoint-initdb.d/init.sql (AUTOMATIC)
  ↓
  ├─ Create databases and users
  ├─ Create heartbeat table ✅ (Required by Debezium)
  ├─ Create extensions
  └─ Grant permissions
  ↓
PostgreSQL service ready
  ↓
Health check passes
  ↓
Kafka Connect can initialize
```

## Database Users & Passwords

| User | Password | Database | Purpose |
|------|----------|----------|---------|
| `middleware` | `middleware123` | `middleware` | Application default user |
| `keycloak` | `keycloak123` | `keycloak` | Keycloak service |
| `app_user` | `app123` | `app` | Application data |
| `analytics_user` | `analytics123` | `analytics` | Analytics queries |
| `rcl` | `rcl` | `warehouse` | RCL (Rust CDC Loader) |
| `middleware_admin` | `admin123` | All | Administrative superuser (use with caution) |

**⚠️ Security Note**: These are default passwords for local development. For production, use environment variables and secret management.

## Volumes

Mounted in docker-compose.yml:
- `postgres_data:/var/lib/postgresql/data` - Persistent data storage
- `./configs/postgres/init.sql:/docker-entrypoint-initdb.d/init.sql` - Initialization script
- `./configs/postgres/postgresql.conf:/etc/postgresql/postgresql.conf` - Server config

## Health Check

```bash
docker-compose exec postgres pg_isready -U middleware -d middleware -h localhost
```

## Manual SQL Execution

Connect to middleware database:
```bash
docker-compose exec postgres psql -U middleware -d middleware
```

View heartbeat table:
```bash
docker-compose exec postgres psql -U middleware -d middleware -c "SELECT * FROM heartbeat LIMIT 10;"
```

## Debezium Connector Integration

The `heartbeat` table is a **critical dependency** for the Debezium PostgreSQL CDC connector.

- **Created by**: `init.sql` (automatic on container startup)
- **Used by**: Debezium connector's `heartbeat.action.query` parameter
- **Purpose**: Detect database connectivity issues and prevent connector stalls

For detailed Debezium setup and troubleshooting, see [`kafka/README.md`](../kafka/README.md).

## Modifying Initialization

To add new databases, users, or tables:

1. Edit `init.sql`
2. Restart PostgreSQL container: `docker-compose down && docker-compose up -d postgres`
   - This will re-execute the initialization script

**Note**: Existing databases and users will not be recreated (they already exist).

## Backup & Restore

Create a dump:
```bash
make db-dump  # in docker-middleware-stack/
```

Restore a dump:
```bash
docker-compose exec -T postgres psql -U middleware -d middleware < backup.sql
```

## References

- [PostgreSQL Official Documentation](https://www.postgresql.org/docs/)
- [PostgreSQL Logical Replication](https://www.postgresql.org/docs/15/logical-replication.html)
- [Docker PostgreSQL Image](https://hub.docker.com/_/postgres)
