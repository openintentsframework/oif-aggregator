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

Copy the example configuration file:

```bash
cp config/config.example.json config/config.json
```

This creates a working configuration with:
- **Server**: Runs on `127.0.0.1:4000`
- **Example solver**: OIF-compatible solver endpoint
- **Reasonable defaults**: Timeouts, retries, and rate limiting
- **Debug logging**: Detailed logs for development

**💡 Tip**: Edit `config/config.json` to customize for your needs. See the [Configuration Guide](configuration.md) for all available options.

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
