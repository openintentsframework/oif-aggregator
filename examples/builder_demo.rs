//! Builder Pattern demonstration example

use oif_aggregator::models::solvers::SolverMetadata;
use oif_aggregator::{
    AggregatorBuilder, Intent, Quote, QuoteRequest, Solver, SolverStatus, config::Settings,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🏗️  OIF Aggregator Builder Pattern Demo");
    println!("=====================================");

    // Demo 1: Basic builder with defaults
    println!("\n1. Basic Builder with Defaults");
    let basic_builder = AggregatorBuilder::new();
    let (router, state) = basic_builder.start().await?;
    let stats = state.aggregator_service.get_stats();
    println!(
        "   ✅ Created aggregator with {} solvers, {} adapters",
        stats.total_solvers, stats.initialized_adapters
    );
    drop(router); // Clean up

    // Demo 2: Builder with custom solver
    println!("\n2. Builder with Custom Solver");
    let mut custom_solver = Solver::new(
        "demo-solver".to_string(),
        "oif-v1".to_string(),
        "http://localhost:8080".to_string(),
        3000,
    );
    custom_solver.metadata.name = Some("Demo Solver".to_string());
    custom_solver.metadata.description = Some("A demonstration solver".to_string());
    custom_solver.metadata.version = Some("1.0.0".to_string());
    custom_solver.metadata.supported_chains = vec![1, 137]; // Now u64 instead of String
    custom_solver.metadata.max_retries = 3;
    custom_solver.status = SolverStatus::Active;

    let solver_builder = AggregatorBuilder::new()
        .with_solver(custom_solver)
        .with_ttl_cleanup(true);

    let (router, state) = solver_builder.start().await?;
    let stats = state.aggregator_service.get_stats();
    println!(
        "   ✅ Created aggregator with {} solvers, {} adapters",
        stats.total_solvers, stats.initialized_adapters
    );
    drop(router); // Clean up

    // Demo 3: Builder with configuration
    println!("\n3. Builder with Configuration");
    let config_builder = AggregatorBuilder::from_config(Settings::default());
    let (router, state) = config_builder.start().await?;
    let stats = state.aggregator_service.get_stats();
    println!(
        "   ✅ Created aggregator with {} solvers, {} adapters",
        stats.total_solvers, stats.initialized_adapters
    );
    drop(router); // Clean up

    // Demo 4: Method chaining
    println!("\n4. Method Chaining Example");
    let chained_builder = AggregatorBuilder::new().with_ttl_cleanup(true);

    let (router, state) = chained_builder.start().await?;
    let stats = state.aggregator_service.get_stats();
    println!(
        "   ✅ Created aggregator with chained methods: {} solvers, {} adapters",
        stats.total_solvers, stats.initialized_adapters
    );
    drop(router); // Clean up

    println!("\n5. Complete Server Setup (Commented Out)");
    println!("   // To start a complete server with all defaults:");
    println!("   // AggregatorBuilder::new().start_server().await?;");
    println!("   //");
    println!("   // This would handle:");
    println!("   // - Loading .env configuration");
    println!("   // - Initializing tracing/logging");
    println!("   // - Setting up TTL cleanup");
    println!("   // - Binding to server address");
    println!("   // - Starting the HTTP server");

    println!("\n✅ Builder Pattern Demo Completed!");
    println!("\nThe builder provides flexible configuration:");
    println!("  🔧 AggregatorBuilder::new() - Start with defaults");
    println!("  📁 .from_config(settings) - Load from configuration");
    println!("  🚀 .with_solver(solver) - Add custom solvers");
    println!("  💾 .with_storage(storage) - Custom storage layer");
    println!("  ⏰ .with_ttl_cleanup(true) - Enable quote TTL");
    println!("  🎯 .start() - Create router and state");
    println!("  🌐 .start_server() - Full server with defaults");

    Ok(())
}
