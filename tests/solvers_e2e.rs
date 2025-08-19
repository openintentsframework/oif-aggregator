//! Solvers API E2E tests
//!
//! Tests for the /v1/solvers endpoints covering solver listing
//! and individual solver retrieval.

mod mocks;

use crate::mocks::TestServer;
use reqwest::Client;

#[tokio::test]
async fn test_get_solvers() {
	let server = TestServer::spawn()
		.await
		.expect("Failed to start test server");
	let client = Client::new();

	let resp = client
		.get(format!("{}/v1/solvers", server.base_url))
		.send()
		.await
		.unwrap();

	assert!(resp.status().is_success());

	let body: serde_json::Value = resp.json().await.unwrap();
	assert!(body["solvers"].is_array());

	server.abort();
}

#[tokio::test]
async fn test_get_solver_by_id() {
	let server = TestServer::spawn()
		.await
		.expect("Failed to start test server");
	let client = Client::new();

	// First get all solvers to find a valid ID
	let solvers_resp = client
		.get(format!("{}/v1/solvers", server.base_url))
		.send()
		.await
		.unwrap();

	assert!(solvers_resp.status().is_success());
	let solvers_body: serde_json::Value = solvers_resp.json().await.unwrap();
	let solvers = solvers_body["solvers"].as_array().unwrap();

	if !solvers.is_empty() {
		let solver_id = solvers[0]["solverId"].as_str().unwrap();

		let resp = client
			.get(format!("{}/v1/solvers/{}", server.base_url, solver_id))
			.send()
			.await
			.unwrap();

		assert!(resp.status().is_success());

		let body: serde_json::Value = resp.json().await.unwrap();
		assert_eq!(body["solverId"].as_str().unwrap(), solver_id);
	}

	server.abort();
}

#[tokio::test]
async fn test_get_solver_not_found() {
	let server = TestServer::spawn()
		.await
		.expect("Failed to start test server");
	let client = Client::new();

	let resp = client
		.get(format!(
			"{}/v1/solvers/non-existent-solver",
			server.base_url
		))
		.send()
		.await
		.unwrap();

	assert_eq!(resp.status(), 404);
	server.abort();
}
