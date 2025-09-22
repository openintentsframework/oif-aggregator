//! Solver adapter service for connecting to specific solvers
//!
//! This service represents a connection to a specific solver and its adapter.
//! It encapsulates the solver configuration and provides a clean interface
//! for interacting with that particular solver.

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use chrono::Utc;
use oif_adapters::AdapterRegistry;
use oif_storage::Storage;
use oif_types::adapters::models::{SubmitOrderRequest, SubmitOrderResponse};
use oif_types::adapters::{GetOrderResponse, GetQuoteResponse};
use oif_types::{GetQuoteRequest, Solver, SolverAdapter, SolverRuntimeConfig};
use thiserror::Error;

use crate::jobs::{BackgroundJob, JobScheduler, SolverMetricsUpdate};

#[derive(Debug, Error)]
pub enum SolverAdapterError {
	#[error("adapter error: {0}")]
	Adapter(String),
	#[error("adapter error with status {status_code}: {message}")]
	AdapterWithStatus { message: String, status_code: u16 },
	#[error("solver not found: {0}")]
	SolverNotFound(String),
	#[error("adapter not found for solver: {0}")]
	AdapterNotFound(String),
	#[error("storage error: {0}")]
	Storage(String),
}

impl SolverAdapterError {
	/// Extract HTTP status code from the error if available
	pub fn status_code(&self) -> Option<u16> {
		match self {
			SolverAdapterError::AdapterWithStatus { status_code, .. } => Some(*status_code),
			SolverAdapterError::Adapter(adapter_error_string) => {
				// Try to extract status code from adapter error string if it contains one
				// This is a fallback for when we can't get the actual AdapterError object
				Self::extract_status_from_string(adapter_error_string)
			},
			_ => None,
		}
	}

	/// Helper to extract status code from error string (fallback method)
	fn extract_status_from_string(error_str: &str) -> Option<u16> {
		// Look for patterns like "HTTP 404:" or "status 500"
		if let Some(start) = error_str.find("HTTP ") {
			if let Some(end) = error_str[start + 5..].find(':') {
				if let Ok(code) = error_str[start + 5..start + 5 + end].parse::<u16>() {
					return Some(code);
				}
			}
		}

		// Look for "status XXX" patterns
		if let Some(start) = error_str.find("status ") {
			let after_status = &error_str[start + 7..];
			if let Some(end) = after_status.find(|c: char| !c.is_ascii_digit()) {
				if let Ok(code) = after_status[..end].parse::<u16>() {
					return Some(code);
				}
			} else if let Ok(code) = after_status.parse::<u16>() {
				return Some(code);
			}
		}

		None
	}
}

/// Trait for solver adapter operations - enables easy mocking in tests
#[async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait SolverAdapterTrait: Send + Sync {
	/// Get quotes from this solver (automatically collects metrics if JobScheduler available)
	async fn get_quotes(
		&self,
		request: &GetQuoteRequest,
	) -> Result<GetQuoteResponse, SolverAdapterError>;

	/// Submit an order to this solver (automatically collects metrics if JobScheduler available)
	async fn submit_order(
		&self,
		request: &SubmitOrderRequest,
	) -> Result<SubmitOrderResponse, SolverAdapterError>;

	/// Get order details from this solver (automatically collects metrics if JobScheduler available)
	async fn get_order_details(
		&self,
		order_id: &str,
	) -> Result<GetOrderResponse, SolverAdapterError>;

	/// Perform health check on this solver (automatically collects metrics if JobScheduler available)
	async fn health_check(&self) -> Result<bool, SolverAdapterError>;

	/// Get what assets/routes this solver supports (automatically collects metrics if JobScheduler available)
	async fn get_supported_assets(
		&self,
	) -> Result<oif_types::SupportedAssetsData, SolverAdapterError>;

	/// Get the solver ID this service is connected to
	fn solver_id(&self) -> &str;
}

/// Service for interacting with a specific solver through its adapter
#[derive(Clone)]
pub struct SolverAdapterService {
	solver: Solver,
	config: SolverRuntimeConfig,
	solver_adapter: Arc<dyn SolverAdapter>,
	job_scheduler: Option<Arc<dyn JobScheduler>>,
}

impl SolverAdapterService {
	/// Create a new solver adapter service for a specific solver by ID
	pub async fn new(
		solver_id: &str,
		adapter_registry: Arc<AdapterRegistry>,
		storage: Arc<dyn Storage>,
		job_scheduler: Option<Arc<dyn JobScheduler>>,
	) -> Result<Self, SolverAdapterError> {
		// 1. Find the solver in storage
		let solver = storage
			.get_solver(solver_id)
			.await
			.map_err(|e| SolverAdapterError::Storage(e.to_string()))?
			.ok_or_else(|| SolverAdapterError::SolverNotFound(solver_id.to_string()))?;

		// 2. Verify the adapter exists
		let solver_adapter = adapter_registry.get(&solver.adapter_id).ok_or_else(|| {
			SolverAdapterError::AdapterNotFound(format!(
				"No adapter found for solver {} (adapter_id: {})",
				solver.solver_id, solver.adapter_id
			))
		})?;

		// 3. Create the service
		let config = SolverRuntimeConfig::from(&solver);
		Ok(Self {
			solver,
			config,
			solver_adapter,
			job_scheduler,
		})
	}

	/// Create a solver adapter service from an existing solver
	pub fn from_solver(
		solver: Solver,
		adapter_registry: Arc<AdapterRegistry>,
		job_scheduler: Option<Arc<dyn JobScheduler>>,
	) -> Result<Self, SolverAdapterError> {
		// Verify the adapter exists
		let solver_adapter = adapter_registry.get(&solver.adapter_id).ok_or_else(|| {
			SolverAdapterError::AdapterNotFound(format!(
				"No adapter found for solver {} (adapter_id: {})",
				solver.solver_id, solver.adapter_id
			))
		})?;

		let config = SolverRuntimeConfig::from(&solver);
		Ok(Self {
			solver,
			config,
			solver_adapter,
			job_scheduler,
		})
	}

	/// Get the adapter for this service's solver
	///
	/// This method is guaranteed to succeed because the adapter existence
	/// is verified during construction in both `new()` and `from_solver()`.
	/// We can safely unwrap here to avoid redundant error handling.
	fn get_adapter(&self) -> &dyn SolverAdapter {
		self.solver_adapter.as_ref()
	}

	/// Extract metrics data from a solver operation result
	fn extract_metrics_from_result<T>(
		&self,
		result: &Result<T, SolverAdapterError>,
		start_time: Instant,
		operation: &str,
	) -> SolverMetricsUpdate {
		let response_time_ms = start_time.elapsed().as_millis() as u64;
		let was_successful = result.is_ok();
		let timestamp = Utc::now();

		// Extract error information from the result
		let (error_message, status_code, was_timeout, error_type) = match result {
			Ok(_) => (None, None, false, None),
			Err(e) => {
				let error_message = Some(e.to_string());
				let status_code = e.status_code();
				let was_timeout = e.to_string().to_lowercase().contains("timeout");

				let error_type = if let Some(code) = status_code {
					// Use HTTP status code for precise categorization
					Some(oif_types::ErrorType::from_http_status(code))
				} else if was_timeout {
					// Timeout errors are service issues
					Some(oif_types::ErrorType::ServiceError)
				} else {
					// Default to unknown for other adapter errors
					Some(oif_types::ErrorType::Unknown)
				};

				(error_message, status_code, was_timeout, error_type)
			},
		};

		SolverMetricsUpdate {
			response_time_ms,
			was_successful,
			was_timeout,
			timestamp,
			error_message,
			status_code,
			error_type,
			operation: operation.to_string(),
		}
	}

	/// Schedule individual metrics job for this solver operation
	async fn schedule_metrics_job(&self, metrics: SolverMetricsUpdate, operation: &str) {
		// Only schedule if job scheduler is available
		let job_scheduler = match &self.job_scheduler {
			Some(scheduler) => scheduler,
			None => {
				tracing::debug!(
					"No job scheduler available for {} metrics collection on solver {}",
					operation,
					self.solver.solver_id
				);
				return;
			},
		};

		// Create individual metrics job
		let job_id = format!(
			"{}-{}-{}",
			operation,
			self.solver.solver_id,
			Utc::now().timestamp_millis()
		);

		let metrics_job = BackgroundJob::AggregationMetricsUpdate {
			aggregation_id: job_id.clone(),
			solver_metrics: vec![(self.solver.solver_id.clone(), metrics)],
			aggregation_timestamp: Utc::now(),
		};

		// Schedule the job immediately
		match job_scheduler
			.schedule_with_delay(metrics_job, std::time::Duration::from_millis(0), None)
			.await
		{
			Ok(scheduled_job_id) => {
				tracing::debug!(
					"Scheduled {} metrics job for solver {}: {}",
					operation,
					self.solver.solver_id,
					scheduled_job_id
				);
			},
			Err(e) => {
				tracing::warn!(
					"Failed to schedule {} metrics job for solver {}: {}",
					operation,
					self.solver.solver_id,
					e
				);
			},
		}
	}

	/// Helper to execute a solver operation with automatic metrics collection
	async fn execute_with_metrics<T, F, Fut>(
		&self,
		operation: &str,
		call: F,
	) -> Result<T, SolverAdapterError>
	where
		F: FnOnce() -> Fut,
		Fut: std::future::Future<Output = Result<T, SolverAdapterError>>,
	{
		let start_time = Instant::now();
		let result = call().await;

		// Extract and schedule metrics
		let metrics = self.extract_metrics_from_result(&result, start_time, operation);
		self.schedule_metrics_job(metrics, operation).await;

		result
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_solver_adapter_error_status_code_extraction() {
		// Test status code extraction from AdapterWithStatus
		let error = SolverAdapterError::AdapterWithStatus {
			message: "Test error".to_string(),
			status_code: 404,
		};
		assert_eq!(error.status_code(), Some(404));

		// Test status code extraction from Adapter error string with "HTTP XXX:" pattern
		let error = SolverAdapterError::Adapter("HTTP 500: Internal Server Error".to_string());
		assert_eq!(error.status_code(), Some(500));

		// Test status code extraction from Adapter error string with "status XXX" pattern
		let error = SolverAdapterError::Adapter("Request failed with status 429".to_string());
		assert_eq!(error.status_code(), Some(429));

		// Test no status code extraction for other error types
		let error = SolverAdapterError::SolverNotFound("test-solver".to_string());
		assert_eq!(error.status_code(), None);

		// Test no status code extraction for Adapter error without status
		let error = SolverAdapterError::Adapter("Some other error".to_string());
		assert_eq!(error.status_code(), None);
	}
}

#[async_trait]
impl SolverAdapterTrait for SolverAdapterService {
	/// Get quotes from this solver (automatically collects metrics if JobScheduler available)
	async fn get_quotes(
		&self,
		request: &GetQuoteRequest,
	) -> Result<GetQuoteResponse, SolverAdapterError> {
		self.execute_with_metrics("get_quotes", || async {
			let adapter = self.get_adapter();
			adapter
				.get_quotes(request, &self.config)
				.await
				.map_err(|e| SolverAdapterError::Adapter(e.to_string()))
		})
		.await
	}

	/// Submit an order to this solver (automatically collects metrics if JobScheduler available)
	async fn submit_order(
		&self,
		request: &SubmitOrderRequest,
	) -> Result<SubmitOrderResponse, SolverAdapterError> {
		self.execute_with_metrics("submit_order", || async {
			let adapter = self.get_adapter();
			adapter
				.submit_order(request, &self.config)
				.await
				.map_err(|e| SolverAdapterError::Adapter(e.to_string()))
		})
		.await
	}

	/// Get order details from this solver (automatically collects metrics if JobScheduler available)
	async fn get_order_details(
		&self,
		order_id: &str,
	) -> Result<GetOrderResponse, SolverAdapterError> {
		self.execute_with_metrics("get_order_details", || async {
			let adapter = self.get_adapter();
			adapter
				.get_order_details(order_id, &self.config)
				.await
				.map_err(|e| SolverAdapterError::Adapter(e.to_string()))
		})
		.await
	}

	/// Perform health check on this solver (automatically collects metrics if JobScheduler available)
	async fn health_check(&self) -> Result<bool, SolverAdapterError> {
		self.execute_with_metrics("health_check", || async {
			let adapter = self.get_adapter();
			adapter
				.health_check(&self.config)
				.await
				.map_err(|e| SolverAdapterError::Adapter(e.to_string()))
		})
		.await
	}

	/// Get what assets/routes this solver supports (automatically collects metrics if JobScheduler available)
	async fn get_supported_assets(
		&self,
	) -> Result<oif_types::SupportedAssetsData, SolverAdapterError> {
		self.execute_with_metrics("get_supported_assets", || async {
			let adapter = self.get_adapter();
			adapter
				.get_supported_assets(&self.config)
				.await
				.map_err(|e| SolverAdapterError::Adapter(e.to_string()))
		})
		.await
	}

	/// Get the solver ID this service is connected to
	fn solver_id(&self) -> &str {
		&self.solver.solver_id
	}
}
