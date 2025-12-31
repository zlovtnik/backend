# Middleware Stack Configuration

This directory contains configuration files for all services in the middleware stack.

## Service Configurations

### [postgres/](postgres/) - PostgreSQL Database
- **Files**: `init.sql`, `postgresql.conf`, `README.md`
- **Purpose**: Database initialization and server configuration
- **Critical Table**: `heartbeat` table for Debezium CDC monitoring
- **See**: [postgres/README.md](postgres/README.md)

### [kafka/](kafka/) - Apache Kafka & Connectors
- **Files**: `debezium-postgres-connector.json`, `README.md`
- **Purpose**: Kafka Connect configuration for Change Data Capture (CDC)
- **Dependency**: Requires `heartbeat` table from PostgreSQL
- **See**: [kafka/README.md](kafka/README.md)

### [keycloak/](keycloak/) - Keycloak IAM
- **Purpose**: Identity and Access Management configuration
- **Database**: Uses PostgreSQL `keycloak` database

### [grafana/](grafana/) - Grafana Monitoring
- **Purpose**: Visualization and monitoring dashboards

### [prometheus/](prometheus/) - Prometheus Metrics
- **Purpose**: Metrics collection and time-series data

### [otel/](otel/) - OpenTelemetry Collector
- **Purpose**: Observability and distributed tracing

### [minio/](minio/) - MinIO Object Storage
- **Purpose**: S3-compatible object storage

### [redis/](redis/) - Redis Cache
- **Purpose**: In-memory caching and session storage

### [schema-registry/](schema-registry/) - Confluent Schema Registry
- **Purpose**: Schema management for Kafka messages

## Initialization Dependencies

Critical initialization order (managed by docker-compose `depends_on`):

```
PostgreSQL
    ↓
    └─→ Initializes databases, users, heartbeat table
    ↓
Kafka + Zookeeper
    ↓
Kafka Connect
    ↓
    └─→ Debezium connector starts
    ↓
    └─→ Inserts heartbeats to database via heartbeat.action.query
```

## Key Integration Points

### PostgreSQL → Kafka (CDC)
- **Connector**: Debezium PostgreSQL Connector
- **Heartbeat Table**: `heartbeat` (created in `postgres/init.sql`)
- **Config**: `kafka/debezium-postgres-connector.json`
- **Documentation**: See `kafka/README.md`

### PostgreSQL → Keycloak
- **Database**: `keycloak` (created in `postgres/init.sql`)
- **User**: `keycloak` with password `keycloak123`
- **Permissions**: Full schema access

### PostgreSQL → Application
- **Database**: `middleware` (default, implicit)
- **User**: `middleware` with password `middleware123`
- **Tables**: Application schemas (via migrations)

## Configuration Best Practices

### Development vs. Production

**This configuration is for LOCAL DEVELOPMENT ONLY**

For production, implement:
- ✅ Environment-based passwords (use `.env` files or secrets management)
- ✅ SSL/TLS for database and service connections
- ✅ Network policies and firewalls
- ✅ Resource limits and autoscaling
- ✅ Persistent volumes on secure storage
- ✅ Backup and disaster recovery

### Making Changes

1. **Database Changes**:
   - Edit `postgres/init.sql`
   - Restart: `docker-compose down && docker-compose up -d postgres`

2. **Debezium Connector Changes**:
   - Edit `kafka/debezium-postgres-connector.json`
   - Restart: `docker-compose restart kafka-connect`

3. **Server Configuration Changes**:
   - Edit `postgres/postgresql.conf`
   - Requires full container restart for some parameters

## Troubleshooting Quick Links

### Heartbeat Table Issues
See [kafka/README.md - Heartbeat Table Missing Error](kafka/README.md#heartbeat-table-missing-error)

### Logical Replication Issues
See [kafka/README.md - Logical Replication Not Enabled](kafka/README.md#logical-replication-not-enabled)

### PostgreSQL Startup Issues
See [postgres/README.md](postgres/README.md)

### Connector Health
See [kafka/README.md - Health Checks](kafka/README.md#health-checks)

## Useful Commands

```bash
# View all config files
ls -la */

# Check heartbeat table
docker-compose exec postgres psql -U middleware -d middleware -c "SELECT COUNT(*), MAX(ts) FROM heartbeat;"

# View Debezium connector status
curl -s http://localhost:8083/connectors/postgres-cdc-connector/status | jq '.'

# Stream Kafka CDC topic
docker-compose exec kafka kafka-console-consumer --bootstrap-server localhost:9093 \
  --topic cdc.public.staging_example --from-beginning

# PostgreSQL shell
docker-compose exec postgres psql -U middleware -d middleware

# Kafka Connect logs
docker-compose logs -f kafka-connect | grep -i heartbeat
```

## Architecture Diagram

```
PostgreSQL (middleware db)
    ├─ init.sql creates:
    │  ├─ heartbeat table (for Debezium monitoring)
    │  ├─ staging_example table (CDC source)
    │  └─ User permissions
    │
    └─ postgresql.conf enables:
       ├─ wal_level = logical
       ├─ max_wal_senders = 4
       └─ max_replication_slots = 4

            ↓ (CDC via Debezium)

Kafka-Connect (with Debezium PostgreSQL Connector)
    ├─ Reads from: PostgreSQL public.staging_example
    ├─ Writes to: Kafka topics (cdc.public.staging_example)
    └─ Heartbeat: INSERT INTO heartbeat (ts) VALUES (now()) every 10s

            ↓ (Event streaming)

Kafka Topics
    ├─ cdc.public.staging_example (change events)
    └─ (Other topics as configured)

            ↓ (Consumers can subscribe)

Application Services
    ├─ Real-time data processing
    ├─ Analytics
    └─ Monitoring dashboards
```

## References

- [Debezium PostgreSQL Connector](https://debezium.io/documentation/reference/stable/connectors/postgresql.html)
- [Kafka Connect Architecture](https://kafka.apache.org/documentation/#connect)
- [PostgreSQL Logical Replication](https://www.postgresql.org/docs/15/logical-replication.html)
- [Docker Compose Services](../docker-compose.yml)
