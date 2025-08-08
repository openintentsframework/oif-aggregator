use crate::handlers::{health, orders, quotes, solvers};
use utoipa::OpenApi;

use oif_types::orders::request::OrdersRequest;
use oif_types::orders::response::{OrderStatusResponse, OrdersResponse};
use oif_types::quotes::request::QuotesRequest;
use oif_types::quotes::response::{QuoteResponse, QuotesResponse};

#[derive(OpenApi)]
#[openapi(
    paths(
        health::health,
        health::ready,
        quotes::post_quotes,
        orders::post_orders,
        orders::get_order_status,
        solvers::get_solvers,
        solvers::get_solver_by_id,
    ),
    components(schemas(
        QuotesRequest, QuoteResponse, QuotesResponse,
        OrdersRequest, OrdersResponse, OrderStatusResponse
    )),
    tags(
        (name = "quotes", description = "Quote related endpoints"),
        (name = "orders", description = "Order related endpoints"),
        (name = "health", description = "Health and readiness endpoints")
    )
)]
pub struct ApiDoc;
