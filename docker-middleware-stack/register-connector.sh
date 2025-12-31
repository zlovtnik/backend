#!/bin/bash

# Debezium PostgreSQL CDC Connector Registration Script
#
# WARNING: The connector uses "snapshot.mode": "initial" which performs a full table snapshot
# on first connector start. This may cause performance issues or prolonged startup time for
# large tables. For production deployments with large datasets, consider alternative snapshot
# modes like "schema_only" or "never" if the initial data load is not required.

# Wait for Kafka Connect to be ready
echo "Waiting for Kafka Connect to be ready..."
ready=false
for i in {1..30}; do
  if curl -s http://localhost:8084/connectors > /dev/null 2>&1; then
    echo "Kafka Connect is ready!"
    ready=true
    break
  fi
  echo "Attempt $i/30: Waiting for Kafka Connect..."
  sleep 2
done

if [ "$ready" = false ]; then
  echo "ERROR: Kafka Connect did not become ready after 30 attempts"
  exit 1
fi

# Register the Debezium PostgreSQL CDC connector
echo "Registering Debezium PostgreSQL CDC connector..."
curl -X POST http://localhost:8084/connectors \
  -H "Content-Type: application/json" \
  -d @./configs/kafka/debezium-postgres-connector.json

echo "Connector registration complete!"
echo ""
echo "View connector status with:"
echo "  curl http://localhost:8084/connectors/postgres-cdc-connector/status"
