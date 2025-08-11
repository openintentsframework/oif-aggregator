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
}
