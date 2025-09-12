# OIF Adapter Guide

The OIF Adapter is the primary adapter for communicating with OIF-compatible solver services. It provides comprehensive JWT authentication support, HTTP client optimization, and flexible configuration through `adapter_metadata`.

## 🎯 Overview

The OIF Adapter (`oif-v1`) implements the standard OIF protocol and serves as the reference implementation for building custom adapters. It demonstrates:

- **JWT Authentication** - Automatic token registration and management
- **HTTP Client Caching** - Connection pooling with authentication support
- **Flexible Configuration** - Extensive use of `adapter_metadata` for customization
- **Error Handling** - Robust error handling and retry logic

## 📋 Basic Configuration

### Minimal Setup

```json
{
  "solvers": {
    "oif-solver-1": {
      "solver_id": "oif-solver-1",
      "adapter_id": "oif-v1",
      "endpoint": "https://example.com/api",
      "enabled": true,
      "name": "OIF Solver",
      "description": "OIF solver"
    }
  }
}
```

### Configuration with Headers

```json
{
  "solvers": {
    "oif-solver-1": {
      "solver_id": "oif-solver-1",
      "adapter_id": "oif-v1", 
      "endpoint": "https://api.oif-solver.com",
      "enabled": true,
      "headers": {
        "X-Client-Version": "1.0.0",
      }
    }
  }
}
```

## 🔐 JWT Authentication

The OIF Adapter supports automatic JWT authentication through the `adapter_metadata` configuration. When enabled, it automatically:

1. **Registers** with the OIF solver using the `/register` endpoint
2. **Caches** JWT tokens per solver instance
3. **Refreshes** tokens automatically when they expire
4. **Includes** `Authorization: Bearer <token>` in all requests

### Basic Authentication Setup

```json
{
  "solvers": {
    "oif-solver-1": {
      "solver_id": "oif-solver-1",
      "adapter_id": "oif-v1",
      "endpoint": "https://api.oif-solver.com",
      "enabled": true,
      "adapter_metadata": {
        "auth": {
          "auth_enabled": true
        }
      }
    }
  }
}
```

### Advanced Authentication Configuration

```json
{
  "solvers": {
    "oif-solver-1": {
      "solver_id": "oif-solver-1", 
      "adapter_id": "oif-v1",
      "endpoint": "https://api.oif-solver.com",
      "enabled": true,
      "adapter_metadata": {
        "auth": {
          "auth_enabled": true,
          "client_name": "Production OIF Aggregator",
          "scopes": ["read-orders", "create-orders"],
          "expiry_hours": 48
        }
      }
    }
  }
}
```

### JWT Configuration Options

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `auth_enabled` | `bool` | `false` | Enable JWT authentication |
| `client_name` | `string` | `"OIF Aggregator - {solver_id}"` | Client name for registration |
| `scopes` | `string[]` | `["read-orders", "create-orders"]` | Requested token scopes |
| `expiry_hours` | `number` | `24` | Token expiry time in hours |

### HTTP Client Caching

The OIF Adapter uses the aggregator's `ClientCache` system for optimal performance:

```rust
// Automatic client caching with JWT token awareness
let client = self.cache.get_authenticated_client(config, jwt_token.as_deref())?;

// Benefits:
// - Connection pooling (up to 10 idle connections per host)
// - Keep-alive connections (90-second reuse)
// - HTTP/2 support with automatic optimization
// - JWT token-aware caching (different tokens use different clients)
// - Thread-safe concurrent access
```

## 📚 Related Documentation

- **[Custom Adapter Guide](custom-adapters.md)** - Building your own adapters
- **[JWT Authentication Guide](../oif_jwt_auth_guide.md)** - Detailed JWT implementation
- **[Configuration Guide](configuration.md)** - Complete configuration reference
- **[API Documentation](api/)** - OIF protocol specification

---

**Source Code**: Check `crates/adapters/src/oif_adapter.rs` for the complete implementation.
