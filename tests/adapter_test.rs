//! Tests for adapter system and HTTP functionality

use chrono::Utc;
use oif_aggregator::adapters::{AdapterError, AdapterFactory};
use oif_aggregator::models::adapters::{AdapterConfig, AdapterType};
use oif_aggregator::models::intents::IntentStatus;
use oif_aggregator::models::{Intent, QuoteRequest};
use oif_aggregator::service::aggregator::AggregatorService;
use serde_json::json;

/// Create a mock adapter configuration for testing
fn create_test_adapter_config() -> AdapterConfig {
    AdapterConfig {
        adapter_id: "test-oif-adapter".to_string(),
        adapter_type: AdapterType::OifV1,
        name: "Test OIF Adapter".to_string(),
        description: Some("Test adapter for unit testing".to_string()),
        version: "1.0.0".to_string(),
        supported_chains: vec![1, 137], // Ethereum and Polygon
        configuration: json!({
            "endpoint": "https://httpbin.org/json",
            "timeout_ms": 5000,
            "headers": {
                "X-Test-Header": "test-value"
            }
        }),
        enabled: true,
        created_at: Utc::now(),
        timeout_ms: Some(5000),
        max_retries: Some(3),
        rate_limit: None,
        headers: None,
        required_capabilities: vec!["quotes".to_string()],
        optional_capabilities: vec!["intents".to_string()],
    }
}

#[test]
fn test_adapter_factory_creation() {
    let factory = AdapterFactory::new();
    assert_eq!(factory.get_all().len(), 0);
}

#[test]
fn test_adapter_config_creation() {
    let config = create_test_adapter_config();
    assert_eq!(config.adapter_id, "test-oif-adapter");
    assert_eq!(config.adapter_type, AdapterType::OifV1);
    assert_eq!(config.supported_chains, vec![1, 137]);
    assert!(config.enabled);
}

#[tokio::test]
async fn test_oif_adapter_creation() {
    let config = create_test_adapter_config();

    let adapter_result = AdapterFactory::create_from_config(&config);
    assert!(adapter_result.is_ok());

    let adapter = adapter_result.unwrap();
    assert_eq!(adapter.adapter_info().adapter_id, "test-oif-adapter");
    assert_eq!(adapter.supported_chains(), &[1, 137]);
}

#[tokio::test]
async fn test_adapter_health_check() {
    let config = create_test_adapter_config();

    if let Ok(adapter) = AdapterFactory::create_from_config(&config) {
        // Health check to httpbin.org should fail (no /health endpoint)
        let health_result = adapter.health_check().await;
        assert!(health_result.is_ok());
        // httpbin.org/health doesn't exist, so should return false
        assert!(!health_result.unwrap());
    }
}

#[test]
fn test_quote_request_creation() {
    let request = QuoteRequest::new(
        "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(), // WETH
        "0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0".to_string(), // USDC
        "1000000000000000000".to_string(),                        // 1 ETH
        1,                                                        // Ethereum mainnet
    );

    assert!(!request.request_id.is_empty());
    assert_eq!(request.chain_id, 1);
    assert_eq!(request.slippage_tolerance, Some(0.005));
}

#[test]
fn test_intent_creation() {
    let intent = Intent::new(
        "0x1234567890123456789012345678901234567890".to_string(),
        0.005,                                   // 0.5% slippage tolerance
        Utc::now() + chrono::Duration::hours(1), // 1 hour deadline
    );

    assert!(!intent.intent_id.is_empty());
    assert_eq!(intent.status, IntentStatus::Pending);
    assert_eq!(intent.slippage_tolerance, 0.005);
}

#[test]
fn test_aggregation_service_creation() {
    let solvers = vec![];
    let service = AggregatorService::new(solvers, 5000);

    let stats = service.get_stats();
    assert_eq!(stats.total_solvers, 0);
    assert_eq!(stats.global_timeout_ms, 5000);
}

#[test]
fn test_adapter_error_types() {
    let config_error = AdapterError::ConfigError("Test config error".to_string());
    assert!(config_error.to_string().contains("Configuration error"));

    let not_found_error = AdapterError::AdapterNotFound {
        adapter_id: "missing-adapter".to_string(),
    };
    assert!(not_found_error.to_string().contains("Adapter not found"));

    let timeout_error = AdapterError::Timeout { timeout_ms: 5000 };
    assert!(timeout_error.to_string().contains("Timeout occurred"));
}

#[test]
fn test_unsupported_adapter_types() {
    let mut config = create_test_adapter_config();

    // Test unsupported adapter types
    config.adapter_type = AdapterType::UniswapV2;
    let result = AdapterFactory::create_from_config(&config);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), AdapterError::ConfigError(_)));

    config.adapter_type = AdapterType::OneInch;
    let result = AdapterFactory::create_from_config(&config);
    assert!(result.is_err());

    config.adapter_type = AdapterType::Custom;
    let result = AdapterFactory::create_from_config(&config);
    assert!(result.is_err());
}

#[test]
fn test_adapter_config_validation() {
    let mut config = create_test_adapter_config();

    // Remove required endpoint
    config.configuration = json!({
        "timeout_ms": 5000
    });

    let result = AdapterFactory::create_from_config(&config);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), AdapterError::ConfigError(_)));
}

#[tokio::test]
async fn test_empty_quote_aggregation() {
    let solvers = vec![];
    let service = AggregatorService::new(solvers, 5000);

    let request = QuoteRequest::new(
        "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
        "0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0".to_string(),
        "1000000000000000000".to_string(),
        1,
    );

    let quotes = service.fetch_quotes(request).await;
    assert_eq!(quotes.len(), 0);
}

#[tokio::test]
async fn test_health_check_empty_service() {
    let solvers = vec![];
    let service = AggregatorService::new(solvers, 5000);

    let health_results = service.health_check_all().await;
    assert_eq!(health_results.len(), 0);
}

// Integration test with mock HTTP server would go here
// This would require a test HTTP server like wiremock
// For now, we test the structure and error handling
