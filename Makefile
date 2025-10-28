# Makefile for Actix Web REST API
# Provides common tasks for backend (Rust) and CI/CD

.PHONY: help build build-backend test test-backend \
        dev dev-backend lint format clean docker-build docker-push \
        docker-up-local docker-down-local docker-up-prod docker-down-prod migrate \
        seed-db check-backend

# Default target
all: build

help: ## Display this help message
	@echo "Available targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

# Build targets
build: build-backend ## Build backend

build-backend: ## Build the Rust backend
	cargo build --release

# Test targets
test: test-backend ## Run tests for backend

test-backend: ## Run Rust backend tests
	cargo test

# Development targets
dev-backend: ## Run backend in development mode
	cargo run

# Code quality
lint: lint-backend ## Run linters

lint-backend: ## Lint Rust backend code
	cargo clippy

format: format-backend ## Format code

format-backend: ## Format Rust backend code
	cargo fmt

# Clean
clean: clean-backend ## Clean build artifacts

clean-backend: ## Clean Rust backend build artifacts
	cargo clean

# Database migration
migrate: ## Run database migrations
	diesel migration run

seed-db: ## Seed database with initial data
	psql -f insert_tenants.sql

# Docker targets
docker-build: ## Build Docker image for backend
	docker build -f Dockerfile -t rcs:local .

docker-up-local: ## Start local Docker containers
	docker-compose -f docker-compose.local.yml up -d

docker-down-local: ## Stop and remove local Docker containers
	docker-compose -f docker-compose.local.yml down

docker-up-prod: ## Start production Docker containers
	docker-compose -f docker-compose.prod.yml up -d

docker-down-prod: ## Stop and remove production Docker containers
	docker-compose -f docker-compose.prod.yml down

# CI/CD targets
ci-build: build-backend docker-build docker-push ## CI build pipeline
ci-test: test-backend docker-build ## CI test pipeline
ci-deploy: docker-push ## CI deploy pipeline (push image)

# Health checks
check-backend: ## Check backend compilation without building
	cargo check
