use serde::{Deserialize, Serialize};
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Lock information for inputs that are already locked
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct Lock {
	/// Type of lock mechanism
	pub kind: LockKind,
	/// Lock-specific parameters
	pub params: Option<serde_json::Value>,
}

/// Supported lock mechanisms
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub enum LockKind {
	TheCompact,
}
