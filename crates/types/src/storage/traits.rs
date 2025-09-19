//! Storage traits for pluggable storage implementations

use crate::{MetricsTimeSeries, Order, RollingMetrics, Solver, StorageResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Generic repository abstraction for basic CRUD operations over an entity type.
///
/// Object-safe variant with `&str` identifiers so it works with trait objects.
#[async_trait]
pub trait Repository<Entity>: Send + Sync {
	async fn create(&self, entity: Entity) -> StorageResult<Entity>;
	async fn get(&self, id: &str) -> StorageResult<Option<Entity>>;
	async fn update(&self, entity: Entity) -> StorageResult<Entity>;
	async fn delete(&self, id: &str) -> StorageResult<bool>;
	async fn count(&self) -> StorageResult<usize>;

	/// List all entities
	async fn list_all(&self) -> StorageResult<Vec<Entity>>;

	/// List entities using offset/limit pagination
	async fn list_paginated(&self, offset: usize, limit: usize) -> StorageResult<Vec<Entity>>;
}

/// Trait for order storage operations (CRUD naming)
#[async_trait]
pub trait OrderStorageTrait: Repository<Order> + Send + Sync {
	/// Get orders with a specific status
	async fn get_by_status(&self, status: crate::OrderStatus) -> StorageResult<Vec<Order>>;
}

/// Trait for solver storage operations (CRUD naming)
#[async_trait]
pub trait SolverStorageTrait: Repository<Solver> + Send + Sync {
	/// Get active solvers only
	async fn get_active(&self) -> StorageResult<Vec<Solver>>;
}

/// Trait for metrics time-series storage operations
#[async_trait]
pub trait MetricsStorageTrait: Send + Sync {
	/// Update or create metrics time-series for a solver
	async fn update_metrics_timeseries(
		&self,
		solver_id: &str,
		timeseries: MetricsTimeSeries,
	) -> StorageResult<()>;

	/// Get metrics time-series for a solver
	async fn get_metrics_timeseries(
		&self,
		solver_id: &str,
	) -> StorageResult<Option<MetricsTimeSeries>>;

	/// Get rolling metrics for a solver for specific time windows
	async fn get_rolling_metrics(&self, solver_id: &str) -> StorageResult<Option<RollingMetrics>>;

	/// Delete metrics time-series for a solver
	async fn delete_metrics_timeseries(&self, solver_id: &str) -> StorageResult<bool>;

	/// Get all solver IDs that have metrics data
	async fn list_solvers_with_metrics(&self) -> StorageResult<Vec<String>>;

	/// Clean up metrics data older than the specified timestamp
	async fn cleanup_old_metrics(&self, older_than: DateTime<Utc>) -> StorageResult<usize>;

	/// Get the total number of metrics time-series records
	async fn count_metrics_timeseries(&self) -> StorageResult<usize>;
}

/// Main storage trait that combines all storage operations
#[async_trait]
pub trait StorageTrait: OrderStorageTrait + SolverStorageTrait + MetricsStorageTrait {
	/// Health check for the storage system
	async fn health_check(&self) -> StorageResult<bool>;

	/// Close the storage connection
	async fn close(&self) -> StorageResult<()>;

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
	async fn create_solver(&self, solver: Solver) -> StorageResult<Solver> {
		<Self as Repository<Solver>>::create(self, solver).await
	}

	/// Update an existing solver
	async fn update_solver(&self, solver: Solver) -> StorageResult<Solver> {
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
	async fn create_order(&self, order: Order) -> StorageResult<Order> {
		<Self as Repository<Order>>::create(self, order).await
	}

	/// Update an existing order
	async fn update_order(&self, order: Order) -> StorageResult<Order> {
		<Self as Repository<Order>>::update(self, order).await
	}

	/// Delete an order by ID
	async fn delete_order(&self, id: &str) -> StorageResult<bool> {
		<Self as Repository<Order>>::delete(self, id).await
	}

	/// Get orders by status
	async fn get_orders_by_status(&self, status: crate::OrderStatus) -> StorageResult<Vec<Order>> {
		<Self as OrderStorageTrait>::get_by_status(self, status).await
	}

	// ===============================
	// Metrics convenience methods
	// ===============================

	/// Update or create metrics time-series for a solver
	async fn update_solver_metrics_timeseries(
		&self,
		solver_id: &str,
		timeseries: MetricsTimeSeries,
	) -> StorageResult<()> {
		<Self as MetricsStorageTrait>::update_metrics_timeseries(self, solver_id, timeseries).await
	}

	/// Get metrics time-series for a solver
	async fn get_solver_metrics_timeseries(
		&self,
		solver_id: &str,
	) -> StorageResult<Option<MetricsTimeSeries>> {
		<Self as MetricsStorageTrait>::get_metrics_timeseries(self, solver_id).await
	}

	/// Get rolling metrics for a solver for specific time windows
	async fn get_solver_rolling_metrics(
		&self,
		solver_id: &str,
	) -> StorageResult<Option<RollingMetrics>> {
		<Self as MetricsStorageTrait>::get_rolling_metrics(self, solver_id).await
	}

	/// Delete metrics time-series for a solver
	async fn delete_solver_metrics_timeseries(&self, solver_id: &str) -> StorageResult<bool> {
		<Self as MetricsStorageTrait>::delete_metrics_timeseries(self, solver_id).await
	}

	/// Get all solver IDs that have metrics data
	async fn list_all_solvers_with_metrics(&self) -> StorageResult<Vec<String>> {
		<Self as MetricsStorageTrait>::list_solvers_with_metrics(self).await
	}

	/// Clean up metrics data older than the specified timestamp
	async fn cleanup_old_solver_metrics(&self, older_than: DateTime<Utc>) -> StorageResult<usize> {
		<Self as MetricsStorageTrait>::cleanup_old_metrics(self, older_than).await
	}

	/// Get the total number of metrics time-series records
	async fn count_solver_metrics_timeseries(&self) -> StorageResult<usize> {
		<Self as MetricsStorageTrait>::count_metrics_timeseries(self).await
	}
}
