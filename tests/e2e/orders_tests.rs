/// E2E tests for orders endpoints

use crate::e2e::{fixtures, TestServer};
use reqwest::Client;

#[tokio::test]
async fn test_orders_stateless_flow() {
    let server = TestServer::spawn().await;
    let client = Client::new();

    // Create order with quote_response (stateless)
    let resp = client
        .post(format!("{}/v1/orders", server.base_url))
        .json(&fixtures::valid_order_request_stateless())
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    let json: serde_json::Value = resp.json().await.unwrap();
    let order_id = json["order_id"].as_str().unwrap();
    assert!(!order_id.is_empty());

    // Query order status
    let resp = client
        .get(format!("{}/v1/orders/{}", server.base_url, order_id))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    let status_json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(status_json["order_id"], order_id);
    assert!(status_json["status"].is_string());

    server.abort();
}

#[tokio::test]
async fn test_orders_missing_user_address() {
    let server = TestServer::spawn().await;
    let client = Client::new();

    let resp = client
        .post(format!("{}/v1/orders", server.base_url))
        .json(&fixtures::invalid_order_request_missing_user())
        .send()
        .await
        .unwrap();

    // Serde deserialization failure -> 422
    assert_eq!(resp.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);

    server.abort();
}

#[tokio::test]
async fn test_orders_missing_quote_data() {
    let server = TestServer::spawn().await;
    let client = Client::new();

    let resp = client
        .post(format!("{}/v1/orders", server.base_url))
        .json(&fixtures::invalid_order_request_missing_quote())
        .send()
        .await
        .unwrap();

    // Business logic validation -> 400
    assert_eq!(resp.status(), reqwest::StatusCode::BAD_REQUEST);

    server.abort();
}

#[tokio::test]
async fn test_orders_quote_not_found() {
    let server = TestServer::spawn().await;
    let client = Client::new();

    let resp = client
        .post(format!("{}/v1/orders", server.base_url))
        .json(&fixtures::order_request_with_invalid_quote_id())
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), reqwest::StatusCode::NOT_FOUND);

    server.abort();
}

#[tokio::test]
async fn test_orders_nonexistent_order_status() {
    let server = TestServer::spawn().await;
    let client = Client::new();

    let resp = client
        .get(format!("{}/v1/orders/nonexistent-order-id", server.base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), reqwest::StatusCode::NOT_FOUND);

    server.abort();
}

#[tokio::test]
async fn test_orders_invalid_order_id_format() {
    let server = TestServer::spawn().await;
    let client = Client::new();

    // Test with special characters that might cause routing issues
    let invalid_ids = ["", "order/with/slashes", "order with spaces", "order<>"];

    for invalid_id in invalid_ids {
        let resp = client
            .get(format!("{}/v1/orders/{}", server.base_url, invalid_id))
            .send()
            .await
            .unwrap();

        // Either 404 (not found) or 400 (bad request) are acceptable
        assert!(resp.status().is_client_error());
    }

    server.abort();
}

#[tokio::test]
async fn test_orders_wrong_http_methods() {
    let server = TestServer::spawn().await;
    let client = Client::new();

    // GET to orders creation endpoint
    let resp = client
        .get(format!("{}/v1/orders", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::METHOD_NOT_ALLOWED);

    // POST to order status endpoint
    let resp = client
        .post(format!("{}/v1/orders/some-id", server.base_url))
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::METHOD_NOT_ALLOWED);

    server.abort();
}
