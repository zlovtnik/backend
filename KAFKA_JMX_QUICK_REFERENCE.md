# Kafka JMX Security - Quick Reference Card

## 🔐 Default Mode: PRODUCTION (Secure)

```bash
# Build
docker build -f Dockerfile.kafka -t kafka:secure .

# Run with security files
docker run -d \
  -p 9092:9092 -p 9093:9093 -p 9999:9999 \
  -v ./jmxremote.password:/etc/kafka/secrets/jmxremote.password:ro \
  -v ./jmxremote.access:/etc/kafka/secrets/jmxremote.access:ro \
  -v ./kafka.server.keystore.jks:/etc/kafka/secrets/kafka.server.keystore.jks:ro \
  -v ./kafka.server.truststore.jks:/etc/kafka/secrets/kafka.server.truststore.jks:ro \
  -e KEYSTORE_PASSWORD="changeit" \
  -e TRUSTSTORE_PASSWORD="changeit" \
  kafka:secure
```

### Features
- ✅ JMX Auth: **ENABLED**
- ✅ JMX SSL: **ENABLED**
- ✅ Keystore: **REQUIRED**
- ✅ Password: **REQUIRED**

---

## ⚠️ Development Mode: INSECURE (Testing Only)

```bash
# Build with dev flag
docker build -f Dockerfile.kafka --build-arg JMX_SECURE=false -t kafka:dev .

# Run (no files needed)
docker run -d -p 9092:9092 -p 9093:9093 -p 9999:9999 kafka:dev
```

### Features
- ❌ JMX Auth: **DISABLED**
- ❌ JMX SSL: **DISABLED**
- ⚠️ Warning displayed on startup
- ⚠️ **FOR LOCAL TESTING ONLY**

---

## 🔑 Generate Security Files (One-Time Setup)

```bash
# 1. Password File
cat > jmxremote.password << 'EOF'
monitorRole  QED
controlRole  R&D
EOF
chmod 600 jmxremote.password

# 2. Access File
cat > jmxremote.access << 'EOF'
monitorRole   readonly
controlRole   readwrite
EOF
chmod 600 jmxremote.access

# 3. Keystore (Self-Signed)
keytool -genkey -alias kafka-broker \
  -keyalg RSA -keysize 2048 \
  -keystore kafka.server.keystore.jks \
  -validity 365 -storepass changeit -keypass changeit \
  -dname "CN=kafka,O=Company,L=City,ST=State,C=US"

# 4. Truststore
keytool -export -alias kafka-broker -file kafka-broker.cer \
  -keystore kafka.server.keystore.jks -storepass changeit

keytool -import -alias kafka-broker -file kafka-broker.cer \
  -keystore kafka.server.truststore.jks -storepass changeit -noprompt

# 5. Verify Permissions
ls -la jmxremote.* kafka.server.*
# Should show: -rw------- (600) or -rw-r----- (640)
```

---

## 🐳 Docker Compose (Production)

```yaml
services:
  kafka:
    build:
      context: .
      dockerfile: Dockerfile.kafka
      args:
        JMX_SECURE: "true"
    ports:
      - "9092:9092"
      - "9093:9093"
      - "9999:9999"
    environment:
      KAFKA_BROKER_ID: 1
      KAFKA_ZOOKEEPER_CONNECT: zookeeper:2181
      KEYSTORE_PASSWORD: ${KEYSTORE_PASSWORD}
      TRUSTSTORE_PASSWORD: ${TRUSTSTORE_PASSWORD}
      JMX_SECURE: "true"
    volumes:
      - ./secrets/jmxremote.password:/etc/kafka/secrets/jmxremote.password:ro
      - ./secrets/jmxremote.access:/etc/kafka/secrets/jmxremote.access:ro
      - ./secrets/kafka.server.keystore.jks:/etc/kafka/secrets/kafka.server.keystore.jks:ro
      - ./secrets/kafka.server.truststore.jks:/etc/kafka/secrets/kafka.server.truststore.jks:ro
```

---

## 🧪 Testing JMX Connection

### Using jconsole
```bash
jconsole \
  -Dcom.sun.management.jmxremote.ssl=true \
  -Djavax.net.ssl.trustStore=/path/to/kafka.server.truststore.jks \
  -Djavax.net.ssl.trustStorePassword=changeit \
  service:jmx:rmi:///jndi/rmi://localhost:9999/jmxrmi
```

### Using command line
```bash
# Test connection exists
nc -zv localhost 9999

# Check Kafka logs
docker logs -f kafka
```

---

## 🔄 Startup Script Logic

```
START CONTAINER
    ↓
READ JMX_SECURE env var
    ↓
    ├─→ JMX_SECURE=true (DEFAULT)
    │   ├─ Check: jmxremote.password exists?
    │   ├─ Check: kafka.server.keystore.jks exists?
    │   ├─ Check: kafka.server.truststore.jks exists?
    │   └─ SET: JMX_OPTS with auth=true, ssl=true
    │
    └─→ JMX_SECURE=false (DEV ONLY)
        ├─ Print: ⚠️  WARNING
        └─ SET: JMX_OPTS with auth=false, ssl=false
    ↓
START KAFKA BROKER
    ↓
HEALTHCHECK: kafka-broker-api-versions
```

---

## 🛠️ Troubleshooting

| Issue | Solution |
|-------|----------|
| `ERROR: JMX password file not found` | Mount file: `-v ./jmxremote.password:/etc/kafka/secrets/jmxremote.password:ro` |
| `ERROR: JMX keystore file not found` | Mount file: `-v ./kafka.server.keystore.jks:/etc/kafka/secrets/kafka.server.keystore.jks:ro` |
| `Permission denied` on files | Set: `chmod 600 jmxremote.* kafka.server.*` |
| `SSL Handshake Exception` | Check truststore contains certificate |
| `Authentication failed` | Verify credentials in jmxremote.password |
| `Connection refused` | Check: `docker port kafka \| grep 9999` |

---

## 📊 Environment Variables

| Var | Default | Usage |
|-----|---------|-------|
| `JMX_SECURE` | `true` | `true`=secure, `false`=dev |
| `KEYSTORE_PASSWORD` | `changeit` | JMX keystore password |
| `TRUSTSTORE_PASSWORD` | `changeit` | JMX truststore password |
| `KAFKA_BROKER_ID` | `1` | Broker instance ID |
| `KAFKA_JMX_PORT` | `9999` | JMX listen port |

---

## 💾 .gitignore Entry

```bash
# JMX and Security Files
jmxremote.password
jmxremote.access
kafka.server.keystore.jks
kafka.server.truststore.jks
kafka-broker.cer
*.pem
*.key
*.crt
.env
secrets/
```

---

## ✅ Security Checklist

- [ ] Generated jmxremote.password with correct permissions (600)
- [ ] Generated jmxremote.access with correct permissions (600)
- [ ] Generated kafka.server.keystore.jks
- [ ] Generated kafka.server.truststore.jks
- [ ] Updated docker run/compose with volume mounts
- [ ] Set KEYSTORE_PASSWORD environment variable
- [ ] Set TRUSTSTORE_PASSWORD environment variable
- [ ] Verified files are NOT in git repo
- [ ] Tested with dev mode first
- [ ] Tested JMX connection with jconsole
- [ ] Updated monitoring tools with credentials
- [ ] Documented password storage location
- [ ] Set rotation schedule for certificates

---

## 📝 Notes

- **Default behavior**: Secure mode (auth + SSL enabled)
- **Development testing**: Use `--build-arg JMX_SECURE=false`
- **Never commit secrets**: Add to .gitignore
- **Rotate certificates**: Every 30-90 days
- **Monitor access**: Enable JMX audit logging
- **Test thoroughly**: Use dev mode before production

---

## 🔗 Documentation

Full details: [KAFKA_JMX_SECURITY_GUIDE.md](KAFKA_JMX_SECURITY_GUIDE.md)
Implementation: [KAFKA_JMX_SECURITY_IMPLEMENTATION.md](KAFKA_JMX_SECURITY_IMPLEMENTATION.md)
