# Kafka Connectors Configuration

This directory contains configuration files for Kafka Connect connectors in the middleware stack.

## Debezium PostgreSQL Connector (`debezium-postgres-connector.json`)

### Overview

The Debezium PostgreSQL connector implements Change Data Capture (CDC) to stream database changes from PostgreSQL to Kafka topics.

**Configuration File**: `debezium-postgres-connector.json`

### Required Database Setup

#### Heartbeat Table (CRITICAL DEPENDENCY)

The connector uses the `heartbeat.action.query` parameter to periodically insert heartbeat records into the database. This is used to detect PostgreSQL connectivity issues and prevent connector stalls.

**Required Schema**:
```sql
CREATE TABLE IF NOT EXISTS heartbeat (
    ts TIMESTAMP NOT NULL DEFAULT NOW()
);
```

**Status**: ✅ **Automatically created** during database initialization via [`configs/postgres/init.sql`](../postgres/init.sql)

### Database Configuration Requirements

1. **Logical Replication** must be enabled:
   - Set `wal_level = logical` in PostgreSQL configuration
   - Set `max_wal_senders >= 4` (number of concurrent replication connections)
   - Set `max_replication_slots >= 4` (number of slots for connectors)

   **Status**: ✅ **Configured in** [`configs/postgres/postgresql.conf`](../postgres/postgresql.conf)

2. **Debezium Publication** is auto-created by the connector:
   - Publication name: `dbz_publication`
   - Slot name: `dbz_slot`

3. **Database User Permissions**:
   - User: `middleware` (configured in docker-compose.yml)
   - Required permissions: `SUPERUSER` or explicit replication permissions

### Connector Configuration Parameters

| Parameter | Value | Purpose |
|-----------|-------|---------|
| `heartbeat.interval.ms` | 10000 (10s) | Frequency of heartbeat inserts |
| `heartbeat.action.query` | `INSERT INTO heartbeat (ts) VALUES (now())` | Heartbeat implementation |
| `snapshot.mode` | `initial` | Take initial snapshot before streaming changes |
| `publication.name` | `dbz_publication` | PostgreSQL logical replication publication |
| `slot.name` | `dbz_slot` | PostgreSQL replication slot for this connector |
| `table.include.list` | `public.staging_example` | Tables to capture (modify as needed) |
| `topic.prefix` | `cdc` | Kafka topic prefix for change events |

### Startup Sequence

The connector expects the following initialization order:

1. **PostgreSQL startup** (via docker-compose `depends_on`)
   - Runs `/docker-entrypoint-initdb.d/init.sql`
   - Creates `heartbeat` table ✅
   - Enables logical replication via `postgresql.conf` ✅

2. **Kafka and Zookeeper startup**

3. **Kafka Connect startup** (depends_on PostgreSQL healthy)
   - Registers the connector configuration
   - Connector creates publication and replication slot
   - Connector begins heartbeat inserts to `heartbeat` table

### Health Checks

**Monitor heartbeat inserts**:
```bash
docker-compose exec postgres psql -U middleware -d middleware -c "SELECT COUNT(*), MAX(ts) FROM heartbeat;"
```

Expected output: Increasing count with recent timestamps

**Check connector status**:
```bash
curl -s http://localhost:8083/connectors/postgres-cdc-connector/status | jq '.connector.state'
```

Expected output: `RUNNING`

**View connector logs**:
```bash
docker-compose logs -f kafka-connect | grep -i heartbeat
```

### Troubleshooting

#### Heartbeat Table Missing Error
```
ERROR: Heartbeat action query failed: relation "heartbeat" does not exist
```

**Fix**: Run the initialization SQL manually:
```bash
docker-compose exec postgres psql -U middleware -d middleware -c "CREATE TABLE IF NOT EXISTS heartbeat (ts TIMESTAMP NOT NULL DEFAULT NOW());"
```

Then restart the connector.

#### Logical Replication Not Enabled
```
ERROR: "wal_level" must be "logical" but is "replica"
```

**Fix**: Update `postgresql.conf` with:
```
wal_level = logical
max_wal_senders = 4
max_replication_slots = 4
```

Then restart PostgreSQL container.

#### Publication Already Exists
```
ERROR: publication "dbz_publication" already exists
```

**Fix**: Either reuse the existing publication or manually remove it:
```bash
docker-compose exec postgres psql -U middleware -d middleware -c "DROP PUBLICATION IF EXISTS dbz_publication CASCADE;"
```

Then restart the connector.

### Monitored Tables

Currently configured to capture changes from:
- `public.staging_example`

To add more tables, modify the `table.include.list` parameter in the connector configuration:
```json
"table.include.list": "public.staging_example,public.your_table,public.another_table"
```

### Performance Tuning

| Parameter | Default | Tuning |
|-----------|---------|--------|
| `heartbeat.interval.ms` | 10000 | ↓ Decrease for more frequent heartbeats (lower latency, more overhead) |
| `max.batch.size` | 2048 | ↑ Increase for higher throughput (more memory usage) |
| `poll.interval.ms` | 1000 | ↓ Decrease for lower replication latency (higher CPU) |
| `snapshot.mode` | initial | Consider `never` if replaying from existing snapshot |

### Debugging

Enable verbose logging in Kafka Connect:
```yaml
# Add to docker-compose.yml kafka-connect service environment:
CONNECT_LOG4J_ROOT_LOGLEVEL: "DEBUG"
CONNECT_LOG4J_LOGGERS: "io.debezium=DEBUG"
```

### References

- [Debezium PostgreSQL Connector Documentation](https://debezium.io/documentation/reference/stable/connectors/postgresql.html)
- [PostgreSQL Logical Replication](https://www.postgresql.org/docs/15/logical-replication.html)
- [Kafka Connect Configuration](https://kafka.apache.org/documentation/#connectconfigs)
