//! ERC-7930 Interoperable Address Standard Implementation
//!
//! This module implements types and utilities for ERC-7930 interoperable addresses,
//! which encode chain information alongside addresses in a binary format to enable
//! cross-chain operations.
//!
//! ## Address Format
//!
//! An ERC-7930 interoperable address is a binary-encoded string serialized as hex:
//! ```text
//! 0x010000011401D8DA6BF26964AF9D7EED9E03E53415D37AA96045
//! ^^---------------------------------- Version: 1 byte (decimal 1)
//!   ^^^^------------------------------ ChainType: 2 bytes (CAIP namespace, e.g., eip155)
//!       ^^---------------------------- ChainReferenceLength: 1 byte (decimal length)
//!         ^^-------------------------- ChainReference: variable length (e.g., uint8 chain ID)
//!           ^^------------------------ AddressLength: 1 byte (decimal length)
//!             ^^^^^^^^^^^^^^^^^^^^^^^^ Address: variable length (e.g., 20 bytes for Ethereum)
//! ```

use crate::quotes::{QuoteValidationError, QuoteValidationResult};
use hex;
use serde;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// CAIP namespace constant - using alternative EIP-155 implementation
const CAIP_EIP155: [u8; 2] = [0x00, 0x00]; // Alternative EIP-155 (default for this system)

/// ERC-7930 Interoperable Address
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct InteropAddress {
	version: u8,              // Version number (e.g., 1) - 1 byte per ERC-7930
	chain_type: [u8; 2],      // CAIP namespace (e.g., 0x0001 for eip155)
	chain_reference: Vec<u8>, // Chain ID (variable length)
	address: Vec<u8>,         // Address (variable length)
}

impl InteropAddress {
	/// Create a new InteropAddress from components
	pub fn new(
		version: u8,
		chain_type: [u8; 2],
		chain_reference: Vec<u8>,
		address: Vec<u8>,
	) -> Self {
		Self {
			version,
			chain_type,
			chain_reference,
			address,
		}
	}

	/// Check if chain type is the supported EIP-155 implementation
	fn is_eip155_chain_type(chain_type: [u8; 2]) -> bool {
		chain_type == CAIP_EIP155
	}

	/// Minimal big-endian encoding for a non-zero u64 (1..=8 bytes)
	fn encode_chain_id_min_be(chain_id: u64) -> QuoteValidationResult<Vec<u8>> {
		if chain_id == 0 {
			return Err(QuoteValidationError::UnsupportedChain { chain_id: 0 });
		}
		let be = chain_id.to_be_bytes();
		// Strip leading zeros
		let first_nz = be.iter().position(|&b| b != 0).unwrap_or(be.len() - 1);
		let out = be[first_nz..].to_vec();
		Ok(out)
	}

	/// Parse an ERC-7930 address from a hex string (e.g., 0x010000011401...)
	pub fn from_hex(hex: &str) -> QuoteValidationResult<Self> {
		if !hex.starts_with("0x") {
			return Err(QuoteValidationError::InvalidTokenAddress {
				field: "missing 0x prefix".to_string(),
			});
		}

		let bytes =
			hex::decode(&hex[2..]).map_err(|_| QuoteValidationError::InvalidTokenAddress {
				field: "invalid hex".to_string(),
			})?;

		// Minimum header = 1 (ver) + 2 (type) + 1 (crl) + 1 (adl) = 5
		if bytes.len() < 5 {
			return Err(QuoteValidationError::InvalidTokenAddress {
				field: "insufficient length (< 5 bytes header)".to_string(),
			});
		}

		let version = bytes[0];
		let chain_type = [bytes[1], bytes[2]];
		let crl = bytes[3] as usize;
		let adl = bytes[4] as usize;

		let expected_total = 5usize
			.checked_add(crl)
			.and_then(|v| v.checked_add(adl))
			.ok_or_else(|| QuoteValidationError::InvalidTokenAddress {
				field: "length overflow".to_string(),
			})?;

		if bytes.len() != expected_total {
			return Err(QuoteValidationError::InvalidTokenAddress {
				field: format!(
					"total length mismatch: expected {}, got {}",
					expected_total,
					bytes.len()
				),
			});
		}

		let chain_reference = bytes[5..5 + crl].to_vec();
		let address = bytes[5 + crl..5 + crl + adl].to_vec();

		let addr = Self {
			version,
			chain_type,
			chain_reference,
			address,
		};

		addr.validate()?;
		Ok(addr)
	}

	/// Convert to hex string (e.g., 0x010000011401...)
	pub fn to_hex(&self) -> String {
		let mut bytes = Vec::new();
		bytes.push(self.version);
		bytes.extend_from_slice(&self.chain_type);
		bytes.push(self.chain_reference.len() as u8);
		bytes.push(self.address.len() as u8);
		bytes.extend_from_slice(&self.chain_reference);
		bytes.extend_from_slice(&self.address);
		format!("0x{}", hex::encode(bytes))
	}

	/// Create from CAIP-10-like text format (e.g., eip155:1:0x...)
	pub fn from_text(text: &str) -> QuoteValidationResult<Self> {
		let parts: Vec<&str> = text.split(':').collect();
		if parts.len() != 3 || parts[0] != "eip155" {
			return Err(QuoteValidationError::InvalidTokenAddress {
				field: "text format, expected eip155:<chain_id>:0x<address>".to_string(),
			});
		}

		let chain_id: u64 = parts[1]
			.parse()
			.map_err(|_| QuoteValidationError::UnsupportedChain { chain_id: 0 })?;

		let addr_str = parts[2];
		if !addr_str.starts_with("0x") {
			return Err(QuoteValidationError::InvalidTokenAddress {
				field: "address missing 0x prefix".to_string(),
			});
		}

		let address =
			hex::decode(&addr_str[2..]).map_err(|_| QuoteValidationError::InvalidTokenAddress {
				field: "address hex".to_string(),
			})?;

		Self::validate_ethereum_address(&address)?;

		let chain_reference = Self::encode_chain_id_min_be(chain_id)?;
		Ok(InteropAddress {
			version: 1u8,
			chain_type: CAIP_EIP155,
			chain_reference,
			address,
		})
	}

	/// Convert to CAIP-10-like text format (eip155:chain_id:address)
	pub fn to_text(&self) -> QuoteValidationResult<String> {
		if !Self::is_eip155_chain_type(self.chain_type) {
			return Err(QuoteValidationError::InvalidTokenAddress {
				field: "unsupported chain type for text format".to_string(),
			});
		}

		let chain_id = self.extract_chain_id()?;
		Ok(format!("eip155:{}:{}", chain_id, self.extract_address()))
	}

	/// Extract chain ID as u64
	pub fn extract_chain_id(&self) -> QuoteValidationResult<u64> {
		if self.chain_reference.is_empty() {
			return Err(QuoteValidationError::MissingRequiredField {
				field: "chain_reference".to_string(),
			});
		}

		// Assume chain_reference is a big-endian integer
		let mut chain_id = 0;
		for &byte in &self.chain_reference {
			chain_id = (chain_id << 8) | (byte as u64);
		}

		if chain_id == 0 {
			return Err(QuoteValidationError::UnsupportedChain { chain_id: 0 });
		}

		Ok(chain_id)
	}

	/// Extract address as hex string
	pub fn extract_address(&self) -> String {
		format!("0x{}", hex::encode(&self.address))
	}

	/// Validate the interoperable address
	pub fn validate(&self) -> QuoteValidationResult<()> {
		// Version
		if self.version != 1 {
			return Err(QuoteValidationError::InvalidTokenAddress {
				field: format!("unsupported version: {}", self.version),
			});
		}

		// Support both EIP-155 chain types (0x0001 and 0x0000)
		if !Self::is_eip155_chain_type(self.chain_type) {
			return Err(QuoteValidationError::InvalidTokenAddress {
				field: format!(
					"unsupported chain type: 0x{:02x}{:02x}",
					self.chain_type[0], self.chain_type[1]
				),
			});
		}

		// Chain reference: 1..=8 bytes, big-endian non-zero integer
		let crl = self.chain_reference.len();
		if crl == 0 || crl > 8 {
			return Err(QuoteValidationError::InvalidTokenAddress {
				field: "chain reference length must be 1..=8 bytes".to_string(),
			});
		}
		// No leading zero padding for "minimal" encoding
		if crl > 1 && self.chain_reference[0] == 0 {
			return Err(QuoteValidationError::InvalidTokenAddress {
				field: "chain reference not minimally encoded (leading zero)".to_string(),
			});
		}
		if self.extract_chain_id()? == 0 {
			return Err(QuoteValidationError::UnsupportedChain { chain_id: 0 });
		}

		// Ethereum address length: 20 bytes
		Self::validate_ethereum_address(&self.address)?;

		Ok(())
	}

	/// Create from chain ID and Ethereum address
	pub fn from_chain_and_address(chain_id: u64, address_hex: &str) -> QuoteValidationResult<Self> {
		if chain_id == 0 {
			return Err(QuoteValidationError::UnsupportedChain { chain_id: 0 });
		}
		if !address_hex.starts_with("0x") {
			return Err(QuoteValidationError::InvalidTokenAddress {
				field: "address missing 0x prefix".to_string(),
			});
		}

		let address_bytes = hex::decode(&address_hex[2..]).map_err(|_| {
			QuoteValidationError::InvalidTokenAddress {
				field: "address hex".to_string(),
			}
		})?;
		Self::validate_ethereum_address(&address_bytes)?;

		let chain_reference = Self::encode_chain_id_min_be(chain_id)?;
		Ok(InteropAddress {
			version: 1u8,
			chain_type: CAIP_EIP155,
			chain_reference,
			address: address_bytes,
		})
	}

	/// Check if this address is on the specified chain
	pub fn is_on_chain(&self, chain_id: u64) -> bool {
		self.extract_chain_id()
			.map(|id| id == chain_id)
			.unwrap_or(false)
	}

	/// Check if this address is the zero address (empty/unset)
	pub fn is_empty(&self) -> bool {
		self.address.iter().all(|&b| b == 0)
	}

	/// Validate Ethereum address (20 bytes)
	fn validate_ethereum_address(address: &[u8]) -> QuoteValidationResult<()> {
		if address.len() != 20 {
			return Err(QuoteValidationError::InvalidTokenAddress {
				field: format!(
					"ethereum address length: expected 20 bytes, got {}",
					address.len()
				),
			});
		}
		Ok(())
	}
}

impl std::fmt::Display for InteropAddress {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.to_hex())
	}
}

// Custom Serde implementation to serialize/deserialize as hex string
impl serde::Serialize for InteropAddress {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serializer.serialize_str(&self.to_hex())
	}
}

impl<'de> serde::Deserialize<'de> for InteropAddress {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let hex_string = String::deserialize(deserializer)?;
		Self::from_hex(&hex_string).map_err(serde::de::Error::custom)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_valid_binary_address() {
		let addr =
			InteropAddress::from_hex("0x010000011401D8DA6BF26964AF9D7EED9E03E53415D37AA96045")
				.unwrap();
		assert_eq!(addr.version, 1u8);
		assert_eq!(addr.chain_type, [0x00, 0x00]);
		assert_eq!(addr.chain_reference, vec![1]);
		assert_eq!(addr.address.len(), 20);
		assert_eq!(
			addr.to_hex(),
			"0x010000011401d8da6bf26964af9d7eed9e03e53415d37aa96045"
		);
		assert!(addr.validate().is_ok());
	}

	#[test]
	fn test_from_text() {
		let addr = InteropAddress::from_text("eip155:1:0xD8DA6BF26964aF9D7eEd9e03E53415D37aA96045")
			.unwrap();
		assert_eq!(addr.version, 1u8);
		assert_eq!(addr.chain_type, [0x00, 0x00]);
		assert_eq!(addr.chain_reference, vec![1]); // Changed from 8-byte to 1-byte for chain ID 1
		assert_eq!(addr.address.len(), 20);
		assert_eq!(
			addr.to_text().unwrap(),
			"eip155:1:0xd8da6bf26964af9d7eed9e03e53415d37aa96045"
		);
	}

	#[test]
	fn test_from_chain_and_address() {
		let addr =
			InteropAddress::from_chain_and_address(1, "0xD8DA6BF26964aF9D7eEd9e03E53415D37aA96045")
				.unwrap();
		assert_eq!(
			addr.to_hex(),
			"0x010000011401d8da6bf26964af9d7eed9e03e53415d37aa96045"
		);
		assert_eq!(addr.extract_chain_id().unwrap(), 1);
		assert_eq!(
			addr.extract_address(),
			"0xd8da6bf26964af9d7eed9e03e53415d37aa96045"
		);
	}

	#[test]
	fn test_is_on_chain() {
		let addr =
			InteropAddress::from_hex("0x010000011401D8DA6BF26964AF9D7eEd9e03E53415D37AA96045")
				.unwrap();
		assert!(addr.is_on_chain(1));
		assert!(!addr.is_on_chain(137));
	}

	#[test]
	fn test_invalid_hex() {
		assert!(InteropAddress::from_hex("0x").is_err());
		assert!(InteropAddress::from_hex("0x000100000101").is_err()); // Too short
		assert!(InteropAddress::from_hex("0xZZZZ").is_err()); // Invalid hex
	}

	#[test]
	fn test_invalid_text() {
		assert!(InteropAddress::from_text("invalid").is_err());
		assert!(InteropAddress::from_text("eip155:1:invalid").is_err());
		assert!(
			InteropAddress::from_text("btc:1:0xD8DA6BF26964aF9D7eEd9e03E53415D37aA96045").is_err()
		);
	}

	#[test]
	fn test_invalid_validation() {
		let mut addr =
			InteropAddress::from_hex("0x010000011401D8DA6BF26964AF9D7eEd9e03E53415D37AA96045")
				.unwrap();
		addr.version = 2u8; // Invalid version
		assert!(addr.validate().is_err());

		addr = InteropAddress::from_hex("0x010000011401D8DA6BF26964AF9D7eEd9e03E53415D37AA96045")
			.unwrap();
		addr.chain_type = [0x00, 0x02]; // Invalid chain type
		assert!(addr.validate().is_err());

		addr = InteropAddress::from_hex("0x010000011401D8DA6BF26964AF9D7eEd9e03E53415D37AA96045")
			.unwrap();
		addr.address = vec![0; 19]; // Invalid address length
		assert!(addr.validate().is_err());
	}

	#[test]
	fn test_display_and_conversions() {
		let addr =
			InteropAddress::from_hex("0x010000011401D8DA6BF26964AF9D7eEd9e03E53415D37AA96045")
				.unwrap();
		assert_eq!(
			addr.to_string(),
			"0x010000011401d8da6bf26964af9d7eed9e03e53415d37aa96045"
		);
		// Test that from_hex works for same address
		let addr_from_hex =
			InteropAddress::from_hex("0x010000011401D8DA6BF26964AF9D7eEd9e03E53415D37AA96045")
				.unwrap();
		assert_eq!(addr, addr_from_hex);
	}

	#[test]
	fn test_serde_serialization() {
		let addr =
			InteropAddress::from_chain_and_address(1, "0xD8DA6BF26964aF9D7eEd9e03E53415D37aA96045")
				.unwrap();

		// Test serialization to JSON string
		let json = serde_json::to_string(&addr).unwrap();
		assert_eq!(
			json,
			"\"0x010000011401d8da6bf26964af9d7eed9e03e53415d37aa96045\""
		);

		// Test deserialization from JSON string
		let deserialized: InteropAddress = serde_json::from_str(&json).unwrap();
		assert_eq!(addr, deserialized);

		// Verify the deserialized address has correct properties
		assert_eq!(deserialized.extract_chain_id().unwrap(), 1);
		assert_eq!(
			deserialized.extract_address(),
			"0xd8da6bf26964af9d7eed9e03e53415d37aa96045"
		);
	}

	#[test]
	fn test_serde_deserialization_from_api_request() {
		// Simulate an API request JSON with binary format InteropAddress
		let json = r#"{
            "user": "0x010000011401d8da6bf26964af9d7eed9e03e53415d37aa96045",
            "asset": "0x010000011401a0b86a33e6417a77c9a0c65f8e69b8b6e2b0c4a0"
        }"#;

		#[derive(serde::Deserialize)]
		struct TestRequest {
			user: InteropAddress,
			asset: InteropAddress,
		}

		let request: TestRequest = serde_json::from_str(json).unwrap();
		assert_eq!(request.user.extract_chain_id().unwrap(), 1);
		assert_eq!(request.asset.extract_chain_id().unwrap(), 1);
		assert_eq!(
			request.user.extract_address(),
			"0xd8da6bf26964af9d7eed9e03e53415d37aa96045"
		);
		assert_eq!(
			request.asset.extract_address(),
			"0xa0b86a33e6417a77c9a0c65f8e69b8b6e2b0c4a0"
		);
	}

	#[test]
	fn test_alternative_chain_type() {
		// Test support for 0x0000 chain type (alternative EIP-155 implementation)
		let addr =
			InteropAddress::from_hex("0x01000002147a6970997970c51812dc3a010c7d01b50e0d17dc79c8")
				.unwrap();
		assert_eq!(addr.version, 1u8);
		assert_eq!(addr.chain_type, [0x00, 0x00]); // Alternative chain type
		assert_eq!(addr.extract_chain_id().unwrap(), 31337);
		assert_eq!(
			addr.extract_address(),
			"0x70997970c51812dc3a010c7d01b50e0d17dc79c8"
		);
		assert!(addr.validate().is_ok());
		assert!(addr.is_on_chain(31337));
		assert!(!addr.is_on_chain(1));
	}

	#[test]
	fn test_is_empty() {
		// Test zero address (empty)
		let zero_addr =
			InteropAddress::from_chain_and_address(1, "0x0000000000000000000000000000000000000000")
				.unwrap();
		assert!(zero_addr.is_empty());

		// Test non-zero address (not empty)
		let addr =
			InteropAddress::from_chain_and_address(1, "0xD8DA6BF26964aF9D7eEd9e03E53415D37aA96045")
				.unwrap();
		assert!(!addr.is_empty());
	}
}
