//! Generic integrity verification service using HMAC-SHA256
//!
//! This service provides functionality to generate and verify HMAC-SHA256 checksums
//! for any data structure that can be serialized to a canonical string format.

use hmac::{Hmac, Mac};
use oif_types::{IntegrityPayload, SecretString};
use sha2::Sha256;
use std::fmt::Write;

type HmacSha256 = Hmac<Sha256>;

/// Generic integrity verification service
///
/// This service can generate and verify HMAC-SHA256 checksums for any data
/// that implements the `IntegrityPayload` trait.
pub struct IntegrityService {
	secret_key: SecretString,
}

impl IntegrityService {
	/// Create a new integrity service with the given secret key
	pub fn new(secret_key: SecretString) -> Self {
		Self { secret_key }
	}

	/// Generate HMAC-SHA256 checksum for any data implementing IntegrityPayload
	pub fn generate_checksum<T: IntegrityPayload>(&self, data: &T) -> Result<String, String> {
		// Create the payload to be signed
		let payload = data.to_integrity_payload();

		// Create HMAC instance
		let mut mac = HmacSha256::new_from_slice(self.secret_key.expose_secret().as_bytes())
			.map_err(|e| format!("Failed to create HMAC: {}", e))?;

		// Update with payload
		mac.update(payload.as_bytes());

		// Finalize and convert to hex string
		let result = mac.finalize();
		let code_bytes = result.into_bytes();

		// Convert to hex string
		let mut hex_string = String::with_capacity(code_bytes.len() * 2);
		for byte in code_bytes {
			write!(&mut hex_string, "{:02x}", byte)
				.map_err(|e| format!("Failed to format hex: {}", e))?;
		}

		Ok(hex_string)
	}

	/// Verify integrity checksum for any data implementing IntegrityPayload
	pub fn verify_checksum<T: IntegrityPayload>(
		&self,
		data: &T,
		expected_checksum: &str,
	) -> Result<bool, String> {
		let calculated_checksum = self.generate_checksum(data)?;
		Ok(constant_time_eq(
			calculated_checksum.as_bytes(),
			expected_checksum.as_bytes(),
		))
	}

	/// Convenience method to generate and attach checksum to a string field
	pub fn sign<T: IntegrityPayload>(&self, data: &T) -> Result<String, String> {
		self.generate_checksum(data)
	}

	/// Convenience method to verify a checksum
	pub fn verify<T: IntegrityPayload>(&self, data: &T, checksum: &str) -> Result<bool, String> {
		self.verify_checksum(data, checksum)
	}

	/// Generate checksum directly from a payload string
	///
	/// This is useful when you already have the canonical payload string
	/// and don't need to implement the IntegrityPayload trait.
	pub fn generate_checksum_from_payload(&self, payload: &str) -> Result<String, String> {
		// Create HMAC instance
		let mut mac = HmacSha256::new_from_slice(self.secret_key.expose_secret().as_bytes())
			.map_err(|e| format!("Failed to create HMAC: {}", e))?;

		// Update with payload
		mac.update(payload.as_bytes());

		// Finalize and convert to hex string
		let result = mac.finalize();
		let code_bytes = result.into_bytes();

		// Convert to hex string
		let mut hex_string = String::with_capacity(code_bytes.len() * 2);
		for byte in code_bytes {
			write!(&mut hex_string, "{:02x}", byte)
				.map_err(|e| format!("Failed to format hex: {}", e))?;
		}

		Ok(hex_string)
	}

	/// Verify checksum directly from a payload string
	pub fn verify_checksum_from_payload(
		&self,
		payload: &str,
		expected_checksum: &str,
	) -> Result<bool, String> {
		let calculated_checksum = self.generate_checksum_from_payload(payload)?;
		Ok(constant_time_eq(
			calculated_checksum.as_bytes(),
			expected_checksum.as_bytes(),
		))
	}
}

/// Constant-time comparison to prevent timing attacks
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
	if a.len() != b.len() {
		return false;
	}

	let mut result = 0u8;
	for (x, y) in a.iter().zip(b.iter()) {
		result |= x ^ y;
	}
	result == 0
}

#[cfg(test)]
mod tests {
	use super::*;

	// Example implementation for testing
	struct TestData {
		id: String,
		value: String,
		timestamp: i64,
	}

	impl IntegrityPayload for TestData {
		fn to_integrity_payload(&self) -> String {
			format!(
				"id={}|value={}|timestamp={}",
				self.id, self.value, self.timestamp
			)
		}
	}

	#[test]
	fn test_generate_checksum() {
		let service = IntegrityService::new(SecretString::from("test-secret-key-12345"));

		let data = TestData {
			id: "test-123".to_string(),
			value: "test-value".to_string(),
			timestamp: 1234567890,
		};

		let checksum = service.generate_checksum(&data).unwrap();
		assert!(!checksum.is_empty());
		assert_eq!(checksum.len(), 64); // SHA256 hex string length
	}

	#[test]
	fn test_verify_checksum() {
		let service = IntegrityService::new(SecretString::from("test-secret-key-12345"));

		let data = TestData {
			id: "test-123".to_string(),
			value: "test-value".to_string(),
			timestamp: 1234567890,
		};

		let checksum = service.generate_checksum(&data).unwrap();
		let is_valid = service.verify_checksum(&data, &checksum).unwrap();
		assert!(is_valid);

		// Test with wrong checksum
		let is_valid = service.verify_checksum(&data, "wrong-checksum").unwrap();
		assert!(!is_valid);
	}

	#[test]
	fn test_deterministic_checksum() {
		let service = IntegrityService::new(SecretString::from("test-secret-key-12345"));

		let data = TestData {
			id: "test-123".to_string(),
			value: "test-value".to_string(),
			timestamp: 1234567890,
		};

		let checksum1 = service.generate_checksum(&data).unwrap();
		let checksum2 = service.generate_checksum(&data).unwrap();
		assert_eq!(checksum1, checksum2);
	}

	#[test]
	fn test_convenience_methods() {
		let service = IntegrityService::new(SecretString::from("test-secret-key-12345"));

		let data = TestData {
			id: "test-123".to_string(),
			value: "test-value".to_string(),
			timestamp: 1234567890,
		};

		let checksum = service.sign(&data).unwrap();
		let is_valid = service.verify(&data, &checksum).unwrap();
		assert!(is_valid);
	}
}
