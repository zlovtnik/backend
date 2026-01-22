#!/bin/bash
set -m

# Start Kafka in background
/etc/confluent/docker/run > /var/log/kafka/startup.log 2>&1 &
KAFKA_PID=$!

# Tail the log to stdout
tail -f /var/log/kafka/startup.log &
TAIL_PID=$!

# Wait for Kafka to finish
wait $KAFKA_PID
EXIT_CODE=$?

echo ""
echo "Kafka exited with code: $EXIT_CODE"

# Kill the tail process
kill $TAIL_PID 2>/dev/null

exit $EXIT_CODE
