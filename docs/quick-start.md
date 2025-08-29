# Quick Start Guide

Get the OIF Aggregator running in minutes with this step-by-step guide.

## 📋 Prerequisites

- **Rust 1.75+** - [Install Rust](https://rustup.rs/)
- **Git** - For cloning the repository

## 🚀 Step 1: Clone and Setup

```bash
# Clone the repository
git clone https://github.com/openintentsframework/oif-aggregator.git
cd oif-aggregator

# Set required integrity secret (generate a secure random string)
export INTEGRITY_SECRET="your-secure-random-string-minimum-32-chars"
```

> **🔐 Security Note**: The integrity secret should be at least 32 characters long. In production, use a cryptographically secure random string.

## ⚙️ Step 2: Create Configuration

Create a basic configuration file:

```bash
mkdir -p config
cat > config/config.json << 'EOF'
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
EOF
```

## 🎯 Step 3: Run the Server

### Basic Server
```bash
cargo run
```

### Development with API Documentation
```bash
cargo run --features openapi
```

The server will start on `http://0.0.0.0:4000` by default.

## 🧪 Step 4: Test the API

### Health Check
```bash
curl http://localhost:4000/health
```

### Get Available Solvers
```bash
curl http://localhost:4000/v1/solvers
```

## 📚 Next Steps

### 🔧 **Configure for Your Needs**
- **[Configuration Guide](configuration.md)** - Complete configuration reference
- **[Security Guide](security.md)** - Production security setup

### 🚀 **Explore the API**
- **[API Documentation](https://openintentsframework.github.io/oif-aggregator/)** - Interactive Swagger UI
- **[Quotes & Aggregation Guide](quotes-and-aggregation.md)** - Understanding how quotes work

### 🛠️ **Extend and Customize**
- **[Custom Adapter Guide](custom-adapters.md)** - Integrate new solvers
- **[Programmatic Usage Examples](#programmatic-usage)** - Use the aggregator in your code

### 🔍 **Monitor and Maintain**
- **[Maintenance Guide](maintenance.md)** - Operations and monitoring

## 💻 Programmatic Usage

### Basic Server Integration

```rust
use oif_aggregator::AggregatorBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start server with default configuration
    AggregatorBuilder::default().start_server().await
}
```

### Custom Configuration

```rust
use oif_aggregator::{AggregatorBuilder, Solver};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create custom solver
    let solver = Solver::new(
        "my-solver".to_string(),
        "oif-v1".to_string(),
        "https://api.my-solver.com".to_string(),
    );

    // Build and start with custom configuration
    let (_router, _state) = AggregatorBuilder::default()
        .with_solver(solver)
        .start().await?;
    
    Ok(())
}
```

### Adding Custom Adapters

See the **[Custom Adapter Guide](custom-adapters.md)** for complete implementation details.

## 🐛 Troubleshooting

### Common Issues

**Server won't start**
- Check that the `INTEGRITY_SECRET` environment variable is set
- Verify the configuration file syntax with a JSON validator
- Ensure port 4000 is available (or change in config)

### Getting Help

- **📚 Documentation**: [Complete Documentation](README.md)
- **🐛 Issues**: [GitHub Issues](https://github.com/openintentsframework/oif-aggregator/issues)
- **💬 Discussions**: [GitHub Discussions](https://github.com/openintentsframework/oif-aggregator/discussions)

---

**🎉 Congratulations!** You now have a running OIF Aggregator. Explore the documentation links above to customize it for your specific needs.
