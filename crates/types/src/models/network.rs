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
	pub name: Option<String>,
	/// Whether the network is a testnet
	pub is_testnet: Option<bool>,
}

impl Network {
	pub fn new(chain_id: u64, name: Option<String>, is_testnet: Option<bool>) -> Self {
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
		Self::new(1, Some("Ethereum".to_string()), Some(false))
	}
}
