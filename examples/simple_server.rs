//! Simple server example using the enhanced Builder pattern

use oif_aggregator::AggregatorBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Start a complete server with all defaults and setup
	// This single line handles:
	// - Loading .env file
	// - Loading configuration (JSON files + env vars) with fallback to defaults
	// - Initializing structured logging/tracing
	// - Setting up TTL cleanup for expired quotes
	// - Initializing adapters from configuration
	// - Binding to server address
	// - Starting the HTTP server

	AggregatorBuilder::new().start_server().await?;

	Ok(())
}
