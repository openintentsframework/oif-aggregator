//! Simple demonstration of pluggable storage with the aggregator
//!
//! This example shows how storage works within the OIF Aggregator context,
//! demonstrating storage operations through the AggregatorBuilder and services.
//!
//! Run with: INTEGRITY_SECRET=demo-secret-key cargo run --example simple_storage_demo

use oif_aggregator::AggregatorBuilder;
use oif_storage::MemoryStore;
use oif_types::storage::StorageTrait;
use oif_types::solvers::Solver;

// Import mock adapter from src/mocks.rs
use oif_aggregator::mocks::MockDemoAdapter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔌 Simple Storage Demo");
    println!("======================");
    println!("Demonstrating storage operations through the aggregator");

    // Demo 1: Default aggregator (uses default memory storage internally)
    println!("\n1. 📦 Default Storage (AggregatorBuilder::default())");
    println!("   Creating aggregator with default memory storage...");
    
    let mock_adapter1 = MockDemoAdapter::new();
    let test_solver1 = Solver::new(
        "default-storage-solver".to_string(),
        "mock-demo-v1".to_string(),
        "http://localhost:8080".to_string(),
        3000,
    );
    
    let (_app1, state1) = AggregatorBuilder::default()
        .with_adapter(Box::new(mock_adapter1))?
        .with_solver(test_solver1)
        .await
        .start()
        .await?;
    
    let (_solvers1, total1, _, _) = state1.solver_service.list_solvers_paginated(None, None).await?;
    println!("   ✅ Default storage: {} solvers", total1);
    
    // Demo 2: Explicit custom memory storage using with_storage()
    println!("\n2. 🔧 Custom Storage (AggregatorBuilder::with_storage())");
    println!("   Creating aggregator with explicit MemoryStore::new()...");
    
    let custom_storage = MemoryStore::new();
    let mock_adapter2 = MockDemoAdapter::new();
    let test_solver2 = Solver::new(
        "custom-storage-solver".to_string(),
        "mock-demo-v1".to_string(),
        "http://localhost:8081".to_string(),
        4000,
    );
    
    let (_app2, state2) = AggregatorBuilder::with_storage(custom_storage)
        .with_adapter(Box::new(mock_adapter2))?
        .with_solver(test_solver2)
        .await
        .start()
        .await?;
    
    let (_solvers2, total2, _, _) = state2.solver_service.list_solvers_paginated(None, None).await?;
    println!("   ✅ Custom storage: {} solvers", total2);
    
    // Demo 3: Pre-populated storage
    println!("\n3. 🗄️  Pre-populated Storage");
    println!("   Creating storage with pre-existing data...");
    
    let pre_populated_storage = MemoryStore::new();
    
    // Add a solver directly to storage before building aggregator
    let pre_solver = Solver::new(
        "pre-existing-solver".to_string(),
        "mock-demo-v1".to_string(),
        "http://localhost:8082".to_string(),
        2000,
    );
    pre_populated_storage.create_solver(pre_solver).await?;
    
    let mock_adapter3 = MockDemoAdapter::new();
    let additional_solver = Solver::new(
        "additional-solver".to_string(),
        "mock-demo-v1".to_string(),
        "http://localhost:8083".to_string(),
        3000,
    );
    
    let (_app3, state3) = AggregatorBuilder::with_storage(pre_populated_storage)
        .with_adapter(Box::new(mock_adapter3))?
        .with_solver(additional_solver)
        .await
        .start()
        .await?;
    
    let (_solvers3, total3, _, _) = state3.solver_service.list_solvers_paginated(None, None).await?;
    println!("   ✅ Pre-populated storage: {} solvers (1 pre-existing + 1 added)", total3);

    // Demo 4: Storage Operations via SolverService
    println!("\n4. 🔍 Storage Operations via SolverService");
    println!("   Using state3 (pre-populated storage) for operations...");
    
    // List all solvers
    let (solvers, total_count, active_count, healthy_count) = 
        state3.solver_service.list_solvers_paginated(None, None).await?;
    
    println!("   📊 Storage statistics:");
    println!("      - Total solvers: {}", total_count);
    println!("      - Active solvers: {}", active_count);
    println!("      - Healthy solvers: {}", healthy_count);
    
    // Show solver details
    println!("   📋 Solver details:");
    for solver in &solvers {
        println!("      - {}: {} ({})", solver.solver_id, solver.adapter_id, solver.endpoint);
    }

    // Demo 5: Direct Storage Interface
    println!("\n5. 🗄️  Direct Storage Interface");
    println!("   Accessing storage directly (what services use internally)...");
    
    // Use storage directly (this is what services use internally)
    let all_solvers = state3.storage.list_all_solvers().await?;
    println!("   📂 Direct storage query: {} solvers found", all_solvers.len());
    
    // Get a specific solver
    if let Some(solver) = all_solvers.first() {
        let retrieved = state3.storage.get_solver(&solver.solver_id).await?;
        match retrieved {
            Some(s) => println!("   ✅ Retrieved solver '{}' successfully", s.solver_id),
            None => println!("   ❌ Solver not found"),
        }
    }

    // Demo 6: Storage health and performance
    println!("\n6. 🏥 Storage Health & Performance");
    
    let start_time = std::time::Instant::now();
    let solver_count = state3.storage.count_solvers().await?;
    let duration = start_time.elapsed();
    
    println!("   ⚡ Performance test:");
    println!("      - Count query: {} solvers in {:?}", solver_count, duration);
    
    // Test concurrent operations
    let start_time = std::time::Instant::now();
    let mut concurrent_results = Vec::new();
    
    for i in 0..3 {
        let storage = state3.storage.clone();
        let handle = tokio::spawn(async move {
            storage.list_all_solvers().await.map(|solvers| (i, solvers.len()))
        });
        concurrent_results.push(handle);
    }
    
    let duration = start_time.elapsed();
    
    println!("   🔄 Concurrent operations test:");
    println!("      - 3 concurrent list operations in {:?}", duration);
    
    for handle in concurrent_results {
        if let Ok(Ok((task_id, count))) = handle.await {
            println!("      - Task {}: {} solvers", task_id, count);
        }
    }

    // Demo 7: Storage abstraction benefits
    println!("\n7. 🎯 Storage Abstraction Benefits");
    println!("   The storage layer provides:");
    println!("   🔸 Unified interface - All services use the same Storage trait");
    println!("   🔸 Type safety - Strong typing for all storage operations");
    println!("   🔸 Async operations - Non-blocking I/O for better performance");
    println!("   🔸 Error handling - Consistent error types across operations");
    println!("   🔸 Pluggable backends - Easy to swap memory/Redis/database");
    
    println!("\n8. 🔄 How Storage Integrates");
    println!("   Storage is used by:");
    println!("   📊 SolverService - Manages solver registration and lookup");
    println!("   📝 OrderService - Stores and retrieves order information");
    println!("   💰 QuoteService - Caches quotes and manages TTL (via aggregator)");
    println!("   🏗️  AggregatorBuilder - Initializes storage and validates data");

    println!("\n9. 🔑 Key Takeaways for Storage Usage");
    println!("   AggregatorBuilder storage options:");
    println!("   📌 AggregatorBuilder::default() - Uses MemoryStore::new() internally");
    println!("   📌 AggregatorBuilder::with_storage(store) - Use your custom storage instance");
    println!("   📌 Pre-populate storage before builder - Add data before aggregator starts");
    println!("   📌 Storage is shared - All services access the same storage instance");

    println!("\n✅ Storage Demo Completed!");
    println!("   The storage layer successfully provides:");
    println!("   • Multiple ways to configure storage via AggregatorBuilder");
    println!("   • Fast in-memory operations for development");
    println!("   • Consistent API for all services");
    println!("   • Easy extensibility for production backends");
    
    Ok(())
}