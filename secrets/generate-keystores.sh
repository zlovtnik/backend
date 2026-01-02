#!/bin/bash
# Script to generate JMX keystores and truststores for Kafka and Keycloak
# Run this script ONCE to generate the required certificates and keystores
# 
# Prerequisites:
# - OpenJDK/OpenSSL installed
# - keytool available in PATH
#
# WARNING: This generates self-signed certificates for development only.
# For production, use proper CA-signed certificates.

set -e

SECRETS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Generate secure random passwords if not explicitly provided
if [ -z "${KEYSTORE_PASSWORD}" ]; then
    KEYSTORE_PASSWORD=$(openssl rand -base64 32 | tr -d "=+/" | cut -c1-25)
    echo "Generated KEYSTORE_PASSWORD: $KEYSTORE_PASSWORD" >&2
    export KEYSTORE_PASSWORD
fi

if [ -z "${TRUSTSTORE_PASSWORD}" ]; then
    TRUSTSTORE_PASSWORD=$(openssl rand -base64 32 | tr -d "=+/" | cut -c1-25)
    echo "Generated TRUSTSTORE_PASSWORD: $TRUSTSTORE_PASSWORD" >&2
    export TRUSTSTORE_PASSWORD
fi

# Verify passwords are set
if [ -z "${KEYSTORE_PASSWORD}" ] || [ -z "${TRUSTSTORE_PASSWORD}" ]; then
    echo "ERROR: Failed to generate or set secure passwords" >&2
    exit 1
fi

CERT_VALIDITY_DAYS=365

echo "=== Generating JMX Keystores and Truststores ==="
echo "Location: $SECRETS_DIR"
echo "=== Generating JMX Keystores and Truststores ==="
echo "Location: $SECRETS_DIR"
echo ""
echo ""

# 1. Generate Kafka keystore
echo "[1/6] Generating Kafka server keystore..."
keytool -genkey \
    -alias kafka-server \
    -keyalg RSA \
    -keysize 2048 \
    -keystore "$SECRETS_DIR/kafka.server.keystore.jks" \
    -validity $CERT_VALIDITY_DAYS \
    -storepass "$KEYSTORE_PASSWORD" \
    -keypass "$KEYSTORE_PASSWORD" \
    -dname "CN=kafka-server,OU=Engineering,O=Company,C=US"

# 2. Export Kafka certificate
echo "[2/6] Exporting Kafka server certificate..."
keytool -export \
    -alias kafka-server \
    -keystore "$SECRETS_DIR/kafka.server.keystore.jks" \
    -file "$SECRETS_DIR/kafka-server.crt" \
    -storepass "$KEYSTORE_PASSWORD"

# 3. Generate Kafka truststore and import certificate
echo "[3/6] Creating Kafka truststore with server certificate..."
keytool -import \
    -alias kafka-server \
    -file "$SECRETS_DIR/kafka-server.crt" \
    -keystore "$SECRETS_DIR/kafka.server.truststore.jks" \
    -storepass "$TRUSTSTORE_PASSWORD" \
    -noprompt

# 4. Generate Keycloak keystore
echo "[4/6] Generating Keycloak server keystore..."
keytool -genkey \
    -alias keycloak-server \
    -keyalg RSA \
    -keysize 2048 \
    -keystore "$SECRETS_DIR/keycloak.server.keystore.jks" \
    -validity $CERT_VALIDITY_DAYS \
    -storepass "$KEYSTORE_PASSWORD" \
    -keypass "$KEYSTORE_PASSWORD" \
    -dname "CN=keycloak-server,OU=Engineering,O=Company,C=US"

# 5. Export Keycloak certificate
echo "[5/6] Exporting Keycloak server certificate..."
keytool -export \
    -alias keycloak-server \
    -keystore "$SECRETS_DIR/keycloak.server.keystore.jks" \
    -file "$SECRETS_DIR/keycloak-server.crt" \
    -storepass "$KEYSTORE_PASSWORD"

# 6. Generate Keycloak truststore and import certificate
echo "[6/6] Creating Keycloak truststore with server certificate..."
keytool -import \
    -alias keycloak-server \
    -file "$SECRETS_DIR/keycloak-server.crt" \
    -keystore "$SECRETS_DIR/keycloak.server.truststore.jks" \
    -storepass "$TRUSTSTORE_PASSWORD" \
    -noprompt

# Set proper permissions
chmod 600 "$SECRETS_DIR"/jmxremote.* 2>/dev/null || true
chmod 600 "$SECRETS_DIR"/*.crt
chmod 600 "$SECRETS_DIR"/jmxremote.*

echo ""
echo "=== Generation Complete ==="
echo "Generated files:"
ls -lh "$SECRETS_DIR"/*.jks "$SECRETS_DIR"/*.crt 2>/dev/null || true
echo ""
echo "✅ Keystores and truststores generated successfully!"
echo "⚠️  These are self-signed certificates for development only."
echo "⚠️  For production, use proper CA-signed certificates."
echo ""
echo "To use these secrets with Docker:"
echo "  docker run -v $SECRETS_DIR:/etc/secrets:ro <image>"
