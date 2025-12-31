.PHONY: help up down restart logs clean status health backup restore check-env test

# Check required environment variables
check-env: ## Check that required JMX environment variables are set
	@echo "Checking required JMX environment variables..."
	@if [ -z "$$JMX_USERNAME" ]; then echo "ERROR: JMX_USERNAME is not set"; exit 1; fi
	@if [ -z "$$JMX_PASSWORD" ]; then echo "ERROR: JMX_PASSWORD is not set"; exit 1; fi
	@echo "All required JMX environment variables are set."

# Default target
help: ## Show this help message
	@echo "Middleware Stack Management Commands:"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-15s %s\n", $$1, $$2}'

# Development commands
up: ## Start all services
	docker-compose up -d

up-build: ## Start all services and rebuild images
	docker-compose up -d --build

down: ## Stop all services
	docker-compose down

restart: ## Restart all services
	docker-compose restart

logs: ## Show logs from all services
	docker-compose logs -f

logs-%: ## Show logs from specific service (e.g., make logs-postgres)
	docker-compose logs -f $*

status: ## Show status of all services
	docker-compose ps

health: ## Check health of all services
	@echo "=== Service Health Check ==="
	@docker-compose ps --format "table {{.Name}}\t{{.Status}}\t{{.Ports}}"
	@echo ""
	@echo "=== Health Endpoints ==="
	@echo "Keycloak:     http://localhost:8080/health/ready"
	@echo "Prometheus:   http://localhost:9090/-/healthy"
	@echo "OTEL:         http://localhost:13133/health"
	@echo "MinIO:        http://localhost:9000/minio/health/live"

# Database commands
db-logs: ## Show database-related logs
	docker-compose logs postgres keycloak

db-shell: ## Connect to PostgreSQL shell
	docker-compose exec postgres psql -U middleware -d middleware

db-dump: ## Create database dump
	docker-compose exec postgres pg_dump -U middleware middleware > middleware_$(shell date +%Y%m%d_%H%M%S).sql

# Kafka commands
kafka-logs: ## Show Kafka-related logs
	docker-compose logs kafka zookeeper schema-registry

kafka-topics: ## List Kafka topics
	docker-compose exec kafka kafka-topics --bootstrap-server localhost:9093 --list

kafka-create-topic: ## Create a Kafka topic (usage: make kafka-create-topic TOPIC=my-topic)
	docker-compose exec kafka kafka-topics --bootstrap-server localhost:9093 --create --topic $(TOPIC) --partitions 3 --replication-factor 1

# Redis commands
redis-logs: ## Show Redis logs
	docker-compose logs redis

redis-cli: ## Connect to Redis CLI
	docker-compose exec redis redis-cli

# MinIO commands
minio-logs: ## Show MinIO logs
	docker-compose logs minio

# Monitoring commands
monitor-logs: ## Show monitoring stack logs
	docker-compose logs prometheus grafana jaeger otel-collector

# Cleanup commands
clean: ## Stop services and remove containers
	docker-compose down --remove-orphans

clean-volumes: ## Stop services and remove containers and volumes (WARNING: destroys data)
	docker-compose down -v --remove-orphans

clean-images: ## Remove all Docker images used by the stack
	docker-compose down --rmi all

prune: ## Deep clean: stop services, remove containers, volumes, and images
	docker-compose down -v --remove-orphans --rmi all
	docker system prune -f

# Backup and restore
backup: ## Backup all persistent data
	@echo "Creating backup directory..."
	$(eval TIMESTAMP := $(shell date +%Y%m%d_%H%M%S))
	@mkdir -p backups/$(TIMESTAMP)
	@echo "Backing up PostgreSQL data..."
	@docker run --rm -v middleware_postgres_data:/data -v $(shell pwd)/backups/$(TIMESTAMP):/backup alpine tar czf /backup/postgres.tar.gz -C /data .
	@echo "Backing up Redis data..."
	@docker run --rm -v middleware_redis_data:/data -v $(shell pwd)/backups/$(TIMESTAMP):/backup alpine tar czf /backup/redis.tar.gz -C /data .
	@echo "Backing up MinIO data..."
	@docker run --rm -v middleware_minio_data:/data -v $(shell pwd)/backups/$(TIMESTAMP):/backup alpine tar czf /backup/minio.tar.gz -C /data .
	@echo "Backup complete: backups/$(TIMESTAMP)"

restore: ## Restore data from backup (usage: make restore BACKUP_DIR=backups/20231218_143022)
	@if [ -z "$(BACKUP_DIR)" ]; then echo "Usage: make restore BACKUP_DIR=backups/20231218_143022"; exit 1; fi
	@echo "Restoring from $(BACKUP_DIR)..."
	@echo "Restoring PostgreSQL data..."
	@docker run --rm -v middleware_postgres_data:/data -v $(shell pwd)/$(BACKUP_DIR):/backup alpine sh -c "cd /data && tar xzf /backup/postgres.tar.gz"
	@echo "Restoring Redis data..."
	@docker run --rm -v middleware_redis_data:/data -v $(shell pwd)/$(BACKUP_DIR):/backup alpine sh -c "cd /data && tar xzf /backup/redis.tar.gz"
	@echo "Restoring MinIO data..."
	@docker run --rm -v middleware_minio_data:/data -v $(shell pwd)/$(BACKUP_DIR):/backup alpine sh -c "cd /data && tar xzf /backup/minio.tar.gz"
	@echo "Restore complete. Restart services with 'make up'"
# Portable timeout function for cross-platform compatibility
# Uses gtimeout (GNU coreutils) if available, otherwise timeout, otherwise shell-based fallback
define timeout_cmd
	$(if $(shell command -v gtimeout 2>/dev/null),gtimeout $1 $2, \
		$(if $(shell command -v timeout 2>/dev/null),timeout $1 $2, \
			sh -c '$2 & pid=$!; (sleep $1 && kill $pid 2>/dev/null &) & killer=$!; wait $pid 2>/dev/null; ret=$?; kill $killer 2>/dev/null; exit $ret' _ ))
endef

# Development helpers
dev: ## Start stack in development mode with logging
	docker-compose up

test: ## Run a quick health check on all services
	@echo "Testing service connectivity..."
	@$(call timeout_cmd,5,bash -c 'until docker-compose exec -T postgres pg_isready -U middleware; do sleep 1; done') && echo "✓ PostgreSQL ready" || echo "✗ PostgreSQL not ready"
	@$(call timeout_cmd,5,bash -c 'until docker-compose exec -T redis redis-cli ping | grep PONG; do sleep 1; done') && echo "✓ Redis ready" || echo "✗ Redis not ready"
	@$(call timeout_cmd,10,bash -c 'until curl -f http://localhost:8080/health/ready; do sleep 1; done') && echo "✓ Keycloak ready" || echo "✗ Keycloak not ready"
	@$(call timeout_cmd,5,bash -c 'until curl -f http://localhost:9090/-/healthy; do sleep 1; done') && echo "✓ Prometheus ready" || echo "✗ Prometheus not ready"
	@echo "Health check complete"

# Info commands
info: ## Show useful information about the stack
	@echo "=== Middleware Stack Information ==="
	@echo "Project:     docker-middleware-stack"
	@echo "Services:    $(shell docker-compose config --services | wc -l)"
	@echo "Networks:    $(shell docker-compose config --services | xargs -I {} docker-compose config | grep -c "networks:")"
	@echo "Volumes:     $(shell docker-compose config | grep -c "volumes:")"
	@echo ""
	@echo "=== Service URLs ==="
	@echo "Keycloak:     http://localhost:8080 (admin/admin123)"
	@echo "MinIO:        http://localhost:9000 (minioadmin/minioadmin123)"
	@echo "Grafana:      http://localhost:3000 (admin/admin123)"
	@echo "Prometheus:   http://localhost:9090"
	@echo "Jaeger:       http://localhost:16686"
	@echo "Kafka UI:     http://localhost:8082"
	@echo "Redis Cmdr:   http://localhost:8083"
	@echo "pgAdmin:      http://localhost:5050 (admin@middleware.local/admin123)"
	@echo ""
	@echo "=== Useful Commands ==="
	@echo "make up          - Start all services"
	@echo "make down        - Stop all services"
	@echo "make logs        - Show all logs"
	@echo "make status      - Show service status"
	@echo "make health      - Check service health"
	@echo "make clean       - Stop and remove containers"
