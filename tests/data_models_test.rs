//! Data model tests

use oif_types::{chrono::Utc, orders::OrderStatus};

mod mocks;
use mocks::{entities::MockEntities, entities::TestConstants};

#[test]
fn test_quote_creation() {
	let quote = MockEntities::quote();

	assert_eq!(quote.solver_id, "test-solver-1");
	assert_eq!(quote.provider, "Test Provider");
	assert_eq!(quote.integrity_checksum, "test-checksum");
	assert!(!quote.orders.is_empty());
	assert!(!quote.details.available_inputs.is_empty());
	assert!(!quote.details.requested_outputs.is_empty());
}

#[test]
fn test_quote_expiration() {
	// Test expired quote
	let expired_quote = MockEntities::expired_quote();
	assert!(expired_quote.valid_until.is_some());
	let expiry = expired_quote.valid_until.unwrap() as i64;
	let now = Utc::now().timestamp();
	assert!(expiry < now, "Quote should be expired");

	// Test expiring quote (5 seconds from now)
	let expiring_quote = MockEntities::expiring_quote(5);
	assert!(expiring_quote.valid_until.is_some());
	let expiry = expiring_quote.valid_until.unwrap() as i64;
	let now = Utc::now().timestamp();
	assert!(expiry > now, "Quote should not be expired yet");
	assert!(expiry <= now + 10, "Quote should expire within 10 seconds");
}

#[test]
fn test_quote_request_creation() {
	let request = MockEntities::quote_request();

	assert_eq!(request.available_inputs.len(), 1);
	assert_eq!(request.requested_outputs.len(), 1);
	assert_eq!(request.min_valid_until, Some(300));

	// Verify it uses our test constants
	let input = &request.available_inputs[0];
	let output = &request.requested_outputs[0];

	assert_eq!(input.amount.to_string(), TestConstants::ONE_ETH_WEI);
	assert_eq!(output.amount.to_string(), TestConstants::TWO_THOUSAND_USDC);
}

#[test]
fn test_order_creation() {
	let order = MockEntities::order();

	assert_eq!(order.order_id, "test-order-123");
	assert_eq!(order.quote_id, Some("test-quote-123".to_string()));
	assert_eq!(order.status, OrderStatus::Created);

	// Test custom amounts
	let custom_order = MockEntities::order_with_amounts("2000000000000000000", "3000000");
	assert_eq!(
		custom_order.input_amount.amount.to_string(),
		"2000000000000000000"
	);
	assert_eq!(custom_order.output_amount.amount.to_string(), "3000000");
}

#[test]
fn test_order_status_transitions() {
	// Test different order statuses
	let created_order = MockEntities::order_with_status(OrderStatus::Created);
	assert_eq!(created_order.status, OrderStatus::Created);

	let pending_order = MockEntities::order_with_status(OrderStatus::Pending);
	assert_eq!(pending_order.status, OrderStatus::Pending);

	let executed_order = MockEntities::order_with_status(OrderStatus::Executed);
	assert_eq!(executed_order.status, OrderStatus::Executed);

	let failed_order = MockEntities::order_with_status(OrderStatus::Failed);
	assert_eq!(failed_order.status, OrderStatus::Failed);
}
