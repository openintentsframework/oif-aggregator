# Configuration Guide

This guide covers all configuration options available in the OIF Aggregator, including environment variables, JSON configuration files, and runtime settings.

## 🔧 Configuration Methods

The OIF Aggregator supports multiple configuration methods with the following precedence (highest to lowest):

1. **Environment Variables** - Override any configuration setting
2. **JSON Configuration File** - Structured configuration in `config/config.json`
3. **Default Values** - Built-in sensible defaults

## 🌍 Environment Variables

> **🐳 Docker Users**: See the [Docker Guide](docker.md) for complete environment variable setup in containerized deployments.

### Required Variables

#### `INTEGRITY_SECRET`
**Required for production use**
- **Purpose**: HMAC-SHA256 key for quote integrity verification
- **Format**: String (minimum 32 characters recommended)
- **Example**: `export INTEGRITY_SECRET="your-secure-random-string-minimum-32-chars"`
- **Security**: Use a cryptographically secure random string

### Optional Environment Variables

#### Server Configuration
```bash
# Server binding configuration
export HOST="0.0.0.0"              # Default: "0.0.0.0"
export PORT="4000"                  # Default: 4000

# Logging configuration
export RUST_LOG="info"              # Default: "info"
# Levels: error, warn, info, debug, trace
```

### Environment Variable Overrides

All JSON configuration can be overridden using environment variables:

```bash
# Override server settings
export HOST="0.0.0.0"
export PORT="8080"

# Override log settings
export RUST_LOG=info
```

## 📄 JSON Configuration

### Configuration File Location

Place your configuration file at: `config/config.json`

### Complete Configuration Example

```json
{
  "server": {
    "host": "0.0.0.0",
    "port": 4000
  },
  "solvers": {
    "example-solver": {
      "solver_id": "example-solver",
      "adapter_id": "oif-v1",
      "endpoint": "http://127.0.0.1:3000/api",
      "enabled": true,
      "headers": null,
      "name": "OIF Solver",
      "description": "OIF Solver Description"
    }
  },
  "aggregation": {
    "global_timeout_ms": 4000,
    "per_solver_timeout_ms": 2000,
    "max_concurrent_solvers": 50,
    "max_retries_per_solver": 2,
    "retry_delay_ms": 100
  },
  "environment": {
    "rate_limiting": {
      "enabled": false,
      "requests_per_minute": 1000,
      "burst_size": 100
    }
  },
  "circuit_breaker": {
    "enabled": true,
    "failure_threshold": 5,
    "success_rate_threshold": 0.2,
    "min_requests_for_rate_check": 30,
    "base_timeout_seconds": 10,
    "max_timeout_seconds": 600,
    "half_open_max_calls": 5,
    "max_recovery_attempts": 10,
    "persistent_failure_action": "ExtendTimeout",
    "metrics_max_age_minutes": 30,
    "service_error_threshold": 0.2,
    "metrics_window_duration_minutes": 15,
    "metrics_max_window_age_minutes": 60
  },
  "logging": {
    "level": "debug",
    "format": "compact",
    "structured": false
  },
  "security": {
    "integrity_secret": {
      "type": "env",
      "value": "INTEGRITY_SECRET"
    }
  }
}
```

## ⚙️ Configuration Sections

### Server Configuration

```json
{
  "server": {
    "host": "127.0.0.1",    // Binding address
    "port": 3000            // Binding port
  }
}
```

**Environment Override**: `HOST`, `PORT`

### Solver Configuration

Define external solvers that the aggregator will query:

```json
{
  "solvers": {
    "solver-id": {
      "solver_id": "unique-identifier",
      "adapter_id": "oif-v1",           // Adapter type: "oif-v1", "lifi-v1"
      "endpoint": "https://api.url",    // Solver API endpoint
      "enabled": true,                  // Enable/disable solver
      "headers": {                      // Optional HTTP headers
        "Authorization": "Bearer token",
        "X-Custom": "value"
      },
      "name": "Display Name",           // Human-readable name
      "description": "Solver description"
    }
  }
}
```

**Supported Adapters**:
- `oif-v1` - Standard OIF protocol
- `across-v1` - Across protocol
- Custom adapters can be implemented

#### Supported Assets Configuration

Solvers can define their asset support in two modes:

**Assets Mode** - Simple any-to-any within asset list (including same-chain):
```json
{
  "solver_id": "oif-solver",
  "adapter_id": "oif-v1",
  "endpoint": "https://api.solver.com",
  "enabled": true,
  "supported_assets": {
    "type": "assets",
    "assets": [
      {
        "chain_id": 1,
        "address": "0xA0b86a33E6441e3A1Fa1E0c3e1b6f8c5d6f8e9a0",
        "symbol": "USDC"
      },
      {
        "chain_id": 137,
        "address": "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174",
        "symbol": "USDC"
      }
    ]
  }
}
```

**Routes Mode** - Precise origin→destination pairs:
```json
{
  "solver_id": "across-solver",
  "adapter_id": "across-v1",
  "endpoint": "https://api.across.to",
  "enabled": true,
  "supported_assets": {
    "type": "routes",
    "assets": [
      {
        "origin_chain_id": 1,
        "origin_token_address": "0xA0b86a33E6441e3A1Fa1E0c3e1b6f8c5d6f8e9a0",
        "origin_token_symbol": "USDC",
        "destination_chain_id": 137,
        "destination_token_address": "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174",
        "destination_token_symbol": "USDC"
      }
    ]
  }
}
```

**Auto-Discovery**: If `supported_assets` is omitted or empty, the aggregator will auto-discover assets/routes from the solver's API.

### Aggregation Configuration

```json
{
  "aggregation": {
    "global_timeout_ms": 5000,
    "per_solver_timeout_ms": 2000,
    "max_concurrent_solvers": 50,
    "max_retries_per_solver": 2,
    "retry_delay_ms": 100,
    "include_unknown_compatibility": false
  }
}
```

**Timeout Settings**:
- `global_timeout_ms` should be larger than `per_solver_timeout_ms`
- Consider network latency to external solvers
- Balance between responsiveness and reliability

**Other Settings**:
- `max_concurrent_solvers`: Limit parallel solver requests
- `max_retries_per_solver`: Retry attempts for failed requests
- `retry_delay_ms`: Delay between retry attempts
- `include_unknown_compatibility`: Query solvers without compatibility metadata

### Environment Configuration

```json
{
  "environment": {
    "rate_limiting": {
      "enabled": false,             // Enable/disable rate limiting
      "requests_per_minute": 1000,   // Requests per minute limit
      "burst_size": 100              // Burst capacity
    }
  }
}
```

### Logging Configuration

```json
{
  "logging": {
    "level": "debug",               // Log level: error, warn, info, debug, trace
    "format": "compact",            // Format: json, compact, pretty
    "structured": false             // Enable structured logging
  }
}
```

**Environment Override**: `RUST_LOG` (overrides `level`)

### Circuit Breaker Configuration

The circuit breaker provides automatic failure protection by temporarily blocking requests to failing solvers to prevent cascading failures.

```json
{
  "circuit_breaker": {
    "enabled": true,                           // Enable circuit breaker protection
    "failure_threshold": 5,                    // Consecutive failures to open circuit
    "success_rate_threshold": 0.2,             // Success rate below which to open (20%)
    "min_requests_for_rate_check": 30,         // Minimum requests before checking rate
    "base_timeout_seconds": 10,                // Base timeout when circuit opens
    "max_timeout_seconds": 600,                // Maximum timeout (10 minutes)
    "half_open_max_calls": 5,                  // Test requests in half-open state
    "max_recovery_attempts": 10,               // Max recovery attempts before action
    "persistent_failure_action": "ExtendTimeout",  // Action for persistent failures
    "metrics_max_age_minutes": 30,             // Max age for metrics evaluation
    "service_error_threshold": 0.2,            // Service error rate threshold (20%)
    "metrics_window_duration_minutes": 15,     // Window duration for rate calculations
    "metrics_max_window_age_minutes": 60       // Max age for windowed metrics
  }
}
```

#### Circuit Breaker Settings

**Core Protection Settings:**
- `enabled` - Enable/disable circuit breaker protection (default: `true`)
- `failure_threshold` - Consecutive failures needed to open circuit (default: `5`)
- `success_rate_threshold` - Success rate below which circuit opens (default: `0.2` = 20%)
- `service_error_threshold` - Service error rate threshold (default: `0.2` = 20%)

**Timeout & Recovery Settings:**
- `base_timeout_seconds` - Initial timeout when circuit opens (default: `10`)
- `max_timeout_seconds` - Maximum timeout duration (default: `600` = 10 minutes)
- `half_open_max_calls` - Test requests allowed in half-open state (default: `5`)
- `max_recovery_attempts` - Recovery attempts before persistent action (default: `10`)

**Metrics & Evaluation Settings:**
- `min_requests_for_rate_check` - Minimum requests before rate evaluation (default: `30`)
- `metrics_max_age_minutes` - Maximum age for metrics to be considered (default: `30`)
- `metrics_window_duration_minutes` - Duration of rolling window for rate calculations (default: `15`)
- `metrics_max_window_age_minutes` - Maximum age for windowed metrics (default: `60`)

**Persistent Failure Actions:**
- `"KeepTrying"` - Continue indefinitely with capped timeout
- `"DisableSolver"` - Administratively disable solver (requires manual re-enable)
- `"ExtendTimeout"` - Use 24-hour timeout between recovery attempts (default)

#### Circuit Breaker States

**Closed** - Normal operation, all requests allowed
**Open** - Blocking requests due to failures, waiting for timeout
**HalfOpen** - Testing recovery with limited requests

#### Example Configurations

```json
{
  "circuit_breaker": {
    "enabled": true,
    "failure_threshold": 8,
    "success_rate_threshold": 0.1,
    "min_requests_for_rate_check": 50,
    "base_timeout_seconds": 15,
    "persistent_failure_action": "ExtendTimeout"
  }
}
```

### Security Configuration

```json
{
  "security": {
    "integrity_secret": {
      "type": "env",                // "env" or "plain"
      "value": "INTEGRITY_SECRET"   // Env var name or direct value
    }
  }
}
```

### Maintenance Configuration (Optional)

```json
{
  "maintenance": {
    "order_retention_days": 10      // Days to keep orders before cleanup
  }
}
```

## 🛠️ Configuration Best Practices

### Security
- Always use environment variables for secrets
- Use strong, random integrity secrets (32+ characters)
- Enable rate limiting in production
- Store secrets securely (environment variables, not config files)

### Performance
- Set appropriate timeouts based on solver response times
- Limit concurrent requests to prevent overloading
- Monitor and adjust based on actual usage patterns

### Reliability
- Configure multiple solvers for redundancy
- Set conservative timeouts initially, then optimize
- Enable health checks and monitoring
- Plan for solver maintenance and downtime

### Development
- Use separate configurations for dev/staging/production
- Use plain secrets for local testing (not for production)
- Use localhost endpoints for local development
- Keep development secrets separate from production

### Circuit Breaker
- **Production**: Enable circuit breaker (`enabled: true`) for reliability
- **Start conservative**: Use higher thresholds initially, then tune based on actual solver behavior
- **Monitor metrics**: Track circuit breaker state changes and adjust thresholds accordingly
- **Consider solver types**: High-throughput solvers may need higher `min_requests_for_rate_check`
- **Balance protection vs availability**: Lower thresholds = faster failure detection but more false positives
- **Use `ExtendTimeout`** for persistent failures in production to reduce noise while allowing recovery

---

**💡 Need Help?** 
- Read the [Quotes & Aggregation Guide](quotes-and-aggregation.md) to understand how configuration affects quote processing
- Check the [Security Guide](security.md) for security-related configuration
- See [Maintenance Guide](maintenance.md) for operational configuration
- Review the [Circuit Breaker Implementation Guide](circuit-breaker.md) for detailed circuit breaker implementation and behavior
- Review examples in the [GitHub repository](https://github.com/openintentsframework/oif-aggregator/tree/main/config)
