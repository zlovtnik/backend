# JMX Security Hardening & Deployment Guide

## Overview

This document provides comprehensive guidance on securing JMX (Java Management Extensions) for Kafka and Keycloak deployments. The setup implements role-based access control with the principle of least privilege.

**Date**: January 2, 2026  
**Status**: Fully Implemented  
**Scope**: Kafka & Keycloak JMX security, file permissions, credential management

---

## 1. Security Architecture

### Role-Based Access Control (RBAC)

Two JMX roles are defined with specific permissions:

| Role | User | Password | Purpose | Access Level | Used By |
|------|------|----------|---------|--------------|---------|
| `monitorRole` | - | `QED` | **Observability** | Read-Only | Grafana, Prometheus, OTEL agents |
| `controlRole` | - | `R&D` | **Operations** | Write (Restricted MBeans) | Kafka ops team, automation scripts |

### Access Control Rules (Principle of Least Privilege)

**File**: `secrets/jmxremote.access`

```
monitorRole   readonly
controlRole   readwrite \
  java.lang.Memory, \
  kafka.server.*, \
  kafka.network.*
```

**Why This Matters:**
- `monitorRole` has **read-only** access - cannot modify anything
- `controlRole` is **restricted to specific MBeans** (not full `readwrite`)
  - Allowed: Memory management, Kafka broker operations, network metrics
  - Denied: JVM runtime (would enable code execution), system management
  - Denied: Unrestricted write operations that could crash the cluster

### Threat Model & Mitigations

| Threat | Impact | Mitigation |
|--------|--------|-----------|
| **Unrestricted controlRole access** | Attacker could restart broker, reset security, execute code | Restricted to operational MBeans only |
| **Plaintext password in logs** | Credentials exposed in Git history | Use environment variables, added to .gitignore |
| **Weak keystore passwords** | SSL keys compromised | Generated with `openssl rand -base64 32` (25+ chars) |
| **World-readable secret files** | Anyone can read credentials | Set permissions to 600 (owner read-write only) |
| **Missing jmxremote.password file** | JMX startup failure | Validated in Dockerfile startup script |

---

## 2. File Configuration

### jmxremote.password

**Location**: `secrets/jmxremote.password`  
**Permissions**: 600 (read-write for owner only)  
**Format**: `<username> <password>` (one per line)

```plaintext
monitorRole  QED
controlRole  R&D
```

⚠️ **CRITICAL**: This file contains plaintext passwords. Protect it at all times.

### jmxremote.access

**Location**: `secrets/jmxremote.access`  
**Permissions**: 600 (read-write for owner only)  
**Format**: Role-based MBean restrictions

```plaintext
# Read-only access for monitoring
monitorRole   readonly

# Write access restricted to operational MBeans
controlRole   readwrite \
  java.lang.Memory, \
  kafka.server.*, \
  kafka.network.*
```

### Keystores & Truststores

**Files**:
- `secrets/kafka.server.keystore.jks`
- `secrets/kafka.server.truststore.jks`
- `secrets/keycloak.server.keystore.jks`
- `secrets/keycloak.server.truststore.jks`

**Permissions**: 600  
**Generation**: `bash secrets/generate-keystores.sh`  
**Password Management**: Via `KEYSTORE_PASSWORD` and `TRUSTSTORE_PASSWORD` env vars

---

## 3. Deployment Checklist

### Pre-Deployment

- [ ] Generate keystores (if not already done):
  ```bash
  bash secrets/generate-keystores.sh
  ```

- [ ] Verify file permissions:
  ```bash
  ls -la secrets/jmxremote.*
  ls -la secrets/*.jks
  # Expected: -rw------- (600)
  ```

- [ ] Verify files are NOT tracked in Git:
  ```bash
  git check-ignore -v secrets/jmxremote.* secrets/*.jks
  # Expected: Each file should be listed as ignored
  ```

- [ ] Set strong passwords for keystore/truststore:
  ```bash
  export KEYSTORE_PASSWORD=$(openssl rand -base64 32 | tr -d "=+/" | cut -c1-25)
  export TRUSTSTORE_PASSWORD=$(openssl rand -base64 32 | tr -d "=+/" | cut -c1-25)
  echo "KEYSTORE_PASSWORD=$KEYSTORE_PASSWORD"
  echo "TRUSTSTORE_PASSWORD=$TRUSTSTORE_PASSWORD"
  ```

### Docker Deployment

#### Option 1: Local Development

```bash
docker-compose up -d kafka keycloak \
  -e KEYSTORE_PASSWORD="your-strong-password" \
  -e TRUSTSTORE_PASSWORD="your-strong-password" \
  -v $(pwd)/secrets:/etc/secrets:ro
```

#### Option 2: Docker Stack (Production)

```yaml
version: '3.8'
services:
  kafka:
    build: .
    environment:
      JMX_SECURE: "true"
      KEYSTORE_PASSWORD: ${KEYSTORE_PASSWORD}
      TRUSTSTORE_PASSWORD: ${TRUSTSTORE_PASSWORD}
    volumes:
      - ./secrets:/etc/secrets:ro
    ports:
      - "9999:9999"  # JMX port

  keycloak:
    build: .
    environment:
      JMX_SECURE: "true"
      KEYSTORE_PASSWORD: ${KEYSTORE_PASSWORD}
      TRUSTSTORE_PASSWORD: ${TRUSTSTORE_PASSWORD}
    volumes:
      - ./secrets:/etc/secrets:ro
```

**Start with environment file:**
```bash
export KEYSTORE_PASSWORD="..."
export TRUSTSTORE_PASSWORD="..."
docker-compose --env-file .env.secrets up
```

#### Option 3: Kubernetes Deployment

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: jmx-secrets
type: Opaque
data:
  jmxremote.password: <base64-encoded-file>
  jmxremote.access: <base64-encoded-file>
  kafka.server.keystore.jks: <base64-encoded-file>
  kafka.server.truststore.jks: <base64-encoded-file>

---
apiVersion: v1
kind: Pod
metadata:
  name: kafka
spec:
  initContainers:
  - name: fix-permissions
    image: busybox
    command: ['sh', '-c', 'chmod 600 /etc/secrets/*']
    volumeMounts:
    - name: jmx-secrets
      mountPath: /etc/secrets
  
  containers:
  - name: kafka
    image: kafka:latest
    env:
    - name: KEYSTORE_PASSWORD
      valueFrom:
        secretKeyRef:
          name: jmx-passwords
          key: keystore-password
    - name: TRUSTSTORE_PASSWORD
      valueFrom:
        secretKeyRef:
          name: jmx-passwords
          key: truststore-password
    volumeMounts:
    - name: jmx-secrets
      mountPath: /etc/secrets
      readOnly: true
  
  volumes:
  - name: jmx-secrets
    secret:
      secretName: jmx-secrets
      defaultMode: 0600  # Critical: Set permissions to 600
  - name: jmx-passwords
    secret:
      secretName: jmx-passwords
```

### Post-Deployment Verification

1. **Check JMX connectivity** (port 9999):
   ```bash
   # Using jconsole
   jconsole -J-Dcom.sun.management.jmxremote.ssl=false localhost:9999
   
   # Using jmxterm (more automated)
   echo "bean kafka.server:type=BrokerTopicMetrics,name=MessagesInPerSec" | \
     jmxterm -h localhost -p 9999 -u monitorRole -p QED
   ```

2. **Verify file permissions in container**:
   ```bash
   docker exec kafka ls -la /etc/secrets/jmxremote.*
   # Expected: -rw------- (600)
   ```

3. **Check for permission errors in logs**:
   ```bash
   docker logs kafka | grep -i "permission\|jmx"
   ```

4. **Test read-only access (monitorRole)**:
   ```bash
   # Connect with monitorRole - should succeed
   # Try to write - should fail
   ```

5. **Test restricted write access (controlRole)**:
   ```bash
   # Connect with controlRole
   # Try to access kafka.server.* MBeans - should succeed
   # Try to access java.lang.Runtime - should fail
   ```

---

## 4. Credential Management

### Default Credentials (CHANGE IN PRODUCTION)

**JMX Users** (in `jmxremote.password`):
- `monitorRole` / `QED` (development only)
- `controlRole` / `R&D` (development only)

**Keystore/Truststore**:
- Randomly generated during `secrets/generate-keystores.sh`
- Passed via `KEYSTORE_PASSWORD` and `TRUSTSTORE_PASSWORD` env vars

### Changing Credentials

1. **Update JMX user passwords** (if compromised):
   ```bash
   # Edit secrets/jmxremote.password
   # Change: monitorRole <new-password>
   # Change: controlRole <new-password>
   # Restart Kafka/Keycloak containers
   ```

2. **Rotate keystore passwords** (quarterly):
   ```bash
   # Regenerate keystores
   bash secrets/generate-keystores.sh
   
   # Update environment variables
   export KEYSTORE_PASSWORD="new-strong-password"
   export TRUSTSTORE_PASSWORD="new-strong-password"
   
   # Restart containers
   docker-compose up -d --force-recreate kafka keycloak
   ```

### Secret Management Best Practices

For production environments, use one of:

**AWS:**
```bash
# Store in AWS Secrets Manager
aws secretsmanager create-secret \
  --name kafka-jmx/keystore-password \
  --secret-string "$(openssl rand -base64 32)"

# Retrieve during deployment
export KEYSTORE_PASSWORD=$(aws secretsmanager get-secret-value \
  --secret-id kafka-jmx/keystore-password \
  --query SecretString --output text)
```

**HashiCorp Vault:**
```bash
vault kv put secret/kafka/jmx \
  keystore_password="$(openssl rand -base64 32)" \
  truststore_password="$(openssl rand -base64 32)"

vault kv get -field=keystore_password secret/kafka/jmx
```

**Azure Key Vault:**
```bash
az keyvault secret set \
  --vault-name my-vault \
  --name kafka-keystore-password \
  --value "$(openssl rand -base64 32)"
```

---

## 5. Monitoring & Troubleshooting

### JMX Connection Testing

```bash
# Test with jconsole (GUI)
jconsole -J-Dcom.sun.management.jmxremote.ssl=false \
  -J-Dcom.sun.management.jmxremote.password.file=secrets/jmxremote.password \
  -J-Dcom.sun.management.jmxremote.access.file=secrets/jmxremote.access \
  localhost:9999

# Test with jmxterm (CLI)
jmxterm -h localhost -p 9999 -u monitorRole -p QED

# Test with netcat (connection only)
timeout 5 nc -zv localhost 9999
```

### Common Issues & Solutions

| Issue | Cause | Solution |
|-------|-------|----------|
| "Permission denied" on startup | File permissions > 600 | `chmod 600 secrets/jmxremote.*` |
| "JMX password file not found" | File not mounted in container | Check Docker volume mount: `-v $(pwd)/secrets:/etc/secrets:ro` |
| "Authentication failed" | Wrong password or user | Verify `jmxremote.password` format: `<user> <password>` |
| "Access denied" to MBean | Role doesn't have permission | Check `jmxremote.access` MBean restrictions |
| "Certificate validation failed" | SSL issues | Verify keystore files mounted, passwords match |

### JMX Logs

Monitor these files for JMX activity:

```bash
# Kafka logs
docker logs kafka | grep -i "jmx"

# Keycloak logs
docker logs keycloak | grep -i "jmx"

# System logs (if running on host)
journalctl -u kafka -f
journalctl -u keycloak -f
```

---

## 6. Version Control & Secrets

### Verification

Confirm these files are NOT tracked:

```bash
# Check .gitignore
cat .gitignore | grep -A 5 "Secrets - JMX"

# Verify files are ignored
git check-ignore -v secrets/jmxremote.* secrets/*.jks

# Check git status
git status | grep secrets
# Expected: "nothing to commit" for secrets directory
```

### If Accidentally Committed

```bash
# Remove from Git history (WARNING: this rewrites history)
git rm --cached secrets/jmxremote.password secrets/jmxremote.access secrets/*.jks
git commit --amend "Remove sensitive JMX files"

# Then force push (be careful!)
git push --force-with-lease

# Rotate all credentials (assume they're compromised)
bash secrets/generate-keystores.sh
# Change jmxremote.password user credentials
```

---

## 7. Documentation Reference

| File | Purpose |
|------|---------|
| `secrets/README.md` | Detailed setup and security notes |
| `secrets/generate-keystores.sh` | Secure keyststore/truststore generation |
| `secrets/jmxremote.password` | JMX user credentials |
| `secrets/jmxremote.access` | JMX role-based access control |
| `Dockerfile.kafka` | Kafka container with JMX validation |
| `.gitignore` | Prevents committing secrets |
| `JMX_SECURITY_HARDENING.md` | This file - comprehensive deployment guide |

---

## 8. Summary of Improvements

### What Was Changed

1. **jmxremote.access**: Updated to restrict `controlRole` to specific operational MBeans instead of full `readwrite`
2. **.gitignore**: Added explicit entries for all JMX secret files to prevent accidental commits
3. **secrets/README.md**: Expanded with detailed role mapping, file permission requirements, and troubleshooting
4. **File Permissions**: Verified all files are set to 600 (owner read-write only)
5. **Documentation**: Created this comprehensive deployment guide

### Security Benefits

✅ **Principle of Least Privilege**: controlRole restricted to necessary operations  
✅ **Credential Protection**: Proper file permissions and VCS exclusion  
✅ **Audit Trail**: Clear documentation of who needs which access  
✅ **Production Ready**: Deployment examples for Docker, Kubernetes, and cloud platforms  
✅ **Incident Response**: Clear guidelines for credential rotation and recovery  

### Next Steps

1. Review credentials in `jmxremote.password` - change for production
2. Run `bash secrets/generate-keystores.sh` to generate fresh keystores
3. Set `KEYSTORE_PASSWORD` and `TRUSTSTORE_PASSWORD` env vars
4. Verify JMX connectivity with monitoring tools
5. Deploy to staging environment first
6. Monitor logs for JMX-related errors during deployment

---

**Last Updated**: January 2, 2026  
**Maintained By**: Security Team  
**Next Review**: Quarterly or after security incidents
