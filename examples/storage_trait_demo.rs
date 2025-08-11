//! Direct demonstration of storage traits without the complex builder

// Direct module access to avoid lib.rs compilation issues
use oif_aggregator::chrono::Utc;
use oif_aggregator::models::solvers::SolverStatus;
use oif_aggregator::models::{Order, Quote, Solver};
use oif_aggregator::storage::{MemoryStore, RedisStore, Storage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	tracing_subscriber::fmt::init();

	println!("🔌 Storage Trait System Demo");
	println!("============================");

	// Create test data
	let solver = create_test_solver();
	let quote = create_test_quote();
	let order = create_test_order();

	// Demo 1: Memory Storage
	println!("\n📦 Testing Memory Storage Implementation:");
	test_storage_implementation(
		Box::new(MemoryStore::new()),
		solver.clone(),
		quote.clone(),
		order.clone(),
	)
	.await?;

	// Demo 2: Redis Storage
	println!("\n📦 Testing Redis Storage Implementation:");
	test_storage_implementation(
		Box::new(RedisStore::with_defaults()),
		solver.clone(),
		quote.clone(),
		order.clone(),
	)
	.await?;

	// Demo 3: Polymorphic storage usage
	println!("\n🔄 Demonstrating Storage Polymorphism:");
	demonstrate_polymorphism().await?;

	println!("\n✅ Storage trait system works perfectly!");
	println!("💡 You can now easily add PostgreSQL, MongoDB, or any other storage backend!");
	Ok(())
}

/// Test any storage implementation through the trait
async fn test_storage_implementation(
	storage: Box<dyn Storage>,
	solver: Solver,
	quote: Quote,
	order: Order,
) -> Result<(), Box<dyn std::error::Error>> {
	println!("   Storage health available");

	// Test solver operations
	(&*storage as &dyn oif_aggregator::traits::SolverStorage)
		.create(solver.clone())
		.await?;
	let retrieved_solver = (&*storage as &dyn oif_aggregator::traits::SolverStorage)
		.get(&solver.solver_id)
		.await?;
	assert!(retrieved_solver.is_some());
	println!("   ✓ Solver operations work");

	// Test quote operations
	(&*storage as &dyn oif_aggregator::traits::QuoteStorage)
		.create(quote.clone())
		.await?;
	let retrieved_quote = (&*storage as &dyn oif_aggregator::traits::QuoteStorage)
		.get(&quote.quote_id)
		.await?;
	assert!(retrieved_quote.is_some());
	println!("   ✓ Quote operations work");

	// Test intent operations
	(&*storage as &dyn oif_aggregator::traits::OrderStorage)
		.create(order.clone())
		.await?;
	let retrieved_order = (&*storage as &dyn oif_aggregator::traits::OrderStorage)
		.get(&order.order_id)
		.await?;
	assert!(retrieved_order.is_some());
	println!("   ✓ Order operations work");

	// Test statistics
	println!("   ✓ Statistics omitted in demo");

	// Test health check
	let health = storage.health_check().await?;
	println!(
		"   ✓ Health check: {}",
		if health { "PASSED" } else { "FAILED" }
	);

	Ok(())
}

/// Demonstrate how storage backends can be swapped at runtime
async fn demonstrate_polymorphism() -> Result<(), Box<dyn std::error::Error>> {
	// Function that works with any storage implementation
	async fn store_data<S: Storage>(
		storage: &S,
		data_type: &str,
	) -> Result<(), Box<dyn std::error::Error>> {
		let solver = create_test_solver();
		(&*storage as &dyn oif_aggregator::traits::SolverStorage)
			.create(solver)
			.await?;
		println!("   ✓ Stored {} data in {} storage", data_type, "generic");
		Ok(())
	}

	// Same function, different storage backends
	let memory_store = MemoryStore::new();
	store_data(&memory_store, "test").await?;

	let redis_store = RedisStore::with_defaults();
	store_data(&redis_store, "test").await?;

	println!("   💡 Same code works with any storage backend!");
	Ok(())
}

// Helper functions to create test data
fn create_test_solver() -> Solver {
	let mut solver = Solver::new(
		"test-solver".to_string(),
		"test-adapter".to_string(),
		"https://api.example.com".to_string(),
		5000,
	);
	solver.metadata.name = Some("Test Solver".to_string());
	solver.status = SolverStatus::Active;
	solver
}

fn create_test_quote() -> Quote {
	Quote::new(
		"test-solver".to_string(),
		"test-request".to_string(),
		"0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0".to_string(),
		"0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
		"1000000".to_string(),
		"2000000".to_string(),
		1,
	)
}

fn create_test_order() -> Order {
	Order::new(
		"0x1234567890123456789012345678901234567890".to_string(),
		0.005,
		Utc::now() + oif_aggregator::chrono::Duration::hours(1),
	)
}
