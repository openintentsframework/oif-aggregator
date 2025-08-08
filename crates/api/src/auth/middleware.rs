//! Authentication middleware using the auth traits

use axum::{
	extract::Request,
	http::{HeaderMap, StatusCode},
	middleware::Next,
	response::Response,
};
use oif_types::auth::{
	AuthRequest, AuthenticationResult, Authenticator, Permission, RateLimiter, RateLimits,
};
use std::sync::Arc;
use tracing::{debug, warn};

/// Auth middleware configuration
#[derive(Debug, Clone)]
pub struct AuthConfig {
	/// Paths that require authentication
	pub protected_paths: Vec<String>,
	/// Paths that are completely public (no auth check)
	pub public_paths: Vec<String>,
	/// Whether to enable rate limiting
	pub enable_rate_limiting: bool,
	/// Default rate limits for unauthenticated users
	pub default_rate_limits: Option<RateLimits>,
}

impl Default for AuthConfig {
	fn default() -> Self {
		Self {
			protected_paths: vec!["/v1/orders".to_string()],
			public_paths: vec!["/health".to_string(), "/ready".to_string()],
			enable_rate_limiting: true,
			default_rate_limits: Some(RateLimits::default()),
		}
	}
}

/// Auth middleware layer
pub struct AuthMiddleware<A, R>
where
	A: Authenticator,
	R: RateLimiter,
{
	authenticator: Arc<A>,
	rate_limiter: Arc<R>,
	config: AuthConfig,
}

impl<A, R> AuthMiddleware<A, R>
where
	A: Authenticator,
	R: RateLimiter,
{
	/// Create new auth middleware
	pub fn new(authenticator: Arc<A>, rate_limiter: Arc<R>) -> Self {
		Self {
			authenticator,
			rate_limiter,
			config: AuthConfig::default(),
		}
	}

	/// Create with custom config
	pub fn with_config(authenticator: Arc<A>, rate_limiter: Arc<R>, config: AuthConfig) -> Self {
		Self {
			authenticator,
			rate_limiter,
			config,
		}
	}

	/// Add a protected path
	pub fn protect_path(mut self, path: &str) -> Self {
		self.config.protected_paths.push(path.to_string());
		self
	}

	/// Add a public path
	pub fn add_public_path(mut self, path: &str) -> Self {
		self.config.public_paths.push(path.to_string());
		self
	}
}

/// Authentication middleware function
pub async fn auth_middleware<A, R>(
	authenticator: Arc<A>,
	rate_limiter: Arc<R>,
	config: AuthConfig,
	request: Request,
	next: Next,
) -> Result<Response, StatusCode>
where
	A: Authenticator,
	R: RateLimiter,
{
	let path = request.uri().path().to_string();
	let method = request.method().to_string();

	// Check if path is public
	if config.public_paths.iter().any(|p| path.starts_with(p)) {
		debug!("Public path {}, skipping auth", path);
		return Ok(next.run(request).await);
	}

	// Convert headers to HashMap
	let headers = headers_to_map(request.headers());

	// Extract client IP (simplified - in production, use proper forwarded headers)
	let client_ip = headers
		.get("x-forwarded-for")
		.or_else(|| headers.get("x-real-ip"))
		.cloned();

	let auth_request = AuthRequest::new(method.clone(), path.clone())
		.with_header(
			"authorization".to_string(),
			headers
				.get("authorization")
				.unwrap_or(&String::new())
				.clone(),
		)
		.with_header(
			"x-api-key".to_string(),
			headers.get("x-api-key").unwrap_or(&String::new()).clone(),
		);

	// Authenticate the request
	let auth_result = authenticator.authenticate(&auth_request).await;

	let (auth_context, rate_limits) = match auth_result {
		AuthenticationResult::Authorized(context) => {
			debug!("Request authenticated for user: {}", context.user_id);
			let limits = authenticator.get_rate_limits(&context);
			(Some(context), limits)
		},
		AuthenticationResult::Bypassed => {
			debug!("Authentication bypassed for path: {}", path);
			(None, config.default_rate_limits.clone())
		},
		AuthenticationResult::Unauthorized(reason) => {
			warn!("Authentication failed for path {}: {}", path, reason);

			// Check if path requires authentication
			if config.protected_paths.iter().any(|p| path.starts_with(p)) {
				return Err(StatusCode::UNAUTHORIZED);
			}

			// Not a protected path, continue with default rate limits
			(None, config.default_rate_limits.clone())
		},
	};

	// Check authorization for protected paths
	if config.protected_paths.iter().any(|p| path.starts_with(p)) {
		if let Some(ref context) = auth_context {
			// Determine required permission based on path and method
			let required_permission = match (path.as_str(), method.as_str()) {
				(p, "POST") if p.starts_with("/v1/orders") => Permission::SubmitOrders,
				(p, "GET") if p.starts_with("/v1/orders") => Permission::ReadOrders,
				(p, "POST") if p.starts_with("/v1/quotes") => Permission::ReadQuotes,
				_ => Permission::ReadQuotes, // Default permission
			};

			if !authenticator.authorize(context, &required_permission).await {
				warn!(
					"Authorization failed for user {} on path {}",
					context.user_id, path
				);
				return Err(StatusCode::FORBIDDEN);
			}
		} else {
			// Protected path but no auth context
			return Err(StatusCode::UNAUTHORIZED);
		}
	}

	// Rate limiting
	if config.enable_rate_limiting {
		if let Some(limits) = rate_limits {
			let rate_key = if let Some(context) = &auth_context {
				format!("user:{}", context.user_id)
			} else {
				format!("ip:{}", client_ip.unwrap_or_else(|| "unknown".to_string()))
			};

			match rate_limiter.check_rate_limit(&rate_key, &limits).await {
				Ok(check) => {
					if !check.allowed {
						warn!("Rate limit exceeded for key: {}", rate_key);
						return Err(StatusCode::TOO_MANY_REQUESTS);
					}

					// Record the request
					if let Err(e) = rate_limiter.record_request(&rate_key).await {
						warn!("Failed to record request for rate limiting: {}", e);
					}
				},
				Err(e) => {
					warn!("Rate limiter error: {}", e);
					// Continue without rate limiting on error
				},
			}
		}
	}

	// Add auth context to request extensions if available
	let mut request = request;
	if let Some(context) = auth_context {
		request.extensions_mut().insert(context);
	}

	Ok(next.run(request).await)
}

/// Helper function to convert HeaderMap to HashMap<String, String>
fn headers_to_map(headers: &HeaderMap) -> std::collections::HashMap<String, String> {
	let mut map = std::collections::HashMap::new();

	for (name, value) in headers.iter() {
		if let Ok(value_str) = value.to_str() {
			map.insert(name.as_str().to_lowercase(), value_str.to_string());
		}
	}

	map
}
