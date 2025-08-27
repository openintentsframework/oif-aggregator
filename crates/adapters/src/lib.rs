//! OIF Adapters
//!
//! Solver-specific adapters for the Open Intent Framework Aggregator.
//!
//! This crate provides both built-in adapters (OIF, LiFi) and infrastructure for
//! external users to implement optimized adapters.
//!
//! # Creating Your Own Adapter
//!
//! ## Optimized Adapter (Recommended)
//!
//! Use the provided `ClientCache` for optimal performance:
//!
//! ```rust,no_run
//! use oif_adapters::{ClientCache, SolverAdapter, AdapterResult};
//! use oif_types::{Adapter, SolverRuntimeConfig, GetQuoteRequest, GetQuoteResponse, Asset, Network};
//! use oif_types::adapters::{models::{SubmitOrderRequest, SubmitOrderResponse}, GetOrderResponse};
//! use async_trait::async_trait;
//! use std::sync::Arc;
//!
//! #[derive(Debug)]
//! pub struct MyCustomAdapter {
//!     config: Adapter,
//!     cache: ClientCache,
//! }
//!
//! impl MyCustomAdapter {
//!     /// Recommended: Use optimized client cache
//!     pub fn new(config: Adapter) -> Self {
//!         Self {
//!             config,
//!             cache: ClientCache::for_adapter(),
//!         }
//!     }
//!     
//!     /// Get optimized HTTP client
//!     fn get_client(&self, solver_config: &SolverRuntimeConfig) -> AdapterResult<Arc<reqwest::Client>> {
//!         let mut client_config = oif_adapters::ClientConfig::from(solver_config);
//!         
//!         self.cache.get_client(&client_config)
//!     }
//! }
//!
//! #[async_trait]
//! impl SolverAdapter for MyCustomAdapter {
//!     fn adapter_id(&self) -> &str { &self.config.adapter_id }
//!     fn adapter_name(&self) -> &str { &self.config.name }
//!     fn adapter_info(&self) -> &Adapter { &self.config }
//!     
//!     async fn get_quotes(&self, _request: &GetQuoteRequest, config: &SolverRuntimeConfig) -> AdapterResult<GetQuoteResponse> {
//!         let _client = self.get_client(config)?; // ← Optimized cached client
//!         // Your implementation here
//!         todo!()
//!     }
//!     
//!     async fn submit_order(&self, _order: &SubmitOrderRequest, _config: &SolverRuntimeConfig) -> AdapterResult<SubmitOrderResponse> { todo!() }
//!     async fn health_check(&self, _config: &SolverRuntimeConfig) -> AdapterResult<bool> { todo!() }
//!     async fn get_order_details(&self, _order_id: &str, _config: &SolverRuntimeConfig) -> AdapterResult<GetOrderResponse> { todo!() }
//!     async fn get_supported_networks(&self, _config: &SolverRuntimeConfig) -> AdapterResult<Vec<Network>> { todo!() }
//!     async fn get_supported_assets(&self, _config: &SolverRuntimeConfig) -> AdapterResult<Vec<Asset>> { todo!() }
//! }
//! ```
//!
//! ## Benefits of Using ClientCache
//!
//! - **Connection Pooling**: Reuse connections across requests
//! - **Keep-Alive**: 90-second connection keep-alive optimization  
//! - **TTL Management**: Automatic 30-minute cache expiration
//! - **Atomic Operations**: Race condition-free caching
//! - **Per-Solver Config**: Isolated configurations for different solvers
//!
//! ## Basic Adapter (Simple Cases)
//!
//! For simple use cases or custom client management:
//!
//! ```rust,no_run
//! use oif_types::{Adapter, SolverRuntimeConfig, AdapterResult};
//!
//! #[derive(Debug)]
//! pub struct BasicAdapter {
//!     config: Adapter,
//! }
//!
//! impl BasicAdapter {
//!     pub fn new(config: Adapter) -> Self {
//!         Self { config }
//!     }
//!     
//!     async fn create_client(&self, solver_config: &SolverRuntimeConfig) -> AdapterResult<reqwest::Client> {
//!         // Create basic client for each request
//!         use oif_types::AdapterError;
//!         Ok(reqwest::Client::builder()
//!             .timeout(std::time::Duration::from_secs(30)) // Default 30 second timeout
//!             .build()
//!             .map_err(AdapterError::HttpError)?)
//!     }
//! }
//! ```

use std::sync::Arc;

pub mod client_cache;
pub mod lifi_adapter;
pub mod oif_adapter;

pub use lifi_adapter::LifiAdapter;
pub use oif_adapter::OifAdapter;
pub use oif_types::{AdapterError, AdapterResult, SolverAdapter};

// Re-export client cache infrastructure for external adapters
pub use client_cache::{adapter_client_cache, ClientCache, ClientConfig};

/// Simple registry for solver adapters
pub struct AdapterRegistry {
	adapters: std::collections::HashMap<String, Arc<dyn SolverAdapter>>,
}

impl Default for AdapterRegistry {
	fn default() -> Self {
		Self::new()
	}
}

impl AdapterRegistry {
	pub fn new() -> Self {
		Self {
			adapters: std::collections::HashMap::new(),
		}
	}

	/// Create registry with default OIF and LiFi adapters
	pub fn with_defaults() -> Self {
		let mut registry = Self::new();

		// Add default OIF adapter
		if let Ok(oif_adapter) = OifAdapter::with_default_config() {
			let _ = registry.register(Box::new(oif_adapter));
		}

		// Add default LiFi adapter
		if let Ok(lifi_adapter) = LifiAdapter::with_default_config() {
			let _ = registry.register(Box::new(lifi_adapter));
		}

		registry
	}

	/// Get all registered adapter IDs
	pub fn get_adapter_ids(&self) -> Vec<String> {
		self.adapters.keys().cloned().collect()
	}

	/// Register an adapter (uses the adapter's own ID)
	pub fn register(&mut self, adapter: Box<dyn SolverAdapter>) -> Result<(), String> {
		let adapter_id = adapter.adapter_id().to_string();

		// Check for duplicate IDs
		if self.adapters.contains_key(&adapter_id) {
			return Err(format!(
				"Adapter with ID '{}' already registered",
				adapter_id
			));
		}

		self.adapters.insert(adapter_id, Arc::from(adapter));
		Ok(())
	}

	pub fn get(&self, id: &str) -> Option<Arc<dyn SolverAdapter>> {
		self.adapters.get(id).cloned()
	}

	pub fn get_all(&self) -> &std::collections::HashMap<String, Arc<dyn SolverAdapter>> {
		&self.adapters
	}
}
