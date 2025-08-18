//! OIF Aggregator Library
//!
//! A high-performance aggregator for Open Intent Framework (OIF) solvers,
//! providing quote aggregation, intent submission, and solver management.

use oif_service::{
	AggregatorTrait, IntegrityService, IntegrityTrait, OrderService, OrderServiceTrait,
	SolverService, SolverServiceTrait,
};
use oif_types::{Asset, Network};

// Core domain types - the most commonly used types
pub use oif_types::{
	chrono,
	// External dependencies for convenience
	serde_json,
	// Core types
	Adapter,
	AdapterError,

	AuthContext,
	AuthRequest,
	// Auth traits
	Authenticator,
	Order,
	OrderError,
	OrderRequest,
	OrderResponse,
	OrderStatus,
	Permission,
	// Primary domain entities
	Quote,
	// Error types
	QuoteError,
	QuoteRequest,
	QuoteResponse,
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
	OrderServiceError,
	SolverServiceError,
	SolverStats,
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

// API layer
pub use oif_api::{create_router, AppState};
// Re-export auth implementations for convenience
pub use oif_api::auth::{ApiKeyAuthenticator, MemoryRateLimiter, NoAuthenticator};

// Adapters
pub use oif_adapters::{AdapterRegistry, AdapterResult, SolverAdapter};

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
	pub mod routes {
		pub use oif_api::{create_router, AppState};
	}
}

pub mod service {
	pub use oif_service::*;
}

pub mod mocks;

use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

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
	adapter_registry: Option<oif_adapters::AdapterRegistry>,
	solvers: Vec<Solver>,
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
			adapter_registry: None,
			solvers: Vec::new(),
		}
	}
}

// Default constructor using MemoryStore for convenience
impl Default for AggregatorBuilder<MemoryStore, NoAuthenticator, MemoryRateLimiter> {
	fn default() -> Self {
		Self::new()
	}
}

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
	/// Upsert solvers defined in Settings into storage so that start() can
	/// load them via `list_all_solvers()`.
	async fn upsert_solvers_from_settings(&self, settings: &Settings) -> Result<(), String> {
		let mut errors = Vec::new();

		for solver_config in settings.enabled_solvers().values() {
			let mut solver = Solver::new(
				solver_config.solver_id.clone(),
				solver_config.adapter_id.clone(),
				solver_config.endpoint.clone(),
				solver_config.timeout_ms,
			);

			// Populate metadata
			solver.metadata.name = solver_config
				.name
				.clone()
				.or_else(|| Some(solver_config.solver_id.clone()));
			solver.metadata.supported_networks = vec![];
			solver.metadata.max_retries = solver_config.max_retries;
			solver.metadata.headers = solver_config.headers.clone();
			if let Some(desc) = &solver_config.description {
				solver.metadata.description = Some(desc.clone());
			}
			if let Some(networks) = &solver_config.supported_networks {
				solver.metadata.supported_networks = networks
					.iter()
					.map(|n| Network::new(n.chain_id, n.name.clone(), n.is_testnet))
					.collect();
			}
			if let Some(assets) = &solver_config.supported_assets {
				solver.metadata.supported_assets = assets
					.iter()
					.map(|a| {
						Asset::new(
							a.address.clone(),
							a.symbol.clone(),
							a.name.clone(),
							a.decimals,
							a.chain_id,
						)
					})
					.collect();
			}
			solver.status = SolverStatus::Active;

			// Validate solver before creating
			if let Err(validation_error) = solver.validate() {
				errors.push(format!(
					"Solver '{}' validation failed: {}",
					solver.solver_id, validation_error
				));
				continue;
			}

			// Try to create solver in storage
			if let Err(storage_error) = self.storage.create_solver(solver.clone()).await {
				errors.push(format!(
					"Failed to create solver '{}': {}",
					solver.solver_id, storage_error
				));
			}
		}

		if !errors.is_empty() {
			return Err(format!(
				"Configuration errors found:\n{}",
				errors.join("\n")
			));
		}

		Ok(())
	}

	/// Upsert collected solvers into storage
	async fn upsert_collected_solvers(&self) -> Result<(), String> {
		let mut errors = Vec::new();

		for solver in &self.solvers {
			// Validate solver before creating
			if let Err(validation_error) = solver.validate() {
				errors.push(format!(
					"Solver '{}' validation failed: {}",
					solver.solver_id, validation_error
				));
				continue;
			}

			// Try to create solver in storage
			if let Err(storage_error) = self.storage.create_solver(solver.clone()).await {
				errors.push(format!(
					"Failed to create solver '{}': {}",
					solver.solver_id, storage_error
				));
			}
		}

		if !errors.is_empty() {
			return Err(format!("Solver creation errors:\n{}", errors.join("\n")));
		}

		Ok(())
	}

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
			adapter_registry: self.adapter_registry,
			solvers: self.solvers,
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
			adapter_registry: self.adapter_registry,
			solvers: self.solvers,
		}
	}

	/// Register a custom adapter (uses adapter's own ID)
	/// Panics if adapter registration fails (this is intentional for startup-time configuration errors)
	pub fn with_adapter(mut self, adapter: Box<dyn SolverAdapter>) -> Self {
		let mut registry = self
			.adapter_registry
			.unwrap_or_else(oif_adapters::AdapterRegistry::with_defaults);
		registry.register(adapter).expect(
			"Failed to register adapter during startup - this is a fatal configuration error",
		);
		self.adapter_registry = Some(registry);
		self
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
		for solver_config in settings.enabled_solvers().values() {
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
			adapter_registry: None,
			solvers: Vec::new(),
		}
	}
	/// Add a solver to the aggregator
	pub fn with_solver(mut self, solver: Solver) -> Self {
		self.solvers.push(solver);
		self
	}

	/// Set custom settings
	pub fn with_settings(mut self, settings: Settings) -> Self {
		self.settings = Some(settings);
		self
	}

	/// Get the current settings
	pub fn settings(&self) -> Option<&Settings> {
		self.settings.as_ref()
	}

	/// Initialize tracing with configuration-based settings
	fn init_tracing_from_settings(
		&self,
		settings: &Settings,
	) -> Result<(), Box<dyn std::error::Error>> {
		use oif_config::settings::LogFormat;

		// Create env filter using config level or environment variable
		let log_level = &settings.logging.level;
		let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
			.unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level));

		// Initialize tracing with the configuration
		match settings.logging.format {
			LogFormat::Json => {
				let subscriber = tracing_subscriber::fmt().json().with_env_filter(env_filter);

				if settings.logging.structured {
					subscriber.with_target(true).with_thread_ids(true).init();
				} else {
					subscriber.init();
				}
			},
			LogFormat::Pretty => {
				let subscriber = tracing_subscriber::fmt()
					.pretty()
					.with_env_filter(env_filter);

				if settings.logging.structured {
					subscriber.with_target(true).with_thread_ids(true).init();
				} else {
					subscriber.init();
				}
			},
			LogFormat::Compact => {
				let subscriber = tracing_subscriber::fmt()
					.compact()
					.with_env_filter(env_filter);

				if settings.logging.structured {
					subscriber.with_target(true).with_thread_ids(true).init();
				} else {
					subscriber.init();
				}
			},
		}

		info!(
			"Logging configuration applied: level={}, format={:?}, structured={}",
			settings.logging.level, settings.logging.format, settings.logging.structured
		);

		Ok(())
	}

	/// Start the aggregator and return the configured router with state
	pub async fn start(self) -> Result<(axum::Router, AppState), Box<dyn std::error::Error>> {
		// Initialize the aggregator service
		let settings = self.settings.clone().unwrap_or_default();
		// Upsert solvers from settings into storage first - fail on any configuration errors
		self.upsert_solvers_from_settings(&settings).await?;
		// Upsert collected solvers from with_solver() calls - fail on any validation/creation errors
		self.upsert_collected_solvers().await?;
		// Use base repository listing for solvers
		let solvers = self
			.storage
			.list_all_solvers()
			.await
			.map_err(|e| format!("Failed to get solvers: {}", e))?;

		info!("Successfully initialized with {} solver(s)", solvers.len());

		// Use custom factory or create with defaults
		let adapter_registry = Arc::new(
			self.adapter_registry
				.unwrap_or_else(oif_adapters::AdapterRegistry::with_defaults),
		);

		// Get integrity secret wrapped in SecretString for secure handling
		let integrity_secret = settings
			.get_integrity_secret_secure()
		.map_err(|e| {
			format!(
				"Failed to resolve integrity secret: {}. Please set the INTEGRITY_SECRET environment variable with a secure random string (minimum 32 characters).",
				e
			)
			})?;
		let integrity_service =
			Arc::new(IntegrityService::new(integrity_secret)) as Arc<dyn IntegrityTrait>;
		let aggregator_service = AggregatorService::new(
			solvers.clone(),
			Arc::clone(&adapter_registry),
			settings.timeouts.global_ms,
			Arc::clone(&integrity_service),
		);

		// Validate that all solvers have matching adapters
		aggregator_service
			.validate_solvers()
			.map_err(|e| format!("Solver validation failed: {}", e))?;

		// Create application state
		let storage_arc: Arc<dyn Storage> = Arc::new(self.storage.clone());
		let app_state = AppState {
			aggregator_service: Arc::new(aggregator_service)
				as Arc<dyn oif_service::AggregatorTrait>,
			order_service: Arc::new(OrderService::new(
				Arc::clone(&storage_arc),
				Arc::clone(&adapter_registry),
				Arc::clone(&integrity_service),
			)) as Arc<dyn OrderServiceTrait>,
			solver_service: Arc::new(SolverService::new(
				Arc::clone(&storage_arc),
				Arc::clone(&adapter_registry),
			)) as Arc<dyn SolverServiceTrait>,
			storage: storage_arc,
			integrity_service,
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

		// Use provided settings or load from config with defaults
		let using_provided_settings = self.settings.is_some();
		let settings = if using_provided_settings {
			self.settings.take().unwrap()
		} else {
			load_config().unwrap_or_default()
		};

		// Initialize tracing with configuration-based settings
		self.init_tracing_from_settings(&settings)?;

		// Log comprehensive service startup information
		log_service_info();

		info!(
			"Using configuration: loaded from {}",
			if using_provided_settings {
				"provided settings"
			} else {
				"config file or defaults"
			}
		);
		info!("Configuration loaded successfully");

		info!("🔧 Configuring OIF Aggregator server");
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
		let (app, _) = self.start().await?;

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
		info!("  GET  /v1/solvers");
		info!("  GET  /v1/solvers/{{id}}");
		if cfg!(feature = "openapi") {
			info!("  GET  /swagger-ui");
			info!("  GET  /api-docs/openapi.json");
		}

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
