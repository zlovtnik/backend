# Middleware Stack

A comprehensive Docker-based middleware infrastructure stack for modern applications, featuring Kafka, PostgreSQL, Keycloak, Redis, MinIO, and OpenTelemetry observability.

## 🏗️ Architecture

The stack includes the following services:

### Core Services
- **Apache Kafka** + **Zookeeper** - Event streaming platform
- **PostgreSQL** - Primary database
- **Keycloak** - Identity and access management
- **Redis** - In-memory data store and cache
- **MinIO** - S3-compatible object storage

### Observability Stack
- **OpenTelemetry Collector** - Telemetry data collection
- **Prometheus** - Metrics collection and storage
- **Grafana** - Visualization and dashboards
- **Jaeger** - Distributed tracing

### Management Tools
- **Kafka UI** - Kafka cluster management
- **Redis Commander** - Redis management interface
- **pgAdmin** - PostgreSQL administration
- **MinIO Console** - Object storage management

## 🚀 Quick Start

### Prerequisites
- Docker Engine 20.10+
- Docker Compose 2.0+
- At least 8GB RAM available
- 20GB free disk space
- GNU coreutils (optional; only needed for setup scripts on macOS/Linux — install with `brew install coreutils` on macOS)

### 1. Clone and Setup
```bash
git clone https://github.com/zlovtnik/backend.git
cd backend/docker-middleware-stack
```

### 2. Configure Environment (Optional)
```bash
cp .env.example .env
# Edit .env file with your custom settings
```

### 3. Start the Stack
```bash
docker-compose up -d
```

### 4. Verify Services
```bash
docker-compose ps
```

All services should show as "healthy" or "running".

## 📋 Service Endpoints

Once started, access your services at:

| Service | URL |
|---------|-----|
| **Keycloak** | http://localhost:8080 |
| **MinIO Console** | http://localhost:9001 |
| **Grafana** | http://localhost:3000 |
| **Prometheus** | http://localhost:9090 |
| **Jaeger** | http://localhost:16686 |
| **Kafka UI** | http://localhost:8082 |
| **Redis Commander** | http://localhost:8083 |
| **pgAdmin** | http://localhost:5050 |
| **OpenTelemetry** | localhost:4317 (gRPC), localhost:4318 (HTTP) |

See `.env.example` for default credentials (change in production immediately).

## 🔧 Configuration

### Environment Variables

The stack uses sensible defaults but can be customized via environment variables:

```bash
# Copy example and modify
cp .env.example .env

# Key settings to customize:
POSTGRES_PASSWORD=your_secure_password
KEYCLOAK_ADMIN_PASSWORD=your_secure_password
MINIO_ACCESS_KEY=your_access_key
MINIO_SECRET_KEY=your_secret_key
```

### Database Initialization

PostgreSQL automatically creates these databases and users:
- `middleware` - General purpose database
- `keycloak` - Keycloak's database
- `app` - Application data
- `analytics` - Analytics and reporting

## 🔍 Health Checks

Monitor service health:

```bash
# Check all services
docker-compose ps

# View logs
docker-compose logs [service-name]

# Check specific service health
curl http://localhost:8080/health/ready  # Keycloak
curl http://localhost:9090/-/healthy     # Prometheus
curl http://localhost:13133/health       # OTEL Collector
```

## 📊 Monitoring & Observability

### Metrics
- **Prometheus**: http://localhost:9090
- Scrapes metrics from all services every 15-30 seconds

### Tracing
- **Jaeger**: http://localhost:16686
- Collects distributed traces via OpenTelemetry

### Dashboards
- **Grafana**: http://localhost:3000
- Pre-configured with Prometheus datasource

### Logs
- All services log to stdout/stderr
- View with: `docker-compose logs -f [service-name]`

## 🛠️ Development Workflow

### Adding Custom Configurations

1. **Keycloak Themes**: Add to `configs/keycloak/`
2. **PostgreSQL Extensions**: Modify `configs/postgres/init.sql`
3. **Redis Config**: Edit `configs/redis/redis.conf`
4. **OTEL Pipeline**: Update `configs/otel/collector-config.yaml`

### Scaling Services

```bash
# Scale Kafka brokers
docker-compose up -d --scale kafka=3

# Scale Redis (requires Redis Cluster setup)
docker-compose up -d --scale redis=3
```

### Backup & Restore

```bash
# Backup volumes
docker run --rm -v middleware_postgres_data:/data -v $(pwd):/backup alpine tar czf /backup/postgres-backup.tar.gz -C /data .

# Restore volumes
docker run --rm -v middleware_postgres_data:/data -v $(pwd):/backup alpine tar xzf /backup/postgres-backup.tar.gz -C /data
```

## 🔒 Security Considerations

### Production Deployment
1. **Change default passwords** in `.env`
2. **Enable SSL/TLS** for external access
3. **Configure network isolation** with custom networks
4. **Set resource limits** appropriately
5. **Enable authentication** on all services
6. **Regular security updates** of Docker images

### Required Environment Variables for Production

For secure production deployments, set these environment variables before importing Keycloak realm configuration:

```bash
# Keycloak Client Secrets (generate strong random values)
KEYCLOAK_MIDDLEWARE_APP_SECRET=your-secure-middleware-app-secret-here
KEYCLOAK_GRAFANA_SECRET=your-secure-grafana-secret-here

# Keycloak User Passwords (use strong passwords)
KEYCLOAK_ADMIN_PASSWORD=your-secure-admin-password
KEYCLOAK_USER_PASSWORD=your-secure-user-password
KEYCLOAK_VIEWER_PASSWORD=your-secure-viewer-password

# Grafana Base URL (for OAuth redirect URIs)
GRAFANA_BASE_URL=https://your-grafana-domain.com

# Prometheus Keycloak Authentication
KEYCLOAK_USERNAME=your-prometheus-username
KEYCLOAK_PASSWORD=your-prometheus-password
```

### Keycloak Realm Setup Process

1. **Set Environment Variables**: Export the variables listed above
2. **Run Substitution Script**:
   ```bash
   ./keycloak-env-substitute.sh
   ```
3. **Import Realm**: Import the modified `configs/keycloak/realm-export.json` into Keycloak
4. **Secure Storage**: The realm file is now excluded from git (see `.gitignore`)

### Network Security
- Services communicate via isolated Docker network
- Only necessary ports exposed to host
- Consider using reverse proxy (nginx/traefik) for production

## 🐛 Troubleshooting

### Common Issues

**Port conflicts:**
```bash
# Check what's using ports
lsof -i :5432
# Change ports in docker-compose.yml or .env
```

**Out of memory:**
```bash
# Check memory usage
docker stats
# Increase Docker memory limit or reduce service memory allocation
```

**Service won't start:**
```bash
# Check logs
docker-compose logs [service-name]
# Verify dependencies are healthy
docker-compose ps
```

### Reset Stack
```bash
# Stop and remove everything
docker-compose down -v --remove-orphans

# Clean up images (optional)
docker system prune -a
```

## 📚 Additional Resources

- [Kafka Documentation](https://kafka.apache.org/documentation/)
- [Keycloak Documentation](https://www.keycloak.org/documentation)
- [OpenTelemetry Documentation](https://opentelemetry.io/docs/)
- [Prometheus Documentation](https://prometheus.io/docs/)
- [MinIO Documentation](https://docs.min.io/)

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Test thoroughly
5. Submit a pull request

## 📄 License

This project is licensed under the MIT License - see the LICENSE file for details.

---

**Happy coding!** 🎉
