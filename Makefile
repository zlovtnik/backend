# Makefile for Actix Web REST API with Frontend
# Provides common tasks for backend (Rust), frontend (TypeScript/React), and CI/CD

.PHONY: help build build-backend build-frontend test test-backend test-frontend \
        dev dev-backend dev-frontend lint format clean docker-build docker-push \
        docker-up-local docker-down-local docker-up-prod docker-down-prod migrate \
        seed-db check-backend check-frontend

# Default target
all: build

help: ## Display this help message
	@echo "Available targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

# Build targets
build: build-backend build-frontend ## Build both backend and frontend

build-backend: ## Build the Rust backend
	cargo build --release

build-frontend: ## Build the frontend
	cd frontend && bun run build

# Test targets
test: test-backend test-frontend ## Run tests for both backend and frontend

test-backend: ## Run Rust backend tests
	cargo test

test-frontend: ## Run frontend tests
	cd frontend && bun run test

# Development targets
dev-backend: ## Run backend in development mode
	cargo run

dev-frontend: ## Run frontend in development mode
	cd frontend && bun run dev

# Code quality
lint: lint-backend ## Run linters

lint-backend: ## Lint Rust backend code
	cargo clippy

format: format-backend ## Format code

format-backend: ## Format Rust backend code
	cargo fmt

# Clean
clean: clean-backend clean-frontend ## Clean build artifacts

clean-backend: ## Clean Rust backend build artifacts
	cargo clean

clean-frontend: ## Clean frontend build artifacts
	cd frontend && rm -rf dist

# Database migration
migrate: ## Run database migrations
	diesel migration run

seed-db: ## Seed database with initial data
	psql -f insert_tenants.sql

# Docker targets
docker-build: ## Build Docker image for backend
	docker build -f Dockerfile.local -t rcs:local .

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
ci-test: test-backend test-frontend docker-build ## CI test pipeline
ci-deploy: docker-push ## CI deploy pipeline (push image)

# Health checks
check-backend: ## Check backend compilation without building
	cargo check

check-frontend: ## Check frontend dependencies and build
	cd frontend && bun install && bun run build --dry-run || echo "Dry run not supported"
