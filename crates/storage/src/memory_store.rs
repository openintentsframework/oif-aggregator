//! In-memory storage implementation using DashMap

use crate::traits::{MetricsStorage, OrderStorage, SolverStorage, Storage, StorageResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use oif_types::{storage::Repository, MetricsTimeSeries, Order, RollingMetrics, Solver};
use std::sync::Arc;

/// In-memory storage for solvers, quotes, and orders
///
/// Performance optimization: MetricsTimeSeries is wrapped in Arc to avoid expensive
/// cloning of large time-series data structures. This significantly improves performance
/// for read operations while only cloning when the trait interface requires owned values.
#[derive(Clone)]
pub struct MemoryStore {
	pub solvers: Arc<DashMap<String, Solver>>,
	pub orders: Arc<DashMap<String, Order>>,
	pub metrics_timeseries: Arc<DashMap<String, Arc<MetricsTimeSeries>>>,
}

impl MemoryStore {
	/// Create a new memory store instance
	pub fn new() -> Self {
		Self {
			solvers: Arc::new(DashMap::new()),
			orders: Arc::new(DashMap::new()),
			metrics_timeseries: Arc::new(DashMap::new()),
		}
	}

	/// Get Arc<MetricsTimeSeries> for efficient access without cloning
	/// This is a more efficient alternative to get_metrics_timeseries when you don't need ownership
	pub async fn get_metrics_timeseries_arc(
		&self,
		solver_id: &str,
	) -> StorageResult<Option<Arc<MetricsTimeSeries>>> {
		Ok(self
			.metrics_timeseries
			.get(solver_id)
			.map(|ts_arc| ts_arc.value().clone()))
	}

	/// Check if metrics exist without loading/cloning the data
	pub async fn has_metrics_timeseries(&self, solver_id: &str) -> StorageResult<bool> {
		Ok(self.metrics_timeseries.contains_key(solver_id))
	}
}

/// Storage statistics
impl Default for MemoryStore {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl Repository<Order> for MemoryStore {
	async fn create(&self, order: Order) -> StorageResult<Order> {
		self.orders.insert(order.order_id.clone(), order.clone());
		Ok(order)
	}

	async fn get(&self, order_id: &str) -> StorageResult<Option<Order>> {
		Ok(self.orders.get(order_id).map(|i| i.clone()))
	}

	async fn update(&self, order: Order) -> StorageResult<Order> {
		self.orders.insert(order.order_id.clone(), order.clone());
		Ok(order)
	}

	async fn delete(&self, order_id: &str) -> StorageResult<bool> {
		Ok(self.orders.remove(order_id).is_some())
	}

	async fn count(&self) -> StorageResult<usize> {
		Ok(self.orders.len())
	}

	async fn list_all(&self) -> StorageResult<Vec<Order>> {
		Ok(self.orders.iter().map(|e| e.value().clone()).collect())
	}

	async fn list_paginated(&self, offset: usize, limit: usize) -> StorageResult<Vec<Order>> {
		let all = self.list_all().await?;
		let start = offset.min(all.len());
		let end = (start + limit).min(all.len());
		Ok(all[start..end].to_vec())
	}
}

#[async_trait]
impl OrderStorage for MemoryStore {
	async fn get_by_status(&self, status: oif_types::OrderStatus) -> StorageResult<Vec<Order>> {
		let orders: Vec<Order> = self
			.orders
			.iter()
			.filter_map(|entry| {
				let order = entry.value();
				if order.status == status {
					Some(order.clone())
				} else {
					None
				}
			})
			.collect();
		Ok(orders)
	}
}

#[async_trait]
impl Repository<Solver> for MemoryStore {
	async fn create(&self, solver: Solver) -> StorageResult<Solver> {
		self.solvers
			.insert(solver.solver_id.clone(), solver.clone());
		Ok(solver)
	}

	async fn get(&self, solver_id: &str) -> StorageResult<Option<Solver>> {
		Ok(self.solvers.get(solver_id).map(|s| s.clone()))
	}

	async fn update(&self, solver: Solver) -> StorageResult<Solver> {
		self.solvers
			.insert(solver.solver_id.clone(), solver.clone());
		Ok(solver)
	}

	async fn delete(&self, solver_id: &str) -> StorageResult<bool> {
		Ok(self.solvers.remove(solver_id).is_some())
	}

	async fn count(&self) -> StorageResult<usize> {
		Ok(self.solvers.len())
	}

	async fn list_all(&self) -> StorageResult<Vec<Solver>> {
		Ok(self.solvers.iter().map(|e| e.value().clone()).collect())
	}

	async fn list_paginated(&self, offset: usize, limit: usize) -> StorageResult<Vec<Solver>> {
		let all = self.list_all().await?;
		let start = offset.min(all.len());
		let end = (start + limit).min(all.len());
		Ok(all[start..end].to_vec())
	}
}

#[async_trait]
impl SolverStorage for MemoryStore {
	async fn get_active(&self) -> StorageResult<Vec<Solver>> {
		use oif_types::SolverStatus;
		let solvers: Vec<Solver> = self
			.solvers
			.iter()
			.filter_map(|entry| {
				let solver = entry.value();
				if solver.status == SolverStatus::Active {
					Some(solver.clone())
				} else {
					None
				}
			})
			.collect();
		Ok(solvers)
	}
}

#[async_trait]
impl MetricsStorage for MemoryStore {
	async fn update_metrics_timeseries(
		&self,
		solver_id: &str,
		timeseries: MetricsTimeSeries,
	) -> StorageResult<()> {
		self.metrics_timeseries
			.insert(solver_id.to_string(), Arc::new(timeseries));
		Ok(())
	}

	async fn get_metrics_timeseries(
		&self,
		solver_id: &str,
	) -> StorageResult<Option<MetricsTimeSeries>> {
		// Clone Arc contents only when trait interface requires owned value
		Ok(self.metrics_timeseries.get(solver_id).map(|ts_arc| {
			let ts: &MetricsTimeSeries = ts_arc.value();
			ts.clone()
		}))
	}

	async fn get_rolling_metrics(&self, solver_id: &str) -> StorageResult<Option<RollingMetrics>> {
		// More efficient: clone only the small RollingMetrics, not the entire timeseries
		Ok(self
			.metrics_timeseries
			.get(solver_id)
			.map(|ts_arc| ts_arc.value().rolling_metrics.clone()))
	}

	async fn delete_metrics_timeseries(&self, solver_id: &str) -> StorageResult<bool> {
		Ok(self.metrics_timeseries.remove(solver_id).is_some())
	}

	async fn list_solvers_with_metrics(&self) -> StorageResult<Vec<String>> {
		Ok(self
			.metrics_timeseries
			.iter()
			.map(|entry| entry.key().clone())
			.collect())
	}

	async fn cleanup_old_metrics(&self, older_than: DateTime<Utc>) -> StorageResult<usize> {
		let mut removed_count = 0;

		// Get a list of keys to remove (to avoid borrowing issues)
		// More efficient: only access last_updated field, no cloning needed
		let keys_to_remove: Vec<String> = self
			.metrics_timeseries
			.iter()
			.filter_map(|entry| {
				let timeseries_arc = entry.value();
				if timeseries_arc.last_updated < older_than {
					Some(entry.key().clone())
				} else {
					None
				}
			})
			.collect();

		// Remove the identified keys
		for key in keys_to_remove {
			if self.metrics_timeseries.remove(&key).is_some() {
				removed_count += 1;
			}
		}

		Ok(removed_count)
	}

	async fn count_metrics_timeseries(&self) -> StorageResult<usize> {
		Ok(self.metrics_timeseries.len())
	}
}

#[async_trait]
impl Storage for MemoryStore {
	async fn health_check(&self) -> StorageResult<bool> {
		// For in-memory storage, just check if the maps are accessible
		Ok(true)
	}

	async fn close(&self) -> StorageResult<()> {
		// For memory store, there's nothing to close
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use oif_types::MetricsTimeSeries;

	#[tokio::test]
	async fn test_arc_optimization_efficiency() {
		let store = MemoryStore::new();

		// Create a test timeseries
		let timeseries = MetricsTimeSeries::new("test-solver".to_string());

		// Store it
		store
			.update_metrics_timeseries("test-solver", timeseries)
			.await
			.unwrap();

		// Test efficient Arc access (no cloning of large data)
		let arc_result = store
			.get_metrics_timeseries_arc("test-solver")
			.await
			.unwrap();
		assert!(arc_result.is_some());

		// Test that we can get multiple Arc references efficiently
		let arc_result2 = store
			.get_metrics_timeseries_arc("test-solver")
			.await
			.unwrap();
		assert!(arc_result2.is_some());

		// Both should reference the same underlying data
		// (in a real scenario, this avoids expensive clones)

		// Test existence check without loading data
		assert!(store.has_metrics_timeseries("test-solver").await.unwrap());
		assert!(!store.has_metrics_timeseries("nonexistent").await.unwrap());
	}
}
