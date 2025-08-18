//! Tests for adapter system and HTTP functionality

use std::sync::Arc;

use oif_adapters::AdapterRegistry;
use oif_aggregator::{service::IntegrityService, AggregatorBuilder};
use oif_service::aggregator::AggregatorService;
use oif_types::orders::OrderStatus;

mod mocks;

use mocks::{configs::MockConfigs, entities::MockEntities};

use crate::mocks::api_fixtures::INTEGRITY_SECRET;

#[test]
fn test_adapter_registry_creation() {
	let registry = AdapterRegistry::new();
	// Registry starts with default adapters (OIF and LiFi)
	assert!(registry.get_all().is_empty());
}

#[test]
fn test_adapter_registry_with_defaults() {
	let registry = AdapterRegistry::with_defaults();
	// Should have default OIF and LiFi adapters
	assert!(registry.get_all().len() >= 2);
}

#[tokio::test]
async fn test_aggregator_builder_with_mock_adapter() {
	// Set required environment variable for tests
	std::env::set_var("INTEGRITY_SECRET", INTEGRITY_SECRET);

	let mock_adapter = oif_aggregator::mocks::MockDemoAdapter::new();
	let mock_solver = oif_aggregator::mocks::mock_solver();

	let result = AggregatorBuilder::default()
		.with_adapter(Box::new(mock_adapter))
		.with_solver(mock_solver)
		.start()
		.await;

	assert!(result.is_ok());

	let (_app, state) = result.unwrap();
	// Verify we can get solver count
	let (_solvers, total_count, _, _) = state
		.solver_service
		.list_solvers_paginated(None, None)
		.await
		.unwrap();
	assert_eq!(total_count, 1);
}

#[test]
fn test_quote_request_creation() {
	// Use mock quote request from fixtures
	let request = MockEntities::quote_request();

	assert_eq!(request.available_inputs.len(), 1);
	assert_eq!(request.requested_outputs.len(), 1);
	assert!(request.min_valid_until.is_some());
}

#[test]
fn test_order_creation() {
	// Use mock order from fixtures
	let order = MockEntities::order();

	assert!(order.order_id.starts_with("test-order-"));
	assert!(order.quote_id.is_some());
	assert_eq!(order.status, OrderStatus::Created);
}

#[tokio::test]
async fn test_aggregation_service_creation() {
	let solvers = vec![];
	let adapter_registry = Arc::new(AdapterRegistry::new());
	let integrity_service = Arc::new(IntegrityService::new(oif_types::SecretString::from_string(
		"test-secret",
	)));

	let service = AggregatorService::new(
		solvers,
		adapter_registry,
		5000,
		integrity_service,
		Arc::new(oif_service::SolverFilterService::new())
			as Arc<dyn oif_service::SolverFilterTrait>,
	);

	// Just verify service was created successfully
	// We can't format debug or get stats, so just verify it exists
	let _ = service; // Service created successfully
}

#[test]
fn test_adapter_error_types() {
	use oif_adapters::AdapterError;

	let not_found_error = AdapterError::NotFound {
		adapter_id: "missing-adapter".to_string(),
	};
	assert!(not_found_error.to_string().contains("Adapter not found"));

	let timeout_error = AdapterError::Timeout { timeout_ms: 5000 };
	assert!(timeout_error.to_string().contains("Timeout occurred"));

	let disabled_error = AdapterError::Disabled {
		adapter_id: "disabled-adapter".to_string(),
	};
	assert!(disabled_error.to_string().contains("Adapter is disabled"));
}

#[tokio::test]
async fn test_quote_aggregation_with_mock() {
	// Set required environment variable for tests
	std::env::set_var("INTEGRITY_SECRET", INTEGRITY_SECRET);

	let mock_adapter = oif_aggregator::mocks::MockDemoAdapter::new();
	let mock_solver = oif_aggregator::mocks::mock_solver();

	let builder_result = AggregatorBuilder::default()
		.with_adapter(Box::new(mock_adapter))
		.with_solver(mock_solver)
		.start()
		.await;

	assert!(builder_result.is_ok());

	let (_app, state) = builder_result.unwrap();

	// Use mock quote request from fixtures
	let request = MockEntities::quote_request();

	let quotes_result = state.aggregator_service.fetch_quotes(request).await;
	assert!(quotes_result.is_ok());

	let (quotes, metadata) = quotes_result.unwrap();
	// Should get at least one quote from mock adapter
	assert!(!quotes.is_empty());
	// Verify metadata was returned with meaningful values
	assert!(metadata.solvers_queried > 0);
	assert_eq!(
		metadata.solver_selection_mode,
		oif_types::quotes::request::SolverSelection::All
	);
}

#[tokio::test]
async fn test_health_check_with_solver_service() {
	// Set required environment variable for tests
	std::env::set_var("INTEGRITY_SECRET", INTEGRITY_SECRET);

	let mock_adapter = oif_aggregator::mocks::MockDemoAdapter::new();
	let mock_solver = oif_aggregator::mocks::mock_solver();

	let builder_result = AggregatorBuilder::default()
		.with_adapter(Box::new(mock_adapter))
		.with_solver(mock_solver)
		.start()
		.await;

	assert!(builder_result.is_ok());

	let (_app, state) = builder_result.unwrap();

	// Test health check through solver service
	let health_result = state.solver_service.health_check_all().await;
	assert!(health_result.is_ok());

	let health_statuses = health_result.unwrap();
	assert_eq!(health_statuses.len(), 1); // One solver configured
}

#[tokio::test]
async fn test_adapter_registry_duplicate_prevention() {
	let mock_adapter1 = oif_aggregator::mocks::MockDemoAdapter::new();
	let mock_adapter2 = oif_aggregator::mocks::MockDemoAdapter::new(); // Same ID

	// Should panic when trying to register duplicate adapter
	let result = std::panic::catch_unwind(|| {
		AggregatorBuilder::default()
			.with_adapter(Box::new(mock_adapter1))
			.with_adapter(Box::new(mock_adapter2)) // Should panic
	});

	assert!(result.is_err());
	// Duplicate adapter should be rejected
}

#[test]
fn test_solver_config_creation() {
	let config = MockConfigs::test_solver_config();

	assert_eq!(config.solver_id, "test-solver");
	assert_eq!(config.adapter_id, "test-adapter");
	assert_eq!(config.endpoint, "http://localhost:8080");
	assert_eq!(config.timeout_ms, 1000);
	assert!(config.enabled);
}
