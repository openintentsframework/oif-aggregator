//! In-memory storage implementation using DashMap with TTL support

use crate::models::{Intent, Quote};
use crate::models::{Solver, adapters::AdapterConfig};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::time::{Duration, interval};
use tracing::{debug, info, warn};

/// In-memory storage for solvers, quotes, and intents with TTL support
#[derive(Clone)]
pub struct MemoryStore {
    pub solvers: Arc<DashMap<String, Solver>>,
    pub quotes: Arc<DashMap<String, Quote>>,
    pub intents: Arc<DashMap<String, Intent>>,
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

    /// Add a solver to the store
    pub fn add_solver(&self, solver: Solver) {
        self.solvers.insert(solver.solver_id.clone(), solver);
    }

    /// Get a solver by ID
    pub fn get_solver(&self, solver_id: &str) -> Option<Solver> {
        self.solvers.get(solver_id).map(|entry| entry.clone())
    }

    /// Add a quote to the store
    pub fn add_quote(&self, quote: Quote) {
        info!(
            "Adding quote {} from solver {}",
            quote.quote_id, quote.solver_id
        );
        self.quotes.insert(quote.quote_id.clone(), quote);
    }

    /// Get a quote by ID, checking for expiration
    pub fn get_quote(&self, quote_id: &str) -> Option<Quote> {
        if let Some(entry) = self.quotes.get(quote_id) {
            let quote = entry.clone();
            if self.quote_ttl_enabled && quote.is_expired() {
                // Remove expired quote
                drop(entry); // Release the read lock
                self.quotes.remove(quote_id);
                warn!("Quote {} has expired and was removed", quote_id);
                return None;
            }
            Some(quote)
        } else {
            None
        }
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

    /// Get all solvers
    pub fn get_all_solvers(&self) -> Vec<Solver> {
        self.solvers.iter().map(|entry| entry.clone()).collect()
    }

    /// Intent management methods
    /// Add an intent to the store
    pub fn add_intent(&self, intent: Intent) {
        info!(
            "Adding intent {} for user {}",
            intent.intent_id, intent.user_address
        );
        self.intents.insert(intent.intent_id.clone(), intent);
    }

    /// Get an intent by ID
    pub fn get_intent(&self, intent_id: &str) -> Option<Intent> {
        self.intents.get(intent_id).map(|entry| entry.clone())
    }

    /// Update intent status
    pub fn update_intent_status(
        &self,
        intent_id: &str,
        status: crate::models::intents::IntentStatus,
    ) -> bool {
        if let Some(mut entry) = self.intents.get_mut(intent_id) {
            entry.status = status;
            true
        } else {
            false
        }
    }

    /// Get intents by user address
    pub fn get_intents_by_user(&self, user_address: &str) -> Vec<Intent> {
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
                if adapter.enabled {
                    Some(adapter.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Storage statistics
    pub fn get_stats(&self) -> StorageStats {
        StorageStats {
            total_solvers: self.solvers.len(),
            total_quotes: self.quotes.len(),
            total_intents: self.intents.len(),
            total_adapters: self.adapters.len(),
            active_quotes: self.get_all_quotes().len(),
        }
    }
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub total_solvers: usize,
    pub total_quotes: usize,
    pub total_intents: usize,
    pub total_adapters: usize,
    pub active_quotes: usize,
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}
