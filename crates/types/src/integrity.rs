//! Integrity verification trait
//!
//! This module defines the `IntegrityPayload` trait that enables
//! data structures to participate in integrity verification.

/// Trait for types that can have their integrity verified
///
/// Types implementing this trait can generate a canonical payload string
/// that will be used for HMAC generation and verification.
pub trait IntegrityPayload {
	/// Generate a canonical payload string for integrity verification
	///
	/// This should include all the critical fields that need to be protected
	/// from tampering, in a consistent format.
	fn to_integrity_payload(&self) -> String;
}
