//! OIF Aggregator Server
//!
//! Main entry point for the aggregator server

use oif_aggregator::AggregatorBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Start the complete server with all defaults and setup handled automatically
	AggregatorBuilder::new().start_server().await
}
