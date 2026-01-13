use axum::{
	routing::{get, post},
	Router,
};
use tower::ServiceBuilder;
use tower_http::{
	compression::CompressionLayer,
	cors::CorsLayer,
	limit::RequestBodyLimitLayer,
	request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
	trace::TraceLayer,
};
use tracing::Level;

use crate::handlers::{get_order, get_solver_by_id, get_solvers, health, post_orders, post_quotes};
use crate::security::add_security_headers;
use crate::state::AppState;
// State is applied at the application level using `.with_state(...)`.
#[cfg(feature = "openapi")]
use crate::openapi::ApiDoc;
#[cfg(feature = "openapi")]
use utoipa::OpenApi;
#[cfg(feature = "openapi")]
use utoipa_swagger_ui::SwaggerUi;

pub fn create_router() -> Router<AppState> {
	// Layers prepared first so they're in scope for all cfg paths
	let cors = CorsLayer::permissive();
	let body_limit = RequestBodyLimitLayer::new(1024 * 1024);
	let trace = TraceLayer::new_for_http()
		.make_span_with(|req: &axum::http::Request<_>| {
			let req_id = req
				.headers()
				.get("x-request-id")
				.and_then(|v| v.to_str().ok())
				.unwrap_or("-");
			tracing::info_span!(
				"http_request",
				method = %req.method(),
				uri = %req.uri(),
				req_id
			)
		})
		.on_request(tower_http::trace::DefaultOnRequest::new().level(Level::INFO))
		.on_response(
			tower_http::trace::DefaultOnResponse::new()
				.level(Level::INFO)
				.latency_unit(tower_http::LatencyUnit::Millis),
		);
	let req_id = ServiceBuilder::new()
		.layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
		.layer(PropagateRequestIdLayer::x_request_id());

	// Base router
	let base_router = Router::new()
		.route("/health", get(health))
		.route("/health/", get(health))
		.route("/api/v1/quotes", post(post_quotes))
		.route("/api/v1/quotes/", post(post_quotes))
		.route("/api/v1/orders", post(post_orders))
		.route("/api/v1/orders/", post(post_orders))
		.route("/api/v1/orders/{id}", get(get_order))
		.route("/api/v1/orders/{id}/", get(get_order))
		.route("/api/v1/solvers", get(get_solvers))
		.route("/api/v1/solvers/", get(get_solvers))
		.route("/api/v1/solvers/{id}", get(get_solver_by_id))
		.route("/api/v1/solvers/{id}/", get(get_solver_by_id));
	// Conditionally add OpenAPI endpoints
	#[cfg(feature = "openapi")]
	let router = {
		// SwaggerUI automatically provides the OpenAPI JSON endpoint
		base_router
			.merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
	};

	#[cfg(not(feature = "openapi"))]
	let router = base_router;

	// Apply common layers
	let router = router
		.layer(cors)
		.layer(CompressionLayer::new())
		.layer(trace)
		.layer(req_id)
		.layer(body_limit);

	add_security_headers(router)
}
