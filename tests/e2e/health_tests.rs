/// E2E tests for health and readiness endpoints

use crate::e2e::TestServer;
use reqwest::Client;

#[tokio::test]
async fn test_health_endpoint() {
    let server = TestServer::spawn().await.expect("Failed to start test server");
    let client = Client::new();

    let resp = client
        .get(format!("{}/health", server.base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    let body = resp.text().await.unwrap();
    assert_eq!(body, "OK");

    server.abort();
}

#[tokio::test]
async fn test_readiness_endpoint() {
    let server = TestServer::spawn().await.expect("Failed to start test server");
    let client = Client::new();

    let resp = client
        .get(format!("{}/ready", server.base_url))
        .send()
        .await
        .unwrap();

    // With empty solvers and healthy memory storage, should be ready
    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["status"], "ready");
    assert_eq!(json["storage_healthy"], true);

    server.abort();
}

#[tokio::test]
async fn test_unknown_endpoint_404() {
    let server = TestServer::spawn().await.expect("Failed to start test server");
    let client = Client::new();

    let resp = client
        .get(format!("{}/unknown-endpoint", server.base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), reqwest::StatusCode::NOT_FOUND);

    server.abort();
}
