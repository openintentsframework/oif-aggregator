//! Security-related HTTP response headers setup

use axum::{
    http::header::{HeaderName, HeaderValue},
    Router,
};
use tower::ServiceBuilder;
use tower_http::set_header::SetResponseHeaderLayer;

/// Apply a stack of sensible default security headers to the provided router.
pub fn add_security_headers<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
	router.layer(
		ServiceBuilder::new()
			.layer(SetResponseHeaderLayer::if_not_present(
				HeaderName::from_static("strict-transport-security"),
				HeaderValue::from_static("max-age=31536000; includeSubDomains; preload"),
			))
			.layer(SetResponseHeaderLayer::if_not_present(
				HeaderName::from_static("x-content-type-options"),
				HeaderValue::from_static("nosniff"),
			))
			.layer(SetResponseHeaderLayer::if_not_present(
				HeaderName::from_static("x-frame-options"),
				HeaderValue::from_static("DENY"),
			))
			.layer(SetResponseHeaderLayer::if_not_present(
				HeaderName::from_static("referrer-policy"),
                HeaderValue::from_static("strict-origin-when-cross-origin"),
			))
			.layer(SetResponseHeaderLayer::if_not_present(
				HeaderName::from_static("x-xss-protection"),
				HeaderValue::from_static("1; mode=block"),
			))
			.layer(SetResponseHeaderLayer::if_not_present(
				HeaderName::from_static("content-security-policy"),
				HeaderValue::from_static("default-src 'self'"),
			))
			.layer(SetResponseHeaderLayer::if_not_present(
				HeaderName::from_static("cache-control"),
				HeaderValue::from_static("no-cache"),
			)),
	)
}
