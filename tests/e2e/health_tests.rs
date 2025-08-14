/// E2E tests for health endpoint

use crate::e2e::TestServer;
use reqwest::Client;
use serde_json::Value;

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
    
    // Parse JSON response
    let json_body: Value = resp.json().await.unwrap();
    
    // Verify the response structure
    assert!(json_body.get("status").is_some());
    assert!(json_body.get("version").is_some());
    assert!(json_body.get("solvers").is_some());
    assert!(json_body.get("storage").is_some());
    
    // Verify solvers structure contains expected fields
    let solvers = json_body.get("solvers").unwrap();
    assert!(solvers.get("total").is_some());
    assert!(solvers.get("active").is_some());
    assert!(solvers.get("inactive").is_some());
    assert!(solvers.get("healthy").is_some());
    assert!(solvers.get("unhealthy").is_some());
    assert!(solvers.get("health_details").is_some());
    
    // Verify storage structure
    let storage = json_body.get("storage").unwrap();
    assert!(storage.get("healthy").is_some());
    assert!(storage.get("backend").is_some());
    
    // Verify version matches package version
    assert_eq!(json_body["version"].as_str().unwrap(), env!("CARGO_PKG_VERSION"));

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
