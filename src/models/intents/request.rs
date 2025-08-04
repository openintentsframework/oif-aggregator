//! Intent request models and validation

use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use std::collections::HashMap;

use super::{
    Intent, IntentFees, IntentMetadata, IntentPriority, IntentQuoteData, IntentValidationError,
    IntentValidationResult,
};

/// API request body for submitting intents
#[derive(Debug, Clone, Deserialize)]
pub struct IntentsRequest {
    /// Associated quote ID (if using a specific quote)
    pub quote_id: Option<String>,

    /// Quote data (if not using quote_id)
    pub quote_response: Option<QuoteResponseRequest>,

    /// User's wallet address
    pub user_address: String,

    /// Maximum acceptable slippage (0.0 to 1.0)
    pub slippage_tolerance: Option<f64>,

    /// Deadline for intent execution (Unix timestamp)
    pub deadline: Option<i64>,

    /// User's signature for authorization
    pub signature: Option<String>,

    /// Referrer information
    pub referrer: Option<String>,

    /// Session identifier
    pub session_id: Option<String>,

    /// Additional metadata
    pub metadata: Option<HashMap<String, serde_json::Value>>,

    /// Client information
    pub client_info: Option<ClientInfo>,

    /// Fee preferences
    pub fee_preferences: Option<FeePreferences>,

    /// Execution preferences
    pub execution_preferences: Option<ExecutionPreferences>,
}

/// Quote data in API request format
#[derive(Debug, Clone, Deserialize)]
pub struct QuoteResponseRequest {
    /// Input token address
    pub token_in: String,

    /// Output token address
    pub token_out: String,

    /// Input amount
    pub amount_in: String,

    /// Expected output amount
    pub amount_out: String,

    /// Blockchain network
    pub chain_id: u64,

    /// Price impact estimation
    pub price_impact: Option<f64>,

    /// Gas estimation
    pub estimated_gas: Option<u64>,

    /// Route information
    pub route: Option<serde_json::Value>,

    /// Quote expiration time (Unix timestamp)
    pub expires_at: Option<i64>,

    /// Quote ID from the solver
    pub solver_quote_id: Option<String>,

    /// Solver that provided this quote
    pub solver_id: Option<String>,
}

/// Client information for tracking and analytics
#[derive(Debug, Clone, Deserialize)]
pub struct ClientInfo {
    /// User agent string
    pub user_agent: Option<String>,

    /// Client application name
    pub app_name: Option<String>,

    /// Client version
    pub app_version: Option<String>,

    /// Platform (web, mobile, desktop)
    pub platform: Option<String>,

    /// IP address (usually set by the server)
    pub ip_address: Option<String>,

    /// Geographic location
    pub location: Option<String>,
}

/// Fee preferences for intent execution
#[derive(Debug, Clone, Deserialize)]
pub struct FeePreferences {
    /// Maximum acceptable platform fee rate (0.0 to 1.0)
    pub max_platform_fee_rate: Option<f64>,

    /// Preferred fee currency
    pub fee_currency: Option<String>,

    /// Whether to pay fees in input token
    pub pay_fees_in_input_token: Option<bool>,

    /// Maximum gas price (in gwei)
    pub max_gas_price_gwei: Option<u64>,

    /// Gas preference (fast, standard, slow)
    pub gas_preference: Option<String>,
}

/// Execution preferences for intent processing
#[derive(Debug, Clone, Deserialize)]
pub struct ExecutionPreferences {
    /// Preferred solver ID
    pub preferred_solver: Option<String>,

    /// Maximum execution time in seconds
    pub max_execution_time_secs: Option<u64>,

    /// Priority level (low, normal, high, critical)
    pub priority: Option<String>,

    /// Whether to allow partial fills
    pub allow_partial_fills: Option<bool>,

    /// Retry preferences
    pub retry_preferences: Option<RetryPreferences>,

    /// MEV protection preferences
    pub mev_protection: Option<MevProtection>,
}

/// Retry preferences for failed intents
#[derive(Debug, Clone, Deserialize)]
pub struct RetryPreferences {
    /// Maximum number of retries
    pub max_retries: Option<u32>,

    /// Retry delay in seconds
    pub retry_delay_secs: Option<u64>,

    /// Whether to use exponential backoff
    pub exponential_backoff: Option<bool>,

    /// Maximum retry delay in seconds
    pub max_retry_delay_secs: Option<u64>,
}

/// MEV protection preferences
#[derive(Debug, Clone, Deserialize)]
pub struct MevProtection {
    /// Whether to enable MEV protection
    pub enabled: Option<bool>,

    /// MEV protection level (basic, advanced)
    pub level: Option<String>,

    /// Maximum acceptable MEV loss percentage
    pub max_mev_loss_percent: Option<f64>,
}

impl IntentsRequest {
    /// Validate the intent request
    pub fn validate(&self) -> IntentValidationResult<()> {
        // Validate user address
        if self.user_address.is_empty() {
            return Err(IntentValidationError::MissingRequiredField {
                field: "user_address".to_string(),
            });
        }

        if !is_valid_ethereum_address(&self.user_address) {
            return Err(IntentValidationError::InvalidUserAddress {
                address: self.user_address.clone(),
            });
        }

        // Validate that either quote_id or quote_response is provided
        match (&self.quote_id, &self.quote_response) {
            (Some(_), Some(_)) => {
                return Err(IntentValidationError::ConflictingQuoteData);
            }
            (None, None) => {
                return Err(IntentValidationError::MissingQuoteData);
            }
            _ => {}
        }

        // Validate quote_id if provided
        if let Some(ref quote_id) = self.quote_id {
            if quote_id.is_empty() {
                return Err(IntentValidationError::InvalidQuoteId {
                    quote_id: quote_id.clone(),
                });
            }
        }

        // Validate quote_response if provided
        if let Some(ref quote_response) = self.quote_response {
            quote_response.validate()?;
        }

        // Validate slippage tolerance
        if let Some(slippage) = self.slippage_tolerance {
            if slippage < 0.0 || slippage > 1.0 {
                return Err(IntentValidationError::InvalidSlippage { slippage });
            }
        }

        // Validate deadline
        if let Some(deadline) = self.deadline {
            let deadline_dt = DateTime::from_timestamp(deadline, 0).ok_or_else(|| {
                IntentValidationError::InvalidDeadline {
                    reason: "Invalid timestamp format".to_string(),
                }
            })?;

            if deadline_dt <= Utc::now() {
                return Err(IntentValidationError::DeadlineInPast);
            }

            if deadline_dt > Utc::now() + Duration::hours(24) {
                return Err(IntentValidationError::DeadlineTooFar { max_hours: 24 });
            }
        }

        // Validate signature format if provided
        if let Some(ref signature) = self.signature {
            if !signature.starts_with("0x") || signature.len() != 132 {
                return Err(IntentValidationError::InvalidSignature {
                    reason: "Signature must be 0x-prefixed 65-byte hex string".to_string(),
                });
            }
        }

        // Validate fee preferences
        if let Some(ref fee_prefs) = self.fee_preferences {
            fee_prefs.validate()?;
        }

        // Validate execution preferences
        if let Some(ref exec_prefs) = self.execution_preferences {
            exec_prefs.validate()?;
        }

        Ok(())
    }

    /// Convert API request to domain intent
    pub fn to_domain(&self, ip_address: Option<String>) -> IntentValidationResult<Intent> {
        // Validate first
        self.validate()?;

        // Determine deadline (default to 20 minutes if not provided)
        let deadline = if let Some(deadline_ts) = self.deadline {
            DateTime::from_timestamp(deadline_ts, 0).ok_or_else(|| {
                IntentValidationError::InvalidDeadline {
                    reason: "Invalid timestamp format".to_string(),
                }
            })?
        } else {
            Utc::now() + Duration::minutes(20)
        };

        // Create base intent
        let mut intent = Intent::new(
            self.user_address.clone(),
            self.slippage_tolerance.unwrap_or(0.005), // Default 0.5%
            deadline,
        );

        // Set quote information
        if let Some(ref quote_id) = self.quote_id {
            intent = intent.with_quote_id(quote_id.clone());
        }

        if let Some(ref quote_response) = self.quote_response {
            let quote_data = quote_response.to_domain()?;
            intent = intent.with_quote_data(quote_data);
        }

        // Set signature
        if let Some(ref signature) = self.signature {
            intent = intent.with_signature(signature.clone());
        }

        // Create metadata
        let mut metadata = IntentMetadata::new("api".to_string());

        if let Some(ref client_info) = self.client_info {
            if let Some(ref user_agent) = client_info.user_agent {
                metadata = metadata.with_user_agent(user_agent.clone());
            }
        }

        if let Some(ref session_id) = self.session_id {
            metadata = metadata.with_session_id(session_id.clone());
        }

        if let Some(ip) = ip_address {
            metadata = metadata.with_ip_address(ip);
        }

        if let Some(ref referrer) = self.referrer {
            metadata = metadata.with_custom_data(
                "referrer".to_string(),
                serde_json::Value::String(referrer.clone()),
            );
        }

        // Add custom metadata
        if let Some(ref custom_metadata) = self.metadata {
            for (key, value) in custom_metadata {
                metadata = metadata.with_custom_data(key.clone(), value.clone());
            }
        }

        // Add client info to metadata
        if let Some(ref client_info) = self.client_info {
            if let Some(ref app_name) = client_info.app_name {
                metadata = metadata.with_custom_data(
                    "app_name".to_string(),
                    serde_json::Value::String(app_name.clone()),
                );
            }
            if let Some(ref app_version) = client_info.app_version {
                metadata = metadata.with_custom_data(
                    "app_version".to_string(),
                    serde_json::Value::String(app_version.clone()),
                );
            }
            if let Some(ref platform) = client_info.platform {
                metadata = metadata.with_custom_data(
                    "platform".to_string(),
                    serde_json::Value::String(platform.clone()),
                );
            }
        }

        intent = intent.with_metadata(metadata);

        // Set priority from execution preferences
        if let Some(ref exec_prefs) = self.execution_preferences {
            if let Some(ref priority_str) = exec_prefs.priority {
                let priority = match priority_str.as_str() {
                    "low" => IntentPriority::Low,
                    "normal" => IntentPriority::Normal,
                    "high" => IntentPriority::High,
                    "critical" => IntentPriority::Critical,
                    _ => IntentPriority::Normal,
                };
                intent = intent.with_priority(priority);
            }
        }

        // Set fees
        let mut fees = IntentFees::default();
        if let Some(ref fee_prefs) = self.fee_preferences {
            if let Some(max_fee_rate) = fee_prefs.max_platform_fee_rate {
                fees.platform_fee_rate = fees.platform_fee_rate.min(max_fee_rate);
            }
            if let Some(ref fee_currency) = fee_prefs.fee_currency {
                fees.fee_currency = fee_currency.clone();
            }
        }
        intent = intent.with_fees(fees);

        Ok(intent)
    }
}

impl QuoteResponseRequest {
    /// Validate quote response data
    pub fn validate(&self) -> IntentValidationResult<()> {
        // Validate token addresses
        if !is_valid_ethereum_address(&self.token_in) {
            return Err(IntentValidationError::UnsupportedToken {
                token_address: self.token_in.clone(),
                chain_id: self.chain_id,
            });
        }

        if !is_valid_ethereum_address(&self.token_out) {
            return Err(IntentValidationError::UnsupportedToken {
                token_address: self.token_out.clone(),
                chain_id: self.chain_id,
            });
        }

        // Validate amounts
        if self.amount_in.is_empty() || self.amount_out.is_empty() {
            return Err(IntentValidationError::MissingRequiredField {
                field: "amount_in or amount_out".to_string(),
            });
        }

        // Try to parse amounts
        self.amount_in
            .parse::<u64>()
            .map_err(|_| IntentValidationError::InvalidConfiguration {
                reason: "Invalid amount_in format".to_string(),
            })?;

        self.amount_out.parse::<u64>().map_err(|_| {
            IntentValidationError::InvalidConfiguration {
                reason: "Invalid amount_out format".to_string(),
            }
        })?;

        // Validate chain ID
        if !is_valid_chain_id(self.chain_id) {
            return Err(IntentValidationError::UnsupportedChain {
                chain_id: self.chain_id,
            });
        }

        // Validate price impact if provided
        if let Some(price_impact) = self.price_impact {
            if price_impact < 0.0 || price_impact > 1.0 {
                return Err(IntentValidationError::InvalidConfiguration {
                    reason: "Price impact must be between 0.0 and 1.0".to_string(),
                });
            }
        }

        // Validate expiration
        if let Some(expires_at) = self.expires_at {
            let expires_dt = DateTime::from_timestamp(expires_at, 0).ok_or_else(|| {
                IntentValidationError::InvalidDeadline {
                    reason: "Invalid expires_at timestamp".to_string(),
                }
            })?;

            if expires_dt <= Utc::now() {
                return Err(IntentValidationError::QuoteExpired {
                    quote_id: self
                        .solver_quote_id
                        .clone()
                        .unwrap_or("unknown".to_string()),
                });
            }
        }

        Ok(())
    }

    /// Convert to domain quote data
    pub fn to_domain(&self) -> IntentValidationResult<IntentQuoteData> {
        self.validate()?;

        let expires_at = if let Some(expires_ts) = self.expires_at {
            DateTime::from_timestamp(expires_ts, 0).ok_or_else(|| {
                IntentValidationError::InvalidDeadline {
                    reason: "Invalid expires_at timestamp".to_string(),
                }
            })?
        } else {
            Utc::now() + Duration::minutes(10) // Default 10 minute expiry
        };

        let mut quote_data = IntentQuoteData::new(
            self.token_in.clone(),
            self.token_out.clone(),
            self.amount_in.clone(),
            self.amount_out.clone(),
            self.chain_id,
            expires_at,
        );

        if let Some(price_impact) = self.price_impact {
            quote_data = quote_data.with_price_impact(price_impact);
        }

        if let Some(estimated_gas) = self.estimated_gas {
            quote_data = quote_data.with_gas_estimate(estimated_gas);
        }

        if let Some(ref route) = self.route {
            quote_data = quote_data.with_route_info(route.clone());
        }

        Ok(quote_data)
    }
}

impl FeePreferences {
    pub fn validate(&self) -> IntentValidationResult<()> {
        if let Some(max_fee_rate) = self.max_platform_fee_rate {
            if max_fee_rate < 0.0 || max_fee_rate > 0.1 {
                return Err(IntentValidationError::InvalidConfiguration {
                    reason: "Max platform fee rate must be between 0.0 and 0.1 (10%)".to_string(),
                });
            }
        }

        if let Some(max_gas_price) = self.max_gas_price_gwei {
            if max_gas_price > 1000 {
                return Err(IntentValidationError::InvalidConfiguration {
                    reason: "Max gas price cannot exceed 1000 gwei".to_string(),
                });
            }
        }

        Ok(())
    }
}

impl ExecutionPreferences {
    pub fn validate(&self) -> IntentValidationResult<()> {
        if let Some(max_execution_time) = self.max_execution_time_secs {
            if max_execution_time > 3600 {
                return Err(IntentValidationError::InvalidConfiguration {
                    reason: "Max execution time cannot exceed 1 hour".to_string(),
                });
            }
        }

        if let Some(ref priority) = self.priority {
            if !matches!(priority.as_str(), "low" | "normal" | "high" | "critical") {
                return Err(IntentValidationError::InvalidConfiguration {
                    reason: "Priority must be one of: low, normal, high, critical".to_string(),
                });
            }
        }

        if let Some(ref retry_prefs) = self.retry_preferences {
            retry_prefs.validate()?;
        }

        Ok(())
    }
}

impl RetryPreferences {
    pub fn validate(&self) -> IntentValidationResult<()> {
        if let Some(max_retries) = self.max_retries {
            if max_retries > 10 {
                return Err(IntentValidationError::InvalidConfiguration {
                    reason: "Max retries cannot exceed 10".to_string(),
                });
            }
        }

        if let Some(max_delay) = self.max_retry_delay_secs {
            if max_delay > 3600 {
                return Err(IntentValidationError::InvalidConfiguration {
                    reason: "Max retry delay cannot exceed 1 hour".to_string(),
                });
            }
        }

        Ok(())
    }
}

/// Validate Ethereum address format
fn is_valid_ethereum_address(address: &str) -> bool {
    address.starts_with("0x")
        && address.len() == 42
        && address[2..].chars().all(|c| c.is_ascii_hexdigit())
}

/// Check if a chain ID is supported
fn is_valid_chain_id(chain_id: u64) -> bool {
    const SUPPORTED_CHAINS: &[u64] = &[
        1,        // Ethereum Mainnet
        5,        // Goerli (testnet)
        10,       // Optimism
        56,       // BSC
        100,      // Gnosis
        137,      // Polygon
        250,      // Fantom
        8453,     // Base
        42161,    // Arbitrum One
        43114,    // Avalanche
        11155111, // Sepolia (testnet)
    ];

    SUPPORTED_CHAINS.contains(&chain_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_valid_request() -> IntentsRequest {
        IntentsRequest {
            quote_id: Some("quote-123".to_string()),
            quote_response: None,
            user_address: "0x742d35Cc6634C0532925a3b8D02d8f56B2E8E36e".to_string(),
            slippage_tolerance: Some(0.005),
            deadline: Some((Utc::now() + Duration::hours(1)).timestamp()),
            signature: Some("0x".to_string() + &"a".repeat(130)),
            referrer: Some("app.example.com".to_string()),
            session_id: Some("session-123".to_string()),
            metadata: Some(HashMap::new()),
            client_info: None,
            fee_preferences: None,
            execution_preferences: None,
        }
    }

    fn create_valid_quote_response() -> QuoteResponseRequest {
        QuoteResponseRequest {
            token_in: "0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0".to_string(),
            token_out: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
            amount_in: "1000000".to_string(),
            amount_out: "2000000000000000000".to_string(),
            chain_id: 1,
            price_impact: Some(0.01),
            estimated_gas: Some(200000),
            route: Some(json!({"path": ["tokenA", "tokenB"]})),
            expires_at: Some((Utc::now() + Duration::minutes(10)).timestamp()),
            solver_quote_id: Some("solver-quote-123".to_string()),
            solver_id: Some("solver-1".to_string()),
        }
    }

    #[test]
    fn test_valid_intent_request() {
        let request = create_valid_request();
        assert!(request.validate().is_ok());

        let intent = request.to_domain(Some("192.168.1.1".to_string())).unwrap();
        assert_eq!(
            intent.user_address,
            "0x742d35Cc6634C0532925a3b8D02d8f56B2E8E36e"
        );
        assert_eq!(intent.slippage_tolerance, 0.005);
        assert_eq!(intent.quote_id, Some("quote-123".to_string()));
    }

    #[test]
    fn test_invalid_user_address() {
        let mut request = create_valid_request();
        request.user_address = "invalid-address".to_string();

        assert!(request.validate().is_err());
    }

    #[test]
    fn test_missing_quote_data() {
        let mut request = create_valid_request();
        request.quote_id = None;
        request.quote_response = None;

        assert!(matches!(
            request.validate(),
            Err(IntentValidationError::MissingQuoteData)
        ));
    }

    #[test]
    fn test_conflicting_quote_data() {
        let mut request = create_valid_request();
        request.quote_response = Some(create_valid_quote_response());

        assert!(matches!(
            request.validate(),
            Err(IntentValidationError::ConflictingQuoteData)
        ));
    }

    #[test]
    fn test_invalid_slippage() {
        let mut request = create_valid_request();
        request.slippage_tolerance = Some(1.5); // 150% is invalid

        assert!(matches!(
            request.validate(),
            Err(IntentValidationError::InvalidSlippage { slippage: 1.5 })
        ));
    }

    #[test]
    fn test_deadline_in_past() {
        let mut request = create_valid_request();
        request.deadline = Some((Utc::now() - Duration::hours(1)).timestamp());

        assert!(matches!(
            request.validate(),
            Err(IntentValidationError::DeadlineInPast)
        ));
    }

    #[test]
    fn test_deadline_too_far() {
        let mut request = create_valid_request();
        request.deadline = Some((Utc::now() + Duration::hours(25)).timestamp());

        assert!(matches!(
            request.validate(),
            Err(IntentValidationError::DeadlineTooFar { max_hours: 24 })
        ));
    }

    #[test]
    fn test_quote_response_validation() {
        let quote_response = create_valid_quote_response();
        assert!(quote_response.validate().is_ok());

        let quote_data = quote_response.to_domain().unwrap();
        assert_eq!(quote_data.chain_id, 1);
        assert_eq!(quote_data.price_impact, Some(0.01));
    }

    #[test]
    fn test_invalid_token_address() {
        let mut quote_response = create_valid_quote_response();
        quote_response.token_in = "invalid".to_string();

        assert!(quote_response.validate().is_err());
    }

    #[test]
    fn test_expired_quote() {
        let mut quote_response = create_valid_quote_response();
        quote_response.expires_at = Some((Utc::now() - Duration::minutes(1)).timestamp());

        assert!(matches!(
            quote_response.validate(),
            Err(IntentValidationError::QuoteExpired { .. })
        ));
    }

    #[test]
    fn test_fee_preferences_validation() {
        let fee_prefs = FeePreferences {
            max_platform_fee_rate: Some(0.05), // 5% is valid
            fee_currency: Some("USDC".to_string()),
            pay_fees_in_input_token: Some(true),
            max_gas_price_gwei: Some(100),
            gas_preference: Some("fast".to_string()),
        };

        assert!(fee_prefs.validate().is_ok());

        let invalid_fee_prefs = FeePreferences {
            max_platform_fee_rate: Some(0.15), // 15% is too high
            ..fee_prefs
        };

        assert!(invalid_fee_prefs.validate().is_err());
    }

    #[test]
    fn test_execution_preferences_validation() {
        let exec_prefs = ExecutionPreferences {
            preferred_solver: Some("solver-1".to_string()),
            max_execution_time_secs: Some(300), // 5 minutes
            priority: Some("high".to_string()),
            allow_partial_fills: Some(false),
            retry_preferences: None,
            mev_protection: None,
        };

        assert!(exec_prefs.validate().is_ok());

        let invalid_exec_prefs = ExecutionPreferences {
            priority: Some("invalid-priority".to_string()),
            ..exec_prefs
        };

        assert!(invalid_exec_prefs.validate().is_err());
    }

    #[test]
    fn test_ethereum_address_validation() {
        assert!(is_valid_ethereum_address(
            "0x742d35Cc6634C0532925a3b8D02d8f56B2E8E36e"
        ));
        assert!(!is_valid_ethereum_address(
            "742d35Cc6634C0532925a3b8D02d8f56B2E8E36e"
        ));
        assert!(!is_valid_ethereum_address(
            "0x742d35Cc6634C0532925a3b8D02d8f56B2E8E36"
        ));
        assert!(!is_valid_ethereum_address(
            "0xGGGd35Cc6634C0532925a3b8D02d8f56B2E8E36e"
        ));
    }

    #[test]
    fn test_chain_id_validation() {
        assert!(is_valid_chain_id(1)); // Ethereum
        assert!(is_valid_chain_id(137)); // Polygon
        assert!(!is_valid_chain_id(999999)); // Invalid
    }

    #[test]
    fn test_request_with_quote_response() {
        let mut request = create_valid_request();
        request.quote_id = None;
        request.quote_response = Some(create_valid_quote_response());

        assert!(request.validate().is_ok());

        let intent = request.to_domain(None).unwrap();
        assert!(intent.quote_data.is_some());
        assert!(intent.quote_id.is_none());
    }

    #[test]
    fn test_request_with_client_info() {
        let mut request = create_valid_request();
        request.client_info = Some(ClientInfo {
            user_agent: Some("Test/1.0".to_string()),
            app_name: Some("TestApp".to_string()),
            app_version: Some("1.0.0".to_string()),
            platform: Some("web".to_string()),
            ip_address: None,
            location: Some("US".to_string()),
        });

        let intent = request.to_domain(Some("192.168.1.1".to_string())).unwrap();
        assert!(intent.metadata.custom_data.contains_key("app_name"));
        assert_eq!(intent.metadata.ip_address, Some("192.168.1.1".to_string()));
    }
}
