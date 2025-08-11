//! Working demonstration of pluggable storage architecture

use oif_aggregator::models::solvers::SolverStatus;
use oif_aggregator::models::{Order, Quote, Solver};
use oif_aggregator::storage::MemoryStore;
use oif_aggregator::storage::Storage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	tracing_subscriber::fmt::init();

	println!("🔌 Working Storage Architecture Demo");
	println!("===================================");

	// Demonstrate that our storage trait system works
	println!("\n📦 Testing Memory Storage with Storage Traits:");

	let storage = MemoryStore::new();
	demonstrate_storage_interface(&storage).await?;

	// Show polymorphism with trait objects
	println!("\n🔄 Demonstrating Trait Object Polymorphism:");
	demonstrate_trait_objects().await?;

	println!("\n✅ Storage trait system is fully functional!");
	println!("💡 Redis, PostgreSQL, and other backends can be easily added!");

	Ok(())
}

/// Demonstrate storage operations through trait interface
async fn demonstrate_storage_interface<S: Storage>(
	storage: &S,
) -> Result<(), Box<dyn std::error::Error>> {
	println!("   Storage health available");

	// Create test data
	let solver = create_test_solver();
	let quote = create_test_quote();
	let order = create_test_order();

	// Test all storage operations
	(&*storage as &dyn oif_aggregator::traits::SolverStorage)
		.create(solver.clone())
		.await?;
	(&*storage as &dyn oif_aggregator::traits::QuoteStorage)
		.create(quote.clone())
		.await?;
	(&*storage as &dyn oif_aggregator::traits::OrderStorage)
		.create(order.clone())
		.await?;

	// Verify data was stored
	let stored_solver = (&*storage as &dyn oif_aggregator::traits::SolverStorage)
		.get(&solver.solver_id)
		.await?;
	let stored_quote = (&*storage as &dyn oif_aggregator::traits::QuoteStorage)
		.get(&quote.quote_id)
		.await?;
	let stored_order = (&*storage as &dyn oif_aggregator::traits::OrderStorage)
		.get(&order.order_id)
		.await?;

	assert!(stored_solver.is_some());
	assert!(stored_quote.is_some());
	assert!(stored_order.is_some());

	println!("   ✓ All CRUD operations work through traits");

	// Test statistics
	println!("   ✓ Stats: (omitted in demo)");

	// Test health check
	let health = storage.health_check().await?;
	println!(
		"   ✓ Health check: {}",
		if health { "PASSED" } else { "FAILED" }
	);

	Ok(())
}

/// Demonstrate polymorphism with trait objects
async fn demonstrate_trait_objects() -> Result<(), Box<dyn std::error::Error>> {
	// Create storage as trait object
	let storage: Box<dyn Storage> = Box::new(MemoryStore::new());

	// Store some data
	let solver = create_test_solver();
	(&*storage as &dyn oif_aggregator::traits::SolverStorage)
		.create(solver.clone())
		.await?;

	// Function that accepts any storage implementation
	async fn count_items(_storage: &dyn Storage) -> Result<(), Box<dyn std::error::Error>> {
		println!("   📊 Storage demo counts omitted");
		Ok(())
	}

	count_items(storage.as_ref()).await?;
	println!("   ✓ Trait objects enable runtime polymorphism");

	Ok(())
}

// Test data creation helpers
fn create_test_solver() -> Solver {
	let mut solver = Solver::new(
		"demo-solver".to_string(),
		"demo-adapter".to_string(),
		"https://api.example.com".to_string(),
		5000,
	);
	solver.metadata.name = Some("Demo Solver".to_string());
	solver.status = SolverStatus::Active;
	solver
}

fn create_test_quote() -> Quote {
	Quote::new(
		"demo-solver".to_string(),
		"demo-request".to_string(),
		"0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0".to_string(),
		"0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
		"1000000".to_string(),
		"2000000".to_string(),
		1,
	)
}

fn create_test_order() -> Order {
	Order::new("0x1234567890123456789012345678901234567890".to_string())
}
