//! Builder Pattern demonstration example
//!
//! This example showcases the different ways to configure and build
//! an OIF Aggregator service using the builder pattern.
//!
//! Run with: INTEGRITY_SECRET="demo-secret-key" cargo run --example builder_demo

use oif_aggregator::AggregatorBuilder;
use oif_types::solvers::Solver;

// Import mock adapter from src/mocks.rs
use oif_aggregator::mocks::{mock_solver, MockDemoAdapter, MockTestAdapter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("🏗️  OIF Aggregator Builder Pattern Demo");
	println!("======================================");

	// Demo 1: Minimal builder with default adapters only
	println!("\n1. 📦 Minimal Builder (Default Adapters Only)");
	println!("   Creating aggregator with OIF and LiFi adapters...");

	let (_app1, _state1) = AggregatorBuilder::default().start().await?;

	println!("   ✅ Built aggregator with default adapters (OIF + LiFi)");

	// Demo 2: Builder with custom mock adapter
	println!("\n2. 🔧 Builder with Custom Mock Adapter");
	println!("   Adding MockDemoAdapter...");

	let mock_adapter = MockDemoAdapter::new();
	let mock_solver = mock_solver();

	let (_app2, _state2) = AggregatorBuilder::default()
		.with_adapter(Box::new(mock_adapter))
		.with_solver(mock_solver)
		.start()
		.await?;

	println!("   ✅ Built aggregator with custom mock adapter added");

	// Demo 3: Builder with multiple custom adapters
	println!("\n3. 🎯 Builder with Multiple Custom Adapters");
	println!("   Adding MockDemoAdapter and MockTestAdapter...");

	let demo_adapter = MockDemoAdapter::new();
	let test_adapter = MockTestAdapter::new();

	let demo_solver = Solver::new(
		"demo-solver-v2".to_string(),
		"mock-demo-v1".to_string(),
		"http://localhost:8080".to_string(),
		3000,
	);

	let test_solver = Solver::new(
		"test-solver-v1".to_string(),
		"mock-test-v1".to_string(),
		"http://localhost:8081".to_string(),
		5000,
	);

	let (_app3, state3) = AggregatorBuilder::default()
		.with_adapter(Box::new(demo_adapter))
		.with_adapter(Box::new(test_adapter))
		.with_solver(demo_solver)
		.with_solver(test_solver)
		.start()
		.await?;

	println!("   ✅ Built aggregator with multiple custom adapters");

	// Use solver service to get solver count
	let (solvers, total_count, active_count, _) = state3
		.solver_service
		.list_solvers_paginated(None, None)
		.await?;
	println!(
		"   📊 Registered solvers: {} total, {} active",
		total_count, active_count
	);
	for solver in solvers.iter().take(5) {
		// Show first 5
		println!(
			"      - {} (adapter: {})",
			solver.solver_id, solver.adapter_id
		);
	}

	// Demo 4: Working server example (commented server startup)
	println!("\n4. 🌐 Working Server Example");
	println!("   Building aggregator for server startup...");

	let server_adapter = MockDemoAdapter::new();
	let server_solver = Solver::new(
		"server-demo-solver".to_string(),
		"mock-demo-v1".to_string(),
		"http://localhost:8080".to_string(),
		3000,
	);

	let (_app4, _state4) = AggregatorBuilder::default()
		.with_adapter(Box::new(server_adapter))
		.with_solver(server_solver)
		.start()
		.await?;

	println!("   ✅ Server-ready aggregator built successfully");
	println!("   💡 To start an actual server, uncomment the server code below");

	// To start a real server, you would do:
	println!("   💡 To start a server: axum::serve(listener, app4).await?");
	println!("   💡 The app4 router is ready to handle HTTP requests");
	println!("   💡 Available endpoints: /health, /v1/quotes, /v1/orders, /v1/solvers");

	// Demo 5: Error handling showcase
	println!("\n5. ⚠️  Error Handling Examples");

	// Try to add adapter with duplicate ID
	println!("   Testing duplicate adapter ID rejection...");
	let duplicate_adapter = MockDemoAdapter::new(); // Same ID as previous

	// Should panic when trying to register duplicate adapter
	let result = std::panic::catch_unwind(|| {
		AggregatorBuilder::default()
			.with_adapter(Box::new(MockDemoAdapter::new()))
			.with_adapter(Box::new(duplicate_adapter))
	});

	match result {
		Ok(_) => println!("   ❌ Should have panicked with duplicate adapter ID"),
		Err(_) => println!("   ✅ Correctly panicked with duplicate adapter"),
	}

	println!("\n6. 📊 Builder Pattern Summary");
	println!("   The AggregatorBuilder provides:");
	println!("   🔸 AggregatorBuilder::default() - Start with OIF/LiFi defaults");
	println!("   🔸 .with_adapter(adapter) - Add custom adapter implementations");
	println!("   🔸 .with_solver(solver) - Register solvers for adapters");
	println!("   🔸 .start() - Build the router and application state");
	println!("   🔸 Automatic validation and error handling");
	println!("   🔸 Duplicate prevention for adapter IDs");

	println!("\n✅ Builder Pattern Demo Completed!");
	println!("   All builder configurations demonstrated successfully!");

	Ok(())
}
