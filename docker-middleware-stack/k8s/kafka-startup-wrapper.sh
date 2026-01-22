#!/bin/bash
set -e

echo "=== Kafka Startup Wrapper ==="
echo "KAFKA_ZOOKEEPER_CONNECT=${KAFKA_ZOOKEEPER_CONNECT}"
echo "KAFKA_ADVERTISED_LISTENERS=${KAFKA_ADVERTISED_LISTENERS}"
echo "KAFKA_LISTENERS=${KAFKA_LISTENERS}"
echo "KAFKA_BROKER_ID=${KAFKA_BROKER_ID}"

# Check Zookeeper connectivity first
echo "Checking Zookeeper connectivity to ${KAFKA_ZOOKEEPER_CONNECT}..."
IFS=':' read -r ZK_HOST ZK_PORT <<< "${KAFKA_ZOOKEEPER_CONNECT}"
for i in {1..30}; do
  if nc -z "$ZK_HOST" "$ZK_PORT" 2>/dev/null; then
    echo "Zookeeper is reachable!"
    break
  else
    echo "Waiting for Zookeeper... ($i/30)"
    sleep 1
  fi
done

echo "Starting Kafka..."
exec /etc/confluent/docker/run
