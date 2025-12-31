#!/bin/bash

# Keycloak Realm Configuration Environment Variable Substitution Script
# This script replaces placeholders in realm-export.json with actual environment variable values
# Run this script before importing the realm into Keycloak

set -e

# Configuration file path (relative to script location)
REALM_FILE="${REALM_FILE:-realm-export.json}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REALM_PATH="${SCRIPT_DIR}/configs/keycloak/${REALM_FILE}"

# Check if realm file exists
if [[ ! -f "$REALM_PATH" ]]; then
    echo "Error: Realm file not found at $REALM_PATH"
    exit 1
fi

# Environment variables with defaults for development
KEYCLOAK_MIDDLEWARE_APP_SECRET="${KEYCLOAK_MIDDLEWARE_APP_SECRET:-middleware-app-secret-dev}"
KEYCLOAK_GRAFANA_SECRET="${KEYCLOAK_GRAFANA_SECRET:-grafana-secret-dev}"
KEYCLOAK_ADMIN_PASSWORD="${KEYCLOAK_ADMIN_PASSWORD:-admin123}"
KEYCLOAK_USER_PASSWORD="${KEYCLOAK_USER_PASSWORD:-user123}"
KEYCLOAK_VIEWER_PASSWORD="${KEYCLOAK_VIEWER_PASSWORD:-viewer123}"
GRAFANA_BASE_URL="${GRAFANA_BASE_URL:-http://localhost:3000}"

echo "Substituting environment variables in $REALM_PATH..."

# Create backup
cp "$REALM_PATH" "${REALM_PATH}.backup"

# Perform substitutions using sed
sed -i.bak \
    -e "s|\${KEYCLOAK_MIDDLEWARE_APP_SECRET}|${KEYCLOAK_MIDDLEWARE_APP_SECRET}|g" \
    -e "s|\${KEYCLOAK_GRAFANA_SECRET}|${KEYCLOAK_GRAFANA_SECRET}|g" \
    -e "s|\${KEYCLOAK_ADMIN_PASSWORD}|${KEYCLOAK_ADMIN_PASSWORD}|g" \
    -e "s|\${KEYCLOAK_USER_PASSWORD}|${KEYCLOAK_USER_PASSWORD}|g" \
    -e "s|\${KEYCLOAK_VIEWER_PASSWORD}|${KEYCLOAK_VIEWER_PASSWORD}|g" \
    -e "s|\${GRAFANA_BASE_URL}|${GRAFANA_BASE_URL}|g" \
    "$REALM_PATH"

echo "Environment variable substitution completed successfully!"
echo ""
echo "Applied substitutions:"
echo "  KEYCLOAK_MIDDLEWARE_APP_SECRET -> ${KEYCLOAK_MIDDLEWARE_APP_SECRET}"
echo "  KEYCLOAK_GRAFANA_SECRET -> ${KEYCLOAK_GRAFANA_SECRET}"
echo "  KEYCLOAK_ADMIN_PASSWORD -> ${KEYCLOAK_ADMIN_PASSWORD}"
echo "  KEYCLOAK_USER_PASSWORD -> ${KEYCLOAK_USER_PASSWORD}"
echo "  KEYCLOAK_VIEWER_PASSWORD -> ${KEYCLOAK_VIEWER_PASSWORD}"
echo "  GRAFANA_BASE_URL -> ${GRAFANA_BASE_URL}"
echo ""
echo "Backup saved as: ${REALM_PATH}.backup"
echo ""
echo "Next steps:"
echo "1. Import the modified realm-export.json into Keycloak"
echo "2. For production, ensure all environment variables are set with secure values"
echo "3. Remove the realm file from source control after import (see .gitignore)"

# Validate JSON syntax
if command -v jq &> /dev/null; then
    echo "Validating JSON syntax..."
    if jq empty "$REALM_PATH" 2>/dev/null; then
        echo "✓ JSON syntax is valid"
    else
        echo "✗ JSON syntax error detected!"
        exit 1
    fi
else
    echo "Warning: jq not found - skipping JSON validation"
fi
