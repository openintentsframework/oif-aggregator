//! Adapter storage model and conversions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{
	Adapter, AdapterCapability, AdapterError, AdapterHealthResult, AdapterMetrics,
	AdapterPerformanceMetrics, AdapterResult, AdapterStatus, AdapterType,
};

/// Storage representation of an adapter
///
/// This model is optimized for storage and persistence.
/// It can be converted to/from the domain Adapter model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterStorage {
	pub adapter_id: String,
	pub adapter_type: AdapterTypeStorage,
	pub name: String,
	pub description: Option<String>,
	pub version: String,
	pub supported_chains: Vec<u64>,
	pub configuration: HashMap<String, serde_json::Value>,
	pub enabled: bool,
	pub status: AdapterStatusStorage,
	pub metrics: AdapterMetricsStorage,
	pub capabilities: Vec<AdapterCapabilityStorage>,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
	pub last_accessed: Option<DateTime<Utc>>,

	// Storage-specific metadata
	pub version_schema: u32,
	pub storage_size_bytes: u64,
	pub access_count: u64,
	pub last_backup: Option<DateTime<Utc>>,
	pub compression_enabled: bool,
	pub archived: bool,
	pub tags: Vec<String>,
}

/// Storage-compatible adapter status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AdapterStatusStorage {
	Active,
	Inactive,
	Error,
	Deprecated,
}

/// Storage-compatible adapter type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum AdapterTypeStorage {
	OifV1,
	LifiV1,
}

/// Storage-compatible adapter capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterCapabilityStorage {
	pub name: String,
	pub available: bool,
	pub version: Option<String>,
	pub description: Option<String>,
	pub required_config: Vec<String>,
	pub last_verified: Option<DateTime<Utc>>,
}

/// Storage-compatible adapter metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterMetricsStorage {
	pub performance: AdapterPerformanceStorage,
	pub last_health_check: Option<AdapterHealthStorage>,
	pub consecutive_failures: u32,
	pub total_accesses: u64,
	pub last_updated: DateTime<Utc>,
	pub uptime_percentage: f64,
	pub last_error: Option<String>,
	pub last_error_at: Option<DateTime<Utc>>,

	// Extended storage metrics
	pub daily_stats: Vec<DailyStats>,
	pub monthly_stats: Vec<MonthlyStats>,
	pub alerts_generated: u32,
	pub maintenance_windows: Vec<MaintenanceWindow>,
	pub performance_baselines: PerformanceBaselines,
}

/// Storage-compatible performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterPerformanceStorage {
	pub total_requests: u64,
	pub successful_requests: u64,
	pub failed_requests: u64,
	pub timeout_requests: u64,
	pub avg_response_time_ms: f64,
	pub min_response_time_ms: u64,
	pub max_response_time_ms: u64,
	pub p95_response_time_ms: u64,
	pub success_rate: f64,
	pub error_rate: f64,
	pub last_reset: DateTime<Utc>,

	// Historical data
	pub hourly_buckets: Vec<HourlyBucket>,
	pub peak_rps: f64,
	pub peak_rps_timestamp: Option<DateTime<Utc>>,
	pub total_data_transferred: u64,
}

/// Storage-compatible health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterHealthStorage {
	pub is_healthy: bool,
	pub response_time_ms: u64,
	pub error_message: Option<String>,
	pub last_check: DateTime<Utc>,
	pub consecutive_failures: u32,
	pub health_score: f64,
	pub capabilities: Vec<String>,
	pub check_id: String,
	pub check_duration_ms: u64,
	pub endpoint_status: HashMap<String, bool>,
}

/// Daily performance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyStats {
	pub date: String, // YYYY-MM-DD format
	pub total_requests: u64,
	pub success_rate: f64,
	pub avg_response_time_ms: f64,
	pub peak_rps: f64,
	pub total_downtime_minutes: u32,
	pub errors_by_type: HashMap<String, u32>,
}

/// Monthly performance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyStats {
	pub month: String, // YYYY-MM format
	pub total_requests: u64,
	pub avg_success_rate: f64,
	pub avg_response_time_ms: f64,
	pub uptime_percentage: f64,
	pub peak_daily_requests: u64,
	pub total_data_transferred: u64,
}

/// Hourly performance bucket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyBucket {
	pub hour: DateTime<Utc>,
	pub request_count: u64,
	pub success_count: u64,
	pub avg_response_time_ms: f64,
	pub peak_response_time_ms: u64,
}

/// Maintenance window information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceWindow {
	pub start_time: DateTime<Utc>,
	pub end_time: DateTime<Utc>,
	pub reason: String,
	pub scheduled: bool,
	pub impact_level: String, // "low", "medium", "high"
}

/// Performance baselines for anomaly detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceBaselines {
	pub baseline_response_time_ms: f64,
	pub baseline_success_rate: f64,
	pub baseline_rps: f64,
	pub calculated_at: DateTime<Utc>,
	pub confidence_interval: f64,
}

impl AdapterStorage {
	/// Create storage adapter from domain adapter
	pub fn from_domain(adapter: Adapter) -> Self {
		let storage_size = estimate_storage_size(&adapter);

		Self {
			adapter_id: adapter.adapter_id,
			adapter_type: AdapterTypeStorage::from_domain(&adapter.adapter_type),
			name: adapter.name,
			description: adapter.description,
			version: adapter.version,
			supported_chains: adapter.supported_chains,
			configuration: adapter.configuration,
			enabled: adapter.enabled,
			status: AdapterStatusStorage::from_domain(&adapter.status),
			metrics: AdapterMetricsStorage::from_domain(adapter.metrics),
			capabilities: adapter
				.capabilities
				.into_iter()
				.map(AdapterCapabilityStorage::from_domain)
				.collect(),
			created_at: adapter.created_at,
			updated_at: adapter.updated_at,
			last_accessed: adapter.last_accessed,
			version_schema: 1,
			storage_size_bytes: storage_size,
			access_count: 0,
			last_backup: None,
			compression_enabled: false,
			archived: false,
			tags: Vec::new(),
		}
	}

	/// Convert storage adapter to domain adapter
	pub fn to_domain(self) -> AdapterResult<Adapter> {
		Ok(Adapter {
			adapter_id: self.adapter_id,
			adapter_type: self.adapter_type.to_domain(),
			name: self.name,
			description: self.description,
			version: self.version,
			supported_chains: self.supported_chains,
			configuration: self.configuration,
			enabled: self.enabled,
			status: self.status.to_domain(),
			metrics: self.metrics.into_domain()?,
			capabilities: self
				.capabilities
				.into_iter()
				.map(|cap| cap.into_domain())
				.collect::<Result<Vec<_>, _>>()?,
			created_at: self.created_at,
			updated_at: self.updated_at,
			last_accessed: self.last_accessed,
		})
	}

	/// Mark as accessed for analytics
	pub fn mark_accessed(&mut self) {
		self.access_count += 1;
		self.last_accessed = Some(Utc::now());
		self.updated_at = Utc::now();
	}

	/// Update status and timestamp
	pub fn update_status(&mut self, status: AdapterStatusStorage) {
		self.status = status;
		self.updated_at = Utc::now();
		self.mark_accessed();
	}

	/// Update metrics from domain metrics
	pub fn update_metrics(&mut self, metrics: AdapterMetrics) {
		self.metrics = AdapterMetricsStorage::from_domain(metrics);
		self.updated_at = Utc::now();
		self.mark_accessed();
	}

	/// Add a tag to the adapter
	pub fn add_tag(&mut self, tag: String) {
		if !self.tags.contains(&tag) {
			self.tags.push(tag);
			self.updated_at = Utc::now();
		}
	}

	/// Remove a tag from the adapter
	pub fn remove_tag(&mut self, tag: &str) {
		if let Some(pos) = self.tags.iter().position(|t| t == tag) {
			self.tags.remove(pos);
			self.updated_at = Utc::now();
		}
	}

	/// Archive the adapter
	pub fn archive(&mut self) {
		self.archived = true;
		self.status = AdapterStatusStorage::Deprecated;
		self.updated_at = Utc::now();
	}

	/// Check if adapter is stale (hasn't been updated recently)
	pub fn is_stale(&self, max_age_hours: i64) -> bool {
		let stale_threshold = Utc::now() - chrono::Duration::hours(max_age_hours);
		self.updated_at < stale_threshold
	}

	/// Compress adapter data (simulate compression)
	pub fn compress(&mut self) {
		if !self.compression_enabled {
			self.compression_enabled = true;
			self.storage_size_bytes = (self.storage_size_bytes as f64 * 0.7) as u64; // Simulate 30% compression
			self.updated_at = Utc::now();
		}
	}

	/// Create a backup record
	pub fn backup(&mut self) {
		self.last_backup = Some(Utc::now());
		self.updated_at = Utc::now();
	}

	/// Get storage statistics
	pub fn storage_stats(&self) -> AdapterStorageStats {
		AdapterStorageStats {
			adapter_id: self.adapter_id.clone(),
			version_schema: self.version_schema,
			storage_size_bytes: self.storage_size_bytes,
			access_count: self.access_count,
			created_at: self.created_at,
			updated_at: self.updated_at,
			last_backup: self.last_backup,
			compression_enabled: self.compression_enabled,
			archived: self.archived,
			tags_count: self.tags.len(),
			is_stale: self.is_stale(24), // 24 hours
		}
	}

	/// Get performance summary
	pub fn performance_summary(&self) -> PerformanceSummary {
		PerformanceSummary {
			total_requests: self.metrics.performance.total_requests,
			success_rate: self.metrics.performance.success_rate,
			avg_response_time_ms: self.metrics.performance.avg_response_time_ms,
			uptime_percentage: self.metrics.uptime_percentage,
			last_24h_requests: self.get_last_24h_requests(),
			peak_rps: self.metrics.performance.peak_rps,
			total_data_transferred: self.metrics.performance.total_data_transferred,
		}
	}

	fn get_last_24h_requests(&self) -> u64 {
		let cutoff = Utc::now() - chrono::Duration::hours(24);
		self.metrics
			.performance
			.hourly_buckets
			.iter()
			.filter(|bucket| bucket.hour > cutoff)
			.map(|bucket| bucket.request_count)
			.sum()
	}
}

impl AdapterStatusStorage {
	fn from_domain(status: &AdapterStatus) -> Self {
		match status {
			AdapterStatus::Active => Self::Active,
			AdapterStatus::Inactive => Self::Inactive,
			AdapterStatus::Error => Self::Error,
			AdapterStatus::Deprecated => Self::Deprecated,
			_ => Self::Active,
		}
	}

	fn to_domain(&self) -> AdapterStatus {
		match self {
			Self::Active => AdapterStatus::Active,
			Self::Inactive => AdapterStatus::Inactive,
			Self::Error => AdapterStatus::Error,
			Self::Deprecated => AdapterStatus::Deprecated,
		}
	}
}

impl AdapterTypeStorage {
	fn from_domain(adapter_type: &AdapterType) -> Self {
		match adapter_type {
			AdapterType::OifV1 => Self::OifV1,
			AdapterType::LifiV1 => Self::LifiV1,
		}
	}

	fn to_domain(&self) -> AdapterType {
		match self {
			Self::OifV1 => AdapterType::OifV1,
			Self::LifiV1 => AdapterType::LifiV1,
		}
	}
}

impl AdapterCapabilityStorage {
	fn from_domain(capability: AdapterCapability) -> Self {
		Self {
			name: capability.name,
			available: capability.available,
			version: capability.version,
			description: capability.description,
			required_config: capability.required_config,
			last_verified: Some(Utc::now()),
		}
	}

	fn into_domain(self) -> AdapterResult<AdapterCapability> {
		Ok(AdapterCapability {
			name: self.name,
			available: self.available,
			version: self.version,
			description: self.description,
			required_config: self.required_config,
		})
	}
}

impl AdapterMetricsStorage {
	fn from_domain(metrics: AdapterMetrics) -> Self {
		let uptime_percentage = metrics.calculate_uptime();
		let baseline_response_time = metrics.performance.avg_response_time_ms;
		let baseline_success_rate = metrics.performance.success_rate;

		Self {
			performance: AdapterPerformanceStorage::from_domain(metrics.performance),
			last_health_check: metrics
				.last_health_check
				.map(AdapterHealthStorage::from_domain),
			consecutive_failures: metrics.consecutive_failures,
			total_accesses: metrics.total_accesses,
			last_updated: metrics.last_updated,
			uptime_percentage,
			last_error: metrics.last_error,
			last_error_at: metrics.last_error_at,
			daily_stats: Vec::new(),
			monthly_stats: Vec::new(),
			alerts_generated: 0,
			maintenance_windows: Vec::new(),
			performance_baselines: PerformanceBaselines {
				baseline_response_time_ms: baseline_response_time,
				baseline_success_rate,
				baseline_rps: 0.0,
				calculated_at: Utc::now(),
				confidence_interval: 0.95,
			},
		}
	}

	fn into_domain(self) -> AdapterResult<AdapterMetrics> {
		Ok(AdapterMetrics {
			performance: self.performance.into_domain(),
			last_health_check: self
				.last_health_check
				.map(|hc| hc.into_domain())
				.transpose()?,
			consecutive_failures: self.consecutive_failures,
			total_accesses: self.total_accesses,
			last_updated: self.last_updated,
			uptime_percentage: self.uptime_percentage,
			last_error: self.last_error,
			last_error_at: self.last_error_at,
		})
	}
}

impl AdapterPerformanceStorage {
	fn from_domain(performance: AdapterPerformanceMetrics) -> Self {
		Self {
			total_requests: performance.total_requests,
			successful_requests: performance.successful_requests,
			failed_requests: performance.failed_requests,
			timeout_requests: performance.timeout_requests,
			avg_response_time_ms: performance.avg_response_time_ms,
			min_response_time_ms: performance.min_response_time_ms,
			max_response_time_ms: performance.max_response_time_ms,
			p95_response_time_ms: performance.p95_response_time_ms,
			success_rate: performance.success_rate,
			error_rate: performance.error_rate,
			last_reset: performance.last_reset,
			hourly_buckets: Vec::new(),
			peak_rps: 0.0,
			peak_rps_timestamp: None,
			total_data_transferred: 0,
		}
	}

	fn into_domain(self) -> AdapterPerformanceMetrics {
		AdapterPerformanceMetrics {
			total_requests: self.total_requests,
			successful_requests: self.successful_requests,
			failed_requests: self.failed_requests,
			timeout_requests: self.timeout_requests,
			avg_response_time_ms: self.avg_response_time_ms,
			min_response_time_ms: self.min_response_time_ms,
			max_response_time_ms: self.max_response_time_ms,
			p95_response_time_ms: self.p95_response_time_ms,
			success_rate: self.success_rate,
			error_rate: self.error_rate,
			last_reset: self.last_reset,
		}
	}
}

impl AdapterHealthStorage {
	fn from_domain(health: AdapterHealthResult) -> Self {
		Self {
			is_healthy: health.is_healthy,
			response_time_ms: health.response_time_ms,
			error_message: health.error_message,
			last_check: health.last_check,
			consecutive_failures: health.consecutive_failures,
			health_score: health.health_score,
			capabilities: health.capabilities,
			check_id: uuid::Uuid::new_v4().to_string(),
			check_duration_ms: 0,
			endpoint_status: HashMap::new(),
		}
	}

	fn into_domain(self) -> AdapterResult<AdapterHealthResult> {
		Ok(AdapterHealthResult {
			is_healthy: self.is_healthy,
			response_time_ms: self.response_time_ms,
			error_message: self.error_message,
			last_check: self.last_check,
			consecutive_failures: self.consecutive_failures,
			health_score: self.health_score,
			capabilities: self.capabilities,
		})
	}
}

/// Storage statistics for an adapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterStorageStats {
	pub adapter_id: String,
	pub version_schema: u32,
	pub storage_size_bytes: u64,
	pub access_count: u64,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
	pub last_backup: Option<DateTime<Utc>>,
	pub compression_enabled: bool,
	pub archived: bool,
	pub tags_count: usize,
	pub is_stale: bool,
}

/// Performance summary for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSummary {
	pub total_requests: u64,
	pub success_rate: f64,
	pub avg_response_time_ms: f64,
	pub uptime_percentage: f64,
	pub last_24h_requests: u64,
	pub peak_rps: f64,
	pub total_data_transferred: u64,
}

/// Storage query filters for adapters
#[derive(Debug, Clone)]
pub struct AdapterStorageFilter {
	pub adapter_id: Option<String>,
	pub adapter_type: Option<AdapterTypeStorage>,
	pub status: Option<AdapterStatusStorage>,
	pub enabled_only: bool,
	pub healthy_only: bool,
	pub supported_chain: Option<u64>,
	pub has_capability: Option<String>,
	pub created_after: Option<DateTime<Utc>>,
	pub created_before: Option<DateTime<Utc>>,
	pub last_accessed_after: Option<DateTime<Utc>>,
	pub min_success_rate: Option<f64>,
	pub max_response_time_ms: Option<f64>,
	pub archived: Option<bool>,
	pub has_tag: Option<String>,
}

impl Default for AdapterStorageFilter {
	fn default() -> Self {
		Self {
			adapter_id: None,
			adapter_type: None,
			status: None,
			enabled_only: false,
			healthy_only: false,
			supported_chain: None,
			has_capability: None,
			created_after: None,
			created_before: None,
			last_accessed_after: None,
			min_success_rate: None,
			max_response_time_ms: None,
			archived: Some(false), // Exclude archived by default
			has_tag: None,
		}
	}
}

impl AdapterStorageFilter {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn with_adapter_id(mut self, adapter_id: String) -> Self {
		self.adapter_id = Some(adapter_id);
		self
	}

	pub fn with_adapter_type(mut self, adapter_type: AdapterTypeStorage) -> Self {
		self.adapter_type = Some(adapter_type);
		self
	}

	pub fn with_status(mut self, status: AdapterStatusStorage) -> Self {
		self.status = Some(status);
		self
	}

	pub fn enabled_only(mut self) -> Self {
		self.enabled_only = true;
		self
	}

	pub fn healthy_only(mut self) -> Self {
		self.healthy_only = true;
		self
	}

	pub fn with_chain_support(mut self, chain_id: u64) -> Self {
		self.supported_chain = Some(chain_id);
		self
	}

	pub fn with_capability(mut self, capability: String) -> Self {
		self.has_capability = Some(capability);
		self
	}

	pub fn include_archived(mut self) -> Self {
		self.archived = None;
		self
	}

	pub fn with_tag(mut self, tag: String) -> Self {
		self.has_tag = Some(tag);
		self
	}

	/// Check if an adapter matches this filter
	pub fn matches(&self, adapter: &AdapterStorage) -> bool {
		if let Some(ref adapter_id) = self.adapter_id {
			if adapter.adapter_id != *adapter_id {
				return false;
			}
		}

		if let Some(ref adapter_type) = self.adapter_type {
			if adapter.adapter_type != *adapter_type {
				return false;
			}
		}

		if let Some(ref status) = self.status {
			if adapter.status != *status {
				return false;
			}
		}

		if self.enabled_only && !adapter.enabled {
			return false;
		}

		if self.healthy_only {
			if let Some(ref health) = adapter.metrics.last_health_check {
				if !health.is_healthy {
					return false;
				}
			} else {
				return false;
			}
		}

		if let Some(chain_id) = self.supported_chain {
			if !adapter.supported_chains.contains(&chain_id) {
				return false;
			}
		}

		if let Some(ref capability) = self.has_capability {
			if !adapter
				.capabilities
				.iter()
				.any(|cap| cap.name == *capability && cap.available)
			{
				return false;
			}
		}

		if let Some(archived) = self.archived {
			if adapter.archived != archived {
				return false;
			}
		}

		if let Some(ref tag) = self.has_tag {
			if !adapter.tags.contains(tag) {
				return false;
			}
		}

		// Time-based filters
		if let Some(created_after) = self.created_after {
			if adapter.created_at <= created_after {
				return false;
			}
		}

		if let Some(created_before) = self.created_before {
			if adapter.created_at >= created_before {
				return false;
			}
		}

		if let Some(last_accessed_after) = self.last_accessed_after {
			if let Some(last_accessed) = adapter.last_accessed {
				if last_accessed <= last_accessed_after {
					return false;
				}
			} else {
				return false;
			}
		}

		// Performance filters
		if let Some(min_rate) = self.min_success_rate {
			if adapter.metrics.performance.success_rate < min_rate {
				return false;
			}
		}

		if let Some(max_time) = self.max_response_time_ms {
			if adapter.metrics.performance.avg_response_time_ms > max_time {
				return false;
			}
		}

		true
	}
}

/// Conversion traits
impl From<Adapter> for AdapterStorage {
	fn from(adapter: Adapter) -> Self {
		Self::from_domain(adapter)
	}
}

impl TryFrom<AdapterStorage> for Adapter {
	type Error = AdapterError;

	fn try_from(storage: AdapterStorage) -> Result<Self, Self::Error> {
		storage.to_domain()
	}
}

/// Estimate storage size for an adapter (in bytes)
fn estimate_storage_size(adapter: &Adapter) -> u64 {
	let base_size = 1024; // Base struct size
	let strings_size = adapter.adapter_id.len()
		+ adapter.name.len()
		+ adapter.version.len()
		+ adapter.description.as_ref().map_or(0, |s| s.len());
	let chains_size = adapter.supported_chains.len() * 8;
	let capabilities_size = adapter
		.capabilities
		.iter()
		.map(|cap| cap.name.len() + cap.description.as_ref().map_or(0, |d| d.len()))
		.sum::<usize>();
	let config_size = adapter
		.configuration
		.iter()
		.map(|(k, v)| k.len() + v.to_string().len())
		.sum::<usize>();

	(base_size + strings_size + chains_size + capabilities_size + config_size) as u64
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::adapters::{Adapter, AdapterType};

	fn create_test_adapter() -> Adapter {
		Adapter::new(
			"test-adapter".to_string(),
			AdapterType::OifV1,
			"Test Adapter".to_string(),
			"1.0.0".to_string(),
		)
		.with_description("Test Description".to_string())
		.with_chains(vec![1, 137])
	}

	#[test]
	fn test_storage_conversion() {
		let adapter = create_test_adapter();
		let adapter_id = adapter.adapter_id.clone();

		// Convert to storage
		let storage = AdapterStorage::from_domain(adapter);
		assert_eq!(storage.adapter_id, adapter_id);
		assert_eq!(storage.version_schema, 1);
		assert_eq!(storage.access_count, 0);
		assert!(storage.storage_size_bytes > 0);
		assert!(!storage.archived);

		// Convert back to domain
		let domain_adapter = storage.to_domain().unwrap();
		assert_eq!(domain_adapter.adapter_id, adapter_id);
	}

	#[test]
	fn test_access_tracking() {
		let adapter = create_test_adapter();
		let mut storage = AdapterStorage::from_domain(adapter);

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
			AdapterStatusStorage::from_domain(&AdapterStatus::Active),
			AdapterStatusStorage::Active
		);
		assert_eq!(
			AdapterStatusStorage::Active.to_domain(),
			AdapterStatus::Active
		);
	}

	#[test]
	fn test_adapter_type_conversion() {
		assert_eq!(
			AdapterTypeStorage::from_domain(&AdapterType::OifV1),
			AdapterTypeStorage::OifV1
		);
		assert_eq!(AdapterTypeStorage::OifV1.to_domain(), AdapterType::OifV1);
	}

	#[test]
	fn test_storage_filter() {
		let adapter = create_test_adapter();
		let storage = AdapterStorage::from_domain(adapter);

		// Test basic filter
		let filter = AdapterStorageFilter::new()
			.with_adapter_id("test-adapter".to_string())
			.with_adapter_type(AdapterTypeStorage::OifV1);
		assert!(filter.matches(&storage));

		// Test non-matching filter
		let filter = AdapterStorageFilter::new().with_adapter_id("different-adapter".to_string());
		assert!(!filter.matches(&storage));

		// Test chain support filter
		let filter = AdapterStorageFilter::new().with_chain_support(1);
		assert!(filter.matches(&storage));

		let filter = AdapterStorageFilter::new().with_chain_support(56); // Not supported
		assert!(!filter.matches(&storage));
	}

	#[test]
	fn test_archiving() {
		let adapter = create_test_adapter();
		let mut storage = AdapterStorage::from_domain(adapter);

		assert!(!storage.archived);
		assert_ne!(storage.status, AdapterStatusStorage::Deprecated);

		storage.archive();
		assert!(storage.archived);
		assert_eq!(storage.status, AdapterStatusStorage::Deprecated);
	}

	#[test]
	fn test_tag_management() {
		let adapter = create_test_adapter();
		let mut storage = AdapterStorage::from_domain(adapter);

		assert!(storage.tags.is_empty());

		storage.add_tag("production".to_string());
		storage.add_tag("critical".to_string());
		assert_eq!(storage.tags.len(), 2);

		storage.remove_tag("production");
		assert_eq!(storage.tags.len(), 1);
		assert_eq!(storage.tags[0], "critical");
	}

	#[test]
	fn test_compression() {
		let adapter = create_test_adapter();
		let mut storage = AdapterStorage::from_domain(adapter);

		let original_size = storage.storage_size_bytes;
		assert!(!storage.compression_enabled);

		storage.compress();
		assert!(storage.compression_enabled);
		assert!(storage.storage_size_bytes < original_size);
	}

	#[test]
	fn test_staleness_check() {
		let adapter = create_test_adapter();
		let mut storage = AdapterStorage::from_domain(adapter);

		// Fresh storage should not be stale
		assert!(!storage.is_stale(24));

		// Manually set old timestamp
		storage.updated_at = Utc::now() - chrono::Duration::days(2);
		assert!(storage.is_stale(24)); // 24 hours threshold
	}

	#[test]
	fn test_storage_stats() {
		let adapter = create_test_adapter();
		let mut storage = AdapterStorage::from_domain(adapter);

		storage.mark_accessed();
		storage.mark_accessed();
		storage.add_tag("test".to_string());

		let stats = storage.storage_stats();
		assert_eq!(stats.adapter_id, "test-adapter");
		assert_eq!(stats.access_count, 2);
		assert_eq!(stats.version_schema, 1);
		assert_eq!(stats.tags_count, 1);
		assert!(!stats.archived);
		assert!(!stats.is_stale);
	}

	#[test]
	fn test_performance_summary() {
		let adapter = create_test_adapter();
		let storage = AdapterStorage::from_domain(adapter);

		let summary = storage.performance_summary();
		assert_eq!(summary.total_requests, 0);
		assert_eq!(summary.success_rate, 0.0);
		assert_eq!(summary.last_24h_requests, 0);
	}
}
