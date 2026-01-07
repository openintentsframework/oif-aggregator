# Custom Adapter Implementation Guide

This guide explains how to create custom adapters for the OIF Aggregator, enabling integration with new solvers and protocols.

## 🎯 Overview

Adapters serve as the bridge between the OIF Aggregator and external solver protocols. They translate the standard OIF requests into solver-specific API calls and convert responses back to the standard format.

### Architecture

```mermaid
graph TD
    A[OIF Aggregator] --> B[Adapter Registry]
    B --> C[OIF Adapter]
    B --> D[Custom Adapter]
    C --> E[OIF-Compatible Solver]
    D --> F[Your Solver API]
```

### Custom Adapter Integration Points

```mermaid
graph LR
    A[Your Solver API] --> B[Custom Adapter]
    B --> C[SolverAdapter Trait]
    C --> D[get_quotes]
    C --> E[submit_order]
    C --> F[health_check]
    C --> G[get_order_details]
    C --> H[get_supported_assets]
    
    B --> J[Adapter Registry]
    J --> K[OIF Aggregator]
    
    style A fill:#fff3e0
    style B fill:#e8f5e8
    style C fill:#f3e5f5
    style K fill:#e1f5fe
```

## 🛠️ Implementing a Custom Adapter

### Reference Implementations

Before implementing your own adapter, check the existing implementations in `crates/adapters/src/` for reference:

- **`oif_adapter.rs`** - Standard OIF protocol adapter implementation
- **`across_adapter.rs`** - Across Protocol integration with dynamic query parameters ([guide](across-adapter.md))
- **`mod.rs`** - Adapter registry and factory patterns

These provide working examples of:
- HTTP client configuration and error handling
- Request/response transformation patterns
- Authentication and header management
- Timeout and retry logic
- Testing patterns and mock implementations

### 1. Dependencies

Add these dependencies to your `Cargo.toml`:

```toml
[dependencies]
oif-types = { path = "../oif-aggregator/crates/types" }
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
reqwest = { version = "0.11", features = ["json"] }
tokio = { version = "1.0", features = ["full"] }
```

### 2. Basic Adapter Structure

Create your adapter by implementing the `SolverAdapter` trait:

```rust
use async_trait::async_trait;
use oif_types::{
    adapters::{
        traits::SolverAdapter,
        SolverRuntimeConfig, AdapterResult,
    },
    models::{Asset, Network},
    Adapter, OifGetQuoteRequest, OifGetQuoteResponse, OifPostOrderRequest, OifPostOrderResponse, OifGetOrderResponse,
};

#[derive(Debug)]
pub struct MyCustomAdapter {
    adapter_info: Adapter,
    client: reqwest::Client,
}

impl MyCustomAdapter {
    pub fn new() -> Self {
        let adapter_info = Adapter::new(
            "my-custom-v1".to_string(),
            "Custom adapter for My Solver".to_string(),
            "My Custom Adapter".to_string(),
            "1.0.0".to_string(),
        );

        Self {
            adapter_info,
            client: reqwest::Client::new(),
        }
    }

    pub fn with_default_config() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SolverAdapter for MyCustomAdapter {
    fn adapter_info(&self) -> &Adapter {
        &self.adapter_info
    }

    async fn get_quotes(
        &self,
        request: &OifGetQuoteRequest,
        config: &SolverRuntimeConfig,
    ) -> AdapterResult<OifGetQuoteResponse> {
        // 1. Convert OifGetQuoteRequest to your solver's model
        // 2. Fetch quotes from solver endpoint  
        // 3. Convert solver response to OifGetQuoteResponse format
        todo!("Implement quote fetching logic")
    }

    async fn submit_order(
        &self,
        order: &OifPostOrderRequest,
        config: &SolverRuntimeConfig,
    ) -> AdapterResult<OifPostOrderResponse> {
        // 1. Convert OifPostOrderRequest to your solver's model
        // 2. Submit order to solver endpoint
        // 3. Convert solver response to OifPostOrderResponse format
        todo!("Implement order submission logic")
    }

    async fn health_check(&self, config: &SolverRuntimeConfig) -> AdapterResult<bool> {
        // 1. Make health check request to solver endpoint
        // 2. Return true if solver is healthy, false otherwise
        todo!("Implement health check logic")
    }

    async fn get_order_details(
        &self,
        order_id: &str,
        config: &SolverRuntimeConfig,
    ) -> AdapterResult<OifGetOrderResponse> {
        // 1. Fetch order status from solver endpoint using order_id
        // 2. Convert solver response to OifGetOrderResponse format
        todo!("Implement order details fetching logic")
    }

    async fn get_supported_assets(
        &self,
        config: &SolverRuntimeConfig,
    ) -> AdapterResult<SupportedAssetsData> {
        // Choose your mode:
        
        // Option 1: Assets mode (any-to-any, including same-chain)
        let assets = vec![/* fetch from API */];
        Ok(SupportedAssetsData::Assets(assets))
        
        // Option 2: Routes mode (precise origin->destination pairs)
        let routes = vec![/* fetch from API */];
        Ok(SupportedAssetsData::Routes(routes))
    }
}
```

### 🎯 Choosing Assets vs Routes Mode

**Use Assets Mode When:**
- ✅ Your solver supports any-to-any conversions within asset list
- ✅ You want to support same-chain swaps
- ✅ Route list would be very large (O(N²) explosion)
- ✅ Example: DEX aggregators, OIF protocol

**Use Routes Mode When:**
- ✅ Your solver has specific supported routes
- ✅ Not all asset pairs are supported
- ✅ You want precise compatibility checking
- ✅ Example: Bridge protocols, Across protocol

**Auto-Discovery vs Manual Configuration:**
- **Auto-Discovery**: Omit `supported_assets` - adapter fetches from API
- **Manual Config**: Define `supported_assets` - use static configuration

## 📝 Registration and Configuration

### 1. Register Your Adapter

```rust
use oif_aggregator::AggregatorBuilder;
use oif_types::solvers::Solver;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create your custom adapter
    let custom_adapter = MyCustomAdapter::new();
    
    // Create a solver configuration for your adapter
    let custom_solver = Solver::new(
        "my-custom-solver".to_string(),
        "my-custom-v1".to_string(),      // Must match your adapter's ID
        "https://api.my-solver.com".to_string(),
    );
    
    // Register adapter and solver with the aggregator
    let (_app, _state) = AggregatorBuilder::default()
        .with_adapter(Box::new(custom_adapter))
        .with_solver(custom_solver)
        .start()
        .await?;
    
    // The app router is ready to handle HTTP requests
    // Available endpoints: /health, /api/v1/quotes, /api/v1/orders, /api/v1/solvers
    
    Ok(())
}
```

### 2. Configuration File

Add your solver to `config/config.json`:

```json
{
  "solvers": {
    "my-solver": {
      "solver_id": "my-solver",
      "adapter_id": "my-custom-v1",
      "endpoint": "https://api.mysolver.com/v1",
      "enabled": true,
      "headers": {
        "Authorization": "Bearer your-api-key",
        "X-Custom-Header": "value"
      },
      "adapter_metadata": {
        "custom_feature": true
      },
      "name": "My Custom Solver",
      "description": "Integration with My Custom Solver"
    }
  }
}
```

### 3. Adapter Metadata Configuration

The `adapter_metadata` field allows you to pass custom JSON configuration to your adapter implementation, enabling flexible customization without code changes.

```rust
#[async_trait]
impl SolverAdapter for MyCustomAdapter {
    async fn get_quotes(
        &self,
        request: &OifGetQuoteRequest,
        config: &SolverRuntimeConfig,
    ) -> AdapterResult<OifGetQuoteResponse> {
        // Access optional adapter metadata
        if let Some(metadata) = &config.adapter_metadata {
            // Parse your custom configuration
            let timeout = metadata.get("timeout_ms")
                .and_then(|v| v.as_u64())
                .unwrap_or(5000);
                
            // Use in your adapter logic
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_millis(timeout))
                .build()?;
        }
        
        // Your implementation here...
        todo!()
    }
}
```

**Configuration Example:**
```json
{
  "solvers": {
    "my-solver": {
      "solver_id": "my-solver",
      "adapter_id": "my-custom-v1", 
      "endpoint": "https://api.mysolver.com/v1",
      "enabled": true,
      "adapter_metadata": {
        "timeout_ms": 10000,
        "retry_attempts": 3,
        "auth": {
          "type": "api_key",
          "header": "X-API-Key"
        }
      }
    }
  }
}
```

The metadata can contain any JSON structure your adapter needs - authentication settings, performance tuning, feature flags, protocol-specific configuration, etc.

## 🔧 Advanced Features

### HTTP Client Caching

The OIF Aggregator provides an optimized HTTP client cache for performance:

```rust
use oif_adapters::{ClientCache, ClientConfig};
use std::sync::Arc;

#[derive(Debug)]
pub struct MyCustomAdapter {
    adapter_info: Adapter,
    cache: ClientCache,
}

impl MyCustomAdapter {
    pub fn new() -> Self {
        Self {
            adapter_info: Adapter::new(
                "my-custom-v1".to_string(),
                "Custom adapter with caching".to_string(), 
                "My Custom Adapter".to_string(),
                "1.0.0".to_string(),
            ),
            cache: ClientCache::for_adapter(),
        }
    }
    
    /// Get optimized HTTP client with connection pooling and keep-alive
    fn get_client(&self, solver_config: &SolverRuntimeConfig) -> AdapterResult<Arc<reqwest::Client>> {
        let client_config = ClientConfig::from(solver_config);
        self.cache.get_client(&client_config)
    }
}
```

**Benefits:**
- Connection pooling and keep-alive
- Automatic client reuse and cleanup
- Authentication header support
- Thread-safe concurrent access

## 🔗 Related Documentation

- **[Configuration Guide](configuration.md)** - How to configure solvers
- **[API Documentation](api/)** - Complete API reference
- **[Quotes & Aggregation Guide](quotes-and-aggregation.md)** - Understanding the aggregation process
- **[OIF Adapter Guide](oif-adapter.md)** - Complete guide for OIF adapter with JWT authentication

## 📚 Reference Implementations

Check these existing adapters for real-world examples:

**`crates/adapters/src/oif_adapter.rs`** - See [OIF Adapter Guide](oif-adapter.md)
- ✅ JWT authentication with `adapter_metadata` 
- ✅ Client caching with authentication support  
- ✅ Structured configuration parsing
- ✅ Token management and refresh logic

**`crates/adapters/src/across_adapter.rs`**
- ✅ Routes-based asset discovery
- ✅ Complex response transformation  

---

**Need Help?** Check the reference implementations above for production-ready code examples, or open an issue on GitHub for specific questions.
