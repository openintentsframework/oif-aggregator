//! End-to-end test utilities and shared fixtures

use axum::Router;
use oif_aggregator::api::routes::{create_router, AppState};
use oif_aggregator::service::AggregatorService;
use oif_aggregator::storage::MemoryStore;
use oif_config::Settings;
use std::sync::Arc;
use tokio::task::JoinHandle;

// Re-export mocks for use in e2e tests
pub use crate::mocks::AppStateBuilder;

/// Test server instance with configurable settings
pub struct TestServer {
	pub base_url: String,
	pub handle: JoinHandle<()>,
}

impl TestServer {
	/// Spawn a test server with default settings
	pub async fn spawn() -> Self {
		Self::spawn_with_settings(Settings::default()).await
	}

	/// Spawn a test server with custom settings (e.g., rate limiting enabled)
	pub async fn spawn_with_settings(_settings: Settings) -> Self {
		// TODO: Apply settings to server configuration
		let state = AppStateBuilder::minimal();

		let app: Router = create_router().with_state(state);

		let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
			.await
			.expect("bind test port");
		let addr = listener.local_addr().unwrap();
		let base_url = format!("http://{}:{}", addr.ip(), addr.port());

		let handle = tokio::spawn(async move {
			let _ = axum::serve(listener, app).await;
		});

		Self { base_url, handle }
	}

	pub fn abort(self) {
		self.handle.abort();
	}
}

/// Re-export API fixtures for backward compatibility
pub mod fixtures {
	use crate::mocks::ApiFixtures;

	// Re-export as functions for backward compatibility
	pub fn valid_quote_request() -> serde_json::Value {
		ApiFixtures::valid_quote_request()
	}

	pub fn invalid_quote_request_empty_token() -> serde_json::Value {
		ApiFixtures::invalid_quote_request_empty_token()
	}

	pub fn valid_order_request_stateless() -> serde_json::Value {
		ApiFixtures::valid_order_request_stateless()
	}

	pub fn invalid_order_request_missing_user() -> serde_json::Value {
		ApiFixtures::invalid_order_request_missing_user()
	}

	pub fn invalid_order_request_missing_quote() -> serde_json::Value {
		ApiFixtures::invalid_order_request_missing_quote()
	}

	pub fn order_request_with_invalid_quote_id() -> serde_json::Value {
		ApiFixtures::order_request_with_invalid_quote_id()
	}
}
