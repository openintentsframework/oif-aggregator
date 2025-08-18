/// Timing-controlled mock adapters for sophisticated e2e testing
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use oif_types::adapters::{
	models::{
		AdapterQuote, AssetAmount, AvailableInput, GetOrderResponse, GetQuoteRequest,
		GetQuoteResponse, OrderResponse, OrderStatus, QuoteDetails, QuoteOrder, RequestedOutput,
		Settlement, SettlementType, SignatureType, SubmitOrderRequest,
	},
	traits::SolverAdapter,
	AdapterResult, AdapterValidationError,
};
use oif_types::chrono::Utc;
use oif_types::serde_json::json;
use oif_types::{AdapterError, InteropAddress, Network, SolverRuntimeConfig, U256};

/// Call tracking for verifying which adapters were actually called
#[derive(Debug, Clone)]
pub struct CallTracker {
	pub calls: Arc<AtomicUsize>,
	#[allow(dead_code)]
	pub id: String,
}

impl CallTracker {
	pub fn new(id: String) -> Self {
		Self {
			calls: Arc::new(AtomicUsize::new(0)),
			id,
		}
	}

	pub fn record_call(&self) {
		self.calls.fetch_add(1, Ordering::SeqCst);
	}

	pub fn call_count(&self) -> usize {
		self.calls.load(Ordering::SeqCst)
	}
}

/// Mock adapter that responds after a configurable delay
#[derive(Debug, Clone)]
pub struct TimingControlledAdapter {
	pub id: String,
	pub name: String,
	pub adapter: oif_types::Adapter,
	pub response_delay_ms: u64,
	pub should_fail: bool,
	pub tracker: CallTracker,
}

impl TimingControlledAdapter {
	/// Create a fast-responding adapter (responds in ~100ms)
	pub fn fast(id: &str) -> Self {
		Self::new(id, 100, false)
	}

	/// Create a slow-responding adapter (responds in ~1500ms)
	pub fn slow(id: &str) -> Self {
		Self::new(id, 1500, false)
	}

	/// Create a very slow adapter that will timeout (responds in ~5000ms)
	pub fn timeout(id: &str) -> Self {
		Self::new(id, 5000, false)
	}

	/// Create an adapter that always fails
	pub fn failing(id: &str) -> Self {
		Self::new(id, 100, true)
	}

	/// Create a custom adapter with specific timing
	pub fn new(id: &str, response_delay_ms: u64, should_fail: bool) -> Self {
		let adapter_id = format!("timing-{}", id);
		let name = format!("Timing Controlled Adapter {}", id);

		Self {
			id: adapter_id.clone(),
			name: name.clone(),
			adapter: oif_types::Adapter {
				adapter_id: adapter_id.clone(),
				name: name.clone(),
				description: Some(format!(
					"Timing controlled adapter with {}ms delay",
					response_delay_ms
				)),
				version: "1.0.0".to_string(),
				configuration: HashMap::new(),
			},
			response_delay_ms,
			should_fail,
			tracker: CallTracker::new(adapter_id),
		}
	}

	#[allow(dead_code)]
	pub fn call_count(&self) -> usize {
		self.tracker.call_count()
	}
}

#[async_trait]
impl SolverAdapter for TimingControlledAdapter {
	fn adapter_id(&self) -> &str {
		&self.id
	}

	fn adapter_name(&self) -> &str {
		&self.name
	}

	fn adapter_info(&self) -> &oif_types::Adapter {
		&self.adapter
	}

	async fn get_quotes(
		&self,
		request: &GetQuoteRequest,
		_config: &SolverRuntimeConfig,
	) -> AdapterResult<GetQuoteResponse> {
		// Record that this adapter was called
		self.tracker.record_call();

		// Simulate processing delay
		tokio::time::sleep(Duration::from_millis(self.response_delay_ms)).await;

		if self.should_fail {
			return Err(oif_types::AdapterError::from(
				AdapterValidationError::InvalidConfiguration {
					reason: format!("Adapter {} configured to fail", self.id),
				},
			));
		}

		// Create a realistic quote response
		let quote_id = format!("{}-quote-{}", self.id, Utc::now().timestamp());

		let available_input = request
			.available_inputs
			.first()
			.cloned()
			.unwrap_or_else(|| AvailableInput {
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
				amount: U256::new("1000000000000000000".to_string()),
				lock: None,
			});

		let requested_output = request
			.requested_outputs
			.first()
			.cloned()
			.unwrap_or_else(|| RequestedOutput {
				asset: InteropAddress::from_chain_and_address(
					1,
					"0xa0b86a33e6417a77c9a0c65f8e69b8b6e2b0c4a0",
				)
				.unwrap(),
				amount: U256::new("1000000".to_string()),
				receiver: InteropAddress::from_chain_and_address(
					1,
					"0x742d35Cc6634C0532925a3b8D2a27F79c5a85b03",
				)
				.unwrap(),
				calldata: None,
			});

		let quote = AdapterQuote {
			quote_id,
			orders: vec![QuoteOrder {
				signature_type: SignatureType::Eip712,
				domain: InteropAddress::from_chain_and_address(
					1,
					"0x1234567890123456789012345678901234567890",
				)
				.unwrap(),
				primary_type: "Order".to_string(),
				message: json!({
					"orderType": "swap",
					"adapter": self.id,
					"delay_ms": self.response_delay_ms
				}),
			}],
			details: QuoteDetails {
				available_inputs: vec![available_input],
				requested_outputs: vec![requested_output],
			},
			valid_until: Some(Utc::now().timestamp() as u64 + 300),
			eta: Some(self.response_delay_ms / 10), // ETA in seconds
			provider: format!("{} Provider", self.name),
		};

		Ok(GetQuoteResponse {
			quotes: vec![quote],
		})
	}

	async fn submit_order(
		&self,
		_request: &SubmitOrderRequest,
		_config: &SolverRuntimeConfig,
	) -> AdapterResult<GetOrderResponse> {
		// Record call and simulate delay
		self.tracker.record_call();
		tokio::time::sleep(Duration::from_millis(self.response_delay_ms)).await;

		if self.should_fail {
			return Err(oif_types::AdapterError::from(
				AdapterValidationError::InvalidConfiguration {
					reason: format!("Adapter {} configured to fail", self.id),
				},
			));
		}

		let order_id = format!("{}-order-{}", self.id, Utc::now().timestamp());
		Ok(GetOrderResponse {
			order: OrderResponse {
				id: order_id,
				quote_id: None,
				status: OrderStatus::Finalized, // Use Finalized to prevent refresh loops
				created_at: Utc::now().timestamp() as u64,
				updated_at: Utc::now().timestamp() as u64,
				input_amount: AssetAmount {
					asset: InteropAddress::from_hex("0x0000000000000000000000000000000000000000")
						.unwrap(),
					amount: U256::new("1000000000000000000".to_string()),
				},
				output_amount: AssetAmount {
					asset: InteropAddress::from_hex("0xa0b86a33e6417a77c9a0c65f8e69b8b6e2b0c4a0")
						.unwrap(),
					amount: U256::new("1000000".to_string()),
				},
				settlement: Settlement {
					settlement_type: SettlementType::Escrow,
					data: json!({}),
				},
				fill_transaction: None,
			},
		})
	}

	async fn get_order_details(
		&self,
		order_id: &str,
		_config: &SolverRuntimeConfig,
	) -> AdapterResult<GetOrderResponse> {
		// Record call and simulate delay
		self.tracker.record_call();
		tokio::time::sleep(Duration::from_millis(self.response_delay_ms)).await;

		if self.should_fail {
			return Err(AdapterError::from(
				AdapterValidationError::InvalidConfiguration {
					reason: format!("Adapter {} configured to fail", self.id),
				},
			));
		}

		Ok(GetOrderResponse {
			order: OrderResponse {
				id: order_id.to_string(),
				quote_id: None,
				status: OrderStatus::Finalized,
				created_at: Utc::now().timestamp() as u64,
				updated_at: Utc::now().timestamp() as u64,
				input_amount: AssetAmount {
					asset: InteropAddress::from_hex("0x0000000000000000000000000000000000000000")
						.unwrap(),
					amount: U256::new("1000000000000000000".to_string()),
				},
				output_amount: AssetAmount {
					asset: InteropAddress::from_hex("0xa0b86a33e6417a77c9a0c65f8e69b8b6e2b0c4a0")
						.unwrap(),
					amount: U256::new("1000000".to_string()),
				},
				settlement: Settlement {
					settlement_type: SettlementType::Escrow,
					data: json!({}),
				},
				fill_transaction: None,
			},
		})
	}

	async fn health_check(&self, _config: &SolverRuntimeConfig) -> AdapterResult<bool> {
		// Record call and simulate delay
		self.tracker.record_call();
		tokio::time::sleep(Duration::from_millis(self.response_delay_ms / 10)).await;

		if self.should_fail {
			return Ok(false);
		}

		Ok(true)
	}

	async fn get_supported_networks(&self) -> AdapterResult<Vec<Network>> {
		Ok(vec![Network {
			chain_id: 1,
			name: "Ethereum Mainnet".to_string(),
			is_testnet: false,
		}])
	}

	async fn get_supported_assets(
		&self,
		_network: &Network,
	) -> AdapterResult<Vec<oif_types::models::Asset>> {
		Ok(vec![])
	}
}
