//! End-to-end test utilities and shared fixtures

use axum::Router;
use oif_aggregator::{api::routes::create_router, AggregatorBuilder};

use tokio::task::JoinHandle;

// E2E test utilities - no longer need AppStateBuilder as we use AggregatorBuilder

/// Test server instance with configurable settings
pub struct TestServer {
    pub base_url: String,
    pub handle: JoinHandle<()>,
}

impl TestServer {
    /// Spawn a test server with default settings
    pub async fn spawn() -> Result<Self, Box<dyn std::error::Error>> {
        Self::spawn_with_mock_adapter().await
    }

    /// Spawn a test server with mock adapter for testing
    pub async fn spawn_with_mock_adapter() -> Result<Self, Box<dyn std::error::Error>> {
        // Set required environment variable for tests
        std::env::set_var("INTEGRITY_SECRET", "test-secret-for-e2e-mod-tests-12345678901234567890");
        
        let mock_adapter = oif_aggregator::mocks::MockDemoAdapter::new();
        let mock_solver = oif_aggregator::mocks::mock_solver();
        
        let (_router, state) = AggregatorBuilder::default()
            .with_adapter(Box::new(mock_adapter))?
            .with_solver(mock_solver)
            .await
            .start()
            .await?;

        let app: Router = create_router().with_state(state);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test port");
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://{}:{}", addr.ip(), addr.port());

        let handle = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        // Give server time to start
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        Ok(Self { base_url, handle })
    }

    /// Spawn a test server with minimal configuration (no solvers)
    pub async fn spawn_minimal() -> Result<Self, Box<dyn std::error::Error>> {
        // Set required environment variable for tests
        std::env::set_var("INTEGRITY_SECRET", "test-secret-for-e2e-mod-tests-12345678901234567890");
        
        let (_router, state) = AggregatorBuilder::default().start().await?;
        let app: Router = create_router().with_state(state);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test port");
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://{}:{}", addr.ip(), addr.port());

        let handle = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        Ok(Self { base_url, handle })
    }


    pub fn abort(self) {
        self.handle.abort();
    }
}

/// Re-export API fixtures for backward compatibility
pub mod fixtures {
    use crate::mocks::ApiFixtures;
    use oif_types::{InteropAddress, serde_json::Value};

    // Re-export key fixtures for backward compatibility
    pub fn valid_quote_request() -> Value {
        ApiFixtures::valid_quote_request()
    }

    pub fn invalid_quote_request_empty_token() -> Value {
        ApiFixtures::invalid_quote_request_missing_user()
    }

    pub fn valid_order_request_stateless() -> Value {
        ApiFixtures::valid_order_request()
    }

    pub fn invalid_order_request_missing_user() -> Value {
        ApiFixtures::invalid_order_request_missing_user()
    }

    pub fn invalid_order_request_missing_quote() -> Value {
        // Create a request missing quote data
        use oif_types::serde_json::json;
        let user_addr = InteropAddress::from_chain_and_address(1, "0x1234567890123456789012345678901234567890").unwrap();
        
        json!({
            "userAddress": user_addr.to_hex()
            // Missing both quoteResponse and quoteId
        })
    }

    pub fn order_request_with_invalid_quote_id() -> Value {
        use oif_types::serde_json::json;
        let user_addr = InteropAddress::from_chain_and_address(1, "0x1234567890123456789012345678901234567890").unwrap();
        
        json!({
            "userAddress": user_addr.to_hex(),
            "quoteId": "non-existent-quote-id"
        })
    }


}