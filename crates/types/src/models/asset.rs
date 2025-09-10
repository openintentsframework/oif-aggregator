//! Blockchain asset/token models

use crate::models::InteropAddress;
use serde::{Deserialize, Serialize};
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Supported blockchain asset/token (internal model with InteropAddress)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct Asset {
	/// Asset address in ERC-7930 interoperable format
	pub address: InteropAddress,
	/// Token symbol (e.g., "ETH", "USDC", "WBTC")
	pub symbol: String,
	/// Human-readable name (e.g., "Ethereum", "USD Coin")
	pub name: String,
	/// Number of decimal places
	pub decimals: u8,
}

impl Asset {
	/// Create a new asset from InteropAddress
	pub fn new(address: InteropAddress, symbol: String, name: String, decimals: u8) -> Self {
		Self {
			address,
			symbol,
			name,
			decimals,
		}
	}

	/// Create asset from plain chain_id and address (for convenience)
	pub fn from_chain_and_address(
		chain_id: u64,
		address: String,
		symbol: String,
		name: String,
		decimals: u8,
	) -> Result<Self, crate::quotes::errors::QuoteValidationError> {
		let interop_address =
			InteropAddress::from_text(&format!("eip155:{}:{}", chain_id, address))?;
		Ok(Self::new(interop_address, symbol, name, decimals))
	}

	/// Extract chain ID
	pub fn chain_id(&self) -> Result<u64, crate::quotes::errors::QuoteValidationError> {
		self.address.extract_chain_id()
	}

	/// Extract plain address
	pub fn plain_address(&self) -> String {
		self.address.extract_address()
	}
}

/// Common asset constants
impl Asset {
	// Ethereum assets
	pub fn eth() -> Self {
		Self::from_chain_and_address(
			1,
			"0x0000000000000000000000000000000000000000".to_string(),
			"ETH".to_string(),
			"Ethereum".to_string(),
			18,
		)
		.expect("Valid ETH asset")
	}

	pub fn usdc_ethereum() -> Self {
		Self::from_chain_and_address(
			1,
			"0xA0b86a33E6441E7C81F7C93451777f5F4dE78e86".to_string(),
			"USDC".to_string(),
			"USD Coin".to_string(),
			6,
		)
		.expect("Valid USDC asset")
	}

	pub fn usdt_ethereum() -> Self {
		Self::from_chain_and_address(
			1,
			"0xdAC17F958D2ee523a2206206994597C13D831ec7".to_string(),
			"USDT".to_string(),
			"Tether USD".to_string(),
			6,
		)
		.expect("Valid USDT asset")
	}

	pub fn wbtc_ethereum() -> Self {
		Self::from_chain_and_address(
			1,
			"0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599".to_string(),
			"WBTC".to_string(),
			"Wrapped Bitcoin".to_string(),
			8,
		)
		.expect("Valid WBTC asset")
	}

	// Polygon assets
	pub fn matic() -> Self {
		Self::from_chain_and_address(
			137,
			"0x0000000000000000000000000000000000001010".to_string(),
			"MATIC".to_string(),
			"Polygon".to_string(),
			18,
		)
		.expect("Valid MATIC asset")
	}

	pub fn usdc_polygon() -> Self {
		Self::from_chain_and_address(
			137,
			"0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".to_string(),
			"USDC".to_string(),
			"USD Coin".to_string(),
			6,
		)
		.expect("Valid USDC Polygon asset")
	}

	// BSC assets
	pub fn bnb() -> Self {
		Self::from_chain_and_address(
			56,
			"0x0000000000000000000000000000000000000000".to_string(),
			"BNB".to_string(),
			"Binance Coin".to_string(),
			18,
		)
		.expect("Valid BNB asset")
	}

	pub fn usdc_bsc() -> Self {
		Self::from_chain_and_address(
			56,
			"0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d".to_string(),
			"USDC".to_string(),
			"USD Coin".to_string(),
			18,
		)
		.expect("Valid USDC BSC asset")
	}

	// Base assets
	pub fn eth_base() -> Self {
		Self::from_chain_and_address(
			8453,
			"0x0000000000000000000000000000000000000000".to_string(),
			"ETH".to_string(),
			"Ethereum".to_string(),
			18,
		)
		.expect("Valid ETH Base asset")
	}

	pub fn usdc_base() -> Self {
		Self::from_chain_and_address(
			8453,
			"0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".to_string(),
			"USDC".to_string(),
			"USD Coin".to_string(),
			6,
		)
		.expect("Valid USDC Base asset")
	}
}
