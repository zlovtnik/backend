# Schema Registry with proper JMX file permissions
FROM confluentinc/cp-schema-registry:7.5.0

# Copy JMX files as root (needed for setting permissions)
COPY --chown=appuser:appuser jmxremote.password /opt/schema-registry/jmxremote.password
COPY --chown=appuser:appuser jmxremote.access /opt/schema-registry/jmxremote.access

# Set restrictive permissions for JMX files
RUN chmod 600 /opt/schema-registry/jmxremote.password && \
    chmod 644 /opt/schema-registry/jmxremote.access && \
    chown appuser:appuser /opt/schema-registry/jmxremote.password && \
    chown appuser:appuser /opt/schema-registry/jmxremote.access

# Switch to non-root user before starting
USER appuser

# Override entrypoint to fix permissions
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
