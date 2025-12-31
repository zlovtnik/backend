#!/bin/bash
set -e

# Fix JMX file permissions if they exist
if [ -f /opt/schema-registry/jmxremote.password ] && [ ! -L /opt/schema-registry/jmxremote.password ]; then
    chmod 600 /opt/schema-registry/jmxremote.password
    chown appuser:appuser /opt/schema-registry/jmxremote.password
fi

if [ -f /opt/schema-registry/jmxremote.access ] && [ ! -L /opt/schema-registry/jmxremote.access ]; then
    chmod 644 /opt/schema-registry/jmxremote.access
    chown appuser:appuser /opt/schema-registry/jmxremote.access
fi

# Drop privileges and execute the default entrypoint
exec gosu appuser /etc/confluent/docker/run
