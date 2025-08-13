use crate::handlers::{health, orders, quotes, solvers};
use utoipa::OpenApi;

use oif_types::orders::request::OrderRequest;
use oif_types::orders::response::{OrderResponse, OrdersResponse};
use oif_types::quotes::request::QuoteRequest;
use oif_types::quotes::response::{QuoteResponse, QuotesResponse};
use oif_types::solvers::response::{SolverResponse, SolversResponse};

#[derive(OpenApi)]
#[openapi(
    paths(
        health::health,
        health::ready,
        quotes::post_quotes,
        orders::post_orders,
        orders::get_order,
        solvers::get_solvers,
        solvers::get_solver_by_id,
    ),
    components(schemas(
        QuoteRequest, QuoteResponse, QuotesResponse,
        OrderRequest, OrderResponse, OrdersResponse,
        SolverResponse, SolversResponse
    )),
    tags(
        (name = "quotes", description = "Quote related endpoints"),
        (name = "orders", description = "Order related endpoints"),
        (name = "health", description = "Health and readiness endpoints")
    )
)]
pub struct ApiDoc;
