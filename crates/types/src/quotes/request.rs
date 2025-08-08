//! Quote request model and validation

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
#[cfg(feature = "openapi")]
use utoipa::ToSchema;
use uuid::Uuid;

use super::{Quote, QuoteValidationError, QuoteValidationResult};

/// API request body for /v1/quotes endpoint
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct QuotesRequest {
	pub token_in: String,
	pub token_out: String,
	pub amount_in: String,
	pub chain_id: u64,
	pub slippage_tolerance: Option<f64>,
	pub recipient: Option<String>,
	pub referrer: Option<String>,
}

impl TryFrom<QuotesRequest> for QuoteRequest {
	type Error = QuoteValidationError;

	fn try_from(api_request: QuotesRequest) -> Result<Self, Self::Error> {
		// Validate request
		if api_request.token_in.is_empty()
			|| api_request.token_out.is_empty()
			|| api_request.amount_in.is_empty()
		{
			return Err(QuoteValidationError::MissingRequiredField {
				field: "token_in, token_out, and amount_in are required".to_string(),
			});
		}

		// Create domain quote request
		let mut quote_request = QuoteRequest::new(
			api_request.token_in,
			api_request.token_out,
			api_request.amount_in,
			api_request.chain_id,
		);

		// Apply optional parameters
		if let Some(slippage) = api_request.slippage_tolerance {
			quote_request.slippage_tolerance = Some(slippage);
		}

		if let Some(recipient) = api_request.recipient {
			quote_request.recipient = Some(recipient);
		}

		if let Some(referrer) = api_request.referrer {
			quote_request.referrer = Some(referrer);
		}

		Ok(quote_request)
	}
}

/// Quote request from the API layer
///
/// This represents a request for quotes that comes from the API.
/// It should be validated and converted to domain models.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct QuoteRequest {
	/// Unique identifier for this request
	pub request_id: String,

	/// Input token address
	pub token_in: String,

	/// Output token address
	pub token_out: String,

	/// Input amount (as string to preserve precision)
	pub amount_in: String,

	/// Chain ID where the swap should occur
	pub chain_id: u64,

	/// Maximum acceptable slippage tolerance (0.005 = 0.5%)
	pub slippage_tolerance: Option<f64>,

	/// Maximum deadline for the quote
	pub deadline: Option<DateTime<Utc>>,

	/// Optional recipient address (if different from user)
	pub recipient: Option<String>,

	/// Optional referrer for tracking
	pub referrer: Option<String>,
}

impl QuoteRequest {
	/// Create a new quote request
	pub fn new(token_in: String, token_out: String, amount_in: String, chain_id: u64) -> Self {
		Self {
			request_id: Uuid::new_v4().to_string(),
			token_in,
			token_out,
			amount_in,
			chain_id,
			slippage_tolerance: Some(0.005), // 0.5% default
			deadline: Some(Utc::now() + Duration::minutes(20)),
			recipient: None,
			referrer: None,
		}
	}

	/// Validate the quote request and return a validation result
	pub fn validate(&self) -> QuoteValidationResult<()> {
		// Validate token addresses
		self.validate_token_address(&self.token_in, "token_in")?;
		self.validate_token_address(&self.token_out, "token_out")?;

		// Validate tokens are different
		if self.token_in == self.token_out {
			return Err(QuoteValidationError::InvalidTokenAddress {
				field: "token_in/token_out".to_string(),
			});
		}

		// Validate amount
		self.validate_amount(&self.amount_in, "amount_in")?;

		// Validate chain ID
		self.validate_chain_id()?;

		// Validate slippage tolerance
		if let Some(slippage) = self.slippage_tolerance {
			self.validate_slippage_tolerance(slippage)?;
		}

		// Validate deadline
		if let Some(deadline) = self.deadline {
			self.validate_deadline(deadline)?;
		}

		// Validate recipient address if provided
		if let Some(ref recipient) = self.recipient {
			self.validate_token_address(recipient, "recipient")?;
		}

		Ok(())
	}

	/// Convert this request to a domain quote template
	/// This creates a quote with the request data but no solver response data yet
	pub fn to_domain_template(&self, solver_id: String) -> QuoteValidationResult<Quote> {
		// Validate first
		self.validate()?;

		// Create the quote template - solver will fill in amount_out and other details
		let quote = Quote::new(
			solver_id,
			self.request_id.clone(),
			self.token_in.clone(),
			self.token_out.clone(),
			self.amount_in.clone(),
			"0".to_string(), // Will be filled by solver
			self.chain_id,
		);

		Ok(quote)
	}

	/// Validate token address format (basic Ethereum address validation)
	fn validate_token_address(&self, address: &str, field: &str) -> QuoteValidationResult<()> {
		if address.is_empty() {
			return Err(QuoteValidationError::MissingRequiredField {
				field: field.to_string(),
			});
		}

		if !address.starts_with("0x") {
			return Err(QuoteValidationError::InvalidTokenAddress {
				field: field.to_string(),
			});
		}

		if address.len() != 42 {
			return Err(QuoteValidationError::InvalidTokenAddress {
				field: field.to_string(),
			});
		}

		// Check if all characters after 0x are valid hex
		if !address[2..].chars().all(|c| c.is_ascii_hexdigit()) {
			return Err(QuoteValidationError::InvalidTokenAddress {
				field: field.to_string(),
			});
		}

		Ok(())
	}

	/// Validate amount format and value
	fn validate_amount(&self, amount: &str, field: &str) -> QuoteValidationResult<()> {
		if amount.is_empty() {
			return Err(QuoteValidationError::MissingRequiredField {
				field: field.to_string(),
			});
		}

		// Try to parse as a number
		let parsed: Result<f64, _> = amount.parse();
		match parsed {
			Ok(value) => {
				if value <= 0.0 {
					return Err(QuoteValidationError::InvalidAmount {
						field: field.to_string(),
						reason: "Amount must be greater than zero".to_string(),
					});
				}
				if !value.is_finite() {
					return Err(QuoteValidationError::InvalidAmount {
						field: field.to_string(),
						reason: "Amount must be finite".to_string(),
					});
				}
			},
			Err(_) => {
				return Err(QuoteValidationError::InvalidAmount {
					field: field.to_string(),
					reason: "Invalid number format".to_string(),
				});
			},
		}

		Ok(())
	}

	/// Validate chain ID (support common networks)
	fn validate_chain_id(&self) -> QuoteValidationResult<()> {
		const SUPPORTED_CHAINS: &[u64] = &[
			1,     // Ethereum Mainnet
			10,    // Optimism
			56,    // BSC
			100,   // Gnosis
			137,   // Polygon
			250,   // Fantom
			8453,  // Base
			42161, // Arbitrum One
			43114, // Avalanche
		];

		if !SUPPORTED_CHAINS.contains(&self.chain_id) {
			return Err(QuoteValidationError::UnsupportedChain {
				chain_id: self.chain_id,
			});
		}

		Ok(())
	}

	/// Validate slippage tolerance
	fn validate_slippage_tolerance(&self, slippage: f64) -> QuoteValidationResult<()> {
		if slippage < 0.0 || slippage > 1.0 {
			return Err(QuoteValidationError::InvalidSlippageTolerance { value: slippage });
		}

		if !slippage.is_finite() {
			return Err(QuoteValidationError::InvalidSlippageTolerance { value: slippage });
		}

		Ok(())
	}

	/// Validate deadline
	fn validate_deadline(&self, deadline: DateTime<Utc>) -> QuoteValidationResult<()> {
		let now = Utc::now();

		if deadline <= now {
			return Err(QuoteValidationError::InvalidDeadline {
				reason: "Deadline must be in the future".to_string(),
			});
		}

		// Check if deadline is too far in the future (max 24 hours)
		let max_deadline = now + Duration::hours(24);
		if deadline > max_deadline {
			return Err(QuoteValidationError::InvalidDeadline {
				reason: "Deadline cannot be more than 24 hours in the future".to_string(),
			});
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn create_valid_request() -> QuoteRequest {
		QuoteRequest::new(
			"0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0".to_string(),
			"0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
			"1000000000000000000".to_string(),
			1,
		)
	}

	#[test]
	fn test_valid_request_validation() {
		let request = create_valid_request();
		assert!(request.validate().is_ok());
	}

	#[test]
	fn test_invalid_token_address() {
		let mut request = create_valid_request();
		request.token_in = "invalid".to_string();
		assert!(request.validate().is_err());
	}

	#[test]
	fn test_same_tokens() {
		let mut request = create_valid_request();
		request.token_out = request.token_in.clone();
		assert!(request.validate().is_err());
	}

	#[test]
	fn test_invalid_amount() {
		let mut request = create_valid_request();
		request.amount_in = "0".to_string();
		assert!(request.validate().is_err());

		request.amount_in = "-100".to_string();
		assert!(request.validate().is_err());

		request.amount_in = "not_a_number".to_string();
		assert!(request.validate().is_err());
	}

	#[test]
	fn test_unsupported_chain() {
		let mut request = create_valid_request();
		request.chain_id = 999999;
		assert!(request.validate().is_err());
	}

	#[test]
	fn test_invalid_slippage() {
		let mut request = create_valid_request();
		request.slippage_tolerance = Some(-0.1);
		assert!(request.validate().is_err());

		request.slippage_tolerance = Some(1.1);
		assert!(request.validate().is_err());
	}

	#[test]
	fn test_invalid_deadline() {
		let mut request = create_valid_request();
		request.deadline = Some(Utc::now() - Duration::minutes(1));
		assert!(request.validate().is_err());

		request.deadline = Some(Utc::now() + Duration::days(2));
		assert!(request.validate().is_err());
	}

	#[test]
	fn test_to_domain_template() {
		let request = create_valid_request();
		let quote = request
			.to_domain_template("test-solver".to_string())
			.unwrap();

		assert_eq!(quote.solver_id, "test-solver");
		assert_eq!(quote.request_id, request.request_id);
		assert_eq!(quote.token_in, request.token_in);
		assert_eq!(quote.token_out, request.token_out);
		assert_eq!(quote.amount_in, request.amount_in);
		assert_eq!(quote.chain_id, request.chain_id);
	}
}
