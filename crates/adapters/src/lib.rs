//! OIF Adapters
//!
//! Solver-specific adapters for the Open Intent Framework Aggregator.

pub mod lifi_adapter;
pub mod oif_adapter;

use async_trait::async_trait;
use oif_types::{Order, Quote, QuoteRequest};

pub use lifi_adapter::LifiAdapter;
pub use oif_adapter::OifAdapter;
pub use oif_types::{AdapterError, AdapterResult};

/// Trait for solver adapters
#[async_trait]
pub trait SolverAdapter: Send + Sync {
	async fn get_quote(&self, request: &QuoteRequest) -> AdapterResult<Quote>;
	async fn submit_order(&self, order: &Order) -> AdapterResult<String>;
	async fn health_check(&self) -> AdapterResult<bool>;
	fn adapter_info(&self) -> &oif_types::AdapterConfig;
	fn supported_chains(&self) -> &[u64];
}

/// Factory for creating solver adapters
pub struct AdapterFactory {
	adapters: std::collections::HashMap<String, Box<dyn SolverAdapter>>,
}

impl AdapterFactory {
	pub fn new() -> Self {
		Self {
			adapters: std::collections::HashMap::new(),
		}
	}

	pub fn create_adapter(
		adapter_type: &str,
		config: oif_types::AdapterConfig,
		endpoint: String,
		timeout_ms: u64,
	) -> AdapterResult<Box<dyn SolverAdapter>> {
		match adapter_type {
			"lifi-v1" => Ok(Box::new(LifiAdapter::new(config, endpoint, timeout_ms)?)),
			"oif-v1" => Ok(Box::new(OifAdapter::new(config, endpoint, timeout_ms)?)),
			_ => Err(AdapterError::UnsupportedAdapter(adapter_type.to_string())),
		}
	}

	pub fn create_from_config(
		config: &oif_types::AdapterConfig,
	) -> AdapterResult<Box<dyn SolverAdapter>> {
		// Backward-compat: default endpoints; prefer create_for_solver when available
		let (adapter_type_str, endpoint) = match config.adapter_type {
			oif_types::AdapterType::OifV1 => {
				("oif-v1", "https://api.oif.example.com/v1".to_string())
			},
			oif_types::AdapterType::LifiV1 => ("lifi-v1", "https://li.quest/v1".to_string()),
		};

		let timeout_ms = 30000; // Default 30 seconds

		Self::create_adapter(adapter_type_str, config.clone(), endpoint, timeout_ms)
	}

	/// Create adapter using solver-provided endpoint and timeout
	pub fn create_for_solver(
		config: &oif_types::AdapterConfig,
		endpoint: String,
		timeout_ms: u64,
	) -> AdapterResult<Box<dyn SolverAdapter>> {
		let adapter_type_str = match config.adapter_type {
			oif_types::AdapterType::OifV1 => "oif-v1",
			oif_types::AdapterType::LifiV1 => "lifi-v1",
		};

		Self::create_adapter(adapter_type_str, config.clone(), endpoint, timeout_ms)
	}

	pub fn register(&mut self, id: String, adapter: Box<dyn SolverAdapter>) {
		self.adapters.insert(id, adapter);
	}

	pub fn add_adapter(&mut self, id: String, adapter: Box<dyn SolverAdapter>) {
		self.adapters.insert(id, adapter);
	}

	pub fn get(&self, id: &str) -> Option<&Box<dyn SolverAdapter>> {
		self.adapters.get(id)
	}

	pub fn get_all(&self) -> &std::collections::HashMap<String, Box<dyn SolverAdapter>> {
		&self.adapters
	}
}
