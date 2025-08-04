# Configuration Guide

The OIF Aggregator supports flexible configuration through multiple sources with the following precedence (highest to lowest):

1. **Environment variables** (highest priority)
2. **`.env` file**
3. **`config/{environment}.json` file**
4. **`config/default.json` file** (lowest priority)

## Environment Variables

Use the `APP__` prefix with double underscores as separators:

```bash
# Server configuration
APP__SERVER__HOST=127.0.0.1
APP__SERVER__PORT=3000

# Timeouts (in milliseconds)
APP__TIMEOUTS__PER_SOLVER_MS=2000
APP__TIMEOUTS__GLOBAL_MS=4000
APP__TIMEOUTS__REQUEST_MS=5000

# Environment settings
APP__ENVIRONMENT__DEBUG=true
APP__ENVIRONMENT__METRICS_ENABLED=true

# Rate limiting
APP__ENVIRONMENT__RATE_LIMITING__ENABLED=false
APP__ENVIRONMENT__RATE_LIMITING__REQUESTS_PER_MINUTE=100

# Logging
APP__LOGGING__LEVEL=debug
APP__LOGGING__FORMAT=pretty
APP__LOGGING__STRUCTURED=false

# Solver configuration
APP__SOLVERS__LIFI_MAINNET__SOLVER_ID=lifi-mainnet
APP__SOLVERS__LIFI_MAINNET__ENDPOINT=https://api.lifi.com/mainnet
APP__SOLVERS__LIFI_MAINNET__TIMEOUT_MS=2000
APP__SOLVERS__LIFI_MAINNET__ENABLED=true
```

## Environment Profiles

Set the runtime environment with:

```bash
RUN_MODE=development  # Uses config/development.json
RUN_MODE=staging      # Uses config/staging.json  
RUN_MODE=production   # Uses config/production.json
```

## Configuration Structure

### Server Settings
- `server.host`: Bind host address (default: "0.0.0.0")
- `server.port`: Bind port (default: 3000)
- `server.workers`: Number of worker threads (optional)

### Solver Configuration
- `solvers.{id}.solver_id`: Unique solver identifier
- `solvers.{id}.adapter_id`: Adapter type to use
- `solvers.{id}.endpoint`: HTTP endpoint URL
- `solvers.{id}.timeout_ms`: Per-solver timeout (1000-3000ms recommended)
- `solvers.{id}.enabled`: Whether solver is active
- `solvers.{id}.max_retries`: Maximum retry attempts
- `solvers.{id}.headers`: Optional HTTP headers

### Timeout Settings
- `timeouts.per_solver_ms`: Individual solver timeout (1000-3000ms)
- `timeouts.global_ms`: Total aggregation timeout (3000-5000ms)
- `timeouts.request_ms`: HTTP client timeout

### Environment Settings
- `environment.profile`: Runtime profile (development/staging/production)
- `environment.debug`: Enable debug mode
- `environment.metrics_enabled`: Enable metrics collection
- `environment.rate_limiting.enabled`: Enable IP-based rate limiting
- `environment.rate_limiting.requests_per_minute`: Rate limit threshold
- `environment.rate_limiting.burst_size`: Burst capacity

### Logging Settings
- `logging.level`: Log level (trace/debug/info/warn/error)
- `logging.format`: Format type (json/pretty/compact)
- `logging.structured`: Enable structured logging

## Example Usage

### Development
```bash
# Use development configuration
RUN_MODE=development cargo run

# Override specific settings
APP__SERVER__PORT=8080 APP__LOGGING__LEVEL=trace cargo run
```

### Production
```bash
# Use production configuration with custom settings
RUN_MODE=production \
APP__SERVER__HOST=0.0.0.0 \
APP__SERVER__PORT=80 \
APP__ENVIRONMENT__DEBUG=false \
cargo run
```

### Docker
```dockerfile
ENV RUN_MODE=production
ENV APP__SERVER__HOST=0.0.0.0
ENV APP__SERVER__PORT=8080
ENV APP__LOGGING__FORMAT=json
```

## Configuration Files

See the `config/` directory for example JSON configurations:
- `config/default.json` - Base configuration
- `config/development.json` - Development overrides
- `config/production.json` - Production settings

Copy `.env.example` to `.env` for local environment variable configuration.