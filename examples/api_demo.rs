//! API demonstration example with mock adapter
//! Run with: INTEGRITY_SECRET=demo-secret-key cargo run --example api_demo

use oif_aggregator::{serde_json::json, AggregatorBuilder};
use reqwest::Client;
use tokio::time::{sleep, Duration};

// Import mock adapter from src/mocks.rs
use oif_aggregator::mocks::{mock_quote_request, mock_solver, MockDemoAdapter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("🚀 OIF Aggregator API Demo with Mock Adapter");
	println!("============================================");

	// Create and start the aggregator with mock adapter
	let mock_adapter = MockDemoAdapter::new();
	let mock_solver = mock_solver();

	let (app, _state) = AggregatorBuilder::default()
		.with_adapter(Box::new(mock_adapter))
		.with_solver(mock_solver)
		.start()
		.await?;

	// Start server in background
	let server_handle = tokio::spawn(async move {
		println!("🌐 Starting server on http://localhost:3000");
		let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
		axum::serve(listener, app).await.unwrap();
	});

	// Wait for server to start
	sleep(Duration::from_secs(2)).await;
	println!("   ✅ Server started successfully");

	let client = Client::new();
	let base_url = "http://localhost:3000";

	// Health check
	println!("\n1. Health Check");
	let health = client.get(format!("{}/health", base_url)).send().await?;
	println!("   Status: {}", health.status());
	if health.status().is_success() {
		let body = health.text().await?;
		println!("   Response: {}", body);
	}

	println!("\n2. Solvers");
	let solvers = client
		.get(format!("{}/v1/solvers", base_url))
		.send()
		.await?;

	let solvers_json: serde_json::Value = solvers.json().await?;
	println!(
		"   Solvers:\n{}",
		serde_json::to_string_pretty(&solvers_json)?
	);

	// Get quotes (ERC-7930 compliant request)
	println!("\n3. Get Quotes");

	let (quote_request, user_addr, _, _) = mock_quote_request();

	println!(
		"Quote request:\n{}",
		serde_json::to_string_pretty(&quote_request)?
	);

	let quotes_response = client
		.post(format!("{}/v1/quotes", base_url))
		.json(&quote_request)
		.send()
		.await?;

	println!("   Status: {}", quotes_response.status());
	if quotes_response.status().is_success() {
		let quotes_json: serde_json::Value = quotes_response.json().await?;
		println!("Quotes:\n{}", serde_json::to_string_pretty(&quotes_json)?);
		let total = quotes_json["totalQuotes"].as_u64().unwrap_or(0);
		println!("   Quotes received: {}", total);

		// Take first quote if available for order submission
		let first_quote = quotes_json["quotes"].get(0).cloned();

		if let Some(quote) = first_quote {
			// Submit order using the first quote
			println!("\n4. Submit Order");
			let order_request = json!({
				"userAddress": user_addr.to_hex(),
				"quoteResponse": quote,
				"signature": null
			});

			println!(
				"Order request:\n{}",
				serde_json::to_string_pretty(&order_request)?
			);

			let order_response = client
				.post(format!("{}/v1/orders", base_url))
				.json(&order_request)
				.send()
				.await?;

			println!("   Status: {}", order_response.status());
			if order_response.status().is_success() {
				let order_json: serde_json::Value = order_response.json().await?;
				let order_id = order_json["orderId"].as_str().unwrap_or("");
				println!("   Order ID: {}", order_id);
				println!("   Status: {}", order_json["status"]);

				// Query order status
				println!("\n4. Query Order Status");
				let status_response = client
					.get(format!("{}/v1/orders/{}", base_url, order_id))
					.send()
					.await?;

				println!("   Status: {}", status_response.status());
				if status_response.status().is_success() {
					let status_json: serde_json::Value = status_response.json().await?;
					println!("   Order Status: {}", status_json["status"]);
				}
			} else {
				let body = order_response.text().await.unwrap_or_default();
				println!("   Order submission failed: {}", body);

				// Check if this is the expected integrity verification error
				if body.contains("INTEGRITY_VERIFICATION_FAILED") {
					println!("   💡 This is expected - the mock quote doesn't have a valid integrity checksum");
					println!(
						"   💡 In production, quotes would have proper integrity verification"
					);
					println!("   ✅ The system is correctly enforcing security!");
				}
			}
		} else {
			println!("   No quotes returned; skipping order submission.");
		}
	} else {
		println!("   Quote request failed: {}", quotes_response.status());
	}

	println!("\n✅ Demo completed!");

	// Shutdown server
	server_handle.abort();
	Ok(())
}
