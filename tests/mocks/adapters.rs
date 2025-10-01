//! Mock adapters for examples and testing
//!
//! This module provides simple, working mock adapters that can be used
//! in examples and tests without complex dependencies.

#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use oif_types::chrono::Utc;

use oif_types::adapters::{traits::SolverAdapter, AdapterResult, AdapterValidationError};
use oif_types::oif::{
	common::{
		AssetAmount, Input, OrderStatus, Output, PostOrderResponseStatus, Settlement,
		SettlementType, SignatureType,
	},
	v0::{
		GetOrderResponse, GetQuoteResponse, Order, OrderPayload,
		PostOrderResponse as SubmitOrderResponse, Quote,
	},
};
use oif_types::serde_json::json;
use oif_types::{models::AssetRoute, InteropAddress, SolverRuntimeConfig, U256};

/// Mock adapter that combines testing features
///
/// This adapter provides:
/// - Call tracking for testing
/// - Configurable response delays for timeout testing
/// - Failure simulation for circuit breaker testing
/// - Asset route support for compatibility testing
/// - OIF v0 compatibility
#[derive(Debug, Clone)]
pub struct MockAdapter {
	pub adapter: oif_types::Adapter,
	call_tracker: Arc<AtomicUsize>,
	pub should_fail: bool,
	pub response_delay_ms: u64,
	pub asset_mode: AssetMode,
}

/// How the adapter should report its supported assets
#[derive(Debug, Clone, PartialEq)]
pub enum AssetMode {
	/// Report specific asset routes
	Routes,
	/// Report empty asset list
	Empty,
	/// Report assets
	Assets,
}

impl MockAdapter {
	/// Create a new unified mock adapter with default settings
	pub fn new() -> Self {
		Self::with_config("unified-mock".to_string(), false, 0, AssetMode::Routes)
	}

	/// Create a mock adapter with custom configuration
	pub fn with_config(
		id: String,
		should_fail: bool,
		response_delay_ms: u64,
		asset_mode: AssetMode,
	) -> Self {
		Self {
			adapter: oif_types::Adapter {
				adapter_id: id.clone(),
				name: format!("{} Adapter", id),
				description: Some(format!("Unified mock adapter: {}", id)),
				version: "1.0.0".to_string(),
				configuration: HashMap::new(),
			},
			call_tracker: Arc::new(AtomicUsize::new(0)),
			should_fail,
			response_delay_ms,
			asset_mode,
		}
	}

	/// Create a fast-responding adapter (100ms delay)
	pub fn fast(id: &str) -> Self {
		Self::with_config(format!("fast-{}", id), false, 100, AssetMode::Routes)
	}

	/// Create a slow-responding adapter (1500ms delay)
	pub fn slow(id: &str) -> Self {
		Self::with_config(format!("slow-{}", id), false, 1500, AssetMode::Routes)
	}

	/// Create a timeout adapter (5000ms delay)
	pub fn timeout(id: &str) -> Self {
		Self::with_config(format!("timeout-{}", id), false, 5000, AssetMode::Routes)
	}

	/// Create a failing adapter
	pub fn failing(id: &str) -> Self {
		Self::with_config(format!("failing-{}", id), true, 100, AssetMode::Routes)
	}

	/// Create a success adapter (no delay, no failure)
	pub fn success(id: &str) -> Self {
		Self::with_config(format!("success-{}", id), false, 0, AssetMode::Routes)
	}

	/// Get the number of times this adapter has been called
	pub fn call_count(&self) -> usize {
		self.call_tracker.load(Ordering::Relaxed)
	}

	/// Reset the call counter
	pub fn reset_calls(&self) {
		self.call_tracker.store(0, Ordering::Relaxed);
	}
}

impl Default for MockAdapter {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl SolverAdapter for MockAdapter {
	fn adapter_info(&self) -> &oif_types::Adapter {
		&self.adapter
	}

	async fn get_quotes(
		&self,
		request: &oif_types::OifGetQuoteRequest,
		_config: &SolverRuntimeConfig,
	) -> AdapterResult<oif_types::OifGetQuoteResponse> {
		// Track the call
		self.call_tracker.fetch_add(1, Ordering::Relaxed);

		// Simulate processing delay
		if self.response_delay_ms > 0 {
			tokio::time::sleep(Duration::from_millis(self.response_delay_ms)).await;
		}

		// Check if this adapter should fail
		if self.should_fail {
			return Err(AdapterValidationError::InvalidConfiguration {
				reason: format!("Adapter {} configured to fail", self.adapter.adapter_id),
			}
			.into());
		}

		// Get inputs and outputs from the request
		let inputs = &request.intent().inputs;
		let outputs = &request.intent().outputs;

		let available_input = inputs.first().cloned().unwrap_or_else(|| Input {
			user: InteropAddress::from_chain_and_address(
				1,
				"0x742d35Cc6634C0532925a3b8D2a27F79c5a85b03",
			)
			.unwrap(),
			asset: InteropAddress::from_chain_and_address(
				1,
				"0x0000000000000000000000000000000000000000",
			)
			.unwrap(),
			amount: Some(U256::new("1000000000000000000".to_string())),
			lock: None,
		});

		let requested_output = outputs.first().cloned().unwrap_or_else(|| Output {
			asset: InteropAddress::from_chain_and_address(
				1,
				"0xa0b86a33e6417a77c9a0c65f8e69b8b6e2b0c4a0",
			)
			.unwrap(),
			amount: Some(U256::new("1000000".to_string())),
			receiver: InteropAddress::from_chain_and_address(
				1,
				"0x742d35Cc6634C0532925a3b8D2a27F79c5a85b03",
			)
			.unwrap(),
			calldata: None,
		});

		let quote = Quote {
			quote_id: Some(format!(
				"{}-quote-{}",
				self.adapter.adapter_id,
				Utc::now().timestamp()
			)),
			order: Order::OifEscrowV0 {
				payload: OrderPayload {
					signature_type: SignatureType::Eip712,
					domain: json!({ "name": "MockAdapter", "version": "1", "chainId": 1 }),
					primary_type: "Order".to_string(),
					message: json!({
						"orderType": "swap",
						"inputAsset": available_input.asset.to_hex(),
						"outputAsset": requested_output.asset.to_hex(),
						"adapter": self.adapter.name
					}),
					types: HashMap::new(),
				},
			},
			valid_until: Some(Utc::now().timestamp() as u64 + 300),
			eta: Some(30),
			provider: Some(format!("{} Provider", self.adapter.name)),
			failure_handling: None,
			partial_fill: false,
			preview: oif_types::oif::common::QuotePreview {
				inputs: vec![available_input],
				outputs: vec![requested_output],
			},
			metadata: Some(json!({
				"adapter_id": self.adapter.adapter_id,
				"response_delay_ms": self.response_delay_ms,
			})),
		};

		Ok(oif_types::OifGetQuoteResponse::V0(GetQuoteResponse {
			quotes: vec![quote],
		}))
	}

	async fn submit_order(
		&self,
		_request: &oif_types::OifPostOrderRequest,
		_config: &SolverRuntimeConfig,
	) -> AdapterResult<oif_types::OifPostOrderResponse> {
		// Track the call
		self.call_tracker.fetch_add(1, Ordering::Relaxed);

		// Simulate processing delay
		if self.response_delay_ms > 0 {
			tokio::time::sleep(Duration::from_millis(self.response_delay_ms)).await;
		}

		// Check if this adapter should fail
		if self.should_fail {
			return Err(AdapterValidationError::InvalidConfiguration {
				reason: format!("Adapter {} configured to fail", self.adapter.adapter_id),
			}
			.into());
		}

		let order_id = format!(
			"{}-order-{}",
			self.adapter.adapter_id,
			Utc::now().timestamp()
		);

		Ok(oif_types::OifPostOrderResponse::V0(SubmitOrderResponse {
			status: PostOrderResponseStatus::Received,
			order_id: Some(order_id),
			message: Some("Order submitted successfully".to_string()),
			order: None,
			metadata: Some(json!({
				"adapter_id": self.adapter.adapter_id,
				"response_delay_ms": self.response_delay_ms,
			})),
		}))
	}

	async fn get_order_details(
		&self,
		order_id: &str,
		_config: &SolverRuntimeConfig,
	) -> AdapterResult<oif_types::OifGetOrderResponse> {
		// Track the call
		self.call_tracker.fetch_add(1, Ordering::Relaxed);

		// Simulate processing delay
		if self.response_delay_ms > 0 {
			tokio::time::sleep(Duration::from_millis(self.response_delay_ms)).await;
		}

		// Check if this adapter should fail
		if self.should_fail {
			return Err(AdapterValidationError::InvalidConfiguration {
				reason: format!("Adapter {} configured to fail", self.adapter.adapter_id),
			}
			.into());
		}

		Ok(oif_types::OifGetOrderResponse::V0(GetOrderResponse {
			id: order_id.to_string(),
			quote_id: Some("mock-quote-123".to_string()),
			status: OrderStatus::Finalized,
			created_at: Utc::now().timestamp() as u64 - 60,
			updated_at: Utc::now().timestamp() as u64,
			input_amounts: vec![AssetAmount {
				asset: InteropAddress::from_chain_and_address(
					1,
					"0x0000000000000000000000000000000000000000",
				)
				.unwrap(),
				amount: Some(U256::new("1000000000000000000".to_string())),
			}],
			output_amounts: vec![AssetAmount {
				asset: InteropAddress::from_chain_and_address(
					1,
					"0xa0b86a33e6417a77c9a0c65f8e69b8b6e2b0c4a0",
				)
				.unwrap(),
				amount: Some(U256::new("1000000".to_string())),
			}],
			settlement: Settlement {
				settlement_type: SettlementType::Escrow,
				data: json!({"escrow_address": "0x1234567890123456789012345678901234567890"}),
			},
			fill_transaction: None,
		}))
	}

	async fn get_supported_assets(
		&self,
		_config: &SolverRuntimeConfig,
	) -> AdapterResult<oif_types::adapters::SupportedAssetsData> {
		match self.asset_mode {
			AssetMode::Routes => {
				Ok(oif_types::adapters::SupportedAssetsData::Routes(vec![
					AssetRoute::with_symbols(
						InteropAddress::from_text(
							"eip155:1:0x0000000000000000000000000000000000000000",
						)
						.unwrap(), // ETH on Ethereum
						"ETH".to_string(),
						InteropAddress::from_text(
							"eip155:1:0xA0b86a33E6417a77C9A0C65f8E69b8B6e2B0C4A0",
						)
						.unwrap(), // USDC on Ethereum
						"USDC".to_string(),
					),
					AssetRoute::with_symbols(
						InteropAddress::from_text(
							"eip155:1:0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
						)
						.unwrap(), // WETH on Ethereum
						"WETH".to_string(),
						InteropAddress::from_text(
							"eip155:1:0xA0b86a33E6417a77C9A0C65f8E69b8B6e2B0C4A0",
						)
						.unwrap(), // USDC on Ethereum
						"USDC".to_string(),
					),
					AssetRoute::with_symbols(
						InteropAddress::from_text(
							"eip155:1:0xA0b86a33E6417a77C9A0C65f8E69b8B6e2B0C4A0",
						)
						.unwrap(), // USDC on Ethereum
						"USDC".to_string(),
						InteropAddress::from_text(
							"eip155:1:0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
						)
						.unwrap(), // WETH on Ethereum
						"WETH".to_string(),
					),
					AssetRoute::with_symbols(
						InteropAddress::from_text(
							"eip155:1:0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
						)
						.unwrap(), // USDC on Ethereum
						"USDC".to_string(),
						InteropAddress::from_text(
							"eip155:10:0x7F5c764cBc14f9669B88837ca1490cCa17c31607",
						)
						.unwrap(), // USDC on Optimism
						"USDC".to_string(),
					),
					AssetRoute::with_symbols(
						InteropAddress::from_text(
							"eip155:10:0x7F5c764cBc14f9669B88837ca1490cCa17c31607",
						)
						.unwrap(), // USDC on Optimism
						"USDC".to_string(),
						InteropAddress::from_text(
							"eip155:1:0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
						)
						.unwrap(), // USDC on Ethereum
						"USDC".to_string(),
					),
				]))
			},
			AssetMode::Empty => Ok(oif_types::adapters::SupportedAssetsData::Routes(vec![])),
			AssetMode::Assets => {
				// Legacy mode - return empty for now
				Ok(oif_types::adapters::SupportedAssetsData::Routes(vec![]))
			},
		}
	}

	async fn health_check(&self, _config: &SolverRuntimeConfig) -> AdapterResult<bool> {
		// Simulate a quick health check delay
		if self.response_delay_ms > 0 {
			tokio::time::sleep(Duration::from_millis(self.response_delay_ms / 10)).await;
		}
		Ok(!self.should_fail)
	}
}

// Convenience functions for creating MockAdapter instances
// These provide the same API as the old wrapper adapters but create MockAdapter directly

/// Create a simple mock adapter for basic testing
pub fn create_mock_adapter() -> MockAdapter {
	MockAdapter::with_config("mock-demo-v1".to_string(), false, 0, AssetMode::Routes)
}

/// Create a mock adapter with custom configuration
pub fn create_mock_adapter_with_config(id: String) -> MockAdapter {
	MockAdapter::with_config(id, false, 0, AssetMode::Routes)
}

/// Create a failing adapter for testing failure scenarios
pub fn create_failing_adapter(id: String) -> MockAdapter {
	MockAdapter::with_config(id, true, 0, AssetMode::Routes)
}

/// Create a success adapter for testing success scenarios
pub fn create_success_adapter(id: String) -> MockAdapter {
	MockAdapter::with_config(id, false, 0, AssetMode::Routes)
}

/// Create a fast-responding adapter (100ms delay)
pub fn create_fast_adapter(id: &str) -> MockAdapter {
	MockAdapter::with_config(format!("timing-{}", id), false, 100, AssetMode::Routes)
}

/// Create a slow-responding adapter (1500ms delay)
pub fn create_slow_adapter(id: &str) -> MockAdapter {
	MockAdapter::with_config(format!("timing-{}", id), false, 1500, AssetMode::Routes)
}

/// Create a timeout adapter (5000ms delay)
pub fn create_timeout_adapter(id: &str) -> MockAdapter {
	MockAdapter::with_config(format!("timing-{}", id), false, 5000, AssetMode::Routes)
}

/// Create a failing adapter with timing
pub fn create_failing_timing_adapter(id: &str) -> MockAdapter {
	MockAdapter::with_config(format!("timing-{}", id), true, 100, AssetMode::Routes)
}
