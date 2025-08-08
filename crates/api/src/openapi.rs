use utoipa::OpenApi;
use crate::handlers::{health, orders, quotes};

use oif_types::quotes::request::QuotesRequest;
use oif_types::quotes::response::{QuoteResponse, QuotesResponse};
use oif_types::orders::request::OrdersRequest;
use oif_types::orders::response::{OrderStatusResponse, OrdersResponse};

#[derive(OpenApi)]
#[openapi(
    paths(
        health::health,
        health::ready,
        quotes::post_quotes,
        orders::post_orders,
        orders::get_order_status,
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


