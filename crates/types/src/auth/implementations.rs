//! Common authentication and rate limiting implementations

use super::errors::*;
use super::traits::*;
use async_trait::async_trait;
use base64::prelude::*;
use chrono::{Duration, Utc};
use dashmap::DashMap;
use std::sync::Arc;
use tracing::{debug, warn};

/// No-op authenticator that allows all requests
#[derive(Debug, Default)]
pub struct NoAuthenticator;

#[async_trait]
impl Authenticator for NoAuthenticator {
	async fn authenticate(&self, _request: &AuthRequest) -> AuthenticationResult {
		debug!("NoAuthenticator: bypassing authentication");
		AuthenticationResult::Bypassed
	}

	async fn authorize(&self, _context: &AuthContext, _permission: &Permission) -> bool {
		true
	}

	fn get_rate_limits(&self, _context: &AuthContext) -> Option<RateLimits> {
		None
	}

	async fn health_check(&self) -> Result<bool, AuthError> {
		Ok(true)
	}

	fn name(&self) -> &str {
		"NoAuthenticator"
	}
}

/// Simple API key authenticator
#[derive(Debug)]
pub struct ApiKeyAuthenticator {
	/// Valid API keys mapped to user contexts
	api_keys: Arc<DashMap<String, AuthContext>>,
}

impl ApiKeyAuthenticator {
	/// Create a new API key authenticator
	pub fn new() -> Self {
		Self {
			api_keys: Arc::new(DashMap::new()),
		}
	}

	/// Add an API key with associated context
	pub fn add_key(&self, api_key: String, context: AuthContext) {
		self.api_keys.insert(api_key, context);
	}

	/// Remove an API key
	pub fn remove_key(&self, api_key: &str) -> Option<AuthContext> {
		self.api_keys.remove(api_key).map(|(_, context)| context)
	}

	/// Create with default admin key
	pub fn with_admin_key(admin_key: String) -> Self {
		let auth = Self::new();
		let admin_context = AuthContext::new("admin".to_string())
			.with_role("admin".to_string())
			.with_permission(Permission::Admin)
			.with_permission(Permission::ReadQuotes)
			.with_permission(Permission::SubmitOrders)
			.with_permission(Permission::ReadOrders)
			.with_permission(Permission::HealthCheck);

		auth.add_key(admin_key, admin_context);
		auth
	}
}

#[async_trait]
impl Authenticator for ApiKeyAuthenticator {
	async fn authenticate(&self, request: &AuthRequest) -> AuthenticationResult {
		if let Some(api_key) = request.get_api_key() {
			if let Some(context) = self.api_keys.get(api_key) {
				if context.is_expired() {
					warn!("API key {} has expired", api_key);
					return AuthenticationResult::Unauthorized("API key expired".to_string());
				}
				debug!(
					"API key {} authenticated for user {}",
					api_key, context.user_id
				);
				return AuthenticationResult::Authorized(context.clone());
			}
		}

		AuthenticationResult::Unauthorized("Invalid or missing API key".to_string())
	}

	async fn authorize(&self, context: &AuthContext, permission: &Permission) -> bool {
		// Admin can do anything
		if context.has_role("admin") || context.has_permission(&Permission::Admin) {
			return true;
		}

		// Check specific permission
		context.has_permission(permission)
	}

	fn get_rate_limits(&self, context: &AuthContext) -> Option<RateLimits> {
		context.rate_limits.clone()
	}

	async fn health_check(&self) -> Result<bool, AuthError> {
		Ok(true)
	}

	fn name(&self) -> &str {
		"ApiKeyAuthenticator"
	}
}

impl Default for ApiKeyAuthenticator {
	fn default() -> Self {
		Self::new()
	}
}

/// In-memory rate limiter
#[derive(Debug)]
pub struct MemoryRateLimiter {
	/// Request counters by key
	counters: Arc<DashMap<String, RequestCounter>>,
}

#[derive(Debug, Clone)]
struct RequestCounter {
	/// Request count in current window
	count: u32,
	/// Window start time
	window_start: chrono::DateTime<Utc>,
	/// Window duration in seconds
	window_duration: u64,
}

impl MemoryRateLimiter {
	/// Create a new memory rate limiter
	pub fn new() -> Self {
		Self {
			counters: Arc::new(DashMap::new()),
		}
	}

	/// Clean up expired windows (should be called periodically)
	pub fn cleanup_expired(&self) {
		let now = Utc::now();
		let mut to_remove = Vec::new();

		for entry in self.counters.iter() {
			let counter = entry.value();
			let window_end =
				counter.window_start + Duration::seconds(counter.window_duration as i64);
			if now > window_end {
				to_remove.push(entry.key().clone());
			}
		}

		for key in to_remove {
			self.counters.remove(&key);
		}
	}
}

#[async_trait]
impl RateLimiter for MemoryRateLimiter {
	async fn check_rate_limit(
		&self,
		key: &str,
		limits: &RateLimits,
	) -> RateLimitResult<RateLimitCheck> {
		let now = Utc::now();
		let window_duration = 60; // 1 minute window

		// Clean up expired entries periodically
		if rand::random::<f64>() < 0.01 {
			self.cleanup_expired();
		}

		let mut entry = self
			.counters
			.entry(key.to_string())
			.or_insert_with(|| RequestCounter {
				count: 0,
				window_start: now,
				window_duration,
			});

		let counter = entry.value_mut();

		// Check if we need a new window
		let window_end = counter.window_start + Duration::seconds(window_duration as i64);
		if now > window_end {
			// Start new window
			counter.count = 0;
			counter.window_start = now;
		}

		let allowed = counter.count < limits.requests_per_minute;
		let remaining = limits.requests_per_minute.saturating_sub(counter.count);
		let reset_at = counter.window_start + Duration::seconds(window_duration as i64);

		Ok(RateLimitCheck {
			allowed,
			remaining,
			reset_at,
			limit: limits.requests_per_minute,
		})
	}

	async fn record_request(&self, key: &str) -> Result<(), RateLimitError> {
		let now = Utc::now();
		let window_duration = 60;

		let mut entry = self
			.counters
			.entry(key.to_string())
			.or_insert_with(|| RequestCounter {
				count: 0,
				window_start: now,
				window_duration,
			});

		let counter = entry.value_mut();

		// Check if we need a new window
		let window_end = counter.window_start + Duration::seconds(window_duration as i64);
		if now > window_end {
			counter.count = 1;
			counter.window_start = now;
		} else {
			counter.count += 1;
		}

		Ok(())
	}

	async fn get_usage(&self, key: &str) -> Result<u32, RateLimitError> {
		if let Some(counter) = self.counters.get(key) {
			Ok(counter.count)
		} else {
			Ok(0)
		}
	}

	async fn reset_limit(&self, key: &str) -> Result<(), RateLimitError> {
		self.counters.remove(key);
		Ok(())
	}

	async fn health_check(&self) -> Result<bool, RateLimitError> {
		Ok(true)
	}

	fn name(&self) -> &str {
		"MemoryRateLimiter"
	}
}

impl Default for MemoryRateLimiter {
	fn default() -> Self {
		Self::new()
	}
}

/// Simple role-based authenticator for testing
#[derive(Debug)]
pub struct SimpleRoleAuthenticator {
	users: Arc<DashMap<String, AuthContext>>,
}

impl SimpleRoleAuthenticator {
	/// Create a new role-based authenticator
	pub fn new() -> Self {
		Self {
			users: Arc::new(DashMap::new()),
		}
	}

	/// Add a user with username/password (stored as basic auth)
	pub fn add_user(&self, username: String, password: String, context: AuthContext) {
		let basic_auth =
			base64::prelude::BASE64_STANDARD.encode(format!("{}:{}", username, password));
		self.users.insert(basic_auth, context);
	}

	/// Create with default test users
	pub fn with_test_users() -> Self {
		let auth = Self::new();

		// Admin user
		let admin_context = AuthContext::new("admin".to_string())
			.with_role("admin".to_string())
			.with_permission(Permission::Admin)
			.with_permission(Permission::ReadQuotes)
			.with_permission(Permission::SubmitOrders)
			.with_permission(Permission::ReadOrders)
			.with_permission(Permission::HealthCheck);
		auth.add_user("admin".to_string(), "admin123".to_string(), admin_context);

		// Regular user
		let user_context = AuthContext::new("user1".to_string())
			.with_role("user".to_string())
			.with_permission(Permission::ReadQuotes)
			.with_permission(Permission::SubmitOrders)
			.with_rate_limits(RateLimits::default());
		auth.add_user("user1".to_string(), "password".to_string(), user_context);

		auth
	}
}

#[async_trait]
impl Authenticator for SimpleRoleAuthenticator {
	async fn authenticate(&self, request: &AuthRequest) -> AuthenticationResult {
		if let Some(auth_header) = request.get_authorization() {
			if let Some(basic_auth) = auth_header.strip_prefix("Basic ") {
				if let Some(context) = self.users.get(basic_auth) {
					if context.is_expired() {
						return AuthenticationResult::Unauthorized(
							"Credentials expired".to_string(),
						);
					}
					return AuthenticationResult::Authorized(context.clone());
				}
			}
		}

		AuthenticationResult::Unauthorized("Invalid credentials".to_string())
	}

	async fn authorize(&self, context: &AuthContext, permission: &Permission) -> bool {
		if context.has_role("admin") {
			return true;
		}

		context.has_permission(permission)
	}

	fn get_rate_limits(&self, context: &AuthContext) -> Option<RateLimits> {
		context.rate_limits.clone()
	}

	async fn health_check(&self) -> Result<bool, AuthError> {
		Ok(true)
	}

	fn name(&self) -> &str {
		"SimpleRoleAuthenticator"
	}
}

impl Default for SimpleRoleAuthenticator {
	fn default() -> Self {
		Self::new()
	}
}
