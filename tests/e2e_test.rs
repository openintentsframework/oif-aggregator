//! End-to-end tests starting a live HTTP server

use axum::Router;
use oif_aggregator::api::routes::{create_router, AppState};
use oif_aggregator::service::{AggregatorService, OrderService, SolverService};
use oif_aggregator::storage::MemoryStore;
use std::sync::Arc;
use tokio::task::JoinHandle;

async fn spawn_server() -> (String, JoinHandle<()>) {
	// Minimal app state (no solvers)
	let aggregator_service = AggregatorService::new(vec![], 5_000);
	let storage = Arc::new(MemoryStore::new());
	let state = AppState {
		aggregator_service: Arc::new(aggregator_service),
		storage: storage.clone(),
		order_service: Arc::new(OrderService::new(storage.clone())),
		solver_service: Arc::new(SolverService::new(storage.clone())),
	};

	let app: Router = create_router().with_state(state);

	let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
		.await
		.expect("bind test port");
	let addr = listener.local_addr().unwrap();
	let base_url = format!("http://{}:{}", addr.ip(), addr.port());

	let handle = tokio::spawn(async move {
		// Ignore serve errors when test aborts the task
		let _ = axum::serve(listener, app).await;
	});

	(base_url, handle)
}

#[tokio::test]
async fn e2e_health_and_request_id() {
	let (base_url, handle) = spawn_server().await;
	let client = reqwest::Client::new();

	// Health
	let resp = client
		.get(format!("{}/health", base_url))
		.send()
		.await
		.unwrap();
	assert!(resp.status().is_success());
	let body = resp.text().await.unwrap();
	assert_eq!(body, "OK");

	// Request ID auto-injection
	let resp = client
		.post(format!("{}/v1/quotes", base_url))
		.json(&serde_json::json!({
			"token_in": "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
			"token_out": "0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0",
			"amount_in": "1000000000000000000",
			"chain_id": 1
		}))
		.send()
		.await
		.unwrap();
	assert!(resp.status().is_success());
	let req_id = resp.headers().get("x-request-id");
	assert!(req_id.is_some());

	// Request ID propagation when provided by client
	let provided = "test-req-id-123";
	let resp = client
		.post(format!("{}/v1/quotes", base_url))
		.header("x-request-id", provided)
		.json(&serde_json::json!({
			"token_in": "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
			"token_out": "0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0",
			"amount_in": "1000000000000000000",
			"chain_id": 1
		}))
		.send()
		.await
		.unwrap();
	assert!(resp.status().is_success());
	let echoed = resp
		.headers()
		.get("x-request-id")
		.and_then(|v| v.to_str().ok())
		.unwrap_or("");
	assert_eq!(echoed, provided);

	handle.abort();
}

#[tokio::test]
async fn e2e_cors_preflight() {
	let (base_url, handle) = spawn_server().await;
	let client = reqwest::Client::new();

	let resp = client
		.request(reqwest::Method::OPTIONS, format!("{}/v1/quotes", base_url))
		.header("Origin", "http://example.com")
		.header("Access-Control-Request-Method", "POST")
		.send()
		.await
		.unwrap();

	// CORS layer should handle preflight (permissive). Some stacks return 200.
	assert!(
		resp.status() == reqwest::StatusCode::NO_CONTENT
			|| resp.status() == reqwest::StatusCode::OK
	);
	let allow_origin = resp.headers().get("access-control-allow-origin");
	assert!(allow_origin.is_some());

	handle.abort();
}

#[tokio::test]
async fn e2e_readiness_probe() {
	let (base_url, handle) = spawn_server().await;
	let client = reqwest::Client::new();

	let resp = client
		.get(format!("{}/ready", base_url))
		.send()
		.await
		.unwrap();

	// With empty solvers and healthy memory storage, should be ready
	assert_eq!(resp.status(), reqwest::StatusCode::OK);
	let json: serde_json::Value = resp.json().await.unwrap();
	assert_eq!(json["status"], "ready");
	assert_eq!(json["storage_healthy"], true);

	handle.abort();
}

#[tokio::test]
async fn e2e_quotes_invalid_and_valid() {
	let (base_url, handle) = spawn_server().await;
	let client = reqwest::Client::new();

	// Invalid (empty token_in)
	let resp = client
		.post(format!("{}/v1/quotes", base_url))
		.json(&serde_json::json!({
			"token_in": "",
			"token_out": "0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0",
			"amount_in": "1000000000000000000",
			"chain_id": 1
		}))
		.send()
		.await
		.unwrap();
	assert_eq!(resp.status(), reqwest::StatusCode::BAD_REQUEST);

	// Valid
	let resp = client
		.post(format!("{}/v1/quotes", base_url))
		.json(&serde_json::json!({
			"token_in": "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
			"token_out": "0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0",
			"amount_in": "1000000000000000000",
			"chain_id": 1,
			"slippage_tolerance": 0.005
		}))
		.send()
		.await
		.unwrap();
	assert_eq!(resp.status(), reqwest::StatusCode::OK);
	let body: serde_json::Value = resp.json().await.unwrap();
	assert!(body["quotes"].is_array());
	assert_eq!(body["total_quotes"], 0);

	handle.abort();
}

#[tokio::test]
async fn e2e_orders_invalid_missing_user_and_missing_quote() {
	let (base_url, handle) = spawn_server().await;
	let client = reqwest::Client::new();

	// Missing user_address (extractor/serde failure -> 422)
	let resp = client
		.post(format!("{}/v1/orders", base_url))
		.json(&serde_json::json!({
			"quote_id": "test-quote-id"
		}))
		.send()
		.await
		.unwrap();
	assert_eq!(resp.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);

	// Neither quote_id nor quote_response
	let resp = client
		.post(format!("{}/v1/orders", base_url))
		.json(&serde_json::json!({
			"user_address": "0x1234567890123456789012345678901234567890"
		}))
		.send()
		.await
		.unwrap();
	assert_eq!(resp.status(), reqwest::StatusCode::BAD_REQUEST);

	handle.abort();
}

#[tokio::test]
async fn e2e_orders_quote_not_found_and_status_flow() {
	let (base_url, handle) = spawn_server().await;
	let client = reqwest::Client::new();

	// Quote not found
	let resp = client
		.post(format!("{}/v1/orders", base_url))
		.json(&serde_json::json!({
			"user_address": "0x1234567890123456789012345678901234567890",
			"quote_id": "non-existent-quote-id"
		}))
		.send()
		.await
		.unwrap();
	assert_eq!(resp.status(), reqwest::StatusCode::NOT_FOUND);

	// Stateless order with quote_response
	let resp = client
		.post(format!("{}/v1/orders", base_url))
		.json(&serde_json::json!({
			"user_address": "0x1234567890123456789012345678901234567890",
			"quote_response": {
				"token_in": "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
				"token_out": "0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0",
				"amount_in": "1000000000000000000",
				"amount_out": "2000000000",
				"chain_id": 1,
				"price_impact": 0.02,
				"estimated_gas": 150000
			},
			"slippage_tolerance": 0.01,
			"deadline": (chrono::Utc::now().timestamp() + 3600)
		}))
		.send()
		.await
		.unwrap();
	assert_eq!(resp.status(), reqwest::StatusCode::OK);
	let json: serde_json::Value = resp.json().await.unwrap();
	let order_id = json["order_id"].as_str().unwrap();

	// Query status
	let resp = client
		.get(format!("{}/v1/orders/{}", base_url, order_id))
		.send()
		.await
		.unwrap();
	assert_eq!(resp.status(), reqwest::StatusCode::OK);
	let status_json: serde_json::Value = resp.json().await.unwrap();
	assert_eq!(status_json["order_id"], order_id);

	handle.abort();
}
