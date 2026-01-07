//! End-to-end tests starting a live HTTP server

use axum::Router;
use oif_aggregator::{api::routes::create_router, AggregatorBuilder};
use reqwest::{get, Client};
use tokio::task::JoinHandle;

// TestServer and fixtures are now in mocks

mod mocks;
use mocks::{
	api_fixtures::{ApiFixtures, INTEGRITY_SECRET},
	TestServer,
};

async fn spawn_server() -> Result<(String, JoinHandle<()>), Box<dyn std::error::Error>> {
	// Set required environment variable for tests
	std::env::set_var("INTEGRITY_SECRET", INTEGRITY_SECRET);

	// Use AggregatorBuilder to create proper app state
	let (_router, state) = AggregatorBuilder::default().start().await?;

	let app: Router = create_router().with_state(state);

	let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
	let addr = listener.local_addr()?;
	let base_url = format!("http://{}:{}", addr.ip(), addr.port());

	let handle = tokio::spawn(async move {
		// Ignore serve errors when test aborts the task
		let _ = axum::serve(listener, app).await;
	});

	// Give server time to start
	tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

	Ok((base_url, handle))
}

#[tokio::test]
async fn test_health_endpoint() {
	let (base_url, handle) = spawn_server().await.expect("Failed to start server");

	let response = get(&format!("{}/health", base_url))
		.await
		.expect("Failed to get health endpoint");

	assert_eq!(response.status(), 503);

	// Parse JSON response
	let json_body: serde_json::Value = response
		.json()
		.await
		.expect("Failed to parse JSON response");

	// Verify the response structure
	assert!(json_body.get("status").is_some());
	assert!(json_body.get("version").is_some());
	assert!(json_body.get("solvers").is_some());
	assert!(json_body.get("storage").is_some());

	// Verify version matches package version
	assert_eq!(
		json_body["version"].as_str().unwrap(),
		env!("CARGO_PKG_VERSION")
	);

	handle.abort();
}

#[tokio::test]
async fn test_quotes_endpoint_empty() {
	let (base_url, handle) = spawn_server().await.expect("Failed to start server");

	// Use mock quote request for consistency
	let request_body = ApiFixtures::valid_quote_request();

	let client = Client::new();
	let response = client
		.post(format!("{}/api/v1/quotes", base_url))
		.json(&request_body)
		.send()
		.await
		.expect("Failed to post to quotes endpoint");

	// Status might be 400 due to validation or 200 with empty results
	assert!(response.status().is_success() || response.status() == 400);

	if response.status().is_success() {
		let json: serde_json::Value = response.json().await.expect("Failed to parse JSON");
		assert_eq!(json["totalQuotes"], 0); // No solvers configured
	}

	handle.abort();
}

#[tokio::test]
async fn test_orders_endpoint_invalid() {
	let (base_url, handle) = spawn_server().await.expect("Failed to start server");

	let client = Client::new();
	let response = client
        .post(format!("{}/api/v1/orders", base_url))
        .json(&ApiFixtures::invalid_order_request_missing_user())  // Use mock invalid request
        .send()
        .await
        .expect("Failed to post to orders endpoint");

	assert!(response.status() == 400 || response.status() == 422); // Bad request or validation error

	handle.abort();
}

#[tokio::test]
async fn test_solvers_endpoint() {
	let (base_url, handle) = spawn_server().await.expect("Failed to start server");

	let response = reqwest::get(&format!("{}/api/v1/solvers", base_url))
		.await
		.expect("Failed to get solvers endpoint");

	assert_eq!(response.status(), 200);

	let json: serde_json::Value = response.json().await.expect("Failed to parse JSON");
	assert!(json["solvers"].is_array());
	assert!(json["totalSolvers"].is_number());

	handle.abort();
}

#[tokio::test]
async fn test_orders_endpoint_with_mock_data() {
	let server = TestServer::spawn_with_mock_adapter()
		.await
		.expect("Failed to start test server");
	let client = Client::new();

	// Get a real quote from the server first
	let quote_request = ApiFixtures::valid_quote_request();
	let quote_response = client
		.post(format!("{}/api/v1/quotes", server.base_url))
		.json(&quote_request)
		.send()
		.await
		.expect("Failed to get quotes");

	assert!(
		quote_response.status().is_success(),
		"Failed to get quotes: {}",
		quote_response.status()
	);

	let quotes_json: serde_json::Value = quote_response
		.json()
		.await
		.expect("Failed to parse quotes response");
	let quotes = quotes_json["quotes"].as_array().expect("No quotes array");

	if quotes.is_empty() {
		//println!("No quotes returned from mock adapter");
		server.handle.abort();
		return;
	}

	let first_quote = &quotes[0];

	// Create order request using the real quote
	let order_request = serde_json::json!({
		"quoteResponse": first_quote,
		"signature": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef12341b",
	});

	// Submit the order
	let order_response = client
		.post(format!("{}/api/v1/orders", server.base_url))
		.json(&order_request)
		.send()
		.await
		.expect("Failed to post to orders endpoint");

	let status = order_response.status();
	if !status.is_success() {
		order_response
			.text()
			.await
			.expect("Failed to read error body");
	}

	// Should now work with proper integrity checksum
	assert!(
		status == 200 || status == 201,
		"Order submission failed with status: {}",
		status
	);

	server.handle.abort();
}

#[tokio::test]
async fn test_get_order_not_found() {
	let (base_url, handle) = spawn_server().await.expect("Failed to start server");

	let response = reqwest::get(&format!("{}/api/v1/orders/non-existent-order", base_url))
		.await
		.expect("Failed to get order endpoint");

	assert_eq!(response.status(), 404); // Not found

	handle.abort();
}
