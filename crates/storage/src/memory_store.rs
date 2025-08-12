//! In-memory storage implementation using DashMap

use crate::traits::{OrderStorage, QuoteStorage, SolverStorage, Storage, StorageResult};
use async_trait::async_trait;
use dashmap::DashMap;
use oif_types::{Order, Quote, Solver};
use std::sync::Arc;

/// In-memory storage for solvers, quotes, and orders
#[derive(Clone)]
pub struct MemoryStore {
	pub solvers: Arc<DashMap<String, Solver>>,
	pub quotes: Arc<DashMap<String, Quote>>,
	pub orders: Arc<DashMap<String, Order>>,
}

impl MemoryStore {
	/// Create a new memory store instance
	pub fn new() -> Self {
		Self {
			solvers: Arc::new(DashMap::new()),
			quotes: Arc::new(DashMap::new()),
			orders: Arc::new(DashMap::new()),
		}
	}
}

/// Storage statistics
impl Default for MemoryStore {
	fn default() -> Self {
		Self::new()
	}
}

// Trait implementations for pluggable storage

#[async_trait]
impl oif_types::storage::Repository<Quote> for MemoryStore {
	async fn create(&self, quote: Quote) -> StorageResult<()> {
		self.quotes.insert(quote.quote_id.clone(), quote);
		Ok(())
	}

	async fn get(&self, quote_id: &str) -> StorageResult<Option<Quote>> {
		Ok(self.quotes.get(quote_id).map(|q| q.clone()))
	}

	async fn delete(&self, quote_id: &str) -> StorageResult<bool> {
		Ok(self.quotes.remove(quote_id).is_some())
	}
	async fn update(&self, quote: Quote) -> StorageResult<()> {
		self.quotes.insert(quote.quote_id.clone(), quote);
		Ok(())
	}

	async fn count(&self) -> StorageResult<usize> {
		Ok(self.quotes.len())
	}

	async fn list_all(&self) -> StorageResult<Vec<Quote>> {
		Ok(self.quotes.iter().map(|e| e.value().clone()).collect())
	}

	async fn list_paginated(&self, offset: usize, limit: usize) -> StorageResult<Vec<Quote>> {
		let all = self.list_all().await?;
		let start = offset.min(all.len());
		let end = (start + limit).min(all.len());
		Ok(all[start..end].to_vec())
	}
}

#[async_trait]
impl QuoteStorage for MemoryStore {
	async fn get_quotes_by_request(&self, request_id: &str) -> StorageResult<Vec<Quote>> {
		let quotes: Vec<Quote> = self
			.quotes
			.iter()
			.filter_map(|entry| {
				let quote = entry.value();
				if quote.request_id == request_id {
					Some(quote.clone())
				} else {
					None
				}
			})
			.collect();
		Ok(quotes)
	}

	async fn get_quotes_by_solver(&self, solver_id: &str) -> StorageResult<Vec<Quote>> {
		let quotes: Vec<Quote> = self
			.quotes
			.iter()
			.filter_map(|entry| {
				let quote = entry.value();
				if quote.solver_id == solver_id {
					Some(quote.clone())
				} else {
					None
				}
			})
			.collect();
		Ok(quotes)
	}
}

#[async_trait]
impl oif_types::storage::Repository<Order> for MemoryStore {
	async fn create(&self, order: Order) -> StorageResult<()> {
		self.orders.insert(order.order_id.clone(), order);
		Ok(())
	}

	async fn get(&self, order_id: &str) -> StorageResult<Option<Order>> {
		Ok(self.orders.get(order_id).map(|i| i.clone()))
	}

	async fn update(&self, order: Order) -> StorageResult<()> {
		self.orders.insert(order.order_id.clone(), order);
		Ok(())
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
	async fn get_by_user(&self, user_address: &str) -> StorageResult<Vec<Order>> {
		let orders: Vec<Order> = self
			.orders
			.iter()
			.filter_map(|entry| {
				let order = entry.value();
				if order.user_address == user_address {
					Some(order.clone())
				} else {
					None
				}
			})
			.collect();
		Ok(orders)
	}

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
impl oif_types::storage::Repository<Solver> for MemoryStore {
	async fn create(&self, solver: Solver) -> StorageResult<()> {
		self.solvers.insert(solver.solver_id.clone(), solver);
		Ok(())
	}

	async fn get(&self, solver_id: &str) -> StorageResult<Option<Solver>> {
		Ok(self.solvers.get(solver_id).map(|s| s.clone()))
	}

	async fn update(&self, solver: Solver) -> StorageResult<()> {
		self.solvers.insert(solver.solver_id.clone(), solver);
		Ok(())
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
