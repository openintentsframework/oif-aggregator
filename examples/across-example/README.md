# Across Solver Tutorial

This tutorial walks you through setting up and running the OIF Aggregator with Across Protocol integration.

## Prerequisites

- Docker and Docker Compose installed

## Step 1: Start the OIF Aggregator

The docker-compose.yml will build and start the aggregator automatically:

```bash
# From this directory
docker-compose up -d
```

## Step 2: Understanding the Across Configuration

This example demonstrates how to configure the OIF Aggregator to work with Across Protocol solvers.

### Configuration Structure

The example includes both mainnet and testnet Across solvers:

```json
{
  "server": { "host": "0.0.0.0", "port": 4000 },
  "solvers": {
    "across-solver-testnet": {
      "solver_id": "across-solver-testnet",
      "adapter_id": "across-v1",
      "endpoint": "https://testnet.across.to/api",
      "timeout_ms": 2000,
      "enabled": true,
      "max_retries": 1,
      "headers": null,
      "name": "Across Solver Testnet",
      "description": "Across Solver Testnet"
    },
    "across-solver-mainnet": {
      "solver_id": "across-solver-mainnet",
      "adapter_id": "across-v1",
      "endpoint": "https://app.across.to/api",
      "timeout_ms": 3000,
      "enabled": true,
      "max_retries": 1,
      "headers": null,
      "name": "Across Solver Mainnet",
      "description": "Across Solver Mainnet"
    }
  },
  "aggregation": {
    "global_timeout_ms": 5000,
    "per_solver_timeout_ms": 3000,
    "max_concurrent_solvers": 50
  }
}
```

### Key Configuration Fields

- **`adapter_id`**: Must be `"across-v1"` to use the Across adapter
- **`endpoint`**: Across API endpoints (mainnet: `https://app.across.to/api`, testnet: `https://testnet.across.to/api`)
- **`solver_id`**: Unique identifier for each solver instance
- **Auto-Discovery**: Routes are automatically discovered from Across `/available-routes` endpoint
- **Timeout Configuration**: Per-solver and global timeout settings

### How Across Integration Works

1. **Route Discovery**: On startup, the aggregator fetches available routes from Across API
2. **Quote Requests**: When quotes are requested, the aggregator calls Across `/suggested-fees` endpoint
3. **Health Monitoring**: Background jobs monitor Across API health via `/available-routes`
4. **Metadata Enrichment**: Responses include detailed Across-specific data like relay fees and fill times

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
- 📁 Mounts custom Across configuration
- 🔗 Connects to Across Protocol API for route discovery

## Step 4: Explore the Across Integration

The aggregator is now running with Across Protocol integration:

```bash
# Check aggregator health
curl http://localhost:4000/health

# Get available solvers (should show Across solver with auto-discovered routes)
curl http://localhost:4000/api/v1/solvers

# Request quotes for cross-chain transfers
curl -X POST http://localhost:4000/api/v1/quotes \
  -H "Content-Type: application/json" \
  -d '{
  "user": "0x000100000314aa37dc742d35cc6634c0532925a3b8d38ba2297c33a9d7",
  "intent": {
    "intentType": "oif-swap",
    "inputs": [
      {
        "user": "0x000100000314aa37dc742d35cc6634c0532925a3b8d38ba2297c33a9d7",
        "asset": "0x000100000314aa37dc4200000000000000000000000000000000000006",
        "amount": "100000000000000"
      }
    ],
    "outputs": [
      {
        "receiver": "0x00010000031401f977742d35cc6634c0532925a3b8d38ba2297c33a9d7",
        "asset": "0x00010000031401f97717b8ee96e3bcb3b04b3e8334de4524520c51cab4",
        "amount": "100000000000000"
      }
    ],
    "swapType": "exact-input",
    "minValidUntil": 300,
    "preference": "speed",
    "partialFill": false
  },
  "supportedTypes": ["oif-escrow-v0"],
  "solverOptions": {
      "timeout": 6000,
      "solverTimeout": 3000
  }
}'
```

**Expected responses:**
- Health check: Status with Across solver health information
- Solvers: Across solver with auto-discovered supported routes
- Quotes: Real quotes from Across Protocol for supported routes

## Step 5: Understanding Across Protocol Integration

At this point, you have:
- ✅ **OIF Aggregator running** on port 4000
- ✅ **Across Protocol connected** with real-time route discovery
- ✅ **Live quotes available** for supported cross-chain routes
- ⚠️ **Order operations not implemented** (see limitations below)

### API Structure
The aggregator exposes these endpoints:
```bash
GET  /health                 # Service health
GET  /api/v1/solvers            # Available solvers (shows Across with routes)
POST /api/v1/quotes             # Request quotes (✅ Fully functional)
POST /api/v1/orders/{id}        # Get order details (❌ Not implemented)
POST /api/v1/orders             # Create orders (❌ Not implemented)
```

### 🚨 Current Limitations

**Order Operations Not Supported:**
- `POST /api/v1/orders` (submit order) - Returns "UnsupportedOperation"
- `GET /api/v1/orders/{id}` (order details) - Returns "UnsupportedOperation"

**Why Order Operations Aren't Implemented:**
1. **Frontend Integration**: Order submission will be handled by frontend applications
2. **User Control**: Users maintain full control over their transactions

**What IS Supported:**
- ✅ **Quote Requests**: Get real quotes from Across Protocol
- ✅ **Route Discovery**: Automatic discovery of supported asset routes  
- ✅ **Health Monitoring**: Real-time Across API health checks
- ✅ **Rich Metadata**: Detailed quote information including fees and timing
- ✅ **Multi-Chain Support**: Ethereum, Optimism, Polygon, Arbitrum, Base, and more
- ✅ **Real-Time Pricing**: Live fee calculations from Across Protocol

### Custom Headers and Timeouts

```json
{
  "solvers": {
    "across-solver-mainnet": {
      "solver_id": "across-solver-mainnet",
      "adapter_id": "across-v1",
      "endpoint": "https://app.across.to/api",
      "timeout_ms": 3000,
      "max_retries": 2,
      "enabled": true,
      "headers": {
        "X-API-Key": "your-api-key",
        "User-Agent": "OIF-Aggregator/1.0"
      }
    }
  },
  "aggregation": {
    "global_timeout_ms": 10000,
    "per_solver_timeout_ms": 4000,
    "max_concurrent_solvers": 5
  }
}
```

### Environment Overrides

Change `docker-compose.yml` environment variables:
```yaml
environment:
  - RUST_LOG=debug           # More verbose logging
  - PORT=8080               # Different port
  - HOST=127.0.0.1          # Local only binding
```

## Step 7: Frontend Integration Planning

### Order Submission Flow

Since order operations are not implemented in the aggregator, frontend applications should:

1. **Get Quotes**: Use the aggregator's `/api/v1/quotes` endpoint
2. **Direct Integration**: Implement Across Protocol SDK directly in frontend
3. **User Signing**: Handle transaction signing in the user's wallet
4. **Status Tracking**: Monitor transaction status via Across APIs

### Future Frontend Support

Order operations will be supported through frontend applications in the near future:
- **Frontend SDKs**: Direct integration with Across Protocol
- **Wallet Integration**: Seamless transaction signing
- **Status Monitoring**: Real-time order tracking
- **User Experience**: Streamlined cross-chain transfers

## Step 8: Clean Up

Stop and remove everything:

```bash
# Stop services
docker-compose down

# Remove everything including volumes
docker-compose down -v
```

## Next Steps

🚀 **Ready for more?**
- Connect additional solver protocols
- Explore the [API documentation](../../docs/)
- Set up monitoring and logging
- Build frontend integration with Across SDK

## Troubleshooting

**Aggregator won't start?**
```bash
# Check logs
docker-compose logs oif-aggregator

# Common issues:
# - INTEGRITY_SECRET not set
# - Port 4000 already in use
# - Config file syntax errors
# - Invalid Across endpoint
```

**Across solver showing as unhealthy?**
```bash
# Check if Across API is accessible
curl https://app.across.to/available-routes

# Verify aggregator logs
docker-compose logs oif-aggregator | grep across

# Test connectivity from container
docker-compose exec oif-aggregator curl https://app.across.to/available-routes
```

**Getting "UnsupportedOperation" for orders?**
```bash
# This is expected behavior - order operations are not implemented
# Use the quotes endpoint instead:
curl -X POST http://localhost:4000/api/v1/quotes -H "Content-Type: application/json" -d '...'
```

**Need debug logs?**
```bash
# Stop and restart with debug logging
docker-compose down
# Edit docker-compose.yml: RUST_LOG=debug
docker-compose up -d
```

**Want to test different routes?**
```bash
# Check available routes first
curl http://localhost:4000/api/v1/solvers

# Then test with supported asset pairs from the routes list
```

---

**🎉 Congratulations!** You've successfully set up the OIF Aggregator with Across Protocol integration. The aggregator can now provide real quotes for cross-chain transfers, with order execution to be handled by frontend applications.
