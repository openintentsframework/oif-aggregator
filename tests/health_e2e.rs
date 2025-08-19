//! Health endpoint E2E tests

mod mocks;

use crate::mocks::TestServer;
use reqwest::Client;

#[tokio::test]
async fn test_health_endpoint() {
	let server = TestServer::spawn()
		.await
		.expect("Failed to start test server");
	let client = Client::new();

	let resp = client
		.get(format!("{}/health", server.base_url))
		.send()
		.await
		.unwrap();

	assert!(resp.status().is_success());
	let body: serde_json::Value = resp.json().await.unwrap();
	assert_eq!(body["status"], "healthy");

	server.abort();
}

// Move all health-related tests here from e2e/health_tests.rs
