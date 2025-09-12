//! HTTP client cache for optimized connection management
//!
//! Provides per-solver client instances with connection pooling and keep-alive optimization.

use dashmap::DashMap;
use oif_types::{AdapterError, AdapterResult, SecretString, SolverRuntimeConfig};
use reqwest::{Client, ClientBuilder};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, warn};

/// Configuration for creating optimized HTTP clients
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClientConfig {
	/// Base endpoint for the solver
	pub base_url: String,
	/// Solver identifier for cache differentiation
	pub solver_id: String,
	/// Maximum number of idle connections per host
	pub max_idle_per_host: usize,
	/// Connection keep-alive timeout
	pub keep_alive_timeout_ms: u64,
	/// Additional headers (for auth, etc.)
	pub headers: Vec<(String, String)>,
}

impl From<&SolverRuntimeConfig> for ClientConfig {
	fn from(solver_config: &SolverRuntimeConfig) -> Self {
		let mut headers = vec![
			("User-Agent".to_string(), "OIF-Aggregator/1.0".to_string()),
			("Content-Type".to_string(), "application/json".to_string()),
			("X-Adapter-Type".to_string(), "OIF-v1".to_string()),
		];

		// Add headers from solver config
		if let Some(solver_headers) = &solver_config.headers {
			for (key, value) in solver_headers {
				headers.push((key.clone(), value.clone()));
			}
		}

		Self {
			base_url: solver_config.endpoint.clone(),
			solver_id: solver_config.solver_id.clone(),
			max_idle_per_host: 10,         // Default: 10 idle connections per host
			keep_alive_timeout_ms: 90_000, // Default: 90 seconds keep-alive
			headers,
		}
	}
}

/// Authentication configuration for HTTP clients
#[derive(Debug, Clone)]
pub enum AuthConfig {
	/// No authentication
	None,
	/// Bearer token authentication (JWT, OAuth2, etc.)
	Bearer { token: SecretString },
	/// API Key authentication with custom header
	ApiKey { header: String, key: SecretString },
	/// Custom authentication with multiple headers and cache key
	Custom {
		headers: Vec<(String, String)>,
		cache_key: Option<String>,
	},
}

impl AuthConfig {
	/// Create JWT Bearer authentication
	pub fn jwt(token: Option<&str>) -> Self {
		match token {
			Some(t) => Self::Bearer {
				token: SecretString::from(t),
			},
			None => Self::None,
		}
	}

	/// Create API Key authentication
	pub fn api_key(header: &str, key: &str) -> Self {
		Self::ApiKey {
			header: header.to_string(),
			key: SecretString::from(key),
		}
	}
}

/// Cached client with creation timestamp for TTL management
#[derive(Debug, Clone)]
struct CachedClient {
	client: Arc<Client>,
	created_at: Instant,
}

impl CachedClient {
	fn new(client: Client) -> Self {
		Self {
			client: Arc::new(client),
			created_at: Instant::now(),
		}
	}

	fn is_expired(&self, ttl: Duration) -> bool {
		self.created_at.elapsed() > ttl
	}
}

/// Thread-safe cache for HTTP clients optimized per solver configuration with TTL
#[derive(Clone, Debug)]
pub struct ClientCache {
	clients: Arc<DashMap<ClientConfig, CachedClient>>,
	ttl: Duration,
}

impl ClientCache {
	/// Create a new client cache with default 30-minute TTL
	pub fn new() -> Self {
		Self {
			clients: Arc::new(DashMap::new()),
			ttl: Duration::from_secs(30 * 60), // 30 minutes
		}
	}

	/// Create a new client cache with custom TTL
	pub fn with_ttl(ttl: Duration) -> Self {
		Self {
			clients: Arc::new(DashMap::new()),
			ttl,
		}
	}

	/// Get or create an optimized client for the given configuration
	pub fn get_client(&self, config: &ClientConfig) -> AdapterResult<Arc<Client>> {
		// Atomic check and potential removal of expired client
		self.clients.remove_if(config, |_, cached_client| {
			let is_expired = cached_client.is_expired(self.ttl);
			if is_expired {
				warn!(
					"Client cache expired for {} (age: {:?}), will create new client",
					config.base_url,
					cached_client.created_at.elapsed()
				);
			}
			is_expired
		});

		// Check if we have a valid (non-expired) client
		if let Some(cached_client_ref) = self.clients.get(config) {
			let cached_client = cached_client_ref.value();
			debug!(
				"Reusing cached client for {} (age: {:?})",
				config.base_url,
				cached_client.created_at.elapsed()
			);
			return Ok(cached_client.client.clone());
		}

		// Create new client with optimized settings
		debug!("Creating new optimized client for {}", config.base_url);
		let client = self.create_optimized_client(config)?;
		let cached_client = CachedClient::new(client);
		let client_arc = cached_client.client.clone();

		// Atomic insert using entry API to handle concurrent access
		use dashmap::mapref::entry::Entry;

		match self.clients.entry(config.clone()) {
			Entry::Occupied(entry) => {
				// Another thread beat us to it, use the existing client
				debug!(
					"Another thread created client for {}, using existing",
					config.base_url
				);
				return Ok(entry.get().client.clone());
			},
			Entry::Vacant(entry) => {
				// We won the race, insert our client
				entry.insert(cached_client);
				debug!("Successfully cached new client for {}", config.base_url);
			},
		}

		Ok(client_arc)
	}

	/// Get or create a client with authentication configuration
	pub fn get_client_with_auth(
		&self,
		solver_config: &SolverRuntimeConfig,
		auth_config: &AuthConfig,
	) -> AdapterResult<Arc<Client>> {
		let mut config = ClientConfig::from(solver_config);

		// Apply authentication configuration
		match auth_config {
			AuthConfig::None => {
				// No authentication - use base config
			},
			AuthConfig::Bearer { token } => {
				config.headers.push((
					"Authorization".to_string(),
					format!("Bearer {}", token.expose_secret()),
				));
			},
			AuthConfig::ApiKey { header, key } => {
				config
					.headers
					.push((header.clone(), key.expose_secret().to_string()));
			},
			AuthConfig::Custom {
				headers,
				cache_key: _,
			} => {
				config.headers.extend(headers.clone());
			},
		}

		// Use existing get_client logic - cache differentiation now by solver_id
		self.get_client(&config)
	}

	// Using solver_id for cache differentiation - no additional hashing or encoding needed

	/// Create an optimized HTTP client for the given configuration
	fn create_optimized_client(&self, config: &ClientConfig) -> AdapterResult<Client> {
		let mut builder = ClientBuilder::new()
            // Connection pool optimization
            .pool_max_idle_per_host(config.max_idle_per_host)
            .pool_idle_timeout(Duration::from_millis(config.keep_alive_timeout_ms))
            // Enable keep-alive and HTTP/2
            .http2_keep_alive_timeout(Duration::from_millis(config.keep_alive_timeout_ms))
            .tcp_keepalive(Duration::from_secs(60));

		// Add custom headers
		let mut header_map = reqwest::header::HeaderMap::new();
		for (key, value) in &config.headers {
			if let (Ok(header_name), Ok(header_value)) = (
				reqwest::header::HeaderName::from_bytes(key.as_bytes()),
				reqwest::header::HeaderValue::from_str(value),
			) {
				header_map.insert(header_name, header_value);
			}
		}
		builder = builder.default_headers(header_map);

		builder.build().map_err(AdapterError::HttpError)
	}

	/// Remove all expired clients from the cache using atomic operations
	pub fn cleanup_expired(&self) -> usize {
		let mut removed_count = 0;

		// Use atomic remove_if for each entry to avoid race conditions
		self.clients.retain(|config, cached_client| {
			let is_expired = cached_client.is_expired(self.ttl);
			if is_expired {
				removed_count += 1;
				debug!(
					"Removed expired client for {} (age: {:?})",
					config.base_url,
					cached_client.created_at.elapsed()
				);
			}
			!is_expired // Keep non-expired clients
		});

		if removed_count > 0 {
			debug!("Cleaned up {} expired clients from cache", removed_count);
		}

		removed_count
	}

	/// Clear the cache (useful for testing or memory management)
	pub fn clear(&self) {
		let count = self.clients.len();
		self.clients.clear();
		debug!("Cleared all {} clients from cache", count);
	}

	/// Get the configured TTL duration
	pub fn ttl(&self) -> Duration {
		self.ttl
	}

	/// Convenience constructor for adapter implementations
	///
	/// This is the recommended way for external adapters to get access
	/// to optimized client caching infrastructure.
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use oif_adapters::ClientCache;
	///
	/// pub struct MyAdapter {
	///     cache: ClientCache,
	/// }
	///
	/// impl MyAdapter {
	///     pub fn new() -> Self {
	///         Self {
	///             cache: ClientCache::for_adapter(),
	///         }
	///     }
	/// }
	/// ```
	pub fn for_adapter() -> Self {
		adapter_client_cache()
	}
}

impl Default for ClientCache {
	fn default() -> Self {
		Self::new()
	}
}

lazy_static::lazy_static! {
	static ref GLOBAL_CLIENT_CACHE: ClientCache = ClientCache::new();
}

/// Get the global client cache instance
pub fn global_client_cache() -> &'static ClientCache {
	&GLOBAL_CLIENT_CACHE
}

pub fn adapter_client_cache() -> ClientCache {
	global_client_cache().clone()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_client_config_from_solver_runtime_config() {
		let solver_config = SolverRuntimeConfig {
			solver_id: "test-solver".to_string(),
			endpoint: "https://api.example.com".to_string(),
			headers: None,
			adapter_metadata: None,
		};

		let client_config = ClientConfig::from(&solver_config);

		assert_eq!(client_config.base_url, "https://api.example.com");
		assert_eq!(client_config.max_idle_per_host, 10);
		assert_eq!(client_config.keep_alive_timeout_ms, 90_000);
	}

	#[tokio::test]
	async fn test_client_cache_reuse() {
		let cache = ClientCache::new();

		let config = ClientConfig {
			base_url: "https://test.com".to_string(),
			solver_id: "test-solver".to_string(),
			max_idle_per_host: 5,
			keep_alive_timeout_ms: 60_000,
			headers: vec![],
		};

		// Get client twice
		let client1 = cache.get_client(&config).unwrap();
		let client2 = cache.get_client(&config).unwrap();

		// Should be the same Arc instance
		assert!(Arc::ptr_eq(&client1, &client2));
	}

	#[tokio::test]
	async fn test_client_cache_ttl_expiration() {
		// Create cache with very short TTL for testing
		let cache = ClientCache::with_ttl(Duration::from_millis(50));

		let config = ClientConfig {
			base_url: "https://test-ttl.com".to_string(),
			solver_id: "test-ttl-solver".to_string(),
			max_idle_per_host: 5,
			keep_alive_timeout_ms: 60_000,
			headers: vec![],
		};

		// Get initial client
		let client1 = cache.get_client(&config).unwrap();

		// Wait for TTL to expire
		tokio::time::sleep(Duration::from_millis(100)).await;

		// Get client again - should be a new instance due to TTL expiration
		let client2 = cache.get_client(&config).unwrap();

		// Should NOT be the same Arc instance (expired and recreated)
		assert!(!Arc::ptr_eq(&client1, &client2));
	}

	#[tokio::test]
	async fn test_concurrent_access_atomicity() {
		use std::sync::Arc;

		let cache = Arc::new(ClientCache::with_ttl(Duration::from_millis(100)));
		let config = ClientConfig {
			base_url: "https://concurrent-test.com".to_string(),
			solver_id: "concurrent-test-solver".to_string(),
			max_idle_per_host: 5,
			keep_alive_timeout_ms: 60_000,
			headers: vec![],
		};

		// Spawn multiple concurrent tasks that try to get/create clients
		let mut handles = vec![];
		for _i in 0..10 {
			let cache_clone = cache.clone();
			let config_clone = config.clone();

			let handle = tokio::spawn(async move {
				let client = cache_clone.get_client(&config_clone).unwrap();
				tokio::time::sleep(Duration::from_millis(50)).await;
				// Return the Arc pointer for comparison
				Arc::as_ptr(&client) as usize
			});
			handles.push(handle);
		}

		// Wait for all tasks and collect results
		let mut results = vec![];
		for handle in handles {
			results.push(handle.await.unwrap());
		}

		// All should have gotten the same client instance (all pointers should be equal)
		let first_pointer = results[0];
		assert!(
			results.iter().all(|&ptr| ptr == first_pointer),
			"All concurrent requests should get the same cached client"
		);

		// Wait for TTL expiration
		tokio::time::sleep(Duration::from_millis(150)).await;

		// Now get a new client - should be different due to TTL expiration
		let new_client = cache.get_client(&config).unwrap();
		let new_pointer = Arc::as_ptr(&new_client) as usize;

		assert_ne!(
			first_pointer, new_pointer,
			"New client after TTL expiration should be different"
		);
	}

	#[test]
	fn test_cache_cloning() {
		let cache1 = ClientCache::new();
		let cache2 = cache1.clone();

		// Both caches should have the same TTL
		assert_eq!(cache1.ttl(), cache2.ttl());

		// Both caches should share the same underlying DashMap
		let config = ClientConfig {
			base_url: "https://clone-test.com".to_string(),
			solver_id: "cache-clone-solver".to_string(),
			max_idle_per_host: 5,
			keep_alive_timeout_ms: 60_000,
			headers: vec![],
		};

		// Insert client via cache1
		let client1 = cache1.get_client(&config).unwrap();

		// Retrieve via cache2 - should get the same client
		let client2 = cache2.get_client(&config).unwrap();

		// Should be the same Arc instance since they share the same DashMap
		assert!(Arc::ptr_eq(&client1, &client2));
	}

	#[tokio::test]
	async fn test_jwt_authenticated_client_caching() {
		let cache = ClientCache::new();
		let solver_config = SolverRuntimeConfig::new(
			"test-jwt-solver".to_string(),
			"https://jwt-test.com".to_string(),
		);

		// Test with JWT token
		let jwt_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.test.signature";

		// Get client twice with same JWT token
		let auth1 = AuthConfig::jwt(Some(jwt_token));
		let client1 = cache.get_client_with_auth(&solver_config, &auth1).unwrap();
		let client2 = cache.get_client_with_auth(&solver_config, &auth1).unwrap();

		// Should reuse the same client (same token = same cache entry)
		assert!(Arc::ptr_eq(&client1, &client2));

		// Test with different JWT token
		let different_jwt_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.different.signature";
		let auth2 = AuthConfig::jwt(Some(different_jwt_token));
		let client3 = cache.get_client_with_auth(&solver_config, &auth2).unwrap();

		// Should be a different client (different token = different cache entry)
		assert!(!Arc::ptr_eq(&client1, &client3));

		// Test without JWT token
		let auth3 = AuthConfig::jwt(None);
		let client4 = cache.get_client_with_auth(&solver_config, &auth3).unwrap();

		// Should be different from JWT clients (no token vs with token)
		assert!(!Arc::ptr_eq(&client1, &client4));
		assert!(!Arc::ptr_eq(&client3, &client4));
	}

	#[tokio::test]
	async fn test_different_auth_strategies() {
		let cache = ClientCache::new();
		let solver_config = SolverRuntimeConfig::new(
			"test-multi-auth-solver".to_string(),
			"https://api.example.com".to_string(),
		);

		// Test API Key authentication
		let api_key_auth = AuthConfig::api_key("X-API-Key", "secret-key-123");
		let api_client = cache
			.get_client_with_auth(&solver_config, &api_key_auth)
			.unwrap();

		// Test Bearer token authentication
		let bearer_auth = AuthConfig::Bearer {
			token: SecretString::from("bearer-token-789"),
		};
		let bearer_client = cache
			.get_client_with_auth(&solver_config, &bearer_auth)
			.unwrap();

		// Test Custom headers authentication
		let custom_auth = AuthConfig::Custom {
			headers: vec![
				("X-Auth-Token".to_string(), "custom-token-456".to_string()),
				("X-Client-ID".to_string(), "test-client".to_string()),
			],
			cache_key: Some("custom:456".to_string()),
		};
		let custom_client = cache
			.get_client_with_auth(&solver_config, &custom_auth)
			.unwrap();

		// All should be different clients due to different auth configurations
		assert!(!Arc::ptr_eq(&api_client, &bearer_client));
		assert!(!Arc::ptr_eq(&api_client, &custom_client));
		assert!(!Arc::ptr_eq(&bearer_client, &custom_client));

		// Same auth config should return same client
		let api_client2 = cache
			.get_client_with_auth(&solver_config, &api_key_auth)
			.unwrap();
		assert!(Arc::ptr_eq(&api_client, &api_client2));
	}
}
