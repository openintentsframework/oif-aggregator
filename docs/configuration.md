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
- Custom adapters can be implemented

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

---

**💡 Need Help?** 
- Read the [Quotes & Aggregation Guide](quotes-and-aggregation.md) to understand how configuration affects quote processing
- Check the [Security Guide](security.md) for security-related configuration
- See [Maintenance Guide](maintenance.md) for operational configuration
- Review examples in the [GitHub repository](https://github.com/openintentsframework/oif-aggregator/tree/main/config)
