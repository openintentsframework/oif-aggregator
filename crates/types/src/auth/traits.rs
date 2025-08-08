//! Core authentication and authorization traits

use super::errors::{AuthError, RateLimitError, RateLimitResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Authentication result with user context
#[derive(Debug, Clone)]
pub enum AuthenticationResult {
	/// Authentication successful with user context
	Authorized(AuthContext),
	/// Authentication failed
	Unauthorized(String),
	/// Authentication bypassed (e.g., for public endpoints)
	Bypassed,
}

/// Authenticated user context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
	/// Unique user identifier
	pub user_id: String,
	/// User roles
	pub roles: Vec<String>,
	/// Specific permissions
	pub permissions: Vec<Permission>,
	/// Rate limiting configuration for this user
	pub rate_limits: Option<RateLimits>,
	/// Additional metadata
	pub metadata: HashMap<String, String>,
	/// When this context was created
	pub created_at: DateTime<Utc>,
	/// When this context expires (for tokens)
	pub expires_at: Option<DateTime<Utc>>,
}

/// Authorization permissions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Permission {
	/// Read quotes
	ReadQuotes,
	/// Submit orders
	SubmitOrders,
	/// Read order status
	ReadOrders,
	/// Admin operations
	Admin,
	/// Health check access
	HealthCheck,
	/// Custom permission
	Custom(String),
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimits {
	/// Requests per minute allowed
	pub requests_per_minute: u32,
	/// Burst size (how many requests can be made immediately)
	pub burst_size: u32,
	/// Custom rate limit windows
	pub custom_windows: Vec<RateLimitWindow>,
}

/// Custom rate limit window
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitWindow {
	/// Window duration in seconds
	pub window_seconds: u64,
	/// Max requests in this window
	pub max_requests: u32,
}

/// Authentication request context
#[derive(Debug, Clone)]
pub struct AuthRequest {
	/// HTTP headers
	pub headers: HashMap<String, String>,
	/// Request path
	pub path: String,
	/// HTTP method
	pub method: String,
	/// Client IP address
	pub client_ip: Option<String>,
	/// User agent
	pub user_agent: Option<String>,
	/// Additional request metadata
	pub metadata: HashMap<String, String>,
}

/// Rate limit check result
#[derive(Debug, Clone)]
pub struct RateLimitCheck {
	/// Whether the request is allowed
	pub allowed: bool,
	/// Remaining requests in current window
	pub remaining: u32,
	/// When the rate limit resets
	pub reset_at: DateTime<Utc>,
	/// Total limit for the window
	pub limit: u32,
}

/// Core authentication trait for custom auth implementations
#[async_trait]
pub trait Authenticator: Send + Sync + std::fmt::Debug {
	/// Authenticate a request and return user context
	async fn authenticate(&self, request: &AuthRequest) -> AuthenticationResult;

	/// Check if user has permission for specific action
	async fn authorize(&self, context: &AuthContext, permission: &Permission) -> bool;

	/// Get rate limits for authenticated user
	fn get_rate_limits(&self, context: &AuthContext) -> Option<RateLimits>;

	/// Health check for auth service
	async fn health_check(&self) -> Result<bool, AuthError>;

	/// Get human-readable name for this authenticator
	fn name(&self) -> &str;
}

/// Rate limiting trait for custom rate limiter implementations
#[async_trait]
pub trait RateLimiter: Send + Sync + std::fmt::Debug {
	/// Check if request is within rate limits
	async fn check_rate_limit(
		&self,
		key: &str,
		limits: &RateLimits,
	) -> RateLimitResult<RateLimitCheck>;

	/// Record a request for rate limiting
	async fn record_request(&self, key: &str) -> Result<(), RateLimitError>;

	/// Get current usage for a key
	async fn get_usage(&self, key: &str) -> Result<u32, RateLimitError>;

	/// Reset rate limit for a key (admin operation)
	async fn reset_limit(&self, key: &str) -> Result<(), RateLimitError>;

	/// Health check for rate limiter
	async fn health_check(&self) -> Result<bool, RateLimitError>;

	/// Get human-readable name for this rate limiter
	fn name(&self) -> &str;
}

impl AuthContext {
	/// Create a new auth context
	pub fn new(user_id: String) -> Self {
		Self {
			user_id,
			roles: Vec::new(),
			permissions: Vec::new(),
			rate_limits: None,
			metadata: HashMap::new(),
			created_at: Utc::now(),
			expires_at: None,
		}
	}

	/// Check if context has expired
	pub fn is_expired(&self) -> bool {
		if let Some(expires_at) = self.expires_at {
			Utc::now() > expires_at
		} else {
			false
		}
	}

	/// Check if user has a specific role
	pub fn has_role(&self, role: &str) -> bool {
		self.roles.iter().any(|r| r == role)
	}

	/// Check if user has a specific permission
	pub fn has_permission(&self, permission: &Permission) -> bool {
		self.permissions.contains(permission)
	}

	/// Add a role to the context
	pub fn with_role(mut self, role: String) -> Self {
		self.roles.push(role);
		self
	}

	/// Add a permission to the context
	pub fn with_permission(mut self, permission: Permission) -> Self {
		self.permissions.push(permission);
		self
	}

	/// Set rate limits
	pub fn with_rate_limits(mut self, rate_limits: RateLimits) -> Self {
		self.rate_limits = Some(rate_limits);
		self
	}

	/// Add metadata
	pub fn with_metadata(mut self, key: String, value: String) -> Self {
		self.metadata.insert(key, value);
		self
	}
}

impl Default for RateLimits {
	fn default() -> Self {
		Self {
			requests_per_minute: 100,
			burst_size: 10,
			custom_windows: Vec::new(),
		}
	}
}

impl AuthRequest {
	/// Create a new auth request from HTTP components
	pub fn new(method: String, path: String) -> Self {
		Self {
			headers: HashMap::new(),
			path,
			method,
			client_ip: None,
			user_agent: None,
			metadata: HashMap::new(),
		}
	}

	/// Add a header
	pub fn with_header(mut self, key: String, value: String) -> Self {
		self.headers.insert(key, value);
		self
	}

	/// Set client IP
	pub fn with_client_ip(mut self, ip: String) -> Self {
		self.client_ip = Some(ip);
		self
	}

	/// Get header value
	pub fn get_header(&self, key: &str) -> Option<&String> {
		self.headers.get(key)
	}

	/// Get authorization header
	pub fn get_authorization(&self) -> Option<&String> {
		self.get_header("authorization")
			.or_else(|| self.get_header("Authorization"))
	}

	/// Get API key from headers
	pub fn get_api_key(&self) -> Option<&String> {
		self.get_header("x-api-key")
			.or_else(|| self.get_header("X-API-Key"))
	}
}
