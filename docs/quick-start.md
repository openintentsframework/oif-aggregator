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
    "host": "127.0.0.1",
    "port": 3000
  },
  "solvers": {
    "demo-solver": {
      "solver_id": "demo-solver",
      "adapter_id": "oif-v1",
      "endpoint": "https://demo-solver.example.com/v1",
      "enabled": true,
      "name": "Demo Solver",
      "description": "Example OIF-compatible solver"
    }
  },
  "aggregation": {
    "per_solver_timeout_ms": 2000,
    "global_timeout_ms": 4000,
    "max_quotes_per_solver": 10,
    "quote_preference": "best_price"
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

### Basic Server (Production Ready)
```bash
cargo run --release
```

### Development with API Documentation
```bash
cargo run --features openapi
```

The server will start on `http://127.0.0.1:3000` by default.

## 🧪 Step 4: Test the API

### Health Check
```bash
curl http://localhost:3000/health
```

### Get Available Solvers
```bash
curl http://localhost:3000/v1/solvers
```

### Request Quotes (Example)
```bash
curl -X POST http://localhost:3000/v1/quotes \
  -H "Content-Type: application/json" \
  -d '{
    "available_inputs": [
      {
        "asset": {
          "contract_address": "0xa0b86a33e6e6372aa1aff50b1a1a9a5f7b3b3b3b",
          "symbol": "USDC",
          "name": "USD Coin",
          "decimals": 6,
          "chain_id": 1
        },
        "max_amount": "1000000000",
        "interop_address": {
          "address": "0x1234567890123456789012345678901234567890",
          "domain": "1"
        }
      }
    ],
    "requested_outputs": [
      {
        "asset": {
          "contract_address": "0xb0c86a33e6e6372aa1aff50b1a1a9a5f7b3b3b3c",
          "symbol": "WETH",
          "name": "Wrapped Ether",
          "decimals": 18,
          "chain_id": 1
        },
        "min_amount": "100000000000000000",
        "interop_address": {
          "address": "0x0987654321098765432109876543210987654321",
          "domain": "1"
        }
      }
    ]
  }'
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
- Ensure port 3000 is available (or change in config)

**No quotes returned**
- Verify solver endpoints are reachable
- Check solver configuration in `config/config.json`
- Review logs for timeout or connection errors

**Permission denied errors**
- Ensure you have write permissions in the project directory
- Check if another process is using the configured port

### Getting Help

- **📚 Documentation**: [Complete Documentation](README.md)
- **🐛 Issues**: [GitHub Issues](https://github.com/openintentsframework/oif-aggregator/issues)
- **💬 Discussions**: [GitHub Discussions](https://github.com/openintentsframework/oif-aggregator/discussions)

---

**🎉 Congratulations!** You now have a running OIF Aggregator. Explore the documentation links above to customize it for your specific needs.
