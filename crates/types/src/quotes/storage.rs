//! Quote storage model and conversions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{Quote, QuoteError, QuoteStatus};

/// Storage representation of a quote
///
/// This model is optimized for storage and persistence.
/// It can be converted to/from the domain Quote model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteStorage {
	pub quote_id: String,
	pub solver_id: String,
	pub request_id: String,
	pub token_in: String,
	pub token_out: String,
	pub amount_in: String,
	pub amount_out: String,
	pub chain_id: u64,
	pub estimated_gas: Option<u64>,
	pub price_impact: Option<f64>,
	pub response_time_ms: u64,
	pub confidence_score: Option<f64>,
	pub status: QuoteStatusStorage,
	pub created_at: DateTime<Utc>,
	pub expires_at: DateTime<Utc>,
	pub raw_response: serde_json::Value,
	pub integrity_checksum: String,

	// Storage-specific metadata
	pub version: u32,
	pub last_accessed: Option<DateTime<Utc>>,
	pub access_count: u64,
}

/// Storage-compatible quote status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum QuoteStatusStorage {
	Valid,
	Expired,
	Failed,
	Processing,
}

impl QuoteStorage {
	/// Mark as accessed for analytics
	pub fn mark_accessed(&mut self) {
		self.last_accessed = Some(Utc::now());
		self.access_count += 1;
	}

	/// Check if the stored quote has expired
	pub fn is_expired(&self) -> bool {
		Utc::now() > self.expires_at
	}

	/// Get TTL in seconds
	pub fn ttl_seconds(&self) -> i64 {
		(self.expires_at - Utc::now()).num_seconds().max(0)
	}

	/// Update the status
	pub fn update_status(&mut self, status: QuoteStatusStorage) {
		self.status = status;
	}
}

impl From<QuoteStatus> for QuoteStatusStorage {
	fn from(status: QuoteStatus) -> Self {
		match status {
			QuoteStatus::Valid => Self::Valid,
			QuoteStatus::Expired => Self::Expired,
			QuoteStatus::Failed => Self::Failed,
			QuoteStatus::Processing => Self::Processing,
		}
	}
}

impl From<QuoteStatusStorage> for QuoteStatus {
	fn from(storage: QuoteStatusStorage) -> Self {
		match storage {
			QuoteStatusStorage::Valid => Self::Valid,
			QuoteStatusStorage::Expired => Self::Expired,
			QuoteStatusStorage::Failed => Self::Failed,
			QuoteStatusStorage::Processing => Self::Processing,
		}
	}
}

/// Conversion traits for easy integration
impl From<Quote> for QuoteStorage {
	fn from(quote: Quote) -> Self {
		Self {
			quote_id: quote.quote_id,
			solver_id: quote.solver_id,
			request_id: quote.request_id,
			token_in: quote.token_in,
			token_out: quote.token_out,
			amount_in: quote.amount_in,
			amount_out: quote.amount_out,
			chain_id: quote.chain_id,
			estimated_gas: quote.estimated_gas,
			price_impact: quote.price_impact,
			response_time_ms: quote.response_time_ms,
			confidence_score: quote.confidence_score,
			status: QuoteStatusStorage::from(quote.status),
			created_at: quote.created_at,
			expires_at: quote.expires_at,
			raw_response: quote.raw_response,
			integrity_checksum: quote.integrity_checksum,
			version: 1,
			last_accessed: None,
			access_count: 0,
		}
	}
}

impl TryFrom<QuoteStorage> for Quote {
	type Error = QuoteError;

	fn try_from(storage: QuoteStorage) -> Result<Self, Self::Error> {
		Ok(Quote {
			quote_id: storage.quote_id,
			solver_id: storage.solver_id,
			request_id: storage.request_id,
			token_in: storage.token_in,
			token_out: storage.token_out,
			amount_in: storage.amount_in,
			amount_out: storage.amount_out,
			chain_id: storage.chain_id,
			estimated_gas: storage.estimated_gas,
			price_impact: storage.price_impact,
			response_time_ms: storage.response_time_ms,
			confidence_score: storage.confidence_score,
			status: QuoteStatus::from(storage.status),
			created_at: storage.created_at,
			expires_at: storage.expires_at,
			raw_response: storage.raw_response,
			integrity_checksum: storage.integrity_checksum,
		})
	}
}
#[cfg(test)]
mod tests {
	use super::*;
	use crate::quotes::Quote;
	use chrono::Duration;

	fn create_test_quote() -> Quote {
		Quote::new(
			"test-solver".to_string(),
			"req-123".to_string(),
			"0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0".to_string(),
			"0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
			"1000000000000000000".to_string(),
			"2500000000".to_string(),
			1,
		)
	}

	#[test]
	fn test_storage_conversion() {
		let quote = create_test_quote();
		let quote_id = quote.quote_id.clone();

		// Convert to storage
		let storage = QuoteStorage::from(quote);
		assert_eq!(storage.quote_id, quote_id);
		assert_eq!(storage.version, 1);
		assert_eq!(storage.access_count, 0);
		assert!(storage.last_accessed.is_none());

		// Convert back to domain
		let domain_quote = Quote::try_from(storage).unwrap();
		assert_eq!(domain_quote.quote_id, quote_id);
	}

	#[test]
	fn test_access_tracking() {
		let quote = create_test_quote();
		let mut storage = QuoteStorage::from(quote);

		assert_eq!(storage.access_count, 0);
		assert!(storage.last_accessed.is_none());

		storage.mark_accessed();
		assert_eq!(storage.access_count, 1);
		assert!(storage.last_accessed.is_some());

		storage.mark_accessed();
		assert_eq!(storage.access_count, 2);
	}

	#[test]
	fn test_status_conversion() {
		assert_eq!(
			QuoteStatusStorage::from(QuoteStatus::Valid),
			QuoteStatusStorage::Valid
		);
		assert_eq!(
			QuoteStatus::from(QuoteStatusStorage::Valid),
			QuoteStatus::Valid
		);
	}
}
