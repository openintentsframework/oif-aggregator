//! OIF Aggregator Library
//!
//! A high-performance aggregator for Open Intent Framework (OIF) solvers,
//! providing quote aggregation, intent submission, and solver management.

// Core domain types - the most commonly used types
pub use oif_types::{
	chrono,
	// External dependencies for convenience
	serde_json,
	// Core types
	Adapter,
	AdapterConfig,
	AdapterError,

	AuthContext,
	AuthRequest,
	// Auth traits
	Authenticator,
	Order,
	OrderError,
	OrderResponse,
	OrderStatus,
	OrdersRequest,
	Permission,
	// Primary domain entities
	Quote,
	// Error types
	QuoteError,
	QuoteRequest,
	QuoteResponse,
	QuoteStatus,
	RateLimiter,
	RateLimits,
	Solver,
	SolverConfig,
	SolverError,
	SolverStatus,
};

// Service layer
pub use oif_service::{
	AggregatorService,
	OrderService,
	OrderServiceError,
	SolverService,
	SolverServiceError,
	// Keep the full module for more advanced usage
};

// Storage layer
pub use oif_storage::{
	traits::{OrderStorage, QuoteStorage, SolverStorage, StorageError, StorageResult},
	MemoryStore, Storage,
};

// Storage traits module for advanced usage
pub mod traits {
	pub use oif_storage::traits::*;
}

// Conditional re-exports based on features
#[cfg(feature = "redis")]
pub use oif_storage::RedisStore;

// API layer
pub use oif_api::{create_router, AppState};
// Re-export auth implementations for convenience
pub use oif_api::auth::{ApiKeyAuthenticator, MemoryRateLimiter, NoAuthenticator};

// Adapters
pub use oif_adapters::{AdapterFactory, AdapterResult, SolverAdapter};

// Config
pub use oif_config::{load_config, log_service_info, log_startup_complete, Settings};

// Module aliases for backward compatibility
pub mod models {
	pub use oif_types::*;
}

pub mod storage {
	pub use oif_storage::*;
}

pub mod config {
	pub use oif_config::*;
}

pub mod adapters {
	pub use oif_adapters::*;
}

pub mod api {
	pub use oif_api::*;
	// Back-compat shim for older imports oif_aggregator::api::routes::{create_router, AppState}
	pub mod routes {
		pub use oif_api::{create_router, AppState};
	}
}

pub mod service {
	pub use oif_service::*;
}

use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{error, info, warn};

// Re-export external dependencies for examples
pub use async_trait;
pub use reqwest;

/// Builder pattern for configuring the aggregator
pub struct AggregatorBuilder<S = MemoryStore, A = NoAuthenticator, R = MemoryRateLimiter>
where
	S: Storage + 'static,
	A: Authenticator + 'static,
	R: RateLimiter + 'static,
{
	settings: Option<Settings>,
	storage: S,
	authenticator: A,
	rate_limiter: R,
	custom_adapter_factory: Option<oif_adapters::AdapterFactory>,
}

impl<S> AggregatorBuilder<S, NoAuthenticator, MemoryRateLimiter>
where
	S: Storage + Clone + 'static,
{
	/// Create a new aggregator builder with the provided storage
	pub fn with_storage(storage: S) -> Self {
		Self {
			settings: None,
			storage,
			authenticator: NoAuthenticator,
			rate_limiter: MemoryRateLimiter::new(),
			custom_adapter_factory: None,
		}
	}
}

// Default constructor using MemoryStore for convenience
impl AggregatorBuilder<MemoryStore, NoAuthenticator, MemoryRateLimiter> {
	/// Create a new aggregator builder with default memory storage
	pub fn new() -> Self {
		Self::with_storage(MemoryStore::new())
	}

	/// Create aggregator builder from configuration using default memory storage
	pub async fn from_config(settings: Settings) -> Self {
		Self::from_config_with_storage(settings, MemoryStore::new()).await
	}
}

impl<S, A, R> AggregatorBuilder<S, A, R>
where
	S: Storage + Clone + 'static,
	A: Authenticator + 'static,
	R: RateLimiter + 'static,
{
	/// Set custom authenticator
	pub fn with_auth<NewA>(self, authenticator: NewA) -> AggregatorBuilder<S, NewA, R>
	where
		NewA: Authenticator + 'static,
	{
		AggregatorBuilder {
			settings: self.settings,
			storage: self.storage,
			authenticator,
			rate_limiter: self.rate_limiter,
			custom_adapter_factory: self.custom_adapter_factory,
		}
	}

	/// Set custom rate limiter
	pub fn with_rate_limiter<NewR>(self, rate_limiter: NewR) -> AggregatorBuilder<S, A, NewR>
	where
		NewR: RateLimiter + 'static,
	{
		AggregatorBuilder {
			settings: self.settings,
			storage: self.storage,
			authenticator: self.authenticator,
			rate_limiter,
			custom_adapter_factory: self.custom_adapter_factory,
		}
	}

	/// Register a custom adapter (uses adapter's own ID)
	pub fn with_adapter(mut self, adapter: Box<dyn SolverAdapter>) -> Result<Self, String> {
		let mut factory = self
			.custom_adapter_factory
			.unwrap_or_else(|| oif_adapters::AdapterFactory::with_defaults());
		factory.register(adapter)?;
		self.custom_adapter_factory = Some(factory);
		Ok(self)
	}
}

impl<S, A, R> AggregatorBuilder<S, A, R>
where
	S: Storage + Clone + 'static,
	A: Authenticator + 'static,
	R: RateLimiter + 'static,
{
	/// Create aggregator builder from configuration with any storage
	pub async fn from_config_with_storage(
		settings: Settings,
		storage: S,
	) -> AggregatorBuilder<S, NoAuthenticator, MemoryRateLimiter> {
		// Load solvers from configuration into the provided storage
		for (_, solver_config) in &settings.enabled_solvers() {
			let mut solver = Solver::new(
				solver_config.solver_id.clone(),
				solver_config.adapter_id.clone(),
				solver_config.endpoint.clone(),
				solver_config.timeout_ms,
			);

			// Update metadata with configuration details
			solver.metadata.name = Some(solver_config.solver_id.clone());
			solver.metadata.supported_networks = vec![];
			solver.metadata.max_retries = solver_config.max_retries;
			solver.metadata.headers = solver_config.headers.clone();
			solver.status = SolverStatus::Active;
			storage
				.create_solver(solver)
				.await
				.expect("Failed to add solver to storage");
		}

		AggregatorBuilder {
			settings: Some(settings),
			storage,
			authenticator: NoAuthenticator,
			rate_limiter: MemoryRateLimiter::new(),
			custom_adapter_factory: None,
		}
	}
	/// Add a solver to the aggregator
	pub async fn with_solver(self, solver: Solver) -> Self {
		self.storage
			.create_solver(solver.clone())
			.await
			.expect("Failed to add solver");
		self
	}

	/// Get the current settings
	pub fn settings(&self) -> Option<&Settings> {
		self.settings.as_ref()
	}

	/// Start the aggregator and return the configured router with state
	pub async fn start(self) -> Result<(axum::Router, AppState), Box<dyn std::error::Error>> {
		// Initialize the aggregator service
		let settings = self.settings.clone().unwrap_or_default();
		// Use base repository listing for solvers
		let solvers = self
			.storage
			.list_all_solvers()
			.await
			.map_err(|e| format!("Failed to get solvers: {}", e))?;

		// Use custom factory or create with defaults
		let adapter_factory = Arc::new(
			self.custom_adapter_factory
				.unwrap_or_else(|| oif_adapters::AdapterFactory::with_defaults()),
		);

		// Create aggregator service
		let aggregator_service = AggregatorService::new(
			solvers.clone(),
			Arc::clone(&adapter_factory),
			settings.timeouts.global_ms,
		);

		// Validate that all solvers have matching adapters
		aggregator_service
			.validate_solvers()
			.map_err(|e| format!("Solver validation failed: {}", e))?;

		// Create solver HashMap for OrderService
		let solver_map: std::collections::HashMap<String, Solver> = solvers
			.into_iter()
			.map(|solver| (solver.solver_id.clone(), solver))
			.collect();

		// Create application state
		let storage_arc: Arc<dyn Storage> = Arc::new(self.storage.clone());
		let app_state = AppState {
			aggregator_service: Arc::new(aggregator_service),
			order_service: Arc::new(OrderService::new(
				Arc::clone(&storage_arc),
				Arc::clone(&adapter_factory),
				solver_map,
			)),
			solver_service: Arc::new(SolverService::new(Arc::clone(&storage_arc))),
			storage: storage_arc,
		};

		// Create router with state
		let router = create_router().with_state(app_state.clone());

		Ok((router, app_state))
	}

	/// Start the complete server with all defaults and setup
	/// This method handles everything needed to run the server, including:
	/// - Loading .env file
	/// - Loading configuration with defaults
	/// - Initializing tracing
	/// - Starting TTL cleanup
	/// - Binding and serving the application
	pub async fn start_server(mut self) -> Result<(), Box<dyn std::error::Error>> {
		// Load .env file if it exists
		dotenvy::dotenv().ok();

		// Initialize basic tracing early so we can see startup logs
		// This can be overridden later based on configuration
		// Use try_init() to avoid panic if already initialized
		let _ = tracing_subscriber::fmt()
			.with_env_filter(
				tracing_subscriber::EnvFilter::try_from_default_env()
					.unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
			)
			.try_init();

		// Log comprehensive service startup information
		log_service_info();

		// Use provided settings or load from config with defaults
		let settings = if self.settings.is_some() {
			info!("Using provided configuration");
			self.settings.take().unwrap()
		} else {
			match load_config() {
				Ok(config) => {
					info!("Configuration loaded successfully");
					config
				},
				Err(e) => {
					error!("Failed to load configuration: {}", e);
					warn!("Using default configuration");
					Settings::default()
				},
			}
		};

		// Note: Basic tracing was already initialized above to capture startup logs
		// Advanced tracing configuration based on settings could be added here if needed

		info!("🔧 Configuring OIF Aggregator server");
		info!("Environment: {:?}", settings.environment.profile);
		info!("Debug mode: {}", settings.is_debug());

		// Log enabled solvers
		let enabled_solvers = settings.enabled_solvers();
		info!("Enabled solvers: {}", enabled_solvers.len());
		for (id, solver) in &enabled_solvers {
			info!(
				"  - {}: {} ({}ms timeout)",
				id, solver.endpoint, solver.timeout_ms
			);
		}

		// Parse bind address
		let bind_addr = settings.bind_address();
		let addr: SocketAddr = bind_addr
			.parse()
			.map_err(|e| format!("Invalid bind address '{}': {}", bind_addr, e))?;

		// Ensure we have proper configuration in the builder
		if self.settings.is_none() {
			self.settings = Some(settings.clone());
		}

		// Create the router using the builder pattern
		let (app, app_state) = self.start().await?;

		// Log application startup info
		let stats = app_state.aggregator_service.get_stats();
		info!(
			"Aggregator service initialized with {} solvers, {} adapters",
			stats.total_solvers, stats.initialized_adapters
		);

		// TTL cleanup is storage-specific and should be handled by the storage implementation
		info!("Storage backend initialized successfully");

		// Start the server
		let listener = tokio::net::TcpListener::bind(addr).await?;

		// Log startup completion with comprehensive information
		log_startup_complete(&bind_addr);
		info!("API endpoints available:");
		info!("  GET  /health");
		info!("  POST /v1/quotes");
		info!("  POST /v1/orders");
		info!("  GET  /v1/orders/{{id}}");
		info!("  GET /v1/solvers/");
		info!("  GET  /v1/solvers/{{id}}");

		// Apply global rate limiting based on settings at the make_service level
		let rate_cfg = &settings.environment.rate_limiting;
		if rate_cfg.enabled {
			use std::time::Duration;
			use tower::limit::RateLimitLayer;
			use tower::ServiceBuilder;
			let make_svc = ServiceBuilder::new()
				.layer(RateLimitLayer::new(
					rate_cfg.requests_per_minute as u64,
					Duration::from_secs(60),
				))
				.service(app.into_make_service());
			axum::serve(listener, make_svc).await?;
		} else {
			axum::serve(listener, app).await?;
		}

		Ok(())
	}
}
