//! Tests for core data models and TTL functionality

mod mocks;

use mocks::{MockEntities, TestConstants};
use oif_storage::{
	traits::{OrderStorage, QuoteStorage, SolverStorage},
	MemoryStore,
};
use oif_types::chrono::{Duration, Utc};
use oif_types::{
	orders::{Order, OrderStatus},
	quotes::{Quote, QuoteRequest},
	solvers::{Solver, SolverStatus},
};

#[tokio::test]
async fn test_quote_ttl() {
	let mut quote = MockEntities::complete_quote();

	// Test fresh quote
	assert!(!quote.is_expired());
	assert!(quote.ttl_seconds() > 0);

	// Test expired quote
	quote.expires_at = Utc::now() - Duration::minutes(1);
	assert!(quote.is_expired());
	assert_eq!(quote.ttl_seconds(), 0);
}

#[tokio::test]
async fn test_memory_store_ttl() {
	let store = MemoryStore::with_ttl_enabled(true);

	// Create a quote that expires in 1 second for testing
	let quote = MockEntities::expiring_quote(1);
	let quote_id = quote.quote_id.clone();

	// Add quote to store
	store.add_quote(quote).await.expect("Failed to add quote");

	// Quote should be available immediately
	assert!(store
		.get_quote(&quote_id)
		.await
		.expect("Failed to get quote")
		.is_some());

	// Wait for expiry
	tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

	// Quote should be automatically removed when accessed
	assert!(store
		.get_quote(&quote_id)
		.await
		.expect("Failed to get quote")
		.is_none());
}

#[test]
fn test_quote_request_creation() {
	let request = QuoteRequest::new(
		"0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(), // WETH
		"0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0".to_string(), // USDC
		"1000000000000000000".to_string(),                        // 1 ETH
		1,                                                        // Ethereum mainnet
	);

	assert!(!request.request_id.is_empty());
	assert_eq!(
		request.token_in,
		"0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
	);
	assert_eq!(
		request.token_out,
		"0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0"
	);
	assert_eq!(request.amount_in, "1000000000000000000");
	assert_eq!(request.chain_id, 1);
	assert_eq!(request.slippage_tolerance, Some(0.005)); // 0.5% default
	assert!(request.deadline.is_some());
}

#[test]
fn test_order_creation() {
	let order = Order::new(
		"0x1234567890123456789012345678901234567890".to_string(),
		0.005,                           // 0.5% slippage tolerance
		Utc::now() + Duration::hours(1), // 1 hour deadline
	);

	assert!(!order.order_id.is_empty());
	assert_eq!(
		order.user_address,
		"0x1234567890123456789012345678901234567890"
	);
	assert_eq!(order.quote_id, None); // New intent constructor doesn't set quote_id
	assert_eq!(order.status, OrderStatus::Pending);
	assert_eq!(order.slippage_tolerance, 0.005);
}

#[test]
fn test_solver_configuration() {
	let mut solver = Solver::new(
		"lifi-mainnet".to_string(),
		"lifi-v1".to_string(),
		"https://api.lifi.com/mainnet".to_string(),
		2000,
	);
	solver.status = SolverStatus::Active;
	solver.metadata.name = Some("LiFi Mainnet".to_string());
	solver.metadata.description = Some("LiFi cross-chain solver".to_string());
	solver.metadata.version = Some("1.0.0".to_string());
	solver.metadata.supported_chains = vec![1, 137]; // Now u64 instead of String
	solver.metadata.max_retries = 3;

	assert_eq!(solver.solver_id, "lifi-mainnet");
	assert_eq!(solver.status, SolverStatus::Active);
	assert_eq!(solver.metadata.supported_chains.len(), 2);
	assert_eq!(solver.metadata.max_retries, 3);
}

#[tokio::test]
async fn test_storage_stats() {
	let store = MemoryStore::new();

	// Add test data
	let mut solver = Solver::new(
		"test-solver".to_string(),
		"test-adapter".to_string(),
		"https://example.com".to_string(),
		1000,
	);
	solver.status = SolverStatus::Active;
	store
		.add_solver(solver)
		.await
		.expect("Failed to add solver");

	let quote = Quote::new(
		"test-solver".to_string(),
		"test-request".to_string(),
		"0xToken1".to_string(),
		"0xToken2".to_string(),
		"1000".to_string(),
		"2000".to_string(),
		1,
	);
	store.add_quote(quote).await.expect("Failed to add quote");

	let order = Order::new("0x123".to_string(), 0.005, Utc::now() + Duration::hours(1));
	store.add_order(order).await.expect("Failed to add intent");

	let stats = store.get_stats().await;
	assert_eq!(stats.total_solvers, 1);
	assert_eq!(stats.total_quotes, 1);
	assert_eq!(stats.total_orders, 1);
	assert_eq!(stats.active_quotes, 1);
}
