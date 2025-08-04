//! OIF Aggregator Library
//!
//! A high-performance aggregator for Open Intent Framework (OIF) solvers,
//! providing quote aggregation, intent submission, and solver management.

pub mod adapters;
pub mod api;
pub mod auth;
pub mod config;
pub mod models;
pub mod service;
pub mod storage;

pub use adapters::{
    AdapterError, AdapterFactory, AdapterResult, SolverAdapter, oif_adapter::OifAdapter,
};
pub use api::routes::{AppState, create_router};
pub use config::{Settings, load_config};
pub use models::{Intent, Quote, QuoteRequest, Solver, SolverConfig, SolverStatus};
pub use service::aggregator::AggregatorService;
pub use storage::MemoryStore;
// TODO: Replace storage::models with new models module gradually
// pub use models::*;

use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{error, info, warn};

/// Builder pattern for configuring the aggregator
pub struct AggregatorBuilder {
    settings: Option<Settings>,
    storage: MemoryStore,
}

impl AggregatorBuilder {
    /// Create a new aggregator builder
    pub fn new() -> Self {
        Self {
            settings: None,
            storage: MemoryStore::new(),
        }
    }

    /// Create aggregator builder from configuration
    pub fn from_config(settings: Settings) -> Self {
        let storage = MemoryStore::new();

        // Load solvers from configuration
        for (_, solver_config) in &settings.enabled_solvers() {
            let mut solver = Solver::new(
                solver_config.solver_id.clone(),
                solver_config.adapter_id.clone(),
                solver_config.endpoint.clone(),
                solver_config.timeout_ms,
            );

            // Update metadata with configuration details
            solver.metadata.name = Some(solver_config.solver_id.clone());
            solver.metadata.supported_chains = vec![1]; // Default to Ethereum mainnet
            solver.metadata.max_retries = solver_config.max_retries;
            solver.metadata.headers = solver_config.headers.clone();
            solver.status = SolverStatus::Active; // Set to active once initialized
            storage.add_solver(solver);
        }

        Self {
            settings: Some(settings),
            storage,
        }
    }

    /// Add a solver to the aggregator
    pub fn with_solver(self, solver: Solver) -> Self {
        self.storage.add_solver(solver.clone());
        self
    }

    /// Get the current settings
    pub fn settings(&self) -> Option<&Settings> {
        self.settings.as_ref()
    }

    /// Add a storage implementation
    pub fn with_storage(mut self, storage: MemoryStore) -> Self {
        self.storage = storage;
        self
    }

    /// Enable TTL cleanup for quotes (enabled by default)
    pub fn with_ttl_cleanup(mut self, enabled: bool) -> Self {
        if enabled {
            // Create a new storage with TTL enabled and copy existing data
            let new_storage = MemoryStore::with_ttl_enabled(true);
            // Copy existing solvers
            for solver in self.storage.get_all_solvers() {
                new_storage.add_solver(solver);
            }
            self.storage = new_storage;
        }
        self
    }

    /// Start the aggregator and return the configured router with state
    pub async fn start(self) -> Result<(axum::Router, AppState), Box<dyn std::error::Error>> {
        // Initialize the aggregator service
        let settings = self.settings.clone().unwrap_or_default();
        let solvers = self.storage.get_all_solvers();

        let mut aggregator_service = AggregatorService::new(solvers, settings.timeouts.global_ms);

        // Initialize adapters
        aggregator_service.initialize_adapters().await?;

        // Create application state
        let app_state = AppState {
            aggregator_service: Arc::new(aggregator_service),
            storage: self.storage.clone(),
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
        let settings = if self.settings.is_some() {
            info!("Using provided configuration");
            self.settings.take().unwrap()
        } else {
            match load_config() {
                Ok(config) => {
                    info!("Configuration loaded successfully");
                    config
                }
                Err(e) => {
                    error!("Failed to load configuration: {}", e);
                    warn!("Using default configuration");
                    Settings::default()
                }
            }
        };

        // Initialize tracing based on configuration
        Self::init_tracing(&settings);

        info!("Starting OIF Aggregator server");
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
            self = Self::from_config(settings.clone());
        }

        // Create the router using the builder pattern
        let (app, app_state) = self.start().await?;

        // Log application startup info
        let stats = app_state.aggregator_service.get_stats();
        info!(
            "Aggregator service initialized with {} solvers, {} adapters",
            stats.total_solvers, stats.initialized_adapters
        );

        // Start TTL cleanup for quotes
        let _cleanup_handle = app_state.storage.start_ttl_cleanup();
        info!("Started TTL cleanup task for expired quotes");

        // Start the server
        let listener = tokio::net::TcpListener::bind(addr).await?;
        info!("Server listening on {}", addr);
        info!("API endpoints available:");
        info!("  GET  /health");
        info!("  POST /v1/quotes");
        info!("  POST /v1/intents");
        info!("  GET  /v1/intents/{{id}}");

        axum::serve(listener, app).await?;

        Ok(())
    }

    /// Initialize tracing based on configuration settings
    fn init_tracing(settings: &Settings) {
        use tracing_subscriber::{EnvFilter, fmt};

        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(&settings.logging.level));

        match settings.logging.format {
            config::settings::LogFormat::Json => {
                fmt().with_env_filter(filter).json().init();
            }
            config::settings::LogFormat::Pretty => {
                fmt().with_env_filter(filter).pretty().init();
            }
            config::settings::LogFormat::Compact => {
                fmt().with_env_filter(filter).compact().init();
            }
        }
    }
}

impl Default for AggregatorBuilder {
    fn default() -> Self {
        Self::new()
    }
}
