# Basic Example Tutorial

This tutorial walks you through setting up and running the OIF Aggregator in just a few minutes.

## Prerequisites

- Docker and Docker Compose installed

## Step 1: Start the OIF Aggregator

The docker-compose.yml will build and start the aggregator automatically:

```bash
# From this directory
docker-compose up -d
```

## Step 2: Understanding the Setup

This example includes:

- **🔧 Custom Config**: `config.json` - simplified configuration for demo
- **🐳 Docker Compose**: Builds and runs the OIF Aggregator with mounted config

### Configuration Highlights

```json
{
  "server": { "host": "0.0.0.0", "port": 4000 },
  "solvers": {
    "demo-solver": {
      "endpoint": "http://127.0.0.1:3000/api",
      "enabled": true
    }
  }
}
```

### Environment Variables

The `docker-compose.yml` demonstrates environment variable overrides:

```yaml
environment:
  - HOST=0.0.0.0              # Server binding
  - PORT=4000                 # Server port
  - RUST_LOG=info             # Log level
  - INTEGRITY_SECRET=example-secret
```

## Step 3: Verify the Service is Running

Check that the aggregator started successfully:

```bash
# View startup logs
docker-compose logs -f

# In another terminal, check health
curl http://localhost:4000/health
```

**What the startup does:**
- 🏗️ Builds the OIF Aggregator from source
- 🚀 Starts OIF Aggregator on port 4000
- 📁 Mounts custom config file
- 🌐 Creates network for future extensions

## Step 4: Explore the API

The aggregator is now running and ready to receive requests:

```bash
# Check aggregator health
curl http://localhost:4000/health

# Get available solvers (will show demo-solver as configured but unhealthy)
curl http://localhost:4000/api/v1/solvers
```

**Expected responses:**
- Health check: Status with solver health information
- Solvers: List of configured solvers (demo-solver will be shown as unhealthy since it's not running)

## Step 5: Understanding the Current Setup

At this point, you have:
- ✅ **OIF Aggregator running** on port 4000
- ✅ **API endpoints available** for quotes and orders  
- ⚠️ **Demo solver configured but not running** (will show as unhealthy)

### API Structure
The aggregator exposes these endpoints:
```bash
GET  /health                 # Service health
GET  /api/v1/solvers            # Available solvers
POST /api/v1/quotes             # Request quotes
POST /api/v1/orders/{id}        # Get order details
POST /api/v1/orders             # Create orders
```

**Note**: Quote and order requests will fail since no solver is running, but the aggregator is ready to forward requests when you connect real solvers.

## Step 6: Customization Options

### Modify Configuration

Edit `config.json` to:
- Add real solver endpoints
- Change timeouts and retries
- Adjust rate limiting
- Update logging levels

### Environment Overrides

Change `docker-compose.yml` environment variables:
```yaml
environment:
  - RUST_LOG=debug           # More verbose logging
  - PORT=8080               # Different port
  - HOST=127.0.0.1          # Local only binding
```

## Step 7: Clean Up

Stop and remove everything:

```bash
# Stop services
docker-compose down

# Remove everything including volumes
docker-compose down -v
```

## Next Steps

🚀 **Ready for more?**
- Try connecting to a real OIF solver
- Explore the [API documentation](../../docs/)
- Set up monitoring and logging
- Deploy to production with security best practices

## Troubleshooting

**Aggregator won't start?**
```bash
# Check logs
docker-compose logs oif-aggregator

# Common issues:
# - INTEGRITY_SECRET not set
# - Port 4000 already in use
# - Config file syntax errors
```

**Want to connect a real solver?**
```bash
# Update config.json with real solver endpoint
# Example: "endpoint": "http://your-solver:3000/api"

# Then restart the service
docker-compose restart
```

**Need debug logs?**
```bash
# Stop and restart with debug logging
docker-compose down
# Edit docker-compose.yml: RUST_LOG=debug
docker-compose up -d
```

---

**🎉 Congratulations!** You've successfully set up the OIF Aggregator with a custom configuration. The aggregator is ready to receive requests and forward them to real solvers when you connect them.
