//! Secure string handling for sensitive data like API keys
//!
//! This module provides a `SecretString` type that uses zeroize to securely
//! clear sensitive data from memory when dropped.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// A secure string type that zeroizes its contents when dropped
///
/// This type is designed to hold sensitive data like API keys, passwords,
/// and other secrets. The underlying string data is automatically cleared
/// from memory when the `SecretString` is dropped.
///
/// # Examples
///
/// ```rust
/// use oif_types::SecretString;
///
/// let api_key = SecretString::new("secret-api-key-12345".to_string());
///
/// // Access the secret value when needed
/// let key_value = api_key.expose_secret();
///
/// // The secret is automatically zeroized when dropped
/// ```
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SecretString {
	inner: String,
}

impl SecretString {
	/// Create a new `SecretString` from a `String`
	pub fn new(secret: String) -> Self {
		Self { inner: secret }
	}

	/// Create a new `SecretString` from a string slice
	pub fn from_str(secret: &str) -> Self {
		Self::new(secret.to_string())
	}

	/// Expose the secret value
	///
	/// Use this method sparingly and only when you need access to the actual
	/// secret value. Consider if there's a way to use the secret without
	/// exposing it directly.
	pub fn expose_secret(&self) -> &str {
		&self.inner
	}

	/// Get the length of the secret without exposing it
	pub fn len(&self) -> usize {
		self.inner.len()
	}

	/// Check if the secret is empty without exposing it
	pub fn is_empty(&self) -> bool {
		self.inner.is_empty()
	}
}

// Drop implementation is automatically provided by ZeroizeOnDrop derive

impl fmt::Debug for SecretString {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("SecretString")
			.field("inner", &"[REDACTED]")
			.finish()
	}
}

impl fmt::Display for SecretString {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "[REDACTED]")
	}
}

impl From<String> for SecretString {
	fn from(secret: String) -> Self {
		Self::new(secret)
	}
}

impl From<&str> for SecretString {
	fn from(secret: &str) -> Self {
		Self::from_str(secret)
	}
}

// Custom serialization to avoid accidentally leaking secrets in logs
impl Serialize for SecretString {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		// Only serialize if explicitly requested via a special marker
		// For normal serialization, we redact the value
		serializer.serialize_str("[REDACTED]")
	}
}

// Custom deserialization for loading secrets from config
impl<'de> Deserialize<'de> for SecretString {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let secret = String::deserialize(deserializer)?;
		Ok(SecretString::new(secret))
	}
}

impl PartialEq for SecretString {
	fn eq(&self, other: &Self) -> bool {
		// Use constant-time comparison to avoid timing attacks
		constant_time_eq(self.inner.as_bytes(), other.inner.as_bytes())
	}
}

/// Constant-time comparison to prevent timing attacks
/// Returns true if the two byte slices are equal
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

impl Eq for SecretString {}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_secret_string_creation() {
		let secret = SecretString::new("test-secret".to_string());
		assert_eq!(secret.expose_secret(), "test-secret");
		assert_eq!(secret.len(), 11);
		assert!(!secret.is_empty());
	}

	#[test]
	fn test_secret_string_from_str() {
		let secret = SecretString::from_str("api-key-123");
		assert_eq!(secret.expose_secret(), "api-key-123");
	}

	#[test]
	fn test_secret_string_debug() {
		let secret = SecretString::new("secret".to_string());
		let debug_str = format!("{:?}", secret);
		assert!(debug_str.contains("[REDACTED]"));
		assert!(!debug_str.contains("secret"));
	}

	#[test]
	fn test_secret_string_display() {
		let secret = SecretString::new("secret".to_string());
		let display_str = format!("{}", secret);
		assert_eq!(display_str, "[REDACTED]");
	}

	#[test]
	fn test_secret_string_equality() {
		let secret1 = SecretString::new("same-secret".to_string());
		let secret2 = SecretString::new("same-secret".to_string());
		let secret3 = SecretString::new("different-secret".to_string());

		assert_eq!(secret1, secret2);
		assert_ne!(secret1, secret3);
	}

	#[test]
	fn test_secret_string_serialization() {
		let secret = SecretString::new("secret-key".to_string());
		let serialized = serde_json::to_string(&secret).unwrap();
		assert_eq!(serialized, "\"[REDACTED]\"");
	}

	#[test]
	fn test_secret_string_deserialization() {
		let json = "\"secret-value\"";
		let secret: SecretString = serde_json::from_str(json).unwrap();
		assert_eq!(secret.expose_secret(), "secret-value");
	}
}
