//! Storage traits for pluggable storage implementations

use crate::{Order, Quote, Solver};
use async_trait::async_trait;
use thiserror::Error;

/// Storage error type
#[derive(Debug, Error)]
pub enum StorageError {
	#[error("Item not found: {id}")]
	NotFound { id: String },
	#[error("Connection error: {message}")]
	Connection { message: String },
	#[error("Serialization error: {message}")]
	Serialization { message: String },
	#[error("Storage operation failed: {message}")]
	Operation { message: String },
}

pub type StorageResult<T> = Result<T, StorageError>;

/// Statistics about storage usage
#[derive(Debug, Clone)]
pub struct StorageStats {
	pub total_quotes: usize,
	pub active_quotes: usize,
	pub total_orders: usize,
	pub total_solvers: usize,
}

/// Trait for quote storage operations
#[async_trait]
pub trait QuoteStorageTrait: Send + Sync {
	/// Add a new quote to storage
	async fn add_quote(&self, quote: Quote) -> StorageResult<()>;

	/// Get a quote by ID
	async fn get_quote(&self, quote_id: &str) -> StorageResult<Option<Quote>>;

	/// Remove a quote by ID
	async fn remove_quote(&self, quote_id: &str) -> StorageResult<bool>;

	/// Get all quotes for a request
	async fn get_quotes_by_request(&self, request_id: &str) -> StorageResult<Vec<Quote>>;

	/// Get all quotes from a solver
	async fn get_quotes_by_solver(&self, solver_id: &str) -> StorageResult<Vec<Quote>>;

	/// Remove expired quotes
	async fn cleanup_expired_quotes(&self) -> StorageResult<usize>;

	/// Get quotes statistics
	async fn quote_stats(&self) -> StorageResult<(usize, usize)>; // (total, active)
}

/// Trait for order storage operations
#[async_trait]
pub trait OrderStorageTrait: Send + Sync {
	/// Add a new intent to storage
	async fn add_order(&self, order: Order) -> StorageResult<()>;

	/// Get an order by ID
	async fn get_order(&self, order_id: &str) -> StorageResult<Option<Order>>;

	/// Update an existing order
	async fn update_order(&self, order: Order) -> StorageResult<()>;

	/// Get all orders for a user
	async fn get_orders_by_user(&self, user_address: &str) -> StorageResult<Vec<Order>>;

	/// Get orders with a specific status
	async fn get_orders_by_status(&self, status: crate::OrderStatus) -> StorageResult<Vec<Order>>;

	/// Remove an order by ID
	async fn remove_order(&self, order_id: &str) -> StorageResult<bool>;

	/// Get order count
	async fn order_count(&self) -> StorageResult<usize>;
}

/// Trait for solver storage operations
#[async_trait]
pub trait SolverStorageTrait: Send + Sync {
	/// Add a new solver to storage
	async fn add_solver(&self, solver: Solver) -> StorageResult<()>;

	/// Get a solver by ID
	async fn get_solver(&self, solver_id: &str) -> StorageResult<Option<Solver>>;

	/// Update an existing solver
	async fn update_solver(&self, solver: Solver) -> StorageResult<()>;

	/// Get all solvers
	async fn get_all_solvers(&self) -> StorageResult<Vec<Solver>>;

	/// Get active solvers only
	async fn get_active_solvers(&self) -> StorageResult<Vec<Solver>>;

	/// Remove a solver by ID
	async fn remove_solver(&self, solver_id: &str) -> StorageResult<bool>;

	/// Get solver count
	async fn solver_count(&self) -> StorageResult<usize>;
}

/// Main storage trait that combines all storage operations
#[async_trait]
pub trait StorageTrait: QuoteStorageTrait + OrderStorageTrait + SolverStorageTrait {
	/// Health check for the storage system
	async fn health_check(&self) -> StorageResult<bool>;

	/// Get overall storage statistics
	async fn stats(&self) -> StorageResult<StorageStats>;

	/// Close the storage connection
	async fn close(&self) -> StorageResult<()>;

	/// Start any background tasks associated with the storage implementation (e.g., TTL cleanup).
	/// Default implementation does nothing.
	async fn start_background_tasks(&self) -> StorageResult<()> {
		Ok(())
	}
}
