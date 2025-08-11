//! Example demonstrating pluggable storage backends

use oif_aggregator::{
	storage::{MemoryStore, RedisStore, Storage},
	AggregatorBuilder, Solver, SolverStatus,
};
use oif_types::Network;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Initialize tracing for the example
	tracing_subscriber::fmt::init();

	println!("🔌 Pluggable Storage Backend Demo");
	println!("==================================");

	// Demo 1: Using Memory Storage (default)
	println!("\n1. Using Memory Storage Backend");
	let memory_builder = AggregatorBuilder::new();
	demonstrate_storage_operations(memory_builder, "Memory").await?;

	// Demo 2: Using Redis Storage Backend
	println!("\n2. Using Redis Storage Backend");
	let redis_store = RedisStore::with_defaults();
	let redis_builder = AggregatorBuilder::with_storage(redis_store);
	demonstrate_storage_operations(redis_builder, "Redis").await?;

	// Demo 3: Using Redis with Custom Configuration (same as demo 2 for simplicity)
	println!("\n3. Using Redis with Custom Configuration");
	let custom_redis_store = RedisStore::new("redis://localhost:6379/1".to_string());
	let custom_builder = AggregatorBuilder::with_storage(custom_redis_store);
	demonstrate_storage_operations(custom_builder, "Redis (Custom Config)").await?;

	// Demo 4: Runtime Storage Switching
	println!("\n4. Runtime Storage Backend Switching");
	demonstrate_storage_switching().await?;

	println!("\n✅ All storage backend demos completed successfully!");
	Ok(())
}

/// Demonstrate storage operations with any storage backend
async fn demonstrate_storage_operations<S>(
	mut builder: AggregatorBuilder<S>,
	storage_name: &str,
) -> Result<(), Box<dyn std::error::Error>>
where
	S: Storage + Clone + 'static,
{
	println!("📦 Testing {} storage backend...", storage_name);

	// Create a test solver
	let mut solver = Solver::new(
		"demo-solver".to_string(),
		"demo-adapter".to_string(),
		"https://api.example.com".to_string(),
		5000,
	);
	solver.metadata.name = Some(format!("{} Solver", storage_name));
	solver.metadata.description = Some("A demonstration solver".to_string());
	solver.metadata.version = Some("1.0.0".to_string());
	solver.metadata.supported_networks = vec![Network::new(1, "Ethereum".to_string(), false)];
	solver.status = SolverStatus::Active;

	// Add solver using the builder
	builder = builder.with_solver(solver).await;

	// Get storage statistics
	let (_, app_state) = builder.start().await?;
	println!("  ✓ Storage health check and types available");

	// Health check
	let health = app_state.storage.health_check().await?;
	println!(
		"  ✓ Health check: {}",
		if health { "PASSED" } else { "FAILED" }
	);

	Ok(())
}

/// Demonstrate switching between storage backends at runtime
async fn demonstrate_storage_switching() -> Result<(), Box<dyn std::error::Error>> {
	println!("🔄 Demonstrating runtime storage switching...");

	// Start with memory storage
	let memory_store = MemoryStore::new();
	println!("  📝 Starting with Memory storage");

	// Add some test data
	let solver = Solver::new(
		"switchable-solver".to_string(),
		"test-adapter".to_string(),
		"https://api.test.com".to_string(),
		3000,
	);

	(&memory_store as &dyn oif_aggregator::traits::SolverStorage)
		.create(solver.clone())
		.await?;

	// Switch to Redis storage
	let redis_store = RedisStore::with_defaults();
	println!("  🔄 Switching to Redis storage");

	// Migrate data (in real apps, you'd have proper migration logic)
	(&redis_store as &dyn oif_aggregator::traits::SolverStorage)
		.create(solver)
		.await?;

	println!("  ✅ Successfully switched storage backends!");

	Ok(())
}

/// Documentation for creating custom storage adapters
///
/// To implement a custom storage backend, you would:
/// 1. Create a struct for your backend (e.g., PostgresStore)  
/// 2. Implement the QuoteStorage, IntentStorage, and SolverStorage traits
/// 3. Implement the Storage trait which combines all three
/// 4. Use it with AggregatorBuilder::with_storage(your_store)
///
/// For a complete working example, see the MemoryStore implementation
/// in the storage crate.
fn _documentation_only() {
	// This function is for documentation purposes only
}
