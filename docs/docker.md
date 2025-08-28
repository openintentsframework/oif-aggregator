# Docker Setup for OIF Aggregator

## Quick Start

### Development
```bash
make dev          # Start with hot reload
make down         # Stop development environment
```

### Production
```bash
docker build -t oif-aggregator .
docker run -p 4000:4000 -e INTEGRITY_SECRET=your-secret oif-aggregator
```

## Environment Variables

All core settings can be configured via environment variables with config file fallbacks:

| Variable | Default | Description |
|----------|---------|-------------|
| `HOST` | `127.0.0.1` | Server bind address |
| `PORT` | `4000` | Server port |
| `RUST_LOG` | `info` | Log level (debug, info, warn, error) |
| `INTEGRITY_SECRET` | - | **Required**: Secret key for data integrity |

## Development Environment

**Features:**
- 🔄 **Hot reload** with bacon - file changes trigger automatic rebuilds
- 📁 **Volume mounts** - local source code changes are reflected immediately  
- ⚡ **Fast builds** - cargo-chef dependency caching
- 🔧 **Debug logging** - `RUST_LOG=debug` for detailed logs

**Commands:**
```bash
# Quick development workflow
make dev                    # Start development server
make logs-dev              # View logs
make down                  # Stop services

# Docker compose equivalents
docker-compose up          # Start development
docker-compose logs -f     # Follow logs
docker-compose down        # Stop
```

**Environment overrides:**
```yaml
# docker-compose.yml
environment:
  - HOST=0.0.0.0           # Bind to all interfaces
  - PORT=4000              # Override config port
  - RUST_LOG=debug         # Override log level
  - INTEGRITY_SECRET=dev-secret
```

## Production Deployment

**Container run:**
```bash
docker run -d \
  --name oif-aggregator \
  -p 8080:8080 \
  -e HOST=0.0.0.0 \
  -e PORT=8080 \
  -e RUST_LOG=warn \
  -e INTEGRITY_SECRET=production-secret \
  oif-aggregator:latest
```

**Docker Compose:**
```yaml
services:
  oif-aggregator:
    image: oif-aggregator:latest
    ports:
      - "8080:8080"
    environment:
      - HOST=0.0.0.0
      - PORT=8080
      - RUST_LOG=info
      - INTEGRITY_SECRET=production-secret
```

**Health check:**
```bash
curl http://localhost:8080/health
```

## Build Optimization

Uses **cargo-chef** for fast Docker builds:
- ⚡ **First build**: ~10 minutes (330+ dependencies)
- 🔄 **Code changes**: ~30-60 seconds (dependencies cached)
- 🏗️ **Dependency changes**: Only rebuilds affected layers

**Multi-stage process:**
1. **Planner**: Analyzes dependencies
2. **Builder**: Caches and builds dependencies
3. **Runtime**: Minimal production image

Perfect for CI/CD and iterative development! 🚀