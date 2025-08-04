//! Adapter module for solver integrations

use crate::models::adapters::{AdapterConfig, AdapterType};
use crate::models::{Intent, IntentResponse, Quote, QuoteRequest};
use async_trait::async_trait;
use std::collections::HashMap;
use thiserror::Error;

/// Custom error types for adapter operations
#[derive(Error, Debug)]
pub enum AdapterError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("Timeout occurred after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },
    #[error("Invalid response format: {0}")]
    InvalidResponse(String),
    #[error("Solver error: {code} - {message}")]
    SolverError { code: String, message: String },
    #[error("Adapter not found: {adapter_id}")]
    AdapterNotFound { adapter_id: String },
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// Result type for adapter operations
pub type AdapterResult<T> = Result<T, AdapterError>;

/// Trait for solver adapters
#[async_trait]
pub trait SolverAdapter: Send + Sync + std::fmt::Debug {
    /// Get adapter information
    fn adapter_info(&self) -> &AdapterConfig;

    /// Get a quote from the solver
    async fn get_quote(&self, request: QuoteRequest) -> AdapterResult<Quote>;

    /// Submit an intent to the solver
    async fn submit_intent(&self, intent: Intent) -> AdapterResult<IntentResponse>;

    /// Health check for the solver
    async fn health_check(&self) -> AdapterResult<bool>;

    /// Get supported chains for this adapter
    fn supported_chains(&self) -> &[u64];
}

/// Factory for creating solver adapters
pub struct AdapterFactory {
    adapters: HashMap<String, Box<dyn SolverAdapter>>,
}

impl AdapterFactory {
    /// Create a new adapter factory
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
        }
    }

    /// Register an adapter
    pub fn register(&mut self, adapter_id: String, adapter: Box<dyn SolverAdapter>) {
        self.adapters.insert(adapter_id, adapter);
    }

    /// Get an adapter by ID
    pub fn get(&self, adapter_id: &str) -> Option<&dyn SolverAdapter> {
        self.adapters.get(adapter_id).map(|a| a.as_ref())
    }

    /// Create adapter from configuration
    pub fn create_from_config(config: &AdapterConfig) -> AdapterResult<Box<dyn SolverAdapter>> {
        match config.adapter_type {
            AdapterType::OifV1 => Ok(Box::new(oif_adapter::OifAdapter::from_config(config)?)),
            AdapterType::UniswapV2 => Err(AdapterError::ConfigError(
                "UniswapV2 adapter not implemented".to_string(),
            )),
            AdapterType::UniswapV3 => Err(AdapterError::ConfigError(
                "UniswapV3 adapter not implemented".to_string(),
            )),
            AdapterType::OneInch => Err(AdapterError::ConfigError(
                "OneInch adapter not implemented".to_string(),
            )),
            AdapterType::Paraswap => Err(AdapterError::ConfigError(
                "Paraswap adapter not implemented".to_string(),
            )),
            AdapterType::Lifi => Ok(Box::new(oif_adapter::LifiAdapter::from_config(config)?)),
            AdapterType::Cowswap => Err(AdapterError::ConfigError(
                "Cowswap adapter not implemented".to_string(),
            )),
            AdapterType::ZeroX => Err(AdapterError::ConfigError(
                "0x adapter not implemented".to_string(),
            )),
            AdapterType::Kyber => Err(AdapterError::ConfigError(
                "Kyber adapter not implemented".to_string(),
            )),
            AdapterType::SushiSwap => Err(AdapterError::ConfigError(
                "SushiSwap adapter not implemented".to_string(),
            )),
            AdapterType::Balancer => Err(AdapterError::ConfigError(
                "Balancer adapter not implemented".to_string(),
            )),
            AdapterType::Curve => Err(AdapterError::ConfigError(
                "Curve adapter not implemented".to_string(),
            )),
            AdapterType::Custom => Err(AdapterError::ConfigError(
                "Custom adapter requires manual implementation".to_string(),
            )),
        }
    }

    /// Get all registered adapters
    pub fn get_all(&self) -> &HashMap<String, Box<dyn SolverAdapter>> {
        &self.adapters
    }
}

impl Default for AdapterFactory {
    fn default() -> Self {
        Self::new()
    }
}

pub mod lifi_adapter;
pub mod oif_adapter;
