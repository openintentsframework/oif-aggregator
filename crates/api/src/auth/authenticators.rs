//! Authentication implementations

use oif_types::auth::{
	errors::AuthError,
	traits::{
		AuthContext, AuthRequest, AuthenticationResult, Authenticator, Permission, RateLimits,
	},
};

use async_trait::async_trait;
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
