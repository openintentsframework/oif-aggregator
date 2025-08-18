//! Simple server example using the enhanced Builder pattern
//!
//! Run with: INTEGRITY_SECRET=demo-secret-key cargo run --example simple_server

use oif_aggregator::AggregatorBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Start a complete server with all defaults and setup
	// This single line handles:
	// - Loading configuration (JSON files + env vars) with fallback to defaults
	// - Initializing structured logging/tracing
	// - Initializing adapters from configuration
	// - Binding to server address
	// - Starting the HTTP server

	AggregatorBuilder::default().start_server().await?;

	Ok(())
}
