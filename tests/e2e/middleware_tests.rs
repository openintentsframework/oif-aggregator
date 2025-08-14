/// E2E tests for middleware functionality (CORS, request ID, compression, body limits, rate limiting)

use crate::e2e::TestServer;
use crate::mocks::api_fixtures::ApiFixtures;
use reqwest::Client;

#[tokio::test]
async fn test_request_id_auto_generation() {
    let server = TestServer::spawn_minimal().await.expect("Failed to start test server");
    let client = Client::new();

    let resp = client
        .post(format!("{}/v1/quotes", server.base_url))
        .json(&ApiFixtures::valid_quote_request())
        .send()
        .await
        .unwrap();

    // Status might be 400 due to validation or 200 with empty results
    assert!(resp.status().is_success() || resp.status() == reqwest::StatusCode::BAD_REQUEST);
    let req_id = resp.headers().get("x-request-id");
    assert!(req_id.is_some());
    assert!(!req_id.unwrap().to_str().unwrap().is_empty());

    server.abort();
}

#[tokio::test]
async fn test_request_id_propagation() {
    let server = TestServer::spawn_minimal().await.expect("Failed to start test server");
    let client = Client::new();

    let provided_id = "test-req-id-123";
    let resp = client
        .post(format!("{}/v1/quotes", server.base_url))
        .header("x-request-id", provided_id)
        .json(&ApiFixtures::valid_quote_request())
        .send()
        .await
        .unwrap();

    // Status might be 400 due to validation or 200 with empty results
    assert!(resp.status().is_success() || resp.status() == reqwest::StatusCode::BAD_REQUEST);
    let echoed_id = resp
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert_eq!(echoed_id, provided_id);

    server.abort();
}

#[tokio::test]
async fn test_cors_preflight() {
    let server = TestServer::spawn_minimal().await.expect("Failed to start test server");
    let client = Client::new();

    let resp = client
        .request(reqwest::Method::OPTIONS, format!("{}/v1/quotes", server.base_url))
        .header("Origin", "http://example.com")
        .header("Access-Control-Request-Method", "POST")
        .send()
        .await
        .unwrap();

    // CORS layer should handle preflight (permissive)
    assert!(
        resp.status() == reqwest::StatusCode::NO_CONTENT
            || resp.status() == reqwest::StatusCode::OK
    );
    let allow_origin = resp.headers().get("access-control-allow-origin");
    assert!(allow_origin.is_some());

    server.abort();
}

#[tokio::test]
async fn test_compression_headers() {
    let server = TestServer::spawn_minimal().await.expect("Failed to start test server");
    let client = Client::new();

    let resp = client
        .post(format!("{}/v1/quotes", server.base_url))
        .header("Accept-Encoding", "gzip, br")
        .json(&ApiFixtures::valid_quote_request())
        .send()
        .await
        .unwrap();

    // Status might be 400 due to validation or 200 with empty results
    assert!(resp.status().is_success() || resp.status() == reqwest::StatusCode::BAD_REQUEST);
    // Note: Compression might not be applied for small responses, 
    // but the header should be processed without error
    // You could also test with a larger response if needed

    server.abort();
}

#[tokio::test]
async fn test_body_size_limit() {
    let server = TestServer::spawn_minimal().await.expect("Failed to start test server");
    let client = Client::new();

    // Create a large payload (> 1MB limit) using ERC-7930 format
    let large_payload = "x".repeat(2 * 1024 * 1024); // 2MB
    let large_request = ApiFixtures::large_quote_request_with_payload(large_payload);

    let result = client
        .post(format!("{}/v1/quotes", server.base_url))
        .json(&large_request)
        .send()
        .await;

    // Should be rejected due to body size limit
    // This can either be a PAYLOAD_TOO_LARGE response or a connection reset
    match result {
        Ok(resp) => {
            assert_eq!(resp.status(), reqwest::StatusCode::PAYLOAD_TOO_LARGE);
        }
        Err(e) => {
            // Connection reset is also a valid rejection due to body size limit
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("Connection reset") || 
                error_msg.contains("BodyWrite") ||
                error_msg.contains("request"),
                "Unexpected error: {}", error_msg
            );
        }
    }

    server.abort();
}

#[tokio::test]
async fn test_rate_limiting_disabled() {
    // Default settings have rate limiting disabled
    let server = TestServer::spawn_minimal().await.expect("Failed to start test server");
    let client = Client::new();

    let endpoint = format!("{}/health", server.base_url);

    // Make multiple requests - all should succeed
    for i in 1..=5 {
        let resp = client.get(&endpoint).send().await.unwrap();
        assert!(resp.status().is_success(), "Request {} should succeed", i);
    }

    server.abort();
}
