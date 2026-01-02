# JMX Credential Rotation & Secret Management Guide

**Date**: January 2, 2026  
**Status**: Critical Security Update  
**Scope**: JMX credential rotation, secret management best practices, incident response

---

## ⚠️ CRITICAL - Credentials Have Been Rotated

The default JMX credentials in `secrets/jmxremote.password` have been replaced with a template.

**Previous Credentials (NOW COMPROMISED/REVOKED):**
- ❌ `monitorRole` / `QED` (REVOKED)
- ❌ `controlRole` / `R&D` (REVOKED)

**Status:**
- ✅ Plaintext credentials removed from repository
- ✅ File added to `.gitignore` to prevent future commits
- ✅ Template created with secure password generation instructions
- ✅ Not previously committed to git history (safe from history leak)

---

## 1. Immediate Actions Required

### For Development Environment

If you were using the default credentials locally:

```bash
# 1. Generate new secure passwords
MONITOR_PWD=$(openssl rand -base64 32 | tr -d "=+/" | cut -c1-25)
CONTROL_PWD=$(openssl rand -base64 32 | tr -d "=+/" | cut -c1-25)

echo "New monitorRole password: $MONITOR_PWD"
echo "New controlRole password: $CONTROL_PWD"

# 2. Update the local file
cat > secrets/jmxremote.password << EOF
monitorRole  $MONITOR_PWD
controlRole  $CONTROL_PWD
EOF

# 3. Set restrictive permissions
chmod 600 secrets/jmxremote.password

# 4. Verify (should show 600 permissions)
ls -la secrets/jmxremote.password

# 5. Update environment variables for Docker/Kubernetes
export JMX_MONITOR_PASSWORD="$MONITOR_PWD"
export JMX_CONTROL_PASSWORD="$CONTROL_PWD"
```

### For Production Environments

If these credentials were exposed to production systems:

1. **IMMEDIATELY revoke** any active JMX connections from monitoring tools
2. **Stop** Kafka/Keycloak containers using old credentials
3. **Generate new passwords** using the procedure above
4. **Update secret manager** with new credentials:
   - AWS Secrets Manager
   - HashiCorp Vault
   - Azure Key Vault
   - Environment variables
5. **Restart** services with new credentials
6. **Audit** JMX access logs for unauthorized access patterns

---

## 2. Generate Secure Passwords

### Using OpenSSL (Recommended)

```bash
# Generate 25-character secure password
openssl rand -base64 32 | tr -d "=+/" | cut -c1-25

# Generate multiple passwords
for role in monitorRole controlRole; do
  pwd=$(openssl rand -base64 32 | tr -d "=+/" | cut -c1-25)
  echo "$role  $pwd"
done
```

### Using /dev/urandom

```bash
# Alternative method
cat /dev/urandom | tr -dc '[:alnum:]!@#$%^&*()_+-=' | head -c 25

# Or with python
python3 -c "import secrets; print(secrets.token_urlsafe(20))"
```

### Password Requirements

- ✅ Minimum length: **16 characters** (25+ recommended)
- ✅ Character set: Mix of uppercase, lowercase, numbers, special characters
- ✅ Unique per role: Don't reuse same password
- ✅ Rotate quarterly: Change credentials every 3 months
- ✅ Strong entropy: Use cryptographically secure random generation

---

## 3. Store in Secret Manager

### AWS Secrets Manager

```bash
# Create new secrets
aws secretsmanager create-secret \
  --name kafka-jmx/monitor-password \
  --secret-string "$(openssl rand -base64 32 | tr -d '=+/' | cut -c1-25)"

aws secretsmanager create-secret \
  --name kafka-jmx/control-password \
  --secret-string "$(openssl rand -base64 32 | tr -d '=+/' | cut -c1-25)"

# Update existing secrets
aws secretsmanager put-secret-value \
  --secret-id kafka-jmx/monitor-password \
  --secret-string "new-password"

# Retrieve during deployment
export MONITOR_PASSWORD=$(aws secretsmanager get-secret-value \
  --secret-id kafka-jmx/monitor-password \
  --query SecretString --output text)
```

### HashiCorp Vault

```bash
# Store credentials
vault kv put secret/kafka/jmx \
  monitor_password="$(openssl rand -base64 32 | tr -d '=+/' | cut -c1-25)" \
  control_password="$(openssl rand -base64 32 | tr -d '=+/' | cut -c1-25)"

# Retrieve during deployment
export MONITOR_PASSWORD=$(vault kv get -field=monitor_password secret/kafka/jmx)
export CONTROL_PASSWORD=$(vault kv get -field=control_password secret/kafka/jmx)

# Set secret versioning
vault secrets enable -version=2 kv  # Enable versioning for audit trail
```

### Azure Key Vault

```bash
# Store credentials
az keyvault secret set \
  --vault-name my-vault \
  --name kafka-jmx-monitor-password \
  --value "$(openssl rand -base64 32 | tr -d '=+/' | cut -c1-25)"

az keyvault secret set \
  --vault-name my-vault \
  --name kafka-jmx-control-password \
  --value "$(openssl rand -base64 32 | tr -d '=+/' | cut -c1-25)"

# Retrieve during deployment
export MONITOR_PASSWORD=$(az keyvault secret show \
  --vault-name my-vault \
  --name kafka-jmx-monitor-password \
  --query value -o tsv)
```

### Environment Variables (Simple, Less Secure)

```bash
# For development/CI environments only (not recommended for production)
export JMX_MONITOR_PASSWORD="generated-secure-password-here"
export JMX_CONTROL_PASSWORD="generated-secure-password-here"

# In Docker
docker run \
  -e JMX_MONITOR_PASSWORD="$JMX_MONITOR_PASSWORD" \
  -e JMX_CONTROL_PASSWORD="$JMX_CONTROL_PASSWORD" \
  kafka:latest
```

---

## 4. Update Deployment Configuration

### Docker Compose

```yaml
version: '3.8'
services:
  kafka:
    build: .
    environment:
      # Read from environment variables set at deployment time
      JMX_MONITOR_PASSWORD: ${JMX_MONITOR_PASSWORD}
      JMX_CONTROL_PASSWORD: ${JMX_CONTROL_PASSWORD}
      KEYSTORE_PASSWORD: ${KEYSTORE_PASSWORD}
      TRUSTSTORE_PASSWORD: ${TRUSTSTORE_PASSWORD}
    volumes:
      - ./secrets:/etc/secrets:ro  # Contains jmxremote.access, keystores
    ports:
      - "9999:9999"  # JMX port (internal only in production)
```

**Deploy with secrets:**
```bash
# Load secrets from secret manager
eval $(aws secretsmanager get-secret-value \
  --secret-id kafka-jmx-credentials \
  --query 'SecretString' --output text | jq -r \
  'to_entries | .[] | "export \(.key)=\(.value)"')

# Start with environment
docker-compose up
```

### Kubernetes

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: kafka-jmx-credentials
type: Opaque
stringData:
  monitor-password: '<SECURE-PASSWORD-HERE>'
  control-password: '<SECURE-PASSWORD-HERE>'

---
apiVersion: v1
kind: ConfigMap
metadata:
  name: kafka-jmx-config
data:
  jmxremote.access: |
    monitorRole   readonly
    controlRole   readwrite \
      java.lang.Memory, \
      kafka.server.*, \
      kafka.network.*

---
apiVersion: v1
kind: Pod
metadata:
  name: kafka
spec:
  containers:
  - name: kafka
    image: kafka:latest
    env:
    - name: JMX_MONITOR_PASSWORD
      valueFrom:
        secretKeyRef:
          name: kafka-jmx-credentials
          key: monitor-password
    - name: JMX_CONTROL_PASSWORD
      valueFrom:
        secretKeyRef:
          name: kafka-jmx-credentials
          key: control-password
    volumeMounts:
    - name: jmx-config
      mountPath: /etc/secrets
      readOnly: true
  volumes:
  - name: jmx-config
    configMap:
      name: kafka-jmx-config
      defaultMode: 0600
```

### Update Docker Entrypoint

In your startup script, read from environment and write credentials securely:

```bash
#!/bin/bash
set -e

# Read from environment (set by secret manager at deploy time)
if [ -z "$JMX_MONITOR_PASSWORD" ]; then
  echo "ERROR: JMX_MONITOR_PASSWORD not set"
  exit 1
fi

if [ -z "$JMX_CONTROL_PASSWORD" ]; then
  echo "ERROR: JMX_CONTROL_PASSWORD not set"
  exit 1
fi

# Write credentials securely to file (with restricted permissions)
umask 077  # Ensure files are created with 600 permissions
cat > /etc/secrets/jmxremote.password << EOF
monitorRole  $JMX_MONITOR_PASSWORD
controlRole  $JMX_CONTROL_PASSWORD
EOF

# Verify permissions
chmod 600 /etc/secrets/jmxremote.password

# Clear environment variables to avoid leaking in process listing
unset JMX_MONITOR_PASSWORD JMX_CONTROL_PASSWORD

# Start Kafka with JMX
exec kafka-broker-start /etc/kafka/server.properties
```

---

## 5. Rotation Schedule

### Quarterly Rotation

```bash
#!/bin/bash
# scheduled-rotation.sh - Run quarterly (cron job every 90 days)

# Generate new passwords
MONITOR_PWD=$(openssl rand -base64 32 | tr -d "=+/" | cut -c1-25)
CONTROL_PWD=$(openssl rand -base64 32 | tr -d "=+/" | cut -c1-25)

# Update secret manager
aws secretsmanager put-secret-value \
  --secret-id kafka-jmx/monitor-password \
  --secret-string "$MONITOR_PWD"

aws secretsmanager put-secret-value \
  --secret-id kafka-jmx/control-password \
  --secret-string "$CONTROL_PWD"

# Log the rotation
echo "$(date): JMX credentials rotated successfully" >> /var/log/kafka-rotation.log

# Alert ops team
mail -s "JMX Credentials Rotated" ops-team@company.com << EOF
JMX credentials have been rotated at $(date).
New credentials stored in AWS Secrets Manager.
Kafka pods will be restarted during next deployment.
EOF
```

### Add to Cron

```bash
# Rotate JMX credentials quarterly (every 90 days)
0 0 1 */3 * /opt/scripts/scheduled-rotation.sh >> /var/log/rotation.log 2>&1
```

### Kubernetes CronJob

```yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: jmx-credential-rotation
spec:
  schedule: "0 0 1 */3 *"  # 1st day of every 3rd month
  jobTemplate:
    spec:
      template:
        spec:
          containers:
          - name: rotator
            image: amazon/aws-cli:latest
            command:
            - /bin/bash
            - -c
            - |
              set -e
              PWD=$(openssl rand -base64 32 | tr -d "=+/" | cut -c1-25)
              aws secretsmanager put-secret-value \
                --secret-id kafka-jmx/monitor-password \
                --secret-string "$PWD"
              echo "Rotated at $(date)"
            env:
            - name: AWS_REGION
              value: us-east-1
            volumeMounts:
            - name: aws-creds
              mountPath: /root/.aws
              readOnly: true
          restartPolicy: OnFailure
          volumes:
          - name: aws-creds
            secret:
              secretName: aws-credentials
```

---

## 6. Incident Response - Credential Compromise

If credentials are compromised:

### Step 1: Immediate Containment (Minutes)

```bash
# Stop JMX connections
docker-compose stop kafka keycloak

# Generate new emergency passwords
EMERGENCY_MONITOR=$(openssl rand -base64 32 | tr -d "=+/" | cut -c1-25)
EMERGENCY_CONTROL=$(openssl rand -base64 32 | tr -d "=+/" | cut -c1-25)

echo "Emergency credentials generated - store safely"
echo "Monitor: $EMERGENCY_MONITOR"
echo "Control: $EMERGENCY_CONTROL"
```

### Step 2: Update Secrets (Hours)

```bash
# Update all secret managers
aws secretsmanager put-secret-value \
  --secret-id kafka-jmx/monitor-password \
  --secret-string "$EMERGENCY_MONITOR"

# Update all running services
kubectl set env statefulset/kafka JMX_MONITOR_PASSWORD="$EMERGENCY_MONITOR" --all-namespaces
```

### Step 3: Investigation (24 Hours)

```bash
# Check JMX access logs
grep "Authentication failed" /var/log/kafka/jmx*.log

# Check failed connection attempts
netstat -an | grep 9999
ss -an | grep 9999

# Audit AWS API calls
aws cloudtrail lookup-events --lookup-attributes AttributeKey=EventName,AttributeValue=AssumeRole
```

### Step 4: Communication & Documentation

- [ ] Notify security team
- [ ] Document in incident log
- [ ] Alert customers (if production)
- [ ] Update incident timeline
- [ ] Post-mortem review (24-48 hours)

---

## 7. File Cleanup

### Remove Plaintext Files

```bash
# Securely erase password files
shred -vfz -n 5 secrets/jmxremote.password  # 5-pass overwrite
shred -vfz -n 5 ~/.bash_history              # Remove from shell history

# Alternative - using dd
dd if=/dev/zero of=secrets/jmxremote.password bs=1M count=1
rm secrets/jmxremote.password

# Verify file is gone
ls -la secrets/jmxremote.password  # Should show "No such file"
```

### Prevent Plaintext Leaks

```bash
# Don't log credentials
set +x  # Disable shell debugging before reading secrets
export JMX_PASSWORD="$secret"
set -x

# Use read -s for interactive entry
read -s "JMX_PASSWORD?Enter password: "

# Don't commit sample passwords
git diff --stat | grep jmxremote
```

---

## 8. Summary

| Action | Status | Next Step |
|--------|--------|-----------|
| Plaintext credentials removed | ✅ Complete | Monitor for leaks |
| New credentials generated | ⏳ Pending | Generate per environment |
| Secret manager updated | ⏳ Pending | Push to AWS/Vault/Azure |
| Deployment updated | ⏳ Pending | Read from env vars |
| Rotation schedule set | ⏳ Pending | Configure cron/K8s job |
| Staff trained | ⏳ Pending | Review this guide |

---

**Last Updated**: January 2, 2026  
**Maintained By**: Security Team  
**Review Frequency**: Quarterly or after incidents
