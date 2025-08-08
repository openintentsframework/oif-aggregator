//! Startup logging demonstration
//!
//! This example shows the enhanced startup logging in action

use oif_aggregator::AggregatorBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Initialize basic tracing to see the logs
	tracing_subscriber::fmt::init();

	// Start the server with enhanced startup logging
	// The log_service_info() function will be called automatically during startup
	AggregatorBuilder::new().start_server().await?;

	Ok(())
}
