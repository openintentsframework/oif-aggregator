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

    /// Get storage statistics
    pub fn storage_stats(&self) -> QuoteStorageStats {
        QuoteStorageStats {
            quote_id: self.quote_id.clone(),
            created_at: self.created_at,
            last_accessed: self.last_accessed,
            access_count: self.access_count,
            ttl_seconds: self.ttl_seconds(),
            is_expired: self.is_expired(),
        }
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

/// Storage statistics for a quote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteStorageStats {
    pub quote_id: String,
    pub created_at: DateTime<Utc>,
    pub last_accessed: Option<DateTime<Utc>>,
    pub access_count: u64,
    pub ttl_seconds: i64,
    pub is_expired: bool,
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

/// Storage query filters
#[derive(Debug, Clone)]
pub struct QuoteStorageFilter {
    pub request_id: Option<String>,
    pub solver_id: Option<String>,
    pub chain_id: Option<u64>,
    pub token_in: Option<String>,
    pub token_out: Option<String>,
    pub status: Option<QuoteStatusStorage>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub include_expired: bool,
}

impl Default for QuoteStorageFilter {
    fn default() -> Self {
        Self {
            request_id: None,
            solver_id: None,
            chain_id: None,
            token_in: None,
            token_out: None,
            status: None,
            created_after: None,
            created_before: None,
            include_expired: false,
        }
    }
}

impl QuoteStorageFilter {
    /// Create a new filter
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by request ID
    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    /// Filter by solver ID
    pub fn with_solver_id(mut self, solver_id: String) -> Self {
        self.solver_id = Some(solver_id);
        self
    }

    /// Filter by chain ID
    pub fn with_chain_id(mut self, chain_id: u64) -> Self {
        self.chain_id = Some(chain_id);
        self
    }

    /// Filter by token pair
    pub fn with_token_pair(mut self, token_in: String, token_out: String) -> Self {
        self.token_in = Some(token_in);
        self.token_out = Some(token_out);
        self
    }

    /// Filter by status
    pub fn with_status(mut self, status: QuoteStatusStorage) -> Self {
        self.status = Some(status);
        self
    }

    /// Include expired quotes
    pub fn include_expired(mut self) -> Self {
        self.include_expired = true;
        self
    }

    /// Check if a quote matches this filter
    pub fn matches(&self, quote: &QuoteStorage) -> bool {
        if let Some(ref request_id) = self.request_id {
            if quote.request_id != *request_id {
                return false;
            }
        }

        if let Some(ref solver_id) = self.solver_id {
            if quote.solver_id != *solver_id {
                return false;
            }
        }

        if let Some(chain_id) = self.chain_id {
            if quote.chain_id != chain_id {
                return false;
            }
        }

        if let Some(ref token_in) = self.token_in {
            if quote.token_in != *token_in {
                return false;
            }
        }

        if let Some(ref token_out) = self.token_out {
            if quote.token_out != *token_out {
                return false;
            }
        }

        if let Some(ref status) = self.status {
            if quote.status != *status {
                return false;
            }
        }

        if let Some(created_after) = self.created_after {
            if quote.created_at <= created_after {
                return false;
            }
        }

        if let Some(created_before) = self.created_before {
            if quote.created_at >= created_before {
                return false;
            }
        }

        if !self.include_expired && quote.is_expired() {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::quotes::Quote;
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

    #[test]
    fn test_storage_filter() {
        let quote = create_test_quote();
        let storage = QuoteStorage::from_domain(quote);

        // Test basic filter
        let filter = QuoteStorageFilter::new()
            .with_request_id("req-123".to_string())
            .with_chain_id(1);
        assert!(filter.matches(&storage));

        // Test non-matching filter
        let filter = QuoteStorageFilter::new().with_request_id("different-req".to_string());
        assert!(!filter.matches(&storage));

        // Test expired filter
        let mut expired_storage = storage.clone();
        expired_storage.expires_at = Utc::now() - Duration::minutes(1);

        let filter = QuoteStorageFilter::new();
        assert!(!filter.matches(&expired_storage)); // Excludes expired by default

        let filter = QuoteStorageFilter::new().include_expired();
        assert!(filter.matches(&expired_storage)); // Includes expired when specified
    }

    #[test]
    fn test_storage_stats() {
        let quote = create_test_quote();
        let mut storage = QuoteStorage::from_domain(quote);

        storage.mark_accessed();
        storage.mark_accessed();

        let stats = storage.storage_stats();
        assert_eq!(stats.quote_id, storage.quote_id);
        assert_eq!(stats.access_count, 2);
        assert!(stats.last_accessed.is_some());
        assert!(!stats.is_expired);
        assert!(stats.ttl_seconds > 0);
    }
}
