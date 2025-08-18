//! U256 model for handling large integers as strings

use serde;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// U256 value represented as a string to preserve precision
///
/// Used for handling large integer values that might overflow native integer types
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct U256(pub String);

impl U256 {
	/// Create a new U256 from a string
	pub fn new(value: String) -> Self {
		Self(value)
	}

	/// Get the raw string value
	pub fn as_str(&self) -> &str {
		&self.0
	}

	/// Try to parse as u128 (for smaller values)
	pub fn as_u128(&self) -> Result<u128, std::num::ParseIntError> {
		self.0.parse()
	}

	/// Try to parse as u64 (for smaller values)
	pub fn as_u64(&self) -> Result<u64, std::num::ParseIntError> {
		self.0.parse()
	}

	/// Check if the value is zero
	pub fn is_zero(&self) -> bool {
		self.0 == "0" || self.0.chars().all(|c| c == '0')
	}

	/// Validate that the string contains only digits
	pub fn validate(&self) -> Result<(), String> {
		if self.0.is_empty() {
			return Err("U256 value cannot be empty".to_string());
		}

		if !self.0.chars().all(|c| c.is_ascii_digit()) {
			return Err("U256 value must contain only digits".to_string());
		}

		Ok(())
	}
}

impl std::fmt::Display for U256 {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl From<String> for U256 {
	fn from(value: String) -> Self {
		Self(value)
	}
}

impl From<&str> for U256 {
	fn from(value: &str) -> Self {
		Self(value.to_string())
	}
}

impl From<u128> for U256 {
	fn from(value: u128) -> Self {
		Self(value.to_string())
	}
}

impl From<u64> for U256 {
	fn from(value: u64) -> Self {
		Self(value.to_string())
	}
}

// Custom Serde implementation to serialize/deserialize as string
impl serde::Serialize for U256 {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serializer.serialize_str(&self.0)
	}
}

impl<'de> serde::Deserialize<'de> for U256 {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let value = String::deserialize(deserializer)?;
		let u256 = Self(value);
		u256.validate().map_err(serde::de::Error::custom)?;
		Ok(u256)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_u256_creation() {
		let val = U256::new("1000000000000000000".to_string());
		assert_eq!(val.as_str(), "1000000000000000000");
	}

	#[test]
	fn test_u256_parsing() {
		let val = U256::new("1000000000000000000".to_string());
		assert_eq!(val.as_u128().unwrap(), 1000000000000000000u128);
	}

	#[test]
	fn test_u256_validation() {
		let val = U256::new("1234567890".to_string());
		assert!(val.validate().is_ok());

		let val = U256::new("abc123".to_string());
		assert!(val.validate().is_err());

		let val = U256::new("".to_string());
		assert!(val.validate().is_err());
	}

	#[test]
	fn test_u256_is_zero() {
		assert!(U256::new("0".to_string()).is_zero());
		assert!(U256::new("000".to_string()).is_zero());
		assert!(!U256::new("1".to_string()).is_zero());
	}

	#[test]
	fn test_u256_conversions() {
		let val: U256 = 1000u64.into();
		assert_eq!(val.as_str(), "1000");

		let val: U256 = "500".into();
		assert_eq!(val.as_u64().unwrap(), 500);
	}

	#[test]
	fn test_u256_serde_serialization() {
		let val = U256::new("1000000000000000000".to_string());

		// Test serialization to JSON string
		let json = serde_json::to_string(&val).unwrap();
		assert_eq!(json, "\"1000000000000000000\"");

		// Test deserialization from JSON string
		let deserialized: U256 = serde_json::from_str(&json).unwrap();
		assert_eq!(val, deserialized);
		assert_eq!(deserialized.as_str(), "1000000000000000000");
	}

	#[test]
	fn test_u256_serde_validation() {
		// Valid numeric string
		let json = "\"123456789\"";
		let val: U256 = serde_json::from_str(json).unwrap();
		assert_eq!(val.as_str(), "123456789");

		// Invalid non-numeric string should fail during deserialization
		let invalid_json = "\"abc123\"";
		assert!(serde_json::from_str::<U256>(invalid_json).is_err());

		// Empty string should fail during deserialization
		let empty_json = "\"\"";
		assert!(serde_json::from_str::<U256>(empty_json).is_err());
	}

	#[test]
	fn test_u256_serde_api_request() {
		// Simulate an API request JSON with U256 amounts
		let json = r#"{
			"amount_in": "1000000000000000000",
			"amount_out": "2500000000"
		}"#;

		#[derive(serde::Deserialize, serde::Serialize)]
		struct TestRequest {
			amount_in: U256,
			amount_out: U256,
		}

		let request: TestRequest = serde_json::from_str(json).unwrap();
		assert_eq!(request.amount_in.as_str(), "1000000000000000000");
		assert_eq!(request.amount_out.as_str(), "2500000000");
		assert!(!request.amount_in.is_zero());
		assert!(!request.amount_out.is_zero());

		// Test serialization back
		let serialized = serde_json::to_string_pretty(&request).unwrap();
		assert!(serialized.contains("\"1000000000000000000\""));
		assert!(serialized.contains("\"2500000000\""));
	}
}
