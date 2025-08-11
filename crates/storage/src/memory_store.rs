//! In-memory storage implementation using DashMap with TTL support

use crate::traits::{OrderStorage, QuoteStorage, SolverStorage, Storage, StorageResult};
use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;
use oif_types::{AdapterConfig, Order, Quote, Solver};
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{debug, info};

/// In-memory storage for solvers, quotes, and intents with TTL support
#[derive(Clone)]
pub struct MemoryStore {
	pub solvers: Arc<DashMap<String, Solver>>,
	pub quotes: Arc<DashMap<String, Quote>>,
	pub intents: Arc<DashMap<String, Order>>,
	pub adapters: Arc<DashMap<String, AdapterConfig>>,
	pub quote_ttl_enabled: bool,
}

impl MemoryStore {
	/// Create a new memory store instance
	pub fn new() -> Self {
		Self {
			solvers: Arc::new(DashMap::new()),
			quotes: Arc::new(DashMap::new()),
			intents: Arc::new(DashMap::new()),
			adapters: Arc::new(DashMap::new()),
			quote_ttl_enabled: true,
		}
	}

	/// Create a new memory store with TTL configuration
	pub fn with_ttl_enabled(ttl_enabled: bool) -> Self {
		Self {
			solvers: Arc::new(DashMap::new()),
			quotes: Arc::new(DashMap::new()),
			intents: Arc::new(DashMap::new()),
			adapters: Arc::new(DashMap::new()),
			quote_ttl_enabled: ttl_enabled,
		}
	}

	/// Start the TTL cleanup task for expired quotes
	pub fn start_ttl_cleanup(&self) -> tokio::task::JoinHandle<()> {
		if !self.quote_ttl_enabled {
			return tokio::spawn(async {});
		}

		let quotes = Arc::clone(&self.quotes);
		tokio::spawn(async move {
			let mut cleanup_interval = interval(Duration::from_secs(60)); // Check every minute

			loop {
				cleanup_interval.tick().await;

				let mut expired_quotes = Vec::new();
				let now = Utc::now();

				// Collect expired quote IDs
				for entry in quotes.iter() {
					if entry.value().expires_at <= now {
						expired_quotes.push(entry.key().clone());
					}
				}

				// Remove expired quotes
				if !expired_quotes.is_empty() {
					debug!("Cleaning up {} expired quotes", expired_quotes.len());
					for quote_id in expired_quotes {
						quotes.remove(&quote_id);
					}
				}
			}
		})
	}

	/// Get all non-expired quotes
	pub fn get_all_quotes(&self) -> Vec<Quote> {
		if self.quote_ttl_enabled {
			self.quotes
				.iter()
				.filter_map(|entry| {
					let quote = entry.value();
					if !quote.is_expired() {
						Some(quote.clone())
					} else {
						None
					}
				})
				.collect()
		} else {
			self.quotes.iter().map(|entry| entry.clone()).collect()
		}
	}

	/// Get quotes by request ID
	pub fn get_quotes_by_request(&self, request_id: &str) -> Vec<Quote> {
		self.quotes
			.iter()
			.filter_map(|entry| {
				let quote = entry.value();
				if quote.request_id == request_id
					&& (!self.quote_ttl_enabled || !quote.is_expired())
				{
					Some(quote.clone())
				} else {
					None
				}
			})
			.collect()
	}

	/// Remove expired quotes manually
	pub fn cleanup_expired_quotes(&self) -> usize {
		if !self.quote_ttl_enabled {
			return 0;
		}

		let mut expired_count = 0;
		let now = Utc::now();
		let mut to_remove = Vec::new();

		for entry in self.quotes.iter() {
			if entry.value().expires_at <= now {
				to_remove.push(entry.key().clone());
			}
		}

		for quote_id in to_remove {
			self.quotes.remove(&quote_id);
			expired_count += 1;
		}

		if expired_count > 0 {
			info!("Cleaned up {} expired quotes", expired_count);
		}

		expired_count
	}

	/// Update intent status
	pub fn update_intent_status(&self, order_id: &str, status: oif_types::OrderStatus) -> bool {
		if let Some(mut entry) = self.intents.get_mut(order_id) {
			entry.status = status;
			true
		} else {
			false
		}
	}

	/// Get intents by user address
	pub fn get_intents_by_user(&self, user_address: &str) -> Vec<Order> {
		self.intents
			.iter()
			.filter_map(|entry| {
				let intent = entry.value();
				if intent.user_address == user_address {
					Some(intent.clone())
				} else {
					None
				}
			})
			.collect()
	}

	/// Adapter management methods
	/// Add an adapter configuration
	pub fn add_adapter(&self, adapter: AdapterConfig) {
		info!(
			"Adding adapter {} of type {:?}",
			adapter.adapter_id, adapter.adapter_type
		);
		self.adapters.insert(adapter.adapter_id.clone(), adapter);
	}

	/// Get an adapter by ID
	pub fn get_adapter(&self, adapter_id: &str) -> Option<AdapterConfig> {
		self.adapters.get(adapter_id).map(|entry| entry.clone())
	}

	/// Get all enabled adapters
	pub fn get_enabled_adapters(&self) -> Vec<AdapterConfig> {
		self.adapters
			.iter()
			.filter_map(|entry| {
				let adapter = entry.value();
				// Simplified: all adapters are considered enabled
				Some(adapter.clone())
			})
			.collect()
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
		if self.quote_ttl_enabled {
			if let Some(quote) = self.quotes.get(quote_id) {
				if quote.is_expired() {
					self.quotes.remove(quote_id);
					return Ok(None);
				}
				return Ok(Some(quote.clone()));
			}
		}
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
		if self.quote_ttl_enabled {
			Ok(self
				.quotes
				.iter()
				.filter_map(|e| {
					if e.value().is_expired() {
						None
					} else {
						Some(e.value().clone())
					}
				})
				.collect())
		} else {
			Ok(self.quotes.iter().map(|e| e.value().clone()).collect())
		}
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
				if quote.request_id == request_id
					&& (!self.quote_ttl_enabled || !quote.is_expired())
				{
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
				if quote.solver_id == solver_id && (!self.quote_ttl_enabled || !quote.is_expired())
				{
					Some(quote.clone())
				} else {
					None
				}
			})
			.collect();
		Ok(quotes)
	}

	async fn cleanup_expired_quotes(&self) -> StorageResult<usize> {
		if !self.quote_ttl_enabled {
			return Ok(0);
		}

		let now = Utc::now();
		let mut removed_count = 0;

		self.quotes.retain(|_key, quote| {
			if quote.expires_at <= now {
				removed_count += 1;
				debug!("Removed expired quote: {}", quote.quote_id);
				false
			} else {
				true
			}
		});

		if removed_count > 0 {
			info!("Cleaned up {} expired quotes", removed_count);
		}

		Ok(removed_count)
	}
}

#[async_trait]
impl oif_types::storage::Repository<Order> for MemoryStore {
	async fn create(&self, order: Order) -> StorageResult<()> {
		self.intents.insert(order.order_id.clone(), order);
		Ok(())
	}

	async fn get(&self, order_id: &str) -> StorageResult<Option<Order>> {
		Ok(self.intents.get(order_id).map(|i| i.clone()))
	}

	async fn update(&self, order: Order) -> StorageResult<()> {
		self.intents.insert(order.order_id.clone(), order);
		Ok(())
	}

	async fn delete(&self, order_id: &str) -> StorageResult<bool> {
		Ok(self.intents.remove(order_id).is_some())
	}

	async fn count(&self) -> StorageResult<usize> {
		Ok(self.intents.len())
	}

	async fn list_all(&self) -> StorageResult<Vec<Order>> {
		Ok(self.intents.iter().map(|e| e.value().clone()).collect())
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
			.intents
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
			.intents
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

	async fn start_background_tasks(&self) -> StorageResult<()> {
		self.start_ttl_cleanup();
		Ok(())
	}
}
