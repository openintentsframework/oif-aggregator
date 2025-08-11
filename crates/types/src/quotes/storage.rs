//! Quote storage model and conversions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{Quote, QuoteError, QuoteResult, QuoteStatus};

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
	/// Create a new storage quote from domain quote
	pub fn from_domain(quote: Quote) -> Self {
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
			status: QuoteStatusStorage::from_domain(quote.status),
			created_at: quote.created_at,
			expires_at: quote.expires_at,
			raw_response: quote.raw_response,
			version: 1,
			last_accessed: None,
			access_count: 0,
		}
	}

	/// Convert storage quote to domain quote
	pub fn to_domain(self) -> QuoteResult<Quote> {
		Ok(Quote {
			quote_id: self.quote_id,
			solver_id: self.solver_id,
			request_id: self.request_id,
			token_in: self.token_in,
			token_out: self.token_out,
			amount_in: self.amount_in,
			amount_out: self.amount_out,
			chain_id: self.chain_id,
			estimated_gas: self.estimated_gas,
			price_impact: self.price_impact,
			response_time_ms: self.response_time_ms,
			confidence_score: self.confidence_score,
			status: self.status.to_domain(),
			created_at: self.created_at,
			expires_at: self.expires_at,
			raw_response: self.raw_response,
		})
	}

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

impl QuoteStatusStorage {
	/// Convert from domain status
	pub fn from_domain(status: QuoteStatus) -> Self {
		match status {
			QuoteStatus::Valid => Self::Valid,
			QuoteStatus::Expired => Self::Expired,
			QuoteStatus::Failed => Self::Failed,
			QuoteStatus::Processing => Self::Processing,
		}
	}

	/// Convert to domain status
	pub fn to_domain(self) -> QuoteStatus {
		match self {
			Self::Valid => QuoteStatus::Valid,
			Self::Expired => QuoteStatus::Expired,
			Self::Failed => QuoteStatus::Failed,
			Self::Processing => QuoteStatus::Processing,
		}
	}
}

/// Conversion traits for easy integration
impl From<Quote> for QuoteStorage {
	fn from(quote: Quote) -> Self {
		Self::from_domain(quote)
	}
}

impl TryFrom<QuoteStorage> for Quote {
	type Error = QuoteError;

	fn try_from(storage: QuoteStorage) -> Result<Self, Self::Error> {
		storage.to_domain()
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
		let storage = QuoteStorage::from_domain(quote);
		assert_eq!(storage.quote_id, quote_id);
		assert_eq!(storage.version, 1);
		assert_eq!(storage.access_count, 0);
		assert!(storage.last_accessed.is_none());

		// Convert back to domain
		let domain_quote = storage.to_domain().unwrap();
		assert_eq!(domain_quote.quote_id, quote_id);
	}

	#[test]
	fn test_access_tracking() {
		let quote = create_test_quote();
		let mut storage = QuoteStorage::from_domain(quote);

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
			QuoteStatusStorage::from_domain(QuoteStatus::Valid),
			QuoteStatusStorage::Valid
		);
		assert_eq!(QuoteStatusStorage::Valid.to_domain(), QuoteStatus::Valid);
	}
}
