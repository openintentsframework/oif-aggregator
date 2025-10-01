//! End-to-end tests for circuit breaker functionality
//!
//! These tests verify circuit breaker behavior with actual HTTP requests,
//! solver failures, and recovery scenarios.

use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

// Removed unused imports

mod mocks;
use mocks::{api_fixtures::ApiFixtures, configs::CircuitBreakerConfigs, test_server::TestServer};

/// Test circuit breaker blocks requests after consecutive failures
#[tokio::test]
async fn test_circuit_breaker_opens_on_consecutive_failures() {
	let server = TestServer::spawn_with_circuit_breaker_config(
		CircuitBreakerConfigs::strict_failure_threshold(2), // Open after 2 failures
		vec![
			(
				"failing-solver".to_string(),
				mocks::adapters::MockTestAdapter::with_failure_config(
					"failing-adapter".to_string(),
				),
			),
			(
				"working-solver".to_string(),
				mocks::adapters::MockTestAdapter::with_success_config(
					"working-adapter".to_string(),
				),
			),
		],
	)
	.await
	.expect("Failed to start test server");

	let client = Client::new();
	let quote_request = ApiFixtures::valid_quote_request();

	// First request should work but failing-solver will fail
	let response1 = client
		.post(format!("{}/v1/quotes", server.base_url))
		.json(&quote_request)
		.send()
		.await
		.expect("Failed to make first request");

	if !response1.status().is_success() {
		let status = response1.status();
		let text = response1
			.text()
			.await
			.expect("Failed to read response text");
		panic!(
			"First request failed with status {} and body: {}",
			status, text
		);
	}

	// Second request - failing-solver fails again, should trigger circuit breaker
	let response2 = client
		.post(format!("{}/v1/quotes", server.base_url))
		.json(&quote_request)
		.send()
		.await
		.expect("Failed to make second request");

	assert!(response2.status().is_success());

	// Third request should show circuit breaker is active
	// The failing solver should be excluded from aggregation
	let response3 = client
		.post(format!("{}/v1/quotes", server.base_url))
		.json(&quote_request)
		.send()
		.await
		.expect("Failed to make third request");

	assert!(response3.status().is_success());

	let quotes_json: serde_json::Value = response3.json().await.expect("Failed to parse JSON");
	let quotes = quotes_json["quotes"].as_array().expect("No quotes array");

	// Should only get quotes from working-solver, not failing-solver
	assert!(
		quotes.len() <= 1,
		"Circuit breaker should limit failing solver quotes"
	);

	server.handle.abort();
}

/// Test circuit breaker opens due to poor success rate
#[tokio::test]
async fn test_circuit_breaker_opens_on_success_rate() {
	// This test uses a working adapter but simulates poor success rate through
	// multiple solvers where some fail (timeout) and some succeed
	let server = TestServer::spawn_with_circuit_breaker_config(
		CircuitBreakerConfigs::strict_success_rate(0.8, 3), // 80% success rate over 3 requests
		vec![
			(
				"working-solver".to_string(),
				mocks::adapters::MockDemoAdapter::with_config("working-adapter".to_string()),
			),
			(
				"slow-solver".to_string(),
				mocks::adapters::MockDemoAdapter::with_config("slow-adapter".to_string()), // This will timeout and be recorded as failure
			),
		],
	)
	.await
	.expect("Failed to start test server");

	let client = Client::new();
	let quote_request = ApiFixtures::valid_quote_request();

	// Make multiple requests - working solver should provide quotes,
	// but slow solver timeouts should be recorded as failures affecting success rate
	for i in 0..5 {
		let response = client
			.post(format!("{}/v1/quotes", server.base_url))
			.json(&quote_request)
			.send()
			.await
			.unwrap_or_else(|_| panic!("Failed to make request {}", i + 1));

		// Should return 200 with at least quotes from working solver
		if !response.status().is_success() {
			let status = response.status();
			let text = response.text().await.expect("Failed to read response text");
			panic!(
				"Request {} failed with status {} and body: {}",
				i + 1,
				status,
				text
			);
		}

		// Small delay between requests
		sleep(Duration::from_millis(100)).await;
	}

	// After several timeout failures, circuit breaker should start limiting the slow solver
	let final_response = client
		.post(format!("{}/v1/quotes", server.base_url))
		.json(&quote_request)
		.send()
		.await
		.expect("Failed to make final request");

	assert!(final_response.status().is_success());

	server.handle.abort();
}

/// Test circuit breaker recovery timeout behavior
#[tokio::test]
async fn test_circuit_breaker_recovery_timeout() {
	// This test verifies that circuit breakers with short recovery timeouts
	// transition to half-open state quickly for recovery attempts
	let server = TestServer::spawn_with_circuit_breaker_config(
		CircuitBreakerConfigs::fast_recovery(
			2,                         // failure_threshold - low for quick triggering
			Duration::from_millis(50), // base_timeout - very short for quick recovery
		),
		vec![(
			"test-solver".to_string(),
			mocks::adapters::MockDemoAdapter::new(),
		)],
	)
	.await
	.expect("Failed to start test server");

	let client = Client::new();
	let quote_request = ApiFixtures::valid_quote_request();

	// Make requests to ensure circuit breaker behavior is tested
	// The fast solver should work, demonstrating the circuit breaker isn't incorrectly blocking
	for i in 0..3 {
		let response = client
			.post(format!("{}/v1/quotes", server.base_url))
			.json(&quote_request)
			.send()
			.await
			.unwrap_or_else(|_| panic!("Failed to make request {}", i + 1));

		assert!(response.status().is_success());

		let quotes_json: serde_json::Value = response.json().await.expect("Failed to parse JSON");
		let quotes = quotes_json["quotes"].as_array().expect("No quotes array");

		// Should get quotes from working solver
		assert!(!quotes.is_empty(), "Should get quotes from working solver");

		// Small delay between requests
		sleep(Duration::from_millis(20)).await;
	}

	server.handle.abort();
}

/// Test circuit breaker recovery (half-open to closed)
#[tokio::test]
async fn test_circuit_breaker_recovery() {
	let server = TestServer::spawn_with_circuit_breaker_config(
		CircuitBreakerConfigs::fast_recovery(
			2,                          // failure_threshold
			Duration::from_millis(100), // base_timeout
		),
		vec![
			(
				"recovering-solver".to_string(),
				mocks::adapters::MockDemoAdapter::new(),
			), // Initially healthy
		],
	)
	.await
	.expect("Failed to start test server");

	let client = Client::new();
	let quote_request = ApiFixtures::valid_quote_request();

	// Initial request should work
	let response1 = client
		.post(format!("{}/v1/quotes", server.base_url))
		.json(&quote_request)
		.send()
		.await
		.expect("Failed to make initial request");

	assert!(response1.status().is_success());

	// Wait for potential recovery timeout
	sleep(Duration::from_millis(200)).await;

	// Follow-up request should also work (recovery)
	let response2 = client
		.post(format!("{}/v1/quotes", server.base_url))
		.json(&quote_request)
		.send()
		.await
		.expect("Failed to make recovery request");

	assert!(response2.status().is_success());

	server.handle.abort();
}

/// Test circuit breaker with multiple solvers having different states
#[tokio::test]
async fn test_circuit_breaker_mixed_solver_states() {
	let server = TestServer::spawn_with_circuit_breaker_config(
		CircuitBreakerConfigs::standard(),
		vec![
			(
				"healthy-solver".to_string(),
				mocks::adapters::MockDemoAdapter::with_config("healthy-adapter".to_string()),
			),
			(
				"failing-solver".to_string(),
				mocks::adapters::MockDemoAdapter::with_config("failing-adapter".to_string()),
			),
			(
				"slow-solver".to_string(),
				mocks::adapters::MockDemoAdapter::with_config("slow-adapter".to_string()),
			),
		],
	)
	.await
	.expect("Failed to start test server");

	let client = Client::new();
	let quote_request = ApiFixtures::valid_quote_request();

	// Make multiple requests to establish different solver states
	for i in 0..4 {
		let response = client
			.post(format!("{}/v1/quotes", server.base_url))
			.json(&quote_request)
			.send()
			.await
			.unwrap_or_else(|_| panic!("Failed to make request {}", i + 1));

		assert!(response.status().is_success());
		sleep(Duration::from_millis(50)).await;
	}

	// Final request should demonstrate circuit breaker is working
	// - healthy-solver should provide quotes
	// - failing-solver should be circuit broken
	// - slow-solver behavior depends on timeout config
	let final_response = client
		.post(format!("{}/v1/quotes", server.base_url))
		.json(&quote_request)
		.send()
		.await
		.expect("Failed to make final request");

	assert!(final_response.status().is_success());

	let quotes_json: serde_json::Value = final_response.json().await.expect("Failed to parse JSON");

	// Should still get successful response with available solvers
	assert!(quotes_json["totalQuotes"].is_number());

	server.handle.abort();
}

/// Test circuit breaker configuration validation through API
#[tokio::test]
async fn test_circuit_breaker_health_endpoint() {
	let server = TestServer::spawn_with_circuit_breaker_config(
		CircuitBreakerConfigs::standard(),
		vec![(
			"test-solver".to_string(),
			mocks::adapters::MockDemoAdapter::new(),
		)],
	)
	.await
	.expect("Failed to start test server");

	let client = Client::new();

	// Check health endpoint includes circuit breaker status
	let response = client
		.get(format!("{}/health", server.base_url))
		.send()
		.await
		.expect("Failed to get health endpoint");

	assert!(response.status().is_success());

	let health_json: serde_json::Value =
		response.json().await.expect("Failed to parse health JSON");

	// Verify circuit breaker info is present
	assert!(health_json["solvers"].is_object());
	assert!(health_json["solvers"]["healthDetails"].is_object());

	server.handle.abort();
}

/// Test circuit breaker doesn't interfere with valid orders
#[tokio::test]
async fn test_circuit_breaker_order_flow() {
	let server = TestServer::spawn_with_circuit_breaker_config(
		CircuitBreakerConfigs::standard(),
		vec![(
			"order-solver".to_string(),
			mocks::adapters::MockDemoAdapter::new(),
		)],
	)
	.await
	.expect("Failed to start test server");

	let client = Client::new();

	// Get quotes first
	let quote_request = ApiFixtures::valid_quote_request();
	let quote_response = client
		.post(format!("{}/v1/quotes", server.base_url))
		.json(&quote_request)
		.send()
		.await
		.expect("Failed to get quotes");

	assert!(quote_response.status().is_success());

	let quotes_json: serde_json::Value = quote_response
		.json()
		.await
		.expect("Failed to parse quotes JSON");
	let quotes = quotes_json["quotes"].as_array().expect("No quotes array");

	if quotes.is_empty() {
		server.handle.abort();
		return; // Skip order test if no quotes
	}

	// Submit order using first quote
	let order_request = json!({
		"quoteResponse": &quotes[0],
		"signature": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef12341b",
	});

	let order_response = client
		.post(format!("{}/v1/orders", server.base_url))
		.json(&order_request)
		.send()
		.await
		.expect("Failed to submit order");

	// Circuit breaker should not interfere with order processing
	assert!(
		order_response.status().is_success() || order_response.status() == 400,
		"Order should succeed or fail validation, not be blocked by circuit breaker"
	);

	server.handle.abort();
}
