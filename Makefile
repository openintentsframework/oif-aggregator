# Makefile for OIF Aggregator Docker operations

.PHONY: help build-dev build-dev-clean build-prod build-prod-clean run-dev run-dev-bg run-prod run-prod-bg logs-dev logs-prod stop-dev stop-prod clean test-dev shell-dev rebuild-dev dev up down

# Default target
help:
	@echo "OIF Aggregator Docker Commands (using Chainguard secure images):"
	@echo ""
	@echo "⚡ Quick Start (Development):"
	@echo "  dev                - Start development environment"
	@echo "  up                 - Start development in background"
	@echo "  down               - Stop development environment"
	@echo ""
	@echo "🚀 Development Commands:"
	@echo "  run-dev            - Run development environment"
	@echo "  run-dev-bg         - Run development environment in background"
	@echo "  stop-dev           - Stop development containers"
	@echo "  logs-dev           - View development logs"
	@echo "  test-dev           - Run tests in development container"
	@echo "  shell-dev          - Shell access to development container"
	@echo "  rebuild-dev        - Rebuild development environment"
	@echo ""
	@echo "🏭 Production Commands:"
	@echo "  run-prod           - Run production environment"
	@echo "  run-prod-bg        - Run production environment in background"
	@echo "  stop-prod          - Stop production containers"
	@echo "  logs-prod          - View production logs"
	@echo ""
	@echo "📦 Build Commands:"
	@echo "  build-dev          - Build development Docker image"
	@echo "  build-dev-clean    - Build development image without cache"
	@echo "  build-prod         - Build production Docker image"
	@echo "  build-prod-clean   - Build production image without cache"
	@echo ""
	@echo "🧹 Cleanup:"
	@echo "  clean              - Remove containers and images"
	@echo ""
	@echo ""

# Build development image (with cargo-chef optimization)
build-dev:
	docker build -f Dockerfile.dev -t oif-aggregator:dev .

# Build development image without cache
build-dev-clean:
	docker build --no-cache -f Dockerfile.dev -t oif-aggregator:dev .

# Build production image (with cargo-chef optimization)
build-prod:
	docker build -t oif-aggregator:latest .

# Build production image without cache (clean build)
build-prod-clean:
	docker build --no-cache -t oif-aggregator:latest .

# Run development environment
run-dev:
	docker compose up

# Run development environment in background
run-dev-bg:
	docker compose up -d

# Run production environment
run-prod:
	@if [ -z "$(INTEGRITY_SECRET)" ]; then \
		echo "Error: INTEGRITY_SECRET environment variable is required"; \
		echo "Usage: INTEGRITY_SECRET=your-secret make run-prod"; \
		exit 1; \
	fi
	docker compose -f docker-compose.prod.yml up

# Run production environment in background
run-prod-bg:
	@if [ -z "$(INTEGRITY_SECRET)" ]; then \
		echo "Error: INTEGRITY_SECRET environment variable is required"; \
		echo "Usage: INTEGRITY_SECRET=your-secret make run-prod-bg"; \
		exit 1; \
	fi
	docker compose -f docker-compose.prod.yml up -d

# View development logs
logs-dev:
	docker compose logs -f

# View production logs
logs-prod:
	docker compose -f docker-compose.prod.yml logs -f

# Stop development containers
stop-dev:
	docker compose down

# Stop production containers
stop-prod:
	docker compose -f docker-compose.prod.yml down

# Clean up containers and images
clean:
	docker compose down -v --rmi all
	docker compose -f docker-compose.prod.yml down -v --rmi all 2>/dev/null || true
	docker system prune -f

# Run tests in development container
test-dev:
	docker compose exec oif-aggregator cargo test

# Shell access to development container
shell-dev:
	docker compose exec oif-aggregator bash

# Rebuild development environment
rebuild-dev:
	docker compose down
	docker compose build --no-cache
	docker compose up

# ⚡ Quick aliases for common development tasks
dev: run-dev
up: run-dev-bg
down: stop-dev
