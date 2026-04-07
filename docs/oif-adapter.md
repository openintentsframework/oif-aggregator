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

### Local Solver Setup

For local development against [`oif-solver`](https://github.com/openintentsframework/oif-solver)-style auth, the aggregator-side config above is only one half of the setup.

Your solver must also:

```bash
export JWT_SECRET=replace_with_a_stable_random_value
export AUTH_PUBLIC_REGISTER_ENABLED=true
```

And the solver runtime config must keep:

```json
{
  "auth_enabled": true
}
```

Without `AUTH_PUBLIC_REGISTER_ENABLED=true`, the solver can require JWTs for `/orders` while still refusing public self-registration at `/api/v1/auth/register`, which prevents the aggregator from bootstrapping its token.

### Local Aggregator Example

```json
{
  "solvers": {
    "example-authenticated-solver": {
      "solver_id": "example-authenticated-solver",
      "adapter_id": "oif-v1",
      "endpoint": "http://127.0.0.1:3000/api/v1",
      "enabled": true,
      "adapter_metadata": {
        "auth": {
          "auth_enabled": true,
          "client_name": "OIF Aggregator - local",
          "scopes": ["read-orders", "create-orders"],
          "expiry_hours": 12
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

### Runtime Behavior

When `adapter_metadata.auth.auth_enabled` is `true`, the adapter will:

1. Call `POST /api/v1/auth/register` when no cached token exists
2. Call `POST /api/v1/auth/refresh` when the access token expires and the refresh token is still valid
3. Send `Authorization: Bearer <token>` on solver requests
4. Retry once after a solver `401` by clearing the cached token and re-registering

### HTTP Client Caching

The OIF Adapter uses the aggregator's `ClientCache` system for optimal performance:

```rust
// Automatic client caching with JWT token awareness
let client = self.cache.get_configured_client(config)?;

// Benefits:
// - Connection pooling (up to 10 idle connections per host)
// - Keep-alive connections (90-second reuse)
// - HTTP/2 support with automatic optimization
// - JWT token-aware caching (different tokens use different clients)
// - Thread-safe concurrent access
```

## 📚 Related Documentation

- **[Custom Adapter Guide](custom-adapters.md)** - Building your own adapters
- **[Across Adapter Guide](across-adapter.md)** - Across Protocol integration guide
- **[JWT Authentication Guide](../oif_jwt_auth_guide.md)** - Detailed JWT implementation
- **[Configuration Guide](configuration.md)** - Complete configuration reference
- **[API Documentation](api/)** - OIF protocol specification

---

**Source Code**: Check `crates/adapters/src/oif_adapter.rs` for the complete implementation.
