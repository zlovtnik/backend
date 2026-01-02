# Secrets Directory for JMX Configuration

This directory contains sensitive JMX authentication and SSL certificate files for Kafka and Keycloak.

## Files

- **jmxremote.password** - JMX authentication credentials (username:password pairs)
- **jmxremote.access** - JMX access control rules (role permissions)
- **kafka.server.keystore.jks** - Kafka JMX SSL keystore (generated)
- **kafka.server.truststore.jks** - Kafka JMX SSL truststore (generated)
- **keycloak.server.keystore.jks** - Keycloak JMX SSL keystore (generated)
- **keycloak.server.truststore.jks** - Keycloak JMX SSL truststore (generated)

## Setup Instructions

### 1. Generate Keystores and Truststores

Run the generation script to create the necessary keystores:

```bash
bash secrets/generate-keystores.sh
```

Or with custom passwords:

```bash
KEYSTORE_PASSWORD=your-strong-password \
TRUSTSTORE_PASSWORD=your-strong-password \
bash secrets/generate-keystores.sh
```

**Requirements:**
- OpenJDK/Oracle JDK installed (`keytool` command available)
- Bash shell

### 2. Docker Usage

Mount the secrets directory at runtime:

**Docker CLI:**
```bash
docker run \
  -v $(pwd)/secrets:/etc/secrets:ro \
  -e KEYSTORE_PASSWORD=changeit \
  -e TRUSTSTORE_PASSWORD=changeit \
  kafka.latest:latest
```

**Docker Compose:**
```yaml
services:
  kafka:
    build: .
    volumes:
      - ./secrets:/etc/secrets:ro
    environment:
      KEYSTORE_PASSWORD: changeit
      TRUSTSTORE_PASSWORD: changeit

  keycloak:
    build: .
    volumes:
      - ./secrets:/etc/secrets:ro
    environment:
      KEYSTORE_PASSWORD: changeit
      TRUSTSTORE_PASSWORD: changeit
```

### 3. Using with Render or Cloud Platforms

For platforms like Render that don't support local file mounting:

1. Upload secrets as encrypted environment variables or secret files
2. The container will look for files at `/etc/secrets/` at runtime
3. Configure the platform to inject these files into the container

Example for Render:
```bash
# Add as environment variable (base64 encoded file content)
export KEYSTORE_B64=$(base64 < secrets/kafka.server.keystore.jks)
```

## Security Notes

⚠️ **CRITICAL - SECURITY REQUIREMENTS:**

### File Permissions & Version Control

1. **DO NOT commit this directory to version control** - it contains sensitive data
   - Verify: `git status` should show NO tracked files in `secrets/`
   - Added to `.gitignore`: All `jmxremote.*` files and `*.jks` keystores
   - If accidentally committed: Use `git rm --cached secrets/jmxremote.*` to untrack

2. **File Ownership & Permissions at Deployment**
   - Owner: Service account running Kafka/Keycloak (e.g., `kafka:kafka` user)
   - Required Permissions: **400 or 600** (read-only or read-write for owner only)
   - **CRITICAL for JMX startup:** Java JMX requires file ownership and strict permissions
   - Set at deployment time:
     ```bash
     chmod 600 secrets/jmxremote.password secrets/jmxremote.access secrets/*.jks
     chown kafka:kafka secrets/jmxremote.*
     ```

3. **Credentials & Passwords**
   - **DO NOT hardcode credentials** - use environment variables or secret management
   - Never share plaintext passwords in logs, Git history, or documentation
   - Use strong passwords (minimum 25 characters with `openssl rand -base64 32`)
   - These keystores use **self-signed certificates** for development only
   - For **production**, use proper CA-signed certificates
   - Rotate credentials regularly (quarterly or after security incidents)
   - Store in secure secret management system:
     - AWS Secrets Manager
     - HashiCorp Vault
     - Azure Key Vault
     - Kubernetes Secrets (with encryption at rest)

### Role-Based Access Control (JMX)

**User-to-Role Mapping:**
- User `monitorRole` (pwd: `QED`) → **Read-Only Access**
  - Used by: Grafana, Prometheus exporters, OpenTelemetry agents
  - Can: View metrics, broker status, topic statistics
  - Cannot: Modify configuration, restart brokers, reset policies
  
- User `controlRole` (pwd: `R&D`) → **Write Access (Restricted MBeans)**
  - Used by: Kafka operators, cluster maintenance scripts
  - Can: Adjust replicas, modify retention, trigger log cleanup
  - Cannot: Unrestricted read-write (see MBean restrictions below)

**JMX Access Restrictions (jmxremote.access):**

The `controlRole` is restricted to specific MBeans to prevent dangerous operations:

```
# Allowed MBeans for controlRole:
- java.lang.Memory          (heap/non-heap management)
- kafka.server.*            (broker operations, topic management)
- kafka.network.*           (network metrics, connection pools)

# NOT Allowed (for security):
- java.lang.Runtime        (would allow code execution)
- com.sun.management.*     (would allow JVM shutdown)
- Full readwrite           (uncontrolled access)
```

**Testing JMX Access:**
```bash
# Connect with monitorRole (readonly)
jconsole -J-Dcom.sun.management.jmxremote.ssl=false \
  -J-Dcom.sun.management.jmxremote.password.file=secrets/jmxremote.password \
  -J-Dcom.sun.management.jmxremote.access.file=secrets/jmxremote.access \
  localhost:9999
```

### Keystore & Truststore Security

These keystores use **self-signed certificates** for development only


## Current Credentials (⚠️ CHANGE IN PRODUCTION)

**JMX Access Control:**

| Role | User | Password | Access | Used By |
|------|------|----------|--------|---------|
| `monitorRole` | - | `QED` | **Read-Only** | Grafana, Prometheus, OTEL agents |
| `controlRole` | - | `R&D` | **Write (Restricted)** | Kafka operators, maintenance scripts |

**Important:**
- These are default development credentials - **MUST BE CHANGED for production**
- Use environment variables: `KEYSTORE_PASSWORD`, `TRUSTSTORE_PASSWORD` (generated via `openssl rand`)
- File `jmxremote.password` contains: `monitorRole QED` and `controlRole R&D` (plaintext - **protect with file permissions!**)
- File `jmxremote.access` defines MBean restrictions (see Security Notes above)

**Keystore/Truststore:**
- Passwords are randomly generated and provided via environment variables
- **DO NOT use defaults** - must be set at deployment via `KEYSTORE_PASSWORD` and `TRUSTSTORE_PASSWORD` env vars
- See `secrets/generate-keystores.sh` for secure random generation


## Certificate Details

Self-signed certificates generated with:
- Algorithm: RSA
- Key Size: 2048 bits
- Validity: 365 days
- Subject: `CN=<service>-server,OU=Engineering,O=Company,C=US`

## Troubleshooting

**Error: "JMX password file not found"**
- Ensure `secrets/` directory is mounted at runtime
- Check Docker volume mount: `-v $(pwd)/secrets:/etc/secrets:ro`
- Verify file exists: `ls -la secrets/jmxremote.password`

**Error: "Permission denied" when JMX starts**
- **CRITICAL:** File permissions must be 400 or 600
  ```bash
  chmod 600 secrets/jmxremote.password secrets/jmxremote.access secrets/*.jks
  ```
- Verify ownership: `ls -l secrets/jmxremote.*` should show your user or `kafka:kafka`
- In Docker: Ensure files are mounted correctly: `-v $(pwd)/secrets:/etc/secrets:ro`
- In Kubernetes: Use proper secret volume mounts and initContainers for permission setup

**Error: "JMX authentication failed" (invalid user/password)**
- Verify jmxremote.password format: `<username> <password>`
- Check jmxremote.access has matching username/role
- Verify no trailing whitespace or line breaks
- Example correct format:
  ```
  monitorRole   QED
  controlRole   R&D
  ```

**Error: "Keystore file not found"**
- Run `bash secrets/generate-keystores.sh` to generate keystores
- Verify files exist: `ls -la secrets/*.jks`
- Check permissions: `chmod 600 secrets/*.jks`

**Error: "Password incorrect" (keystore/truststore)**
- Ensure `KEYSTORE_PASSWORD` env var matches password used during generation
- Check for special characters - must match exactly
- If lost, regenerate with `secrets/generate-keystores.sh` and update env vars

**Error: "JMX access denied - operation not permitted"**
- Your user role may not have access to that MBean
- Check `jmxremote.access` for role permissions
- `monitorRole` has read-only access (no write operations)
- `controlRole` is restricted to operational MBeans (not full readwrite)
- To test: Use `jconsole` to connect with different roles

**File not tracked in git (good!)**
- Verify with: `git check-ignore -v secrets/jmxremote.*`
- If files are tracked, remove them: `git rm --cached secrets/jmxremote.*`
- Then ensure `.gitignore` contains: `secrets/jmxremote.*` and `secrets/*.jks`


## References

- [JMX Security Documentation](https://docs.oracle.com/javase/8/docs/technotes/guides/management/agent.html)
- [Java Keytool Documentation](https://docs.oracle.com/javase/8/docs/technotes/tools/unix/keytool.html)
- [Kafka Security](https://kafka.apache.org/documentation/#security)
- [Keycloak Security](https://www.keycloak.org/docs/latest/server_admin/index.html#_security_headers)
