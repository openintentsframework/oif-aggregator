use utoipa::OpenApi;

use crate::handlers::health::{HealthResponse, StorageHealthInfo};
use oif_service::SolverStats;
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
        (name = "quotes", description = "Quote related endpoints"),
        (name = "orders", description = "Order related endpoints"),
        (name = "health", description = "Health endpoints"),
        (name = "solvers", description = "Solver related endpoints")
    )
)]
pub struct ApiDoc;
