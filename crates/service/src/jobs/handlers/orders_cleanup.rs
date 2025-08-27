//! Orders cleanup handler for background jobs

use async_trait::async_trait;
use std::sync::Arc;

use crate::jobs::generic_handler::GenericJobHandler;
use crate::jobs::types::JobResult;
use crate::order::{OrderServiceError, OrderServiceTrait};

/// Parameters for order cleanup operations
#[derive(Debug, Clone)]
pub struct OrdersCleanupParams {
	/// How many days to retain orders in final status
	pub retention_days: u32,
}

impl OrdersCleanupParams {
	/// Create new cleanup params with specified retention period
	pub fn new(retention_days: u32) -> Self {
		Self { retention_days }
	}
}

/// Handler for cleaning up old orders in final status
pub struct OrdersCleanupHandler {
	order_service: Arc<dyn OrderServiceTrait>,
}

impl OrdersCleanupHandler {
	/// Create a new orders cleanup handler
	pub fn new(order_service: Arc<dyn OrderServiceTrait>) -> Self {
		Self { order_service }
	}
}

#[async_trait]
impl GenericJobHandler<OrdersCleanupParams> for OrdersCleanupHandler {
	async fn handle(&self, params: OrdersCleanupParams) -> JobResult<()> {
		tracing::info!(
			"Starting order cleanup for orders older than {} days",
			params.retention_days
		);

		match self
			.order_service
			.cleanup_old_orders(params.retention_days)
			.await
		{
			Ok(deleted_count) => {
				tracing::info!(
					"Order cleanup completed successfully: {} orders deleted",
					deleted_count
				);
				Ok(())
			},
			Err(OrderServiceError::Storage(e)) => {
				let error_msg = format!("Storage error during order cleanup: {}", e);
				tracing::error!("{}", error_msg);
				Err(crate::jobs::types::JobError::Storage(error_msg))
			},
			Err(e) => {
				let error_msg = format!("Order cleanup failed: {}", e);
				tracing::error!("{}", error_msg);
				Err(crate::jobs::types::JobError::ProcessingFailed { message: error_msg })
			},
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::order::MockOrderServiceTrait;

	#[tokio::test]
	async fn test_orders_cleanup_handler() {
		let mut mock_order_service = MockOrderServiceTrait::new();

		// Setup expectation
		mock_order_service
			.expect_cleanup_old_orders()
			.with(mockall::predicate::eq(10))
			.times(1)
			.returning(|_| Ok(5));

		let handler = OrdersCleanupHandler::new(Arc::new(mock_order_service));
		let params = OrdersCleanupParams::new(10);

		let result = handler.handle(params).await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_orders_cleanup_handler_with_custom_retention() {
		let mut mock_order_service = MockOrderServiceTrait::new();

		mock_order_service
			.expect_cleanup_old_orders()
			.with(mockall::predicate::eq(30))
			.times(1)
			.returning(|_| Ok(2));

		let handler = OrdersCleanupHandler::new(Arc::new(mock_order_service));
		let params = OrdersCleanupParams { retention_days: 30 };

		let result = handler.handle(params).await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_orders_cleanup_handler_storage_error() {
		let mut mock_order_service = MockOrderServiceTrait::new();

		mock_order_service
			.expect_cleanup_old_orders()
			.returning(|_| {
				Err(crate::order::OrderServiceError::Storage(
					"Storage error".to_string(),
				))
			});

		let handler = OrdersCleanupHandler::new(Arc::new(mock_order_service));
		let params = OrdersCleanupParams::new(10);

		let result = handler.handle(params).await;
		assert!(result.is_err());

		if let Err(crate::jobs::types::JobError::Storage(msg)) = result {
			assert!(msg.contains("Storage error"));
		} else {
			panic!("Expected Storage error");
		}
	}
}
