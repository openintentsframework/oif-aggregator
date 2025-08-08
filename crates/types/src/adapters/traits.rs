//! Core adapter traits for user implementations

use super::{AdapterConfig, AdapterResult};
use crate::{Order, Quote, QuoteRequest};
use async_trait::async_trait;

/// Core trait for solver adapter implementations
///
/// This trait defines the interface that all solver adapters must implement.
/// Users can create custom adapters by implementing this trait.
#[async_trait]
pub trait SolverAdapter: Send + Sync + std::fmt::Debug {
	/// Get adapter configuration information
	fn adapter_info(&self) -> &AdapterConfig;

	/// Get a quote from the solver
	async fn get_quote(&self, request: QuoteRequest) -> AdapterResult<Quote>;

	/// Submit an order to the solver
	async fn submit_order(&self, order: &Order) -> AdapterResult<String>;

	/// Health check for the solver
	async fn health_check(&self) -> AdapterResult<bool>;

	/// Get supported chains for this adapter
	fn supported_chains(&self) -> &[u64];

	/// Get human-readable name for this adapter
	fn name(&self) -> &str {
		&self.adapter_info().name
	}

	/// Get adapter version
	fn version(&self) -> &str {
		&self.adapter_info().version
	}

	/// Check if adapter supports a specific chain
	fn supports_chain(&self, chain_id: u64) -> bool {
		self.supported_chains().contains(&chain_id)
	}
}
