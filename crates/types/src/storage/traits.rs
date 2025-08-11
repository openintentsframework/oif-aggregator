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

/// Generic repository abstraction for basic CRUD operations over an entity type.
///
/// Object-safe variant with `&str` identifiers so it works with trait objects.
#[async_trait]
pub trait Repository<Entity>: Send + Sync {
	async fn create(&self, entity: Entity) -> StorageResult<()>;
	async fn get(&self, id: &str) -> StorageResult<Option<Entity>>;
	async fn update(&self, entity: Entity) -> StorageResult<()>;
	async fn delete(&self, id: &str) -> StorageResult<bool>;
	async fn count(&self) -> StorageResult<usize>;

	/// List all entities
	async fn list_all(&self) -> StorageResult<Vec<Entity>>;

	/// List entities using offset/limit pagination
	async fn list_paginated(&self, offset: usize, limit: usize) -> StorageResult<Vec<Entity>>;
}

/// Trait for quote storage operations (CRUD naming)
#[async_trait]
pub trait QuoteStorageTrait: Repository<Quote> + Send + Sync {
	/// Get all quotes for a request
	async fn get_quotes_by_request(&self, request_id: &str) -> StorageResult<Vec<Quote>>;

	/// Get all quotes from a solver
	async fn get_quotes_by_solver(&self, solver_id: &str) -> StorageResult<Vec<Quote>>;

	/// Remove expired quotes
	async fn cleanup_expired_quotes(&self) -> StorageResult<usize>;
}

/// Trait for order storage operations (CRUD naming)
#[async_trait]
pub trait OrderStorageTrait: Repository<Order> + Send + Sync {
	/// Get all orders for a user
	async fn get_by_user(&self, user_address: &str) -> StorageResult<Vec<Order>>;

	/// Get orders with a specific status
	async fn get_by_status(&self, status: crate::OrderStatus) -> StorageResult<Vec<Order>>;
}

/// Trait for solver storage operations (CRUD naming)
#[async_trait]
pub trait SolverStorageTrait: Repository<Solver> + Send + Sync {
	/// Get active solvers only
	async fn get_active(&self) -> StorageResult<Vec<Solver>>;
}

/// Main storage trait that combines all storage operations
#[async_trait]
pub trait StorageTrait: QuoteStorageTrait + OrderStorageTrait + SolverStorageTrait {
	/// Health check for the storage system
	async fn health_check(&self) -> StorageResult<bool>;

	/// Close the storage connection
	async fn close(&self) -> StorageResult<()>;

	/// Start any background tasks associated with the storage implementation (e.g., TTL cleanup).
	/// Default implementation does nothing.
	async fn start_background_tasks(&self) -> StorageResult<()> {
		Ok(())
	}

	// ===============================
	// Solver convenience methods
	// ===============================

	/// List all solvers
	async fn list_all_solvers(&self) -> StorageResult<Vec<Solver>> {
		<Self as Repository<Solver>>::list_all(self).await
	}

	/// List solvers with pagination
	async fn list_solvers_paginated(
		&self,
		offset: usize,
		limit: usize,
	) -> StorageResult<Vec<Solver>> {
		<Self as Repository<Solver>>::list_paginated(self, offset, limit).await
	}

	/// Count total solvers
	async fn count_solvers(&self) -> StorageResult<usize> {
		<Self as Repository<Solver>>::count(self).await
	}

	/// Get a specific solver by ID
	async fn get_solver(&self, id: &str) -> StorageResult<Option<Solver>> {
		<Self as Repository<Solver>>::get(self, id).await
	}

	/// Create a new solver
	async fn create_solver(&self, solver: Solver) -> StorageResult<()> {
		<Self as Repository<Solver>>::create(self, solver).await
	}

	/// Update an existing solver
	async fn update_solver(&self, solver: Solver) -> StorageResult<()> {
		<Self as Repository<Solver>>::update(self, solver).await
	}

	/// Delete a solver by ID
	async fn delete_solver(&self, id: &str) -> StorageResult<bool> {
		<Self as Repository<Solver>>::delete(self, id).await
	}

	/// Get active solvers only
	async fn get_active_solvers(&self) -> StorageResult<Vec<Solver>> {
		<Self as SolverStorageTrait>::get_active(self).await
	}

	// ===============================
	// Order convenience methods
	// ===============================

	/// List all orders
	async fn list_all_orders(&self) -> StorageResult<Vec<Order>> {
		<Self as Repository<Order>>::list_all(self).await
	}

	/// List orders with pagination
	async fn list_orders_paginated(
		&self,
		offset: usize,
		limit: usize,
	) -> StorageResult<Vec<Order>> {
		<Self as Repository<Order>>::list_paginated(self, offset, limit).await
	}

	/// Count total orders
	async fn count_orders(&self) -> StorageResult<usize> {
		<Self as Repository<Order>>::count(self).await
	}

	/// Get a specific order by ID
	async fn get_order(&self, id: &str) -> StorageResult<Option<Order>> {
		<Self as Repository<Order>>::get(self, id).await
	}

	/// Create a new order
	async fn create_order(&self, order: Order) -> StorageResult<()> {
		<Self as Repository<Order>>::create(self, order).await
	}

	/// Update an existing order
	async fn update_order(&self, order: Order) -> StorageResult<()> {
		<Self as Repository<Order>>::update(self, order).await
	}

	/// Delete an order by ID
	async fn delete_order(&self, id: &str) -> StorageResult<bool> {
		<Self as Repository<Order>>::delete(self, id).await
	}

	/// Get orders by user address
	async fn get_orders_by_user(&self, user_address: &str) -> StorageResult<Vec<Order>> {
		<Self as OrderStorageTrait>::get_by_user(self, user_address).await
	}

	/// Get orders by status
	async fn get_orders_by_status(&self, status: crate::OrderStatus) -> StorageResult<Vec<Order>> {
		<Self as OrderStorageTrait>::get_by_status(self, status).await
	}

	// ===============================
	// Quote convenience methods
	// ===============================

	/// List all quotes
	async fn list_all_quotes(&self) -> StorageResult<Vec<Quote>> {
		<Self as Repository<Quote>>::list_all(self).await
	}

	/// List quotes with pagination
	async fn list_quotes_paginated(
		&self,
		offset: usize,
		limit: usize,
	) -> StorageResult<Vec<Quote>> {
		<Self as Repository<Quote>>::list_paginated(self, offset, limit).await
	}

	/// Count total quotes
	async fn count_quotes(&self) -> StorageResult<usize> {
		<Self as Repository<Quote>>::count(self).await
	}

	/// Get a specific quote by ID
	async fn get_quote(&self, id: &str) -> StorageResult<Option<Quote>> {
		<Self as Repository<Quote>>::get(self, id).await
	}

	/// Create a new quote
	async fn create_quote(&self, quote: Quote) -> StorageResult<()> {
		<Self as Repository<Quote>>::create(self, quote).await
	}

	/// Update an existing quote
	async fn update_quote(&self, quote: Quote) -> StorageResult<()> {
		<Self as Repository<Quote>>::update(self, quote).await
	}

	/// Delete a quote by ID
	async fn delete_quote(&self, id: &str) -> StorageResult<bool> {
		<Self as Repository<Quote>>::delete(self, id).await
	}

	/// Get quotes by request ID
	async fn get_quotes_by_request(&self, request_id: &str) -> StorageResult<Vec<Quote>> {
		<Self as QuoteStorageTrait>::get_quotes_by_request(self, request_id).await
	}

	/// Get quotes by solver ID
	async fn get_quotes_by_solver(&self, solver_id: &str) -> StorageResult<Vec<Quote>> {
		<Self as QuoteStorageTrait>::get_quotes_by_solver(self, solver_id).await
	}

	/// Clean up expired quotes
	async fn cleanup_expired_quotes(&self) -> StorageResult<usize> {
		<Self as QuoteStorageTrait>::cleanup_expired_quotes(self).await
	}
}
