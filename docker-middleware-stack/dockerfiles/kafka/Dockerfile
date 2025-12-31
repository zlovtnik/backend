# Build custom Kafka Docker image with custom configurations

FROM confluentinc/cp-kafka:7.5.0

# Set environment variables
ENV KAFKA_HOME=/opt/kafka \
    KAFKA_HEAP_OPTS="-Xms512M -Xmx1G" \
    KAFKA_JVM_PERFORMANCE_OPTS="-XX:+UseG1GC -XX:MaxGCPauseMillis=20 -XX:InitiatingHeapOccupancyPercent=35"

# Install additional tools if needed
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    jq \
    && rm -rf /var/lib/apt/lists/*

# Copy custom configuration files (optional)
# COPY server.properties ${KAFKA_HOME}/config/server.properties
# COPY log4j.properties ${KAFKA_HOME}/config/log4j.properties

# Create volumes for data persistence
RUN mkdir -p /var/lib/kafka/data /var/log/kafka && \
    chmod -R 755 /var/lib/kafka/data /var/log/kafka

# Expose ports
EXPOSE 9092 9093 9999

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
    CMD kafka-broker-api-versions --bootstrap-server localhost:9093 || exit 1

# Run Kafka
CMD ["kafka-server-start", "/opt/kafka/config/server.properties"]
