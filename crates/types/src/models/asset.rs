//! Blockchain asset/token models

use serde::{Deserialize, Serialize};
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Supported blockchain asset/token
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct Asset {
	/// Contract address (use "0x0" for native tokens)
	pub address: String,
	/// Token symbol (e.g., "ETH", "USDC", "WBTC")
	pub symbol: String,
	/// Human-readable name (e.g., "Ethereum", "USD Coin")
	pub name: String,
	/// Number of decimal places
	pub decimals: u8,
	/// Chain ID where this asset exists
	pub chain_id: u64,
}

impl Asset {
	pub fn new(address: String, symbol: String, name: String, decimals: u8, chain_id: u64) -> Self {
		Self {
			address,
			symbol,
			name,
			decimals,
			chain_id,
		}
	}
}

/// Common asset constants
impl Asset {
	// Ethereum assets
	pub fn eth() -> Self {
		Self::new(
			"0x0000000000000000000000000000000000000000".to_string(),
			"ETH".to_string(),
			"Ethereum".to_string(),
			18,
			1,
		)
	}

	pub fn usdc_ethereum() -> Self {
		Self::new(
			"0xA0b86a33E6441E7C81F7C93451777f5F4dE78e86".to_string(),
			"USDC".to_string(),
			"USD Coin".to_string(),
			6,
			1,
		)
	}

	pub fn usdt_ethereum() -> Self {
		Self::new(
			"0xdAC17F958D2ee523a2206206994597C13D831ec7".to_string(),
			"USDT".to_string(),
			"Tether USD".to_string(),
			6,
			1,
		)
	}

	pub fn wbtc_ethereum() -> Self {
		Self::new(
			"0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599".to_string(),
			"WBTC".to_string(),
			"Wrapped Bitcoin".to_string(),
			8,
			1,
		)
	}

	// Polygon assets
	pub fn matic() -> Self {
		Self::new(
			"0x0000000000000000000000000000000000001010".to_string(),
			"MATIC".to_string(),
			"Polygon".to_string(),
			18,
			137,
		)
	}

	pub fn usdc_polygon() -> Self {
		Self::new(
			"0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".to_string(),
			"USDC".to_string(),
			"USD Coin".to_string(),
			6,
			137,
		)
	}

	// BSC assets
	pub fn bnb() -> Self {
		Self::new(
			"0x0000000000000000000000000000000000000000".to_string(),
			"BNB".to_string(),
			"Binance Coin".to_string(),
			18,
			56,
		)
	}

	pub fn usdc_bsc() -> Self {
		Self::new(
			"0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d".to_string(),
			"USDC".to_string(),
			"USD Coin".to_string(),
			18,
			56,
		)
	}

	// Base assets
	pub fn eth_base() -> Self {
		Self::new(
			"0x0000000000000000000000000000000000000000".to_string(),
			"ETH".to_string(),
			"Ethereum".to_string(),
			18,
			8453,
		)
	}

	pub fn usdc_base() -> Self {
		Self::new(
			"0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".to_string(),
			"USDC".to_string(),
			"USD Coin".to_string(),
			6,
			8453,
		)
	}
}
