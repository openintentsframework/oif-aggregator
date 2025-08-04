//! Intent storage model and conversions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::errors::IntentResult;
use super::{
    Intent, IntentError, IntentExecutionResult, IntentFees, IntentMetadata, IntentPriority,
    IntentQuoteData, IntentStatus,
};

/// Storage representation of an intent
///
/// This model is optimized for storage and persistence.
/// It can be converted to/from the domain Intent model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentStorage {
    pub intent_id: String,
    pub quote_id: Option<String>,
    pub quote_data: Option<IntentQuoteDataStorage>,
    pub user_address: String,
    pub slippage_tolerance: f64,
    pub deadline: DateTime<Utc>,
    pub signature: Option<String>,
    pub metadata: IntentMetadataStorage,
    pub status: IntentStatusStorage,
    pub priority: IntentPriorityStorage,
    pub execution_history: Vec<IntentExecutionResultStorage>,
    pub retry_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_processed_at: Option<DateTime<Utc>>,
    pub estimated_execution_time_ms: Option<u64>,
    pub assigned_solver: Option<String>,
    pub fees: IntentFeesStorage,

    // Storage-specific metadata
    pub version_schema: u32,
    pub storage_size_bytes: u64,
    pub access_count: u64,
    pub last_accessed: Option<DateTime<Utc>>,
    pub indexed_at: DateTime<Utc>,
    pub archived: bool,
    pub tags: Vec<String>,
    pub partition_key: String,      // For efficient querying by user/date
    pub expiry_date: DateTime<Utc>, // For automated cleanup
}

/// Storage-compatible intent status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum IntentStatusStorage {
    Pending,
    Submitted,
    Queued,
    Executing,
    Success,
    Failed,
    Cancelled,
    Expired,
    Simulating,
    ReviewRequired,
}

/// Storage-compatible intent priority
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum IntentPriorityStorage {
    Low,
    Normal,
    High,
    Critical,
}

/// Storage-compatible intent metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentMetadataStorage {
    pub source: String,
    pub user_agent: Option<String>,
    pub referrer: Option<String>,
    pub session_id: Option<String>,
    pub ip_address: Option<String>,
    pub custom_data: HashMap<String, serde_json::Value>,
    pub location: Option<String>,
    pub client_version: Option<String>,

    // Storage-specific metadata
    pub processed_for_analytics: bool,
    pub compliance_checked: bool,
    pub risk_score: Option<f64>,
    pub fraud_flags: Vec<String>,
}

/// Storage-compatible quote data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentQuoteDataStorage {
    pub token_in: String,
    pub token_out: String,
    pub amount_in: String,
    pub amount_out: String,
    pub chain_id: u64,
    pub price_impact: Option<f64>,
    pub estimated_gas: Option<u64>,
    pub route_info: Option<serde_json::Value>,
    pub expires_at: DateTime<Utc>,

    // Additional storage fields
    pub token_in_symbol: Option<String>,
    pub token_out_symbol: Option<String>,
    pub usd_value: Option<f64>,
    pub source_solver: Option<String>,
    pub quote_created_at: Option<DateTime<Utc>>,
}

/// Storage-compatible fee information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentFeesStorage {
    pub platform_fee_rate: f64,
    pub platform_fee_amount: Option<String>,
    pub gas_fee: Option<String>,
    pub solver_fee: Option<String>,
    pub total_estimated_fee: Option<String>,
    pub fee_currency: String,

    // Fee tracking for analytics
    pub actual_platform_fee: Option<String>,
    pub actual_gas_fee: Option<String>,
    pub actual_solver_fee: Option<String>,
    pub total_actual_fee: Option<String>,
    pub fee_difference_percent: Option<f64>,
}

/// Storage-compatible execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentExecutionResultStorage {
    pub success: bool,
    pub transaction_hash: Option<String>,
    pub block_number: Option<u64>,
    pub gas_used: Option<u64>,
    pub effective_gas_price: Option<String>,
    pub amount_in: Option<String>,
    pub amount_out: Option<String>,
    pub actual_price_impact: Option<f64>,
    pub gas_cost: Option<String>,
    pub execution_time_ms: Option<u64>,
    pub error_message: Option<String>,
    pub solver_used: Option<String>,
    pub retry_count: u32,
    pub timestamp: DateTime<Utc>,

    // Extended execution tracking
    pub mempool_time_ms: Option<u64>,
    pub confirmation_time_ms: Option<u64>,
    pub network_congestion_score: Option<f64>,
    pub mev_detected: bool,
    pub sandwich_attack_detected: bool,
    pub execution_environment: String, // mainnet, testnet, simulation
}

/// Daily intent statistics for analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyIntentStats {
    pub date: String, // YYYY-MM-DD format
    pub total_intents: u64,
    pub successful_intents: u64,
    pub failed_intents: u64,
    pub cancelled_intents: u64,
    pub avg_execution_time_ms: f64,
    pub total_volume_usd: f64,
    pub unique_users: u64,
    pub top_solvers: HashMap<String, u64>,
    pub error_breakdown: HashMap<String, u32>,
}

/// Intent performance metrics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentPerformanceMetrics {
    pub intent_id: String,
    pub user_address: String,
    pub execution_score: f64,
    pub gas_efficiency: f64,
    pub price_efficiency: f64,
    pub time_efficiency: f64,
    pub total_fees_usd: f64,
    pub profit_generated_usd: Option<f64>,
    pub solver_performance: HashMap<String, f64>,
    pub calculated_at: DateTime<Utc>,
}

impl IntentStorage {
    /// Create storage intent from domain intent
    pub fn from_domain(intent: Intent) -> Self {
        let storage_size = estimate_storage_size(&intent);
        let partition_key = generate_partition_key(&intent.user_address, &intent.created_at);
        let expiry_date = intent.deadline + chrono::Duration::days(30); // Keep for 30 days after deadline

        Self {
            intent_id: intent.intent_id,
            quote_id: intent.quote_id,
            quote_data: intent.quote_data.map(IntentQuoteDataStorage::from_domain),
            user_address: intent.user_address,
            slippage_tolerance: intent.slippage_tolerance,
            deadline: intent.deadline,
            signature: intent.signature,
            metadata: IntentMetadataStorage::from_domain(intent.metadata),
            status: IntentStatusStorage::from_domain(intent.status),
            priority: IntentPriorityStorage::from_domain(intent.priority),
            execution_history: intent
                .execution_history
                .into_iter()
                .map(IntentExecutionResultStorage::from_domain)
                .collect(),
            retry_count: intent.retry_count,
            created_at: intent.created_at,
            updated_at: intent.updated_at,
            last_processed_at: intent.last_processed_at,
            estimated_execution_time_ms: intent.estimated_execution_time_ms,
            assigned_solver: intent.assigned_solver,
            fees: IntentFeesStorage::from_domain(intent.fees),
            version_schema: 1,
            storage_size_bytes: storage_size,
            access_count: 0,
            last_accessed: None,
            indexed_at: Utc::now(),
            archived: false,
            tags: Vec::new(),
            partition_key,
            expiry_date,
        }
    }

    /// Convert storage intent to domain intent
    pub fn to_domain(self) -> IntentResult<Intent> {
        Ok(Intent {
            intent_id: self.intent_id,
            quote_id: self.quote_id,
            quote_data: self.quote_data.map(|qd| qd.to_domain()).transpose()?,
            user_address: self.user_address,
            slippage_tolerance: self.slippage_tolerance,
            deadline: self.deadline,
            signature: self.signature,
            metadata: self.metadata.to_domain(),
            status: self.status.to_domain(),
            priority: self.priority.to_domain(),
            execution_history: self
                .execution_history
                .into_iter()
                .map(|er| er.to_domain())
                .collect::<Result<Vec<_>, _>>()?,
            retry_count: self.retry_count,
            created_at: self.created_at,
            updated_at: self.updated_at,
            last_processed_at: self.last_processed_at,
            estimated_execution_time_ms: self.estimated_execution_time_ms,
            assigned_solver: self.assigned_solver,
            fees: self.fees.to_domain()?,
        })
    }

    /// Mark as accessed for analytics
    pub fn mark_accessed(&mut self) {
        self.access_count += 1;
        self.last_accessed = Some(Utc::now());
    }

    /// Update status and timestamp
    pub fn update_status(&mut self, status: IntentStatusStorage) {
        self.status = status;
        self.updated_at = Utc::now();
        self.mark_accessed();
    }

    /// Add a tag for categorization
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
            self.updated_at = Utc::now();
        }
    }

    /// Remove a tag
    pub fn remove_tag(&mut self, tag: &str) {
        if let Some(pos) = self.tags.iter().position(|t| t == tag) {
            self.tags.remove(pos);
            self.updated_at = Utc::now();
        }
    }

    /// Archive the intent
    pub fn archive(&mut self) {
        self.archived = true;
        self.updated_at = Utc::now();
    }

    /// Check if intent should be cleaned up
    pub fn should_cleanup(&self) -> bool {
        Utc::now() > self.expiry_date
    }

    /// Check if intent is stale (hasn't been accessed recently)
    pub fn is_stale(&self, max_age_hours: i64) -> bool {
        if let Some(last_accessed) = self.last_accessed {
            let stale_threshold = Utc::now() - chrono::Duration::hours(max_age_hours);
            last_accessed < stale_threshold
        } else {
            let stale_threshold = Utc::now() - chrono::Duration::hours(max_age_hours);
            self.created_at < stale_threshold
        }
    }

    /// Calculate performance metrics
    pub fn calculate_performance_metrics(&self) -> IntentPerformanceMetrics {
        let execution_score = if self.status == IntentStatusStorage::Success {
            let base_score = 100.0;
            let retry_penalty = self.retry_count as f64 * 10.0;
            let time_bonus = if let Some(exec_time) = self.estimated_execution_time_ms {
                if exec_time < 5000 { 10.0 } else { 0.0 }
            } else {
                0.0
            };

            (base_score - retry_penalty + time_bonus).max(0.0)
        } else {
            0.0
        };

        // Calculate gas efficiency (simplified)
        let gas_efficiency = if let Some(result) = self.execution_history.iter().find(|r| r.success)
        {
            if let Some(gas_used) = result.gas_used {
                if gas_used < 100000 {
                    100.0
                } else {
                    (200000.0 / gas_used as f64 * 100.0).min(100.0)
                }
            } else {
                50.0
            }
        } else {
            0.0
        };

        // Calculate price efficiency (simplified)
        let price_efficiency =
            if let Some(result) = self.execution_history.iter().find(|r| r.success) {
                if let Some(price_impact) = result.actual_price_impact {
                    ((1.0 - price_impact) * 100.0).max(0.0)
                } else {
                    80.0
                }
            } else {
                0.0
            };

        // Calculate time efficiency
        let time_efficiency =
            if let Some(result) = self.execution_history.iter().find(|r| r.success) {
                if let Some(exec_time) = result.execution_time_ms {
                    if exec_time < 10000 {
                        100.0
                    } else {
                        (20000.0 / exec_time as f64 * 100.0).min(100.0)
                    }
                } else {
                    50.0
                }
            } else {
                0.0
            };

        // Calculate total fees
        let total_fees_usd = self
            .fees
            .total_actual_fee
            .as_ref()
            .or(self.fees.total_estimated_fee.as_ref())
            .and_then(|fee| fee.parse::<f64>().ok())
            .unwrap_or(0.0);

        IntentPerformanceMetrics {
            intent_id: self.intent_id.clone(),
            user_address: self.user_address.clone(),
            execution_score,
            gas_efficiency,
            price_efficiency,
            time_efficiency,
            total_fees_usd,
            profit_generated_usd: None, // Would need market data to calculate
            solver_performance: HashMap::new(), // Would aggregate from execution history
            calculated_at: Utc::now(),
        }
    }

    /// Get storage statistics
    pub fn storage_stats(&self) -> IntentStorageStats {
        IntentStorageStats {
            intent_id: self.intent_id.clone(),
            user_address: self.user_address.clone(),
            version_schema: self.version_schema,
            storage_size_bytes: self.storage_size_bytes,
            access_count: self.access_count,
            created_at: self.created_at,
            updated_at: self.updated_at,
            last_accessed: self.last_accessed,
            indexed_at: self.indexed_at,
            archived: self.archived,
            tags_count: self.tags.len(),
            partition_key: self.partition_key.clone(),
            expiry_date: self.expiry_date,
            is_stale: self.is_stale(24), // 24 hours
            should_cleanup: self.should_cleanup(),
        }
    }
}

impl IntentStatusStorage {
    fn from_domain(status: IntentStatus) -> Self {
        match status {
            IntentStatus::Pending => Self::Pending,
            IntentStatus::Submitted => Self::Submitted,
            IntentStatus::Queued => Self::Queued,
            IntentStatus::Executing => Self::Executing,
            IntentStatus::Success => Self::Success,
            IntentStatus::Failed => Self::Failed,
            IntentStatus::Cancelled => Self::Cancelled,
            IntentStatus::Expired => Self::Expired,
            IntentStatus::Simulating => Self::Simulating,
            IntentStatus::ReviewRequired => Self::ReviewRequired,
        }
    }

    fn to_domain(self) -> IntentStatus {
        match self {
            Self::Pending => IntentStatus::Pending,
            Self::Submitted => IntentStatus::Submitted,
            Self::Queued => IntentStatus::Queued,
            Self::Executing => IntentStatus::Executing,
            Self::Success => IntentStatus::Success,
            Self::Failed => IntentStatus::Failed,
            Self::Cancelled => IntentStatus::Cancelled,
            Self::Expired => IntentStatus::Expired,
            Self::Simulating => IntentStatus::Simulating,
            Self::ReviewRequired => IntentStatus::ReviewRequired,
        }
    }
}

impl IntentPriorityStorage {
    fn from_domain(priority: IntentPriority) -> Self {
        match priority {
            IntentPriority::Low => Self::Low,
            IntentPriority::Normal => Self::Normal,
            IntentPriority::High => Self::High,
            IntentPriority::Critical => Self::Critical,
        }
    }

    fn to_domain(self) -> IntentPriority {
        match self {
            Self::Low => IntentPriority::Low,
            Self::Normal => IntentPriority::Normal,
            Self::High => IntentPriority::High,
            Self::Critical => IntentPriority::Critical,
        }
    }
}

impl IntentMetadataStorage {
    fn from_domain(metadata: IntentMetadata) -> Self {
        Self {
            source: metadata.source,
            user_agent: metadata.user_agent,
            referrer: metadata.referrer,
            session_id: metadata.session_id,
            ip_address: metadata.ip_address,
            custom_data: metadata.custom_data,
            location: metadata.location,
            client_version: metadata.client_version,
            processed_for_analytics: false,
            compliance_checked: false,
            risk_score: None,
            fraud_flags: Vec::new(),
        }
    }

    fn to_domain(self) -> IntentMetadata {
        IntentMetadata {
            source: self.source,
            user_agent: self.user_agent,
            referrer: self.referrer,
            session_id: self.session_id,
            ip_address: self.ip_address,
            custom_data: self.custom_data,
            location: self.location,
            client_version: self.client_version,
        }
    }
}

impl IntentQuoteDataStorage {
    fn from_domain(quote_data: IntentQuoteData) -> Self {
        Self {
            token_in: quote_data.token_in,
            token_out: quote_data.token_out,
            amount_in: quote_data.amount_in,
            amount_out: quote_data.amount_out,
            chain_id: quote_data.chain_id,
            price_impact: quote_data.price_impact,
            estimated_gas: quote_data.estimated_gas,
            route_info: quote_data.route_info,
            expires_at: quote_data.expires_at,
            token_in_symbol: None,
            token_out_symbol: None,
            usd_value: None,
            source_solver: None,
            quote_created_at: Some(Utc::now()),
        }
    }

    fn to_domain(self) -> IntentResult<IntentQuoteData> {
        Ok(IntentQuoteData {
            token_in: self.token_in,
            token_out: self.token_out,
            amount_in: self.amount_in,
            amount_out: self.amount_out,
            chain_id: self.chain_id,
            price_impact: self.price_impact,
            estimated_gas: self.estimated_gas,
            route_info: self.route_info,
            expires_at: self.expires_at,
        })
    }
}

impl IntentFeesStorage {
    fn from_domain(fees: IntentFees) -> Self {
        Self {
            platform_fee_rate: fees.platform_fee_rate,
            platform_fee_amount: fees.platform_fee_amount,
            gas_fee: fees.gas_fee,
            solver_fee: fees.solver_fee,
            total_estimated_fee: fees.total_estimated_fee,
            fee_currency: fees.fee_currency,
            actual_platform_fee: None,
            actual_gas_fee: None,
            actual_solver_fee: None,
            total_actual_fee: None,
            fee_difference_percent: None,
        }
    }

    fn to_domain(self) -> IntentResult<IntentFees> {
        Ok(IntentFees {
            platform_fee_rate: self.platform_fee_rate,
            platform_fee_amount: self.platform_fee_amount,
            gas_fee: self.gas_fee,
            solver_fee: self.solver_fee,
            total_estimated_fee: self.total_estimated_fee,
            fee_currency: self.fee_currency,
        })
    }
}

impl IntentExecutionResultStorage {
    fn from_domain(result: IntentExecutionResult) -> Self {
        Self {
            success: result.success,
            transaction_hash: result.transaction_hash,
            block_number: result.block_number,
            gas_used: result.gas_used,
            effective_gas_price: result.effective_gas_price,
            amount_in: result.amount_in,
            amount_out: result.amount_out,
            actual_price_impact: result.actual_price_impact,
            gas_cost: result.gas_cost,
            execution_time_ms: result.execution_time_ms,
            error_message: result.error_message,
            solver_used: result.solver_used,
            retry_count: result.retry_count,
            timestamp: result.timestamp,
            mempool_time_ms: None,
            confirmation_time_ms: None,
            network_congestion_score: None,
            mev_detected: false,
            sandwich_attack_detected: false,
            execution_environment: "mainnet".to_string(),
        }
    }

    fn to_domain(self) -> IntentResult<IntentExecutionResult> {
        Ok(IntentExecutionResult {
            success: self.success,
            transaction_hash: self.transaction_hash,
            block_number: self.block_number,
            gas_used: self.gas_used,
            effective_gas_price: self.effective_gas_price,
            amount_in: self.amount_in,
            amount_out: self.amount_out,
            actual_price_impact: self.actual_price_impact,
            gas_cost: self.gas_cost,
            execution_time_ms: self.execution_time_ms,
            error_message: self.error_message,
            solver_used: self.solver_used,
            retry_count: self.retry_count,
            timestamp: self.timestamp,
        })
    }
}

/// Storage statistics for an intent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentStorageStats {
    pub intent_id: String,
    pub user_address: String,
    pub version_schema: u32,
    pub storage_size_bytes: u64,
    pub access_count: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_accessed: Option<DateTime<Utc>>,
    pub indexed_at: DateTime<Utc>,
    pub archived: bool,
    pub tags_count: usize,
    pub partition_key: String,
    pub expiry_date: DateTime<Utc>,
    pub is_stale: bool,
    pub should_cleanup: bool,
}

/// Storage query filters for intents
#[derive(Debug, Clone)]
pub struct IntentStorageFilter {
    pub user_address: Option<String>,
    pub status: Option<IntentStatusStorage>,
    pub priority: Option<IntentPriorityStorage>,
    pub chain_id: Option<u64>,
    pub solver_id: Option<String>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub deadline_after: Option<DateTime<Utc>>,
    pub deadline_before: Option<DateTime<Utc>>,
    pub has_tag: Option<String>,
    pub archived: Option<bool>,
    pub min_value_usd: Option<f64>,
    pub max_value_usd: Option<f64>,
    pub partition_key: Option<String>,
}

impl Default for IntentStorageFilter {
    fn default() -> Self {
        Self {
            user_address: None,
            status: None,
            priority: None,
            chain_id: None,
            solver_id: None,
            created_after: None,
            created_before: None,
            deadline_after: None,
            deadline_before: None,
            has_tag: None,
            archived: Some(false), // Exclude archived by default
            min_value_usd: None,
            max_value_usd: None,
            partition_key: None,
        }
    }
}

impl IntentStorageFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_user(user_address: String) -> Self {
        Self {
            user_address: Some(user_address),
            ..Self::default()
        }
    }

    pub fn with_status(mut self, status: IntentStatusStorage) -> Self {
        self.status = Some(status);
        self
    }

    pub fn with_priority(mut self, priority: IntentPriorityStorage) -> Self {
        self.priority = Some(priority);
        self
    }

    pub fn include_archived(mut self) -> Self {
        self.archived = None;
        self
    }

    /// Check if an intent matches this filter
    pub fn matches(&self, intent: &IntentStorage) -> bool {
        if let Some(ref user_address) = self.user_address {
            if intent.user_address != *user_address {
                return false;
            }
        }

        if let Some(ref status) = self.status {
            if intent.status != *status {
                return false;
            }
        }

        if let Some(ref priority) = self.priority {
            if intent.priority != *priority {
                return false;
            }
        }

        if let Some(chain_id) = self.chain_id {
            if let Some(ref quote_data) = intent.quote_data {
                if quote_data.chain_id != chain_id {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(ref solver_id) = self.solver_id {
            if intent.assigned_solver.as_ref() != Some(solver_id) {
                return false;
            }
        }

        if let Some(archived) = self.archived {
            if intent.archived != archived {
                return false;
            }
        }

        if let Some(ref tag) = self.has_tag {
            if !intent.tags.contains(tag) {
                return false;
            }
        }

        // Time-based filters
        if let Some(created_after) = self.created_after {
            if intent.created_at <= created_after {
                return false;
            }
        }

        if let Some(created_before) = self.created_before {
            if intent.created_at >= created_before {
                return false;
            }
        }

        if let Some(deadline_after) = self.deadline_after {
            if intent.deadline <= deadline_after {
                return false;
            }
        }

        if let Some(deadline_before) = self.deadline_before {
            if intent.deadline >= deadline_before {
                return false;
            }
        }

        // Value-based filters (would need USD conversion)
        if let Some(min_value) = self.min_value_usd {
            if let Some(ref quote_data) = intent.quote_data {
                if let Some(usd_value) = quote_data.usd_value {
                    if usd_value < min_value {
                        return false;
                    }
                } else {
                    return false; // No USD value available
                }
            } else {
                return false;
            }
        }

        if let Some(max_value) = self.max_value_usd {
            if let Some(ref quote_data) = intent.quote_data {
                if let Some(usd_value) = quote_data.usd_value {
                    if usd_value > max_value {
                        return false;
                    }
                }
            }
        }

        if let Some(ref partition_key) = self.partition_key {
            if intent.partition_key != *partition_key {
                return false;
            }
        }

        true
    }
}

/// Conversion traits
impl From<Intent> for IntentStorage {
    fn from(intent: Intent) -> Self {
        Self::from_domain(intent)
    }
}

impl TryFrom<IntentStorage> for Intent {
    type Error = IntentError;

    fn try_from(storage: IntentStorage) -> Result<Self, Self::Error> {
        storage.to_domain()
    }
}

/// Generate partition key for efficient querying
fn generate_partition_key(user_address: &str, created_at: &DateTime<Utc>) -> String {
    let user_prefix = &user_address[..6]; // First 6 chars of address
    let date_str = created_at.format("%Y-%m").to_string(); // YYYY-MM
    format!("{}#{}", user_prefix, date_str)
}

/// Estimate storage size for an intent (in bytes)
fn estimate_storage_size(intent: &Intent) -> u64 {
    let base_size = 1024; // Base struct size
    let strings_size = intent.intent_id.len()
        + intent.user_address.len()
        + intent.quote_id.as_ref().map_or(0, |s| s.len())
        + intent.signature.as_ref().map_or(0, |s| s.len())
        + intent.assigned_solver.as_ref().map_or(0, |s| s.len());
    let metadata_size = intent
        .metadata
        .custom_data
        .iter()
        .map(|(k, v)| k.len() + v.to_string().len())
        .sum::<usize>();
    let execution_history_size = intent.execution_history.len() * 512; // Estimate per result

    (base_size + strings_size + metadata_size + execution_history_size) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::intents::{Intent, IntentPriority};
    use chrono::{Duration, Utc};

    fn create_test_intent() -> Intent {
        Intent::new(
            "0x742d35Cc6634C0532925a3b8D02d8f56B2E8E36e".to_string(),
            0.005,
            Utc::now() + Duration::hours(1),
        )
        .with_priority(IntentPriority::High)
        .with_quote_id("quote-123".to_string())
    }

    #[test]
    fn test_storage_conversion() {
        let intent = create_test_intent();
        let intent_id = intent.intent_id.clone();

        // Convert to storage
        let storage = IntentStorage::from_domain(intent);
        assert_eq!(storage.intent_id, intent_id);
        assert_eq!(storage.version_schema, 1);
        assert_eq!(storage.access_count, 0);
        assert!(storage.storage_size_bytes > 0);
        assert!(!storage.archived);
        assert!(storage.partition_key.contains("#"));

        // Convert back to domain
        let domain_intent = storage.to_domain().unwrap();
        assert_eq!(domain_intent.intent_id, intent_id);
    }

    #[test]
    fn test_access_tracking() {
        let intent = create_test_intent();
        let mut storage = IntentStorage::from_domain(intent);

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
            IntentStatusStorage::from_domain(IntentStatus::Pending),
            IntentStatusStorage::Pending
        );
        assert_eq!(
            IntentStatusStorage::Success.to_domain(),
            IntentStatus::Success
        );
    }

    #[test]
    fn test_priority_conversion() {
        assert_eq!(
            IntentPriorityStorage::from_domain(IntentPriority::High),
            IntentPriorityStorage::High
        );
        assert_eq!(
            IntentPriorityStorage::Critical.to_domain(),
            IntentPriority::Critical
        );
    }

    #[test]
    fn test_storage_filter() {
        let intent = create_test_intent();
        let storage = IntentStorage::from_domain(intent);

        // Test basic filter
        let filter =
            IntentStorageFilter::for_user("0x742d35Cc6634C0532925a3b8D02d8f56B2E8E36e".to_string())
                .with_status(IntentStatusStorage::Pending)
                .with_priority(IntentPriorityStorage::High);
        assert!(filter.matches(&storage));

        // Test non-matching filter
        let filter = IntentStorageFilter::for_user("0xDifferentAddress".to_string());
        assert!(!filter.matches(&storage));

        // Test status filter
        let filter = IntentStorageFilter::new().with_status(IntentStatusStorage::Success);
        assert!(!filter.matches(&storage)); // Storage has Pending status
    }

    #[test]
    fn test_archiving() {
        let intent = create_test_intent();
        let mut storage = IntentStorage::from_domain(intent);

        assert!(!storage.archived);

        storage.archive();
        assert!(storage.archived);
    }

    #[test]
    fn test_tag_management() {
        let intent = create_test_intent();
        let mut storage = IntentStorage::from_domain(intent);

        assert!(storage.tags.is_empty());

        storage.add_tag("high-value".to_string());
        storage.add_tag("urgent".to_string());
        assert_eq!(storage.tags.len(), 2);

        storage.remove_tag("high-value");
        assert_eq!(storage.tags.len(), 1);
        assert_eq!(storage.tags[0], "urgent");
    }

    #[test]
    fn test_cleanup_check() {
        let intent = create_test_intent();
        let mut storage = IntentStorage::from_domain(intent);

        // Fresh storage should not need cleanup
        assert!(!storage.should_cleanup());

        // Set expiry in the past
        storage.expiry_date = Utc::now() - Duration::days(1);
        assert!(storage.should_cleanup());
    }

    #[test]
    fn test_staleness_check() {
        let intent = create_test_intent();
        let mut storage = IntentStorage::from_domain(intent);

        // Fresh storage should not be stale
        assert!(!storage.is_stale(24));

        // Set old created_at
        storage.created_at = Utc::now() - Duration::days(2);
        assert!(storage.is_stale(24)); // 24 hours threshold

        // But if accessed recently, it shouldn't be stale
        storage.mark_accessed();
        assert!(!storage.is_stale(24));
    }

    #[test]
    fn test_performance_metrics() {
        let intent = create_test_intent();
        let mut storage = IntentStorage::from_domain(intent);

        // Add successful execution
        let execution_result = IntentExecutionResultStorage {
            success: true,
            transaction_hash: Some("0x123".to_string()),
            gas_used: Some(50000),
            execution_time_ms: Some(3000),
            actual_price_impact: Some(0.005),
            ..Default::default()
        };
        storage.execution_history.push(execution_result);
        storage.status = IntentStatusStorage::Success;

        let metrics = storage.calculate_performance_metrics();
        assert!(metrics.execution_score > 90.0); // High score for successful execution
        assert!(metrics.gas_efficiency > 50.0);
        assert!(metrics.time_efficiency > 50.0);
    }

    #[test]
    fn test_storage_stats() {
        let intent = create_test_intent();
        let mut storage = IntentStorage::from_domain(intent);

        storage.mark_accessed();
        storage.mark_accessed();
        storage.add_tag("test".to_string());

        let stats = storage.storage_stats();
        assert_eq!(stats.intent_id, storage.intent_id);
        assert_eq!(stats.access_count, 2);
        assert_eq!(stats.version_schema, 1);
        assert_eq!(stats.tags_count, 1);
        assert!(!stats.archived);
        assert!(!stats.is_stale);
    }

    #[test]
    fn test_partition_key_generation() {
        let user_address = "0x742d35Cc6634C0532925a3b8D02d8f56B2E8E36e";
        let created_at = Utc::now();

        let partition_key = generate_partition_key(user_address, &created_at);

        assert!(partition_key.contains("0x742d"));
        assert!(partition_key.contains("#"));
        assert!(partition_key.contains(&created_at.format("%Y-%m").to_string()));
    }

    #[test]
    fn test_storage_size_estimation() {
        let intent = create_test_intent();
        let size = estimate_storage_size(&intent);
        assert!(size > 1024); // Should be at least base size
    }
}

impl Default for IntentExecutionResultStorage {
    fn default() -> Self {
        Self {
            success: false,
            transaction_hash: None,
            block_number: None,
            gas_used: None,
            effective_gas_price: None,
            amount_in: None,
            amount_out: None,
            actual_price_impact: None,
            gas_cost: None,
            execution_time_ms: None,
            error_message: None,
            solver_used: None,
            retry_count: 0,
            timestamp: Utc::now(),
            mempool_time_ms: None,
            confirmation_time_ms: None,
            network_congestion_score: None,
            mev_detected: false,
            sandwich_attack_detected: false,
            execution_environment: "mainnet".to_string(),
        }
    }
}
