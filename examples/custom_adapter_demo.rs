//! Example demonstrating how to register custom adapter implementations

use async_trait::async_trait;
use oif_aggregator::{AggregatorBuilder, MemoryStore};
use oif_types::StorageTrait;
use oif_types::{
	Adapter, AdapterError, AdapterResult, Asset, Network, Order, OrderDetails, Quote, QuoteRequest,
	Solver, SolverAdapter, SolverRuntimeConfig,
};
use std::time::Duration;
use tracing::info;

/// Example custom adapter implementation
/// This adapter is stateless and receives solver configuration at runtime
#[derive(Debug)]
pub struct CustomSolverAdapter {
	config: Adapter,
}

impl CustomSolverAdapter {
	pub fn new() -> Self {
		let config = Adapter::new(
			"custom-v1".to_string(),
			"Custom Protocol v1".to_string(),
			"Custom Demo Adapter".to_string(),
			"1.0.0".to_string(),
		);

		Self { config }
	}
}

#[async_trait]
impl SolverAdapter for CustomSolverAdapter {
	fn adapter_id(&self) -> &str {
		"custom-v1"
	}

	fn adapter_name(&self) -> &str {
		"Custom Demo Adapter"
	}

	fn adapter_info(&self) -> &Adapter {
		&self.config
	}

	async fn get_quote(
		&self,
		request: QuoteRequest,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<Quote> {
		info!(
			"Custom adapter {} getting quote for request {} via solver {} (endpoint: {})",
			self.config.adapter_id, request.request_id, config.solver_id, config.endpoint
		);

		// Simulate some processing time (could use config.timeout_ms for realistic behavior)
		tokio::time::sleep(Duration::from_millis(100)).await;

		// Create a mock quote response using the solver ID from runtime config
		let quote = Quote::new(
			config.solver_id.clone(), // Use solver's ID, not adapter's
			request.request_id.clone(),
			request.token_in.clone(),
			request.token_out.clone(),
			request.amount_in.clone(),
			"1000000000000000000".to_string(), // 1 ETH out for demo
			request.chain_id,
		)
		.with_estimated_gas(21000)
		.with_response_time(100)
		.with_confidence_score(0.95);

		info!(
			"Custom adapter {} returning quote {} for solver {}",
			self.config.adapter_id, quote.quote_id, config.solver_id
		);
		Ok(quote)
	}

	async fn submit_order(
		&self,
		order: &Order,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<String> {
		info!(
			"Custom adapter {} submitting order {} via solver {} (endpoint: {})",
			self.config.adapter_id, order.order_id, config.solver_id, config.endpoint
		);

		// Mock order submission
		tokio::time::sleep(Duration::from_millis(50)).await;

		Ok(format!("0x{:x}", 0x1234567890abcdefu64))
	}

	async fn health_check(&self, config: &SolverRuntimeConfig) -> AdapterResult<bool> {
		info!(
			"Custom adapter {} health check for solver {} (endpoint: {})",
			self.config.adapter_id, config.solver_id, config.endpoint
		);
		Ok(true)
	}

	async fn get_order_details(
		&self,
		order_id: &str,
		config: &SolverRuntimeConfig,
	) -> AdapterResult<OrderDetails> {
		info!(
			"Custom adapter {} getting order details for {} via solver {}",
			self.config.adapter_id, order_id, config.solver_id
		);

		let details = OrderDetails::new(order_id.to_string(), "pending".to_string());
		Ok(details)
	}

	async fn get_supported_networks(&self) -> AdapterResult<Vec<Network>> {
		Ok(vec![
			Network::new(1, "Ethereum".to_string(), false),
			Network::new(137, "Polygon".to_string(), false),
		])
	}

	async fn get_supported_assets(&self, _network: &Network) -> AdapterResult<Vec<Asset>> {
		Err(AdapterError::NotImplemented(
			"Custom adapter asset support not implemented".to_string(),
		))
	}
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Initialize tracing
	tracing_subscriber::fmt::init();

	info!("🚀 Starting custom adapter demo");

	// Create storage and add a solver
	let storage = MemoryStore::new();
	let solver = Solver::new(
		"custom-solver-1".to_string(),
		"custom-v1".to_string(), // This should match the adapter type we register
		"https://api.custom-solver.com".to_string(),
		5000,
	);

	storage.create_solver(solver.clone()).await?;

	// Create and register custom adapter
	let custom_adapter = CustomSolverAdapter::new();

	// Build aggregator with custom adapter
	let builder = AggregatorBuilder::with_storage(storage)
		.with_adapter(Box::new(custom_adapter))
		.map_err(|e| format!("Failed to register adapter: {}", e))?;

	// Start the aggregator
	let (_router, app_state) = builder.start().await?;

	info!("✅ Aggregator started with custom adapter");

	// Test the custom adapter
	let request = QuoteRequest::new(
		"0xA0b86a33E6441e4C791B41d77c3dBD2D50e9cC51".to_string(), // Token in
		"0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(), // Token out
		"1000000000000000000".to_string(),                        // 1 ETH
		1,                                                        // Ethereum mainnet
	);

	let quotes = app_state.aggregator_service.fetch_quotes(request).await;

	info!("📊 Received {} quotes:", quotes.len());
	for quote in quotes {
		info!(
			"  - Quote {}: {} {} -> {} {} (confidence: {:.2})",
			quote.quote_id,
			quote.amount_in,
			quote.token_in,
			quote.amount_out,
			quote.token_out,
			quote.confidence_score.unwrap_or(0.0)
		);
	}

	info!("🎉 Custom adapter demo completed successfully!");

	Ok(())
}
