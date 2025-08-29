use utoipa::OpenApi;

use oif_service::SolverStats;
use oif_types::models::health::{HealthResponse, StorageHealthInfo};
use oif_types::orders::request::OrderRequest;
use oif_types::orders::response::OrderResponse;
use oif_types::quotes::request::QuoteRequest;
use oif_types::quotes::response::QuotesResponse;
use oif_types::solvers::response::{SolverResponse, SolversResponse};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::health::health,
        crate::handlers::quotes::post_quotes,
        crate::handlers::orders::post_orders,
        crate::handlers::orders::get_order,
        crate::handlers::solvers::get_solvers,
        crate::handlers::solvers::get_solver_by_id,
    ),
    components(schemas(
        QuoteRequest, QuotesResponse,
        OrderRequest, OrderResponse,
        SolverResponse, SolversResponse,
        HealthResponse, StorageHealthInfo, SolverStats
    )),
    tags(
        (name = "quotes", description = "Request and manage price quotes from multiple solvers for cross-chain transactions"),
        (name = "orders", description = "Submit, track, and manage cross-chain orders through the aggregator"),
        (name = "health", description = "System health checks and diagnostics for monitoring service status"),
        (name = "solvers", description = "Discover and interact with available solvers and their capabilities")
    ),
    info(
        title = "OIF Aggregator API",
        version = "0.1.0",
        description = "Open Intents Framework (OIF) Aggregator provides a unified API for cross-chain transaction aggregation. This service connects to multiple solvers to find the best execution paths for user intents across different blockchain networks.",
        license(
            name = "MIT",
            url = "https://github.com/openintentsframework/oif-aggregator/blob/main/LICENSE"
        ),
        contact(
            name = "OIF Team",
            url = "https://github.com/openintentsframework/"
        )
    ),
)]
pub struct ApiDoc;
