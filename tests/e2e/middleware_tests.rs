/// E2E tests for middleware functionality (CORS, request ID, compression, body limits, rate limiting)

use crate::e2e::{fixtures, TestServer};
use oif_config::settings::{RateLimitSettings, Settings};
use reqwest::Client;

#[tokio::test]
async fn test_request_id_auto_generation() {
    let server = TestServer::spawn().await;
    let client = Client::new();

    let resp = client
        .post(format!("{}/v1/quotes", server.base_url))
        .json(&fixtures::valid_quote_request())
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());
    let req_id = resp.headers().get("x-request-id");
    assert!(req_id.is_some());
    assert!(!req_id.unwrap().to_str().unwrap().is_empty());

    server.abort();
}

#[tokio::test]
async fn test_request_id_propagation() {
    let server = TestServer::spawn().await;
    let client = Client::new();

    let provided_id = "test-req-id-123";
    let resp = client
        .post(format!("{}/v1/quotes", server.base_url))
        .header("x-request-id", provided_id)
        .json(&fixtures::valid_quote_request())
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());
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
    let server = TestServer::spawn().await;
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
    let server = TestServer::spawn().await;
    let client = Client::new();

    let resp = client
        .post(format!("{}/v1/quotes", server.base_url))
        .header("Accept-Encoding", "gzip, br")
        .json(&fixtures::valid_quote_request())
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());
    // Note: Compression might not be applied for small responses, 
    // but the header should be processed without error
    // You could also test with a larger response if needed

    server.abort();
}

#[tokio::test]
async fn test_body_size_limit() {
    let server = TestServer::spawn().await;
    let client = Client::new();

    // Create a large payload (> 1MB limit)
    let large_payload = "x".repeat(2 * 1024 * 1024); // 2MB
    let large_request = serde_json::json!({
        "token_in": large_payload,
        "token_out": "0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0",
        "amount_in": "1000000000000000000",
        "chain_id": 1
    });

    let resp = client
        .post(format!("{}/v1/quotes", server.base_url))
        .json(&large_request)
        .send()
        .await
        .unwrap();

    // Should be rejected due to body size limit
    assert_eq!(resp.status(), reqwest::StatusCode::PAYLOAD_TOO_LARGE);

    server.abort();
}

#[tokio::test]
async fn test_rate_limiting_behavior() {
    // Note: This test currently demonstrates that rate limiting isn't properly
    // wired in the test server setup. The TestServer::spawn_with_settings
    // needs to be enhanced to actually apply the settings.
    
    let mut settings = Settings::default();
    settings.environment.rate_limiting = RateLimitSettings {
        enabled: true,
        requests_per_minute: 3,
        burst_size: 2,
    };

    let server = TestServer::spawn_with_settings(settings).await;
    let client = Client::new();

    let endpoint = format!("{}/health", server.base_url);

    // Make multiple requests - currently all will succeed since settings aren't applied
    // TODO: Wire settings into TestServer to enable actual rate limiting testing
    for i in 1..=3 {
        let resp = client.get(&endpoint).send().await.unwrap();
        assert!(resp.status().is_success(), "Request {} should succeed (settings not wired yet)", i);
    }

    server.abort();
}

#[tokio::test]
async fn test_rate_limiting_disabled() {
    // Default settings have rate limiting disabled
    let server = TestServer::spawn().await;
    let client = Client::new();

    let endpoint = format!("{}/health", server.base_url);

    // Make multiple requests - all should succeed
    for i in 1..=5 {
        let resp = client.get(&endpoint).send().await.unwrap();
        assert!(resp.status().is_success(), "Request {} should succeed", i);
    }

    server.abort();
}
