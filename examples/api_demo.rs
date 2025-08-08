//! API demonstration example

use oif_aggregator::serde_json::json;
use reqwest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// This example demonstrates how to interact with the OIF Aggregator API

	println!("🚀 OIF Aggregator API Demo");
	println!("==========================");

	let client = reqwest::Client::new();
	let base_url = "http://localhost:3000";

	// Health check
	println!("\n1. Health Check");
	let health = client.get(&format!("{}/health", base_url)).send().await?;
	println!("   Status: {}", health.status());
	if health.status().is_success() {
		let body = health.text().await?;
		println!("   Response: {}", body);
	}

	// Get quotes
	println!("\n2. Get Quotes");
	let quote_request = json!({
		"token_in": "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
		"token_out": "0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0",
		"amount_in": "1000000000000000000",
		"chain_id": 1,
		"slippage_tolerance": 0.005
	});

	let quotes_response = client
		.post(&format!("{}/v1/quotes", base_url))
		.json(&quote_request)
		.send()
		.await?;

	println!("   Status: {}", quotes_response.status());
	if quotes_response.status().is_success() {
		let quotes_json: serde_json::Value = quotes_response.json().await?;
		println!("   Quotes received: {}", quotes_json["total_quotes"]);
		println!("   Request ID: {}", quotes_json["request_id"]);
	}

	// Submit order (stateless)
	println!("\n3. Submit Order (Stateless)");
	let order_request = json!({
		"user_address": "0x1234567890123456789012345678901234567890",
		"quote_response": {
			"amount_out": "2000000000",
			"solver_id": "example-solver"
		},
		"slippage_tolerance": 0.01
	});

	let order_response = client
		.post(&format!("{}/v1/orders", base_url))
		.json(&order_request)
		.send()
		.await?;

	println!("   Status: {}", order_response.status());
	if order_response.status().is_success() {
		let order_json: serde_json::Value = order_response.json().await?;
		let order_id = order_json["order_id"].as_str().unwrap();
		println!("   Order ID: {}", order_id);
		println!("   Status: {}", order_json["status"]);

		// Query order status
		println!("\n4. Query Order Status");
		let status_response = client
			.get(&format!("{}/v1/orders/{}", base_url, order_id))
			.send()
			.await?;

		println!("   Status: {}", status_response.status());
		if status_response.status().is_success() {
			let status_json: serde_json::Value = status_response.json().await?;
			println!("   Order Status: {}", status_json["status"]);
			println!("   User: {}", status_json["user_address"]);
		}
	}

	println!("\n✅ Demo completed!");
	println!("\nTo run this demo:");
	println!("1. Start the server: cargo run");
	println!("2. Run this example: cargo run --example api_demo");

	Ok(())
}
