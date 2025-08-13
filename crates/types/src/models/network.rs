//! Blockchain network models

use serde::{Deserialize, Serialize};
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Supported blockchain network
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct Network {
	/// Chain ID (e.g., 1 for Ethereum mainnet, 137 for Polygon)
	pub chain_id: u64,
	/// Human-readable name (e.g., "Ethereum", "Polygon", "Base")
	pub name: String,
	/// Whether the network is a testnet
	pub is_testnet: bool,
}

impl Network {
	pub fn new(chain_id: u64, name: String, is_testnet: bool) -> Self {
		Self {
			chain_id,
			name,
			is_testnet,
		}
	}
}

/// Common network constants
impl Network {
	/// Ethereum mainnet
	pub fn ethereum() -> Self {
		Self::new(1, "Ethereum".to_string(), false)
	}

	/// Polygon mainnet
	pub fn polygon() -> Self {
		Self::new(137, "Polygon".to_string(), false)
	}

	/// Binance Smart Chain
	pub fn bsc() -> Self {
		Self::new(56, "BSC".to_string(), false)
	}

	/// Arbitrum One
	pub fn arbitrum() -> Self {
		Self::new(42161, "Arbitrum One".to_string(), false)
	}

	/// Base
	pub fn base() -> Self {
		Self::new(8453, "Base".to_string(), false)
	}

	/// Optimism
	pub fn optimism() -> Self {
		Self::new(10, "Optimism".to_string(), false)
	}

	/// Avalanche
	pub fn avalanche() -> Self {
		Self::new(43114, "Avalanche".to_string(), false)
	}

	/// Fantom
	pub fn fantom() -> Self {
		Self::new(250, "Fantom".to_string(), false)
	}

	/// Ethereum Sepolia testnet
	pub fn sepolia() -> Self {
		Self::new(11155111, "Sepolia".to_string(), true)
	}

	/// Polygon Mumbai testnet
	pub fn mumbai() -> Self {
		Self::new(80001, "Mumbai".to_string(), true)
	}
}
