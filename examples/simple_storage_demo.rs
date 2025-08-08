//! Simple demonstration of pluggable storage

use oif_aggregator::{
	models::{solvers::SolverStatus, Solver},
	storage::{MemoryStore, Storage},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	tracing_subscriber::fmt::init();

	println!("🔌 Simple Storage Demo");
	println!("=====================");

	// Create a test solver
	let mut solver = Solver::new(
		"demo-solver".to_string(),
		"demo-adapter".to_string(),
		"https://api.example.com".to_string(),
		5000,
	);
	solver.metadata.name = Some("Demo Solver".to_string());
	solver.status = SolverStatus::Active;

	// Demo 1: Memory Storage
	println!("\n📦 Testing Memory Storage:");
	let memory_store = MemoryStore::new();
	test_storage(&memory_store, solver.clone()).await?;

	// Demo 2: Redis Storage (commented out for examples - requires Redis server)
	// println!("\n📦 Testing Redis Storage:");
	// let redis_store = RedisStore::with_defaults();
	// test_storage(&redis_store, solver.clone()).await?;

	println!("\n✅ Storage backends are fully pluggable!");
	Ok(())
}

/// Test any storage implementation
async fn test_storage<S: Storage>(
	storage: &S,
	solver: Solver,
) -> Result<(), Box<dyn std::error::Error>> {
	println!("   Storage type: {}", storage.storage_type());

	// Add solver
	storage.add_solver(solver).await?;

	// Get stats
	let stats = storage.get_storage_stats().await?;
	println!("   Solvers: {}", stats.total_solvers);
	println!("   Health: {}", storage.health_check().await?);

	Ok(())
}
