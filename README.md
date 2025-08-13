# OIF Aggregator

A high-performance aggregator for **Open Intent Framework (OIF)** solvers, providing quote aggregation, intent submission, and solver management.

## 🚀 Quick Start

### Run the Server

```bash
# Clone and run with defaults
git clone https://github.com/openintentsframework/oif-aggregator.git
cd oif-aggregator

# Set required environment variable (generate a secure random string)
export INTEGRITY_SECRET="your-secure-random-string-minimum-32-chars"

# Run with OpenAPI documentation
cargo run --features openapi

# Or run with defaults
cargo run
```

The server will start on `http://127.0.0.1:3000` by default.

### API Endpoints

Once running, the following endpoints are available:

- **`GET /health`** - Health check
- **`POST /v1/quotes`** - Get quotes from multiple solvers
- **`POST /v1/orders`** - Submit order for execution
- **`GET /v1/orders/{id}`** - Get order
- **`GET /v1/solvers`** - List all solvers
- **`GET /v1/solvers/{id}`** - Get solver details

## 📖 Features

### 🎯 Core Capabilities
- **Multi-Solver Quote Aggregation** - Fetch quotes from multiple DeFi solvers concurrently
- **ERC-7930 Compliance** - Full support for the Open Intent Framework standard
- **Intent-Based Architecture** - Submit intents and track execution through to settlement
- **Integrity Verification** - HMAC-SHA256 checksums prevent quote tampering
- **Flexible Storage** - In-memory or Redis storage backends
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

## 🛠️ Configuration

### Environment Variables

Set the required integrity secret:

```bash
export INTEGRITY_SECRET="your-secure-random-string-minimum-32-chars"
```

# Option 1: Using openssl
```bash
export INTEGRITY_SECRET=$(openssl rand -base64 32)
```

# Option 2: Using system random
```bash
export INTEGRITY_SECRET=$(head -c 32 /dev/urandom | base64)
```

# Option 3: Manual (replace with your own secure random string, minimum 32 characters)
```bash
export INTEGRITY_SECRET="your-secure-random-string-minimum-32-chars"
```

### Configuration Files

Configuration can be provided via JSON file in the `config/` directory:

```json
{
  "server": {
    "host": "127.0.0.1",
    "port": 3000
  },
  "solvers": {
    "my-solver": {
      "solver_id": "my-solver",
      "adapter_id": "oif-v1",
      "endpoint": "https://api.solver.com/v1",
      "timeout_ms": 2000,
      "enabled": true,
      "max_retries": 3
    }
  },
  "timeouts": {
    "per_solver_ms": 2000,
    "global_ms": 4000
  }
}
```

See [`docs/configuration.md`](docs/configuration.md) for complete configuration options.

## 💻 Programmatic Usage

### Basic Server

```rust
use oif_aggregator::AggregatorBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start complete server with defaults
    AggregatorBuilder::new().start_server().await
}
```

### Custom Configuration

```rust
use oif_aggregator::{AggregatorBuilder, Solver};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create custom solver
    let mut solver = Solver::new(
        "my-solver".to_string(),
        "oif-v1".to_string(),
        "https://api.solver.com".to_string(),
        3000,
    );

    // Build and start with custom solver
    let (router, state) = AggregatorBuilder::new()
        .with_solver(solver).await
        .start().await?;
        
    // Use router and state as needed
    Ok(())
}
```

### Custom Adapter

```rust
use oif_aggregator::{AggregatorBuilder, SolverAdapter};
use async_trait::async_trait;

// Implement custom adapter
struct MyCustomAdapter;

#[async_trait]
impl SolverAdapter for MyCustomAdapter {
    fn id(&self) -> &str { "my-custom-v1" }
    fn name(&self) -> &str { "My Custom Adapter" }
    
    async fn get_quotes(
        &self,
        request: &GetQuoteRequest,
        config: &SolverRuntimeConfig,
    ) -> AdapterResult<GetQuoteResponse> {
        // Your adapter implementation
        todo!()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Register and use custom adapter
    AggregatorBuilder::new()
        .with_adapter(Box::new(MyCustomAdapter))?
        .start_server().await
}
```

## 🏗️ Architecture

The OIF Aggregator follows a modular, crate-based architecture:

```
oif-aggregator/
├── crates/
│   ├── types/       # Core domain models and ERC-7930 types
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

## 🔐 Security

### Authentication & Rate Limiting

Multiple authentication strategies are supported:

```rust
use oif_aggregator::{AggregatorBuilder, ApiKeyAuthenticator, MemoryRateLimiter};

let builder = AggregatorBuilder::new()
    .with_auth(ApiKeyAuthenticator::new())
    .with_rate_limiter(MemoryRateLimiter::with_limits(100, 10));
```

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

# Custom adapter demo
cargo run --example trait_architecture_demo
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
    "total": 2,
    "active": 2,
    "inactive": 0
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

- **Documentation**: See [`docs/`](docs/) directory
- **Examples**: Check [`examples/`](examples/) directory  
- **Issues**: Open a GitHub issue
- **Configuration**: See [`docs/configuration.md`](docs/configuration.md)

---

**Built with ❤️ for the Open Intent Framework ecosystem**