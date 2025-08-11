//! Comprehensive example demonstrating the new trait-based architecture
//!
//! This example shows how to:
//! 1. Implement custom authentication
//! 2. Implement custom storage
//! 3. Implement custom solver adapters
//! 4. Wire everything together using the AggregatorBuilder

use async_trait::async_trait;
use oif_aggregator::models::{AuthError, AuthenticationResult};
use oif_aggregator::*;
use tokio;

// Example of custom authentication implementation
#[derive(Debug)]
struct CustomJwtAuthenticator {
	secret: String,
}

#[async_trait]
impl Authenticator for CustomJwtAuthenticator {
	async fn authenticate(&self, request: &AuthRequest) -> AuthenticationResult {
		// In a real implementation, you would:
		// 1. Extract JWT from Authorization header
		// 2. Verify signature with self.secret
		// 3. Parse claims and create AuthContext

		if let Some(auth_header) = request.get_authorization() {
			if auth_header.starts_with("Bearer valid-jwt-token") {
				let context = AuthContext::new("user-from-jwt".to_string())
					.with_role("premium_user".to_string())
					.with_permission(Permission::ReadQuotes)
					.with_permission(Permission::SubmitOrders)
					.with_rate_limits(RateLimits {
						requests_per_minute: 200, // Premium users get higher limits
						burst_size: 20,
						custom_windows: vec![],
					});
				return AuthenticationResult::Authorized(context);
			}
		}

		AuthenticationResult::Unauthorized("Invalid JWT token".to_string())
	}

	async fn authorize(&self, context: &AuthContext, permission: &Permission) -> bool {
		context.has_permission(permission) || context.has_role("admin")
	}

	fn get_rate_limits(&self, context: &AuthContext) -> Option<RateLimits> {
		context.rate_limits.clone()
	}

	async fn health_check(&self) -> Result<bool, AuthError> {
		Ok(true)
	}

	fn name(&self) -> &str {
		"CustomJwtAuthenticator"
	}
}

// Example of custom storage implementation
#[derive(Debug, Clone)]
struct CustomPostgresStorage {
	// In real implementation, this would be a connection pool
	connection_info: String,
}

#[async_trait]
impl QuoteStorage for CustomPostgresStorage {
	async fn get_quotes_by_request(&self, request_id: &str) -> StorageResult<Vec<Quote>> {
		println!("Fetching quotes for request {} from PostgreSQL", request_id);
		Ok(vec![])
	}

	async fn get_quotes_by_solver(&self, solver_id: &str) -> StorageResult<Vec<Quote>> {
		println!("Fetching quotes for solver {} from PostgreSQL", solver_id);
		Ok(vec![])
	}

	async fn cleanup_expired_quotes(&self) -> StorageResult<usize> {
		println!("Cleaning up expired quotes in PostgreSQL");
		Ok(0)
	}
}

#[async_trait]
impl oif_aggregator::models::storage::Repository<Quote> for CustomPostgresStorage {
	async fn create(&self, quote: Quote) -> StorageResult<()> {
		// In real implementation: INSERT INTO quotes ...
		println!("Storing quote {} in PostgreSQL", quote.quote_id);
		Ok(())
	}

	async fn get(&self, quote_id: &str) -> StorageResult<Option<Quote>> {
		// In real implementation: SELECT * FROM quotes WHERE id = ?
		println!("Fetching quote {} from PostgreSQL", quote_id);
		Ok(None) // Not found for this demo
	}

	async fn delete(&self, quote_id: &str) -> StorageResult<bool> {
		println!("Removing quote {} from PostgreSQL", quote_id);
		Ok(true)
	}
	async fn update(&self, _quote: Quote) -> StorageResult<()> {
		Ok(())
	}
	async fn count(&self) -> StorageResult<usize> {
		Ok(0)
	}
	async fn list_all(&self) -> StorageResult<Vec<Quote>> {
		Ok(vec![])
	}
	async fn list_paginated(&self, _offset: usize, _limit: usize) -> StorageResult<Vec<Quote>> {
		Ok(vec![])
	}
}

#[async_trait]
impl OrderStorage for CustomPostgresStorage {
	async fn get_by_user(&self, user_address: &str) -> StorageResult<Vec<Order>> {
		println!("Fetching orders for user {} from PostgreSQL", user_address);
		Ok(vec![])
	}

	async fn get_by_status(&self, status: OrderStatus) -> StorageResult<Vec<Order>> {
		println!("Fetching orders with status {:?} from PostgreSQL", status);
		Ok(vec![])
	}
}

#[async_trait]
impl oif_aggregator::models::storage::Repository<Order> for CustomPostgresStorage {
	async fn create(&self, order: Order) -> StorageResult<()> {
		println!("Storing order {} in PostgreSQL", order.order_id);
		Ok(())
	}

	async fn get(&self, order_id: &str) -> StorageResult<Option<Order>> {
		println!("Fetching order {} from PostgreSQL", order_id);
		Ok(None)
	}

	async fn update(&self, order: Order) -> StorageResult<()> {
		println!("Updating order {} in PostgreSQL", order.order_id);
		Ok(())
	}
	async fn delete(&self, order_id: &str) -> StorageResult<bool> {
		println!("Removing order {} from PostgreSQL", order_id);
		Ok(true)
	}
	async fn count(&self) -> StorageResult<usize> {
		Ok(0)
	}
	async fn list_all(&self) -> StorageResult<Vec<Order>> {
		Ok(vec![])
	}
	async fn list_paginated(&self, _offset: usize, _limit: usize) -> StorageResult<Vec<Order>> {
		Ok(vec![])
	}
}

#[async_trait]
impl SolverStorage for CustomPostgresStorage {
	async fn get_active(&self) -> StorageResult<Vec<Solver>> {
		println!("Fetching active solvers from PostgreSQL");
		Ok(vec![])
	}
}

#[async_trait]
impl oif_aggregator::models::storage::Repository<Solver> for CustomPostgresStorage {
	async fn create(&self, solver: Solver) -> StorageResult<()> {
		println!("Storing solver {} in PostgreSQL", solver.solver_id);
		Ok(())
	}

	async fn get(&self, solver_id: &str) -> StorageResult<Option<Solver>> {
		println!("Fetching solver {} from PostgreSQL", solver_id);
		Ok(None)
	}

	async fn update(&self, solver: Solver) -> StorageResult<()> {
		println!("Updating solver {} in PostgreSQL", solver.solver_id);
		Ok(())
	}
	async fn delete(&self, solver_id: &str) -> StorageResult<bool> {
		println!("Removing solver {} from PostgreSQL", solver_id);
		Ok(true)
	}
	async fn count(&self) -> StorageResult<usize> {
		Ok(0)
	}
	async fn list_all(&self) -> StorageResult<Vec<Solver>> {
		Ok(vec![])
	}
	async fn list_paginated(&self, _offset: usize, _limit: usize) -> StorageResult<Vec<Solver>> {
		Ok(vec![])
	}
}

#[async_trait]
impl Storage for CustomPostgresStorage {
	async fn health_check(&self) -> StorageResult<bool> {
		println!("PostgreSQL health check passed");
		Ok(true)
	}

	async fn close(&self) -> StorageResult<()> {
		println!("Closing PostgreSQL connections");
		Ok(())
	}

	async fn start_background_tasks(&self) -> StorageResult<()> {
		println!("Starting PostgreSQL background tasks");
		Ok(())
	}
}

// Example of custom solver adapter
#[derive(Debug)]
struct CustomUniswapAdapter {
	config: AdapterConfig,
	client: reqwest::Client,
	endpoint: String,
}

impl CustomUniswapAdapter {
	fn new() -> Self {
		Self {
			config: AdapterConfig {
				adapter_id: "custom-uniswap".to_string(),
				adapter_type: AdapterType::OifV1,
				name: "Custom Uniswap V3 Adapter".to_string(),
				description: Some("Custom implementation for Uniswap V3".to_string()),
				version: "1.0.0".to_string(),
			},
			client: reqwest::Client::new(),
			endpoint: "https://api.uniswap.org/v3".to_string(),
		}
	}
}

#[async_trait]
impl SolverAdapter for CustomUniswapAdapter {
	fn adapter_info(&self) -> &AdapterConfig {
		&self.config
	}

	async fn get_quote(&self, request: &QuoteRequest) -> AdapterResult<Quote> {
		println!(
			"Getting quote from Uniswap V3 for {}/{}",
			request.token_in.clone(),
			request.token_out.clone()
		);

		// In real implementation:
		// 1. Call Uniswap V3 quoter contract
		// 2. Calculate optimal route
		// 3. Estimate gas costs
		// 4. Return structured quote

		let quote = Quote::new(
			"custom-uniswap".to_string(),
			request.request_id.clone(),
			request.token_in.clone(),
			request.token_out.clone(),
			request.amount_in.clone(),
			"2500000000".to_string(), // Mock output amount
			request.chain_id,
		)
		.with_estimated_gas(120000)
		.with_price_impact(0.01)
		.with_response_time(150);

		Ok(quote)
	}

	async fn submit_order(&self, order: &Order) -> AdapterResult<String> {
		println!("Submitting order {} to Uniswap V3", order.order_id);

		// In real implementation:
		// 1. Build swap transaction
		// 2. Submit to mempool
		// 3. Return transaction hash

		Ok("0x1234567890abcdef".to_string())
	}

	async fn health_check(&self) -> AdapterResult<bool> {
		println!("Uniswap V3 adapter health check");
		Ok(true)
	}

	fn supported_chains(&self) -> &[u64] {
		const CHAINS: &[u64] = &[1, 137, 42161];
		CHAINS
	}
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Initialize tracing
	tracing_subscriber::fmt()
		.with_max_level(tracing::Level::INFO)
		.init();

	println!("🚀 Trait Architecture Demo");
	println!("========================\n");

	// 1. Create custom implementations
	let custom_auth = CustomJwtAuthenticator {
		secret: "super-secret-key".to_string(),
	};

	let custom_storage = CustomPostgresStorage {
		connection_info: "postgresql://localhost:5432/aggregator".to_string(),
	};

	let redis_rate_limiter = MemoryRateLimiter::new(); // Could be RedisRateLimiter

	println!("✅ Created custom implementations:");
	println!("   - Auth: {}", custom_auth.name());
	println!("   - Storage: PostgreSQL");
	println!("   - Rate Limiter: {}\n", redis_rate_limiter.name());

	// 2. Test individual components
	println!("🧪 Testing components:");

	// Test auth
	let auth_request = AuthRequest::new("POST".to_string(), "/v1/orders".to_string()).with_header(
		"authorization".to_string(),
		"Bearer valid-jwt-token".to_string(),
	);

	match custom_auth.authenticate(&auth_request).await {
		AuthenticationResult::Authorized(context) => {
			println!("   ✅ Auth successful: user_id={}", context.user_id);
		},
		AuthenticationResult::Unauthorized(reason) => {
			println!("   ❌ Auth failed: {}", reason);
		},
		AuthenticationResult::Bypassed => {
			println!("   ⏭️  Auth bypassed");
		},
	}

	// Test storage
	if custom_storage.health_check().await.is_ok() {
		println!("   ✅ Storage health check passed");
	}

	// Test rate limiter
	if redis_rate_limiter.health_check().await.is_ok() {
		println!("   ✅ Rate limiter health check passed");
	}

	println!();

	// 3. Build aggregator with custom components
	println!("🏗️  Building aggregator with custom traits...");

	let builder = AggregatorBuilder::with_storage(custom_storage)
		.with_auth_and_limiter(custom_auth, redis_rate_limiter);

	println!("   ✅ AggregatorBuilder configured with:");
	println!("      - Custom JWT authentication");
	println!("      - Custom PostgreSQL storage");
	println!("      - Memory-based rate limiting");

	// Note: In a real application, you would call:
	// builder.start_server().await?;

	println!("\n🎉 Trait architecture demo completed!");
	println!("\n💡 Key Benefits:");
	println!("   - Pluggable authentication (JWT, OAuth, API keys, etc.)");
	println!("   - Pluggable storage (PostgreSQL, Redis, MongoDB, etc.)");
	println!("   - Pluggable rate limiting (Redis, Memory, etc.)");
	println!("   - Type-safe builder pattern");
	println!("   - Easy testing with mock implementations");

	Ok(())
}
