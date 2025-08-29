# OIF Aggregator

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/openintentsframework/oif-aggregator)
[![API Docs](https://img.shields.io/badge/API-Documentation-blue?style=flat&logo=swagger)](https://openintentsframework.github.io/oif-aggregator/)

A high-performance aggregator for **Open Intent Framework (OIF)** solvers, providing quote aggregation, intent submission, and solver management.

## 🚀 Quick Start

**Get up and running in 5 minutes** with our step-by-step guide:

👉 **[Quick Start Guide](docs/quick-start.md)** - Complete setup with working examples

### TL;DR
```bash
git clone https://github.com/openintentsframework/oif-aggregator.git
cd oif-aggregator
export INTEGRITY_SECRET="your-secure-random-string-minimum-32-chars"
# Create config/config.json (see Quick Start Guide)
cargo run --features openapi
```

**📚 Next Steps:**
- **[Quick Start Guide](docs/quick-start.md)** - Detailed setup with configuration
- **[Configuration Guide](docs/configuration.md)** - Complete configuration reference
- **[API Documentation](https://openintentsframework.github.io/oif-aggregator/)** - Interactive API testing

### API Documentation

📚 **Interactive API Documentation**: [https://openintentsframework.github.io/oif-aggregator/](https://openintentsframework.github.io/oif-aggregator/)

### API Endpoints

Once running, the following endpoints are available:

- **`GET /health`** - Health check
- **`POST /v1/quotes`** - Get quotes from multiple solvers
- **`POST /v1/orders`** - Submit order for execution
- **`GET /v1/orders/{id}`** - Get order
- **`GET /v1/solvers`** - List all solvers
- **`GET /v1/solvers/{id}`** - Get solver details
- **`GET /swagger-ui`** - Swagger UI - Available when run with openapi feature
- **`GET /api-docs/openapi.json`** - OpenAPI specification - Available when run with openapi feature


## 📖 Features

### 🎯 Core Capabilities
- **Multi-Solver Quote Aggregation** - Fetch quotes from multiple DeFi solvers concurrently
- **ERC-7930 Compliance** - Full support for the Open Intent Framework standard
- **Intent-Based Architecture** - Submit intents and track execution through to settlement
- **Integrity Verification** - HMAC-SHA256 checksums prevent quote tampering
- **Flexible Storage** - In-memory storage backend
- **Rate Limiting** - Built-in IP-based rate limiting

### 🔧 Extensibility
- **Custom Adapters** - Easily integrate new solver protocols
- **Builder Pattern** - Flexible programmatic configuration
- **Pluggable Authentication** - Multiple auth strategies (API keys, none, custom)
- **Configurable Timeouts** - Per-solver and global timeout management

### ⚡ Performance & Reliability  
- **Async/Await** - Built on Tokio for high concurrency
- **Error Handling** - Comprehensive error types and recovery
- **Structured Logging** - JSON and pretty-print log formats

## ⚙️ Configuration

The OIF Aggregator supports flexible configuration through environment variables and JSON files.

**📚 Complete Setup Instructions:**
- **[Quick Start Guide](docs/quick-start.md)** - Get running with basic configuration
- **[Configuration Guide](docs/configuration.md)** - Complete configuration reference
- **[Security Guide](docs/security.md)** - Production security setup

**🔐 Required:** Set the integrity secret environment variable:
```bash
export INTEGRITY_SECRET="your-secure-random-string-minimum-32-chars"
```

## 🔌 Integration & Extension

**Building applications that use the OIF Aggregator:**

- **[Quick Start Guide](docs/quick-start.md)** - Complete programmatic usage examples
- **[Custom Adapter Guide](docs/custom-adapters.md)** - Integrate new solver protocols  
- **[Configuration Guide](docs/configuration.md)** - Flexible setup options
- **[API Documentation](https://openintentsframework.github.io/oif-aggregator/)** - Complete HTTP API reference

**Examples included:**
- Basic server integration
- Custom solver configuration  
- Custom adapter implementation
- Advanced builder patterns

## 🏗️ Architecture

The OIF Aggregator follows a modular, crate-based architecture:

```
oif-aggregator/
├── crates/
│   ├── types/       # Core domain models and types
│   ├── service/     # Business logic and aggregation
│   ├── api/         # HTTP API and routing  
│   ├── storage/     # Storage abstractions and implementations
│   ├── adapters/    # Solver adapter implementations
│   └── config/      # Configuration loading and management
├── config/          # JSON configuration
├── examples/        # Usage examples and demos
└── src/             # Main library and binary
```

### Key Components

- **`AggregatorService`** - Core quote aggregation logic
- **`OrderService`** - Intent submission and tracking
- **`SolverService`** - Solver management and discovery
- **`AdapterRegistry`** - Manages protocol adapters (OIF, LiFi, custom)
- **`Storage`** - Trait for persistence (memory, Redis)
- **`IntegrityService`** - HMAC-SHA256 quote verification

## 🧪 Development

### Prerequisites

- Rust 1.75+ 

### Building

```bash
# Build all crates
cargo build

# Build with all features
cargo build --all-features

# Run tests
cargo test

# Run with OpenAPI docs
cargo run --features openapi
```

### Examples

Explore the [`examples/`](examples/) directory:

```bash
# Simple server
cargo run --example simple_server

# Builder pattern demo  
cargo run --example builder_demo

```


## 📊 Monitoring

### Health Checks

```bash
curl http://localhost:3000/health
```

Response:
```json
{
  "status": "healthy",
  "timestamp": 1703123456,
  "version": "0.1.0",
  "solvers": {
    "total": 1,
    "active": 1,
    "inactive": 0,
    "healthy": 1,
    "unhealthy": 0,
    "health_details": {
      "example-solver": false
    }
  },
  "storage": {
    "healthy": true,
    "backend": "memory"
  }
}
```

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Code Style

- Follow Rust naming conventions (`snake_case` for functions, `PascalCase` for types)
- Use `cargo fmt` for formatting
- Ensure `cargo clippy` passes
- Add tests for new functionality

## 📄 License

This project is licensed under the [MIT License](LICENSE).

## 🆘 Support

- **🚀 Quick Start**: [Quick Start Guide](docs/quick-start.md) - Get running in 5 minutes
- **📚 Documentation Hub**: [Complete Documentation](docs/) - Comprehensive guides and references
- **🔧 API Documentation**: [Interactive Swagger UI](https://openintentsframework.github.io/oif-aggregator/)
- **💰 Quotes & Aggregation**: [Quotes Guide](docs/quotes-and-aggregation.md) - How quote aggregation works
- **⚙️ Configuration**: [Configuration Guide](docs/configuration.md) - Complete setup reference
- **🔌 Custom Adapters**: [Custom Adapter Guide](docs/custom-adapters.md) - How to implement solver integrations
- **🔒 Security**: [Security Guide](docs/security.md) - Best practices and guidelines
- **🛠️ Maintenance**: [Maintenance Guide](docs/maintenance.md) - Operations and monitoring
- **📝 Examples**: Check [`examples/`](examples/) directory  
- **🐛 Issues**: Open a GitHub issue

---

**Built with ❤️ for the Open Intent Framework ecosystem**