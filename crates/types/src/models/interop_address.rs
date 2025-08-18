//! ERC-7930 Interoperable Address Standard Implementation
//!
//! This module implements types and utilities for ERC-7930 interoperable addresses,
//! which encode chain information alongside addresses in a binary format to enable
//! cross-chain operations.
//!
//! ## Address Format
//!
//! An ERC-7930 interoperable address is a binary-encoded string serialized as hex:
//! ```
//! 0x00010000010114D8DA6BF26964AF9D7EED9E03E53415D37AA96045
//! ^^^^---------------------------------- Version: 4 bytes (decimal 1)
//!     ^^^^------------------------------ ChainType: 2 bytes (CAIP namespace, e.g., eip155)
//!         ^^---------------------------- ChainReferenceLength: 1 byte (decimal length)
//!           ^^-------------------------- ChainReference: variable length (e.g., uint8 chain ID)
//!             ^^------------------------ AddressLength: 1 byte (decimal length)
//!               ^^^^^^^^^^^^^^^^^^^^^^^^ Address: variable length (e.g., 20 bytes for Ethereum)
//! ```

use crate::quotes::{QuoteValidationError, QuoteValidationResult};
use hex;
use serde;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// ERC-7930 Interoperable Address
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct InteropAddress {
	version: u32,             // Version number (e.g., 1)
	chain_type: [u8; 2],      // CAIP namespace (e.g., 0x0001 for eip155)
	chain_reference: Vec<u8>, // Chain ID (variable length)
	address: Vec<u8>,         // Address (variable length)
}

impl InteropAddress {
	/// Create a new InteropAddress from components
	pub fn new(
		version: u32,
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

	/// Parse an ERC-7930 address from a hex string (e.g., 0x00010000010114...)
	pub fn from_hex(hex: &str) -> QuoteValidationResult<Self> {
		if !hex.starts_with("0x") || hex.len() < 2 {
			return Err(QuoteValidationError::InvalidTokenAddress {
				field: "hex format".to_string(),
			});
		}

		let bytes =
			hex::decode(&hex[2..]).map_err(|_| QuoteValidationError::InvalidTokenAddress {
				field: "invalid hex".to_string(),
			})?;

		if bytes.len() < 8 {
			return Err(QuoteValidationError::InvalidTokenAddress {
				field: "insufficient length".to_string(),
			});
		}

		let version = u32::from_be_bytes(bytes[0..4].try_into().map_err(|_| {
			QuoteValidationError::InvalidTokenAddress {
				field: "version parsing".to_string(),
			}
		})?);

		let chain_type =
			bytes[4..6]
				.try_into()
				.map_err(|_| QuoteValidationError::InvalidTokenAddress {
					field: "chain type parsing".to_string(),
				})?;

		let chain_ref_len = bytes[6] as usize;
		let addr_len = bytes[7] as usize;

		if bytes.len() < 8 + chain_ref_len + addr_len {
			return Err(QuoteValidationError::InvalidTokenAddress {
				field: "invalid length for chain reference or address".to_string(),
			});
		}

		let chain_reference = bytes[8..8 + chain_ref_len].to_vec();
		let address = bytes[8 + chain_ref_len..8 + chain_ref_len + addr_len].to_vec();

		let address = Self {
			version,
			chain_type,
			chain_reference,
			address,
		};

		address.validate()?;
		Ok(address)
	}

	/// Convert to hex string (e.g., 0x00010000010114...)
	pub fn to_hex(&self) -> String {
		let mut bytes = Vec::new();
		bytes.extend_from_slice(&self.version.to_be_bytes());
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
				field: "text format, expected eip155:chain_id:address".to_string(),
			});
		}

		let chain_id: u64 = parts[1]
			.parse()
			.map_err(|_| QuoteValidationError::UnsupportedChain { chain_id: 0 })?;

		let address =
			hex::decode(&parts[2][2..]).map_err(|_| QuoteValidationError::InvalidTokenAddress {
				field: "address hex".to_string(),
			})?;

		Self::validate_ethereum_address(&address)?;

		// Use minimal byte encoding for chain ID
		let chain_reference = if chain_id <= 255 {
			vec![chain_id as u8]
		} else if chain_id <= 65535 {
			(chain_id as u16).to_be_bytes().to_vec()
		} else if chain_id <= 16777215 {
			let bytes = (chain_id as u32).to_be_bytes();
			bytes[1..].to_vec() // Skip the first byte (24-bit encoding)
		} else {
			chain_id.to_be_bytes().to_vec()
		};

		Ok(InteropAddress {
			version: 1,
			chain_type: [0x00, 0x01], // eip155
			chain_reference,
			address,
		})
	}

	/// Convert to CAIP-10-like text format (eip155:chain_id:address)
	pub fn to_text(&self) -> QuoteValidationResult<String> {
		if self.chain_type != [0x00, 0x01] {
			return Err(QuoteValidationError::InvalidTokenAddress {
				field: "unsupported chain type for text format".to_string(),
			});
		}

		let chain_id = self.extract_chain_id()?;
		let address = hex::encode(&self.address);
		Ok(format!("eip155:{}:0x{}", chain_id, address))
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

	/// Extract token address (alias for extract_address for backward compatibility)
	pub fn extract_token_address(&self) -> QuoteValidationResult<String> {
		Ok(self.extract_address())
	}

	/// Validate the interoperable address
	pub fn validate(&self) -> QuoteValidationResult<()> {
		// Validate version
		if self.version != 1 {
			return Err(QuoteValidationError::InvalidTokenAddress {
				field: format!("unsupported version: {}", self.version),
			});
		}

		// Validate chain type (e.g., eip155 = 0x0001)
		if self.chain_type != [0x00, 0x01] {
			return Err(QuoteValidationError::InvalidTokenAddress {
				field: "unsupported chain type".to_string(),
			});
		}

		// Validate chain reference
		if self.chain_reference.is_empty() {
			return Err(QuoteValidationError::MissingRequiredField {
				field: "chain_reference".to_string(),
			});
		}
		if self.chain_reference.len() > 8 {
			return Err(QuoteValidationError::InvalidTokenAddress {
				field: "chain reference too long".to_string(),
			});
		}
		if self.extract_chain_id()? == 0 {
			return Err(QuoteValidationError::UnsupportedChain { chain_id: 0 });
		}

		// Validate address (Ethereum-specific: 20 bytes)
		Self::validate_ethereum_address(&self.address)?;

		Ok(())
	}

	/// Create from chain ID and Ethereum address
	pub fn from_chain_and_address(chain_id: u64, address: &str) -> QuoteValidationResult<Self> {
		if chain_id == 0 {
			return Err(QuoteValidationError::UnsupportedChain { chain_id: 0 });
		}

		let address_bytes =
			hex::decode(&address[2..]).map_err(|_| QuoteValidationError::InvalidTokenAddress {
				field: "address hex".to_string(),
			})?;

		Self::validate_ethereum_address(&address_bytes)?;

		// Use minimal byte encoding for chain ID
		let chain_reference = if chain_id <= 255 {
			vec![chain_id as u8]
		} else if chain_id <= 65535 {
			(chain_id as u16).to_be_bytes().to_vec()
		} else if chain_id <= 16777215 {
			let bytes = (chain_id as u32).to_be_bytes();
			bytes[1..].to_vec() // Skip the first byte (24-bit encoding)
		} else {
			chain_id.to_be_bytes().to_vec()
		};

		Ok(InteropAddress {
			version: 1,
			chain_type: [0x00, 0x01], // eip155
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

impl From<String> for InteropAddress {
	fn from(hex: String) -> Self {
		Self::from_hex(&hex).unwrap_or_else(|_| InteropAddress {
			version: 0,
			chain_type: [0, 0],
			chain_reference: Vec::new(),
			address: Vec::new(),
		})
	}
}

impl From<&str> for InteropAddress {
	fn from(hex: &str) -> Self {
		Self::from_hex(hex).unwrap_or_else(|_| InteropAddress {
			version: 0,
			chain_type: [0, 0],
			chain_reference: Vec::new(),
			address: Vec::new(),
		})
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
		let addr = InteropAddress::from_hex(
			"0x000000010001011401D8DA6BF26964AF9D7EED9E03E53415D37AA96045",
		)
		.unwrap();
		assert_eq!(addr.version, 1);
		assert_eq!(addr.chain_type, [0x00, 0x01]);
		assert_eq!(addr.chain_reference, vec![1]);
		assert_eq!(addr.address.len(), 20);
		assert_eq!(
			addr.to_hex(),
			"0x000000010001011401d8da6bf26964af9d7eed9e03e53415d37aa96045"
		);
		assert!(addr.validate().is_ok());
	}

	#[test]
	fn test_from_text() {
		let addr = InteropAddress::from_text("eip155:1:0xD8DA6BF26964aF9D7eEd9e03E53415D37aA96045")
			.unwrap();
		assert_eq!(addr.version, 1);
		assert_eq!(addr.chain_type, [0x00, 0x01]);
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
			"0x000000010001011401d8da6bf26964af9d7eed9e03e53415d37aa96045"
		);
		assert_eq!(addr.extract_chain_id().unwrap(), 1);
		assert_eq!(
			addr.extract_address(),
			"0xd8da6bf26964af9d7eed9e03e53415d37aa96045"
		);
	}

	#[test]
	fn test_is_on_chain() {
		let addr = InteropAddress::from_hex(
			"0x000000010001011401D8DA6BF26964AF9D7eEd9e03E53415D37AA96045",
		)
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
		let mut addr = InteropAddress::from_hex(
			"0x000000010001011401D8DA6BF26964AF9D7eEd9e03E53415D37AA96045",
		)
		.unwrap();
		addr.version = 2; // Invalid version
		assert!(addr.validate().is_err());

		addr = InteropAddress::from_hex(
			"0x000000010001011401D8DA6BF26964AF9D7eEd9e03E53415D37AA96045",
		)
		.unwrap();
		addr.chain_type = [0x00, 0x02]; // Invalid chain type
		assert!(addr.validate().is_err());

		addr = InteropAddress::from_hex(
			"0x000000010001011401D8DA6BF26964AF9D7eEd9e03E53415D37AA96045",
		)
		.unwrap();
		addr.address = vec![0; 19]; // Invalid address length
		assert!(addr.validate().is_err());
	}

	#[test]
	fn test_display_and_conversions() {
		let addr = InteropAddress::from_hex(
			"0x000000010001011401D8DA6BF26964AF9D7eEd9e03E53415D37AA96045",
		)
		.unwrap();
		assert_eq!(
			addr.to_string(),
			"0x000000010001011401d8da6bf26964af9d7eed9e03e53415d37aa96045"
		);
		let addr_from_str =
			InteropAddress::from("0x000000010001011401D8DA6BF26964AF9D7eEd9e03E53415D37AA96045");
		assert_eq!(addr, addr_from_str);
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
			"\"0x000000010001011401d8da6bf26964af9d7eed9e03e53415d37aa96045\""
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
            "user": "0x000000010001011401d8da6bf26964af9d7eed9e03e53415d37aa96045",
            "asset": "0x000000010001011401a0b86a33e6417a77c9a0c65f8e69b8b6e2b0c4a0"
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
}
