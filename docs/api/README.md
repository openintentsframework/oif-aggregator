# API Documentation

This directory contains the auto-generated OpenAPI documentation for the OIF Aggregator API.

## Files

- `openapi.json` - OpenAPI 3.1 specification (auto-generated)
- `index.html` - Swagger UI interface for interactive API documentation

## Local Development

To regenerate the OpenAPI specification:

```bash
# From project root
./scripts/generate-openapi.sh
```

To view the documentation locally, you can serve this directory with any HTTP server:

```bash
# Using Node.js - Multiple options:

# 1. http-server (most popular, no installation needed)
cd docs/api && npx http-server -p 8080 -c-1

# 2. serve (simple and fast)
cd docs/api && npx serve -p 8080

# 3. live-server (with auto-reload for development)
cd docs/api && npx live-server --port=8080 --no-browser

# 4. local-web-server (feature-rich)
cd docs/api && npx local-web-server --port 8080

# Using Python (alternative)
cd docs/api && python -m http.server 8080

# Then open http://localhost:8080 in your browser
```

## Production

The documentation is automatically deployed to GitHub Pages when changes are pushed to the main branch. The workflow:

1. Builds the project with OpenAPI features enabled
2. Generates the latest OpenAPI specification
3. Creates a Swagger UI interface
4. Deploys to GitHub Pages

## API Features

The documented API includes:

- **Health endpoints** - Service health checks and diagnostics
- **Quote endpoints** - Request quotes from multiple solvers
- **Order endpoints** - Submit and track orders
- **Solver endpoints** - Manage and query available solvers

## OpenAPI Generator

The OpenAPI specification is generated using a dedicated binary:

```bash
# Generate spec directly
cargo run --bin generate_openapi --features openapi -p oif-api

# Or use the script
./scripts/generate-openapi.sh
```
