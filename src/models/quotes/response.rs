//! Quote response model for API layer

use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::{Quote, QuoteResult};

/// Response format for individual quotes in the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteResponse {
    pub quote_id: String,
    pub solver_id: String,
    pub amount_in: String,
    pub amount_out: String,
    pub token_in: TokenInfo,
    pub token_out: TokenInfo,
    pub estimated_gas: Option<u64>,
    pub price_impact: Option<f64>,
    pub expires_at: i64,
    pub response_time_ms: u64,
    pub confidence_score: Option<f64>,
}

/// API response format for individual quotes (alias for backward compatibility)
pub type QuoteApiResponse = QuoteResponse;

/// Token information for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub address: String,
    pub symbol: String,
    pub decimals: u8,
    pub chain_id: u64,
    pub name: Option<String>,
}

/// API token info format (alias for backward compatibility)
pub type TokenApiInfo = TokenInfo;

/// Route step in a multi-hop transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteStep {
    /// Protocol used for this step (e.g., "uniswap-v2", "curve", "balancer")
    pub protocol: String,

    /// Pool or pair address for this step
    pub pool_address: Option<String>,

    /// Input token for this step
    pub token_in: TokenInfo,

    /// Output token for this step
    pub token_out: TokenInfo,

    /// Input amount for this step
    pub amount_in: String,

    /// Output amount for this step
    pub amount_out: String,

    /// Fee tier for this step (if applicable)
    pub fee_tier: Option<u32>,

    /// Percentage of total amount routed through this step
    pub portion: Option<f64>,
}

/// Fee information for a quote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeInfo {
    /// Type of fee (e.g., "protocol", "gas", "bridge", "referral")
    pub fee_type: String,

    /// Fee amount (in the specified token)
    pub amount: String,

    /// Token in which the fee is denominated
    pub token: TokenInfo,

    /// Recipient of the fee
    pub recipient: Option<String>,

    /// Percentage of trade amount (for percentage-based fees)
    pub percentage: Option<f64>,
}

/// Collection of quotes response for API endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotesResponse {
    pub request_id: String,
    pub quotes: Vec<QuoteResponse>,
    pub timestamp: i64,
    pub total_quotes: usize,
}

impl QuoteResponse {
    /// Create a new quote response from a domain quote
    pub fn from_domain(quote: &Quote) -> QuoteResult<Self> {
        Ok(Self {
            quote_id: quote.quote_id.clone(),
            solver_id: quote.solver_id.clone(),
            amount_in: quote.amount_in.clone(),
            amount_out: quote.amount_out.clone(),
            token_in: TokenInfo::from_address(&quote.token_in, quote.chain_id),
            token_out: TokenInfo::from_address(&quote.token_out, quote.chain_id),
            estimated_gas: quote.estimated_gas,
            price_impact: quote.price_impact,
            expires_at: quote.expires_at.timestamp(),
            response_time_ms: quote.response_time_ms,
            confidence_score: quote.confidence_score,
        })
    }
}

impl TokenInfo {
    /// Create token info from address (simplified - in real system would lookup from registry)
    pub fn from_address(address: &str, chain_id: u64) -> Self {
        // In a real system, this would lookup token metadata from a registry or cache
        // For now, we'll use some common known tokens
        let (symbol, decimals, name) = match (address.to_lowercase().as_str(), chain_id) {
            // Ethereum mainnet
            ("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2", 1) => {
                ("WETH", 18, Some("Wrapped Ether"))
            }
            ("0xa0b86a33e6417a77c9a0c65f8e69b8b6e2b0c4a0", 1) => ("USDC", 6, Some("USD Coin")),
            ("0xdac17f958d2ee523a2206206994597c13d831ec7", 1) => ("USDT", 6, Some("Tether USD")),
            ("0x6b175474e89094c44da98b954eedeac495271d0f", 1) => {
                ("DAI", 18, Some("Dai Stablecoin"))
            }

            // Polygon
            ("0x0d500b1d8e8ef31e21c99d1db9a6444d3adf1270", 137) => {
                ("WMATIC", 18, Some("Wrapped MATIC"))
            }
            ("0x2791bca1f2de4661ed88a30c99a7a9449aa84174", 137) => {
                ("USDC", 6, Some("USD Coin (PoS)"))
            }

            // Arbitrum
            ("0x82af49447d8a07e3bd95bd0d56f35241523fbab1", 42161) => {
                ("WETH", 18, Some("Wrapped Ether"))
            }
            ("0xff970a61a04b1ca14834a43f5de4533ebddb5cc8", 42161) => {
                ("USDC", 6, Some("USD Coin (Arb1)"))
            }

            // Base
            ("0x4200000000000000000000000000000000000006", 8453) => {
                ("WETH", 18, Some("Wrapped Ether"))
            }
            ("0x833589fcd6edb6e08f4c7c32d4f71b54bda02913", 8453) => ("USDC", 6, Some("USD Coin")),

            // Default fallback
            _ => ("UNKNOWN", 18, None),
        };

        Self {
            address: address.to_string(),
            symbol: symbol.to_string(),
            decimals,
            chain_id,
            name: name.map(|s| s.to_string()),
        }
    }
}

impl QuotesResponse {
    /// Create a quotes response from domain quotes
    pub fn from_domain_quotes(request_id: String, quotes: Vec<Quote>) -> QuoteResult<Self> {
        let quote_responses: Result<Vec<_>, _> =
            quotes.iter().map(QuoteResponse::from_domain).collect();

        Ok(Self {
            request_id,
            quotes: quote_responses?,
            timestamp: Utc::now().timestamp(),
            total_quotes: quotes.len(),
        })
    }

    /// Create empty quotes response
    pub fn empty(request_id: String) -> Self {
        Self {
            request_id,
            quotes: Vec::new(),
            timestamp: Utc::now().timestamp(),
            total_quotes: 0,
        }
    }

    /// Sort quotes by amount_out (best first)
    pub fn sort_by_best_quote(&mut self) {
        self.quotes.sort_by(|a, b| {
            let a_amount: f64 = a.amount_out.parse().unwrap_or(0.0);
            let b_amount: f64 = b.amount_out.parse().unwrap_or(0.0);
            b_amount
                .partial_cmp(&a_amount)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    /// Filter out expired quotes
    pub fn filter_expired(&mut self) {
        let now = Utc::now().timestamp();
        self.quotes.retain(|quote| quote.expires_at > now);
        self.total_quotes = self.quotes.len();
    }
}

/// Convert from domain Quote to API QuoteResponse
impl From<Quote> for QuoteResponse {
    fn from(quote: Quote) -> Self {
        Self {
            quote_id: quote.quote_id,
            solver_id: quote.solver_id,
            amount_in: quote.amount_in,
            amount_out: quote.amount_out,
            token_in: TokenInfo::from_address(&quote.token_in, quote.chain_id),
            token_out: TokenInfo::from_address(&quote.token_out, quote.chain_id),
            estimated_gas: quote.estimated_gas,
            price_impact: quote.price_impact,
            expires_at: quote.expires_at.timestamp(),
            response_time_ms: quote.response_time_ms,
            confidence_score: quote.confidence_score,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::quotes::Quote;
    use chrono::{Duration, Utc};

    fn create_test_quote() -> Quote {
        Quote::new(
            "test-solver".to_string(),
            "req-123".to_string(),
            "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
            "0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0".to_string(),
            "1000000000000000000".to_string(),
            "2500000000".to_string(),
            1,
        )
        .with_estimated_gas(21000)
        .with_price_impact(0.01)
        .with_response_time(150)
        .with_confidence_score(0.95)
    }

    #[test]
    fn test_quote_response_from_domain() {
        let quote = create_test_quote();
        let response = QuoteResponse::from_domain(&quote).unwrap();

        assert_eq!(response.quote_id, quote.quote_id);
        assert_eq!(response.solver_id, quote.solver_id);
        assert_eq!(response.amount_in, quote.amount_in);
        assert_eq!(response.amount_out, quote.amount_out);
        assert_eq!(response.estimated_gas, Some(21000));
        assert_eq!(response.price_impact, Some(0.01));
        assert_eq!(response.response_time_ms, 150);
        assert_eq!(response.confidence_score, Some(0.95));
    }

    #[test]
    fn test_token_info_known_tokens() {
        let weth = TokenInfo::from_address("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", 1);
        assert_eq!(weth.symbol, "WETH");
        assert_eq!(weth.decimals, 18);
        assert_eq!(weth.name, Some("Wrapped Ether".to_string()));

        let usdc = TokenInfo::from_address("0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0", 1);
        assert_eq!(usdc.symbol, "USDC");
        assert_eq!(usdc.decimals, 6);
    }

    #[test]
    fn test_quotes_response_creation() {
        let quote1 = create_test_quote();
        let mut quote2 = create_test_quote();
        quote2.amount_out = "3000000000".to_string(); // Better quote

        let quotes = vec![quote1, quote2];
        let mut response =
            QuotesResponse::from_domain_quotes("req-123".to_string(), quotes).unwrap();

        assert_eq!(response.request_id, "req-123");
        assert_eq!(response.total_quotes, 2);

        // Test sorting
        response.sort_by_best_quote();
        assert_eq!(response.quotes[0].amount_out, "3000000000");
        assert_eq!(response.quotes[1].amount_out, "2500000000");
    }

    #[test]
    fn test_filter_expired_quotes() {
        let mut quote1 = create_test_quote();
        let quote2 = create_test_quote();

        // Make one quote expired
        quote1.expires_at = Utc::now() - Duration::minutes(1);

        let quotes = vec![quote1, quote2];
        let mut response =
            QuotesResponse::from_domain_quotes("req-123".to_string(), quotes).unwrap();

        assert_eq!(response.total_quotes, 2);

        response.filter_expired();
        assert_eq!(response.total_quotes, 1);
    }

    #[test]
    fn test_empty_response() {
        let response = QuotesResponse::empty("req-123".to_string());
        assert_eq!(response.request_id, "req-123");
        assert_eq!(response.total_quotes, 0);
        assert!(response.quotes.is_empty());
    }
}
