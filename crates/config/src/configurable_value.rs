//! Configurable value types that can load from environment variables or plain values

use oif_types::SecretString;
use serde::{Deserialize, Serialize};
use std::fmt;

/// A configurable value that can be loaded from environment variables or used as plain text
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConfigurableValue {
	/// Type of value: "env" for environment variable, "plain" for direct value
	#[serde(rename = "type")]
	pub value_type: ValueType,
	/// The value: either environment variable name or the actual value
	pub value: String,
}

/// Type of configurable value
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ValueType {
	/// Load value from environment variable (name specified in `value` field)
	Env,
	/// Use the value directly from the `value` field
	Plain,
}

impl ConfigurableValue {
	/// Create a new environment variable reference
	pub fn from_env(env_var_name: &str) -> Self {
		Self {
			value_type: ValueType::Env,
			value: env_var_name.to_string(),
		}
	}

	/// Create a new plain value
	pub fn from_plain(plain_value: &str) -> Self {
		Self {
			value_type: ValueType::Plain,
			value: plain_value.to_string(),
		}
	}

	/// Resolve the actual value based on the type
	///
	/// For `Env` type, reads from environment variable.
	/// For `Plain` type, returns the value directly.
	pub fn resolve(&self) -> Result<String, ConfigurableValueError> {
		match self.value_type {
			ValueType::Env => std::env::var(&self.value).map_err(|_| {
				ConfigurableValueError::EnvironmentVariableNotFound(self.value.clone())
			}),
			ValueType::Plain => Ok(self.value.clone()),
		}
	}

	/// Resolve the value for secure handling (returns the string, caller should wrap in SecretString)
	pub fn resolve_for_secret(&self) -> Result<SecretString, ConfigurableValueError> {
		let resolved_value = self.resolve()?;
		Ok(SecretString::from_str(&resolved_value))
	}

	/// Get a default warning message if this is an insecure configuration
	pub fn is_insecure_default(&self) -> bool {
		matches!(self.value_type, ValueType::Plain)
	}

	/// Get a description of this configurable value for logging
	pub fn description(&self) -> String {
		match self.value_type {
			ValueType::Env => format!("environment variable '{}'", self.value),
			ValueType::Plain => {
				if self.is_insecure_default() {
					"insecure default value".to_string()
				} else {
					"configured plain value".to_string()
				}
			},
		}
	}
}

/// Errors that can occur when resolving configurable values
#[derive(Debug, thiserror::Error)]
pub enum ConfigurableValueError {
	#[error("Environment variable '{0}' not found")]
	EnvironmentVariableNotFound(String),
}

// Custom Display implementation to avoid showing sensitive data in logs
impl fmt::Display for ConfigurableValue {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self.value_type {
			ValueType::Env => write!(f, "env:{}", self.value),
			ValueType::Plain => {
				if self.is_insecure_default() {
					write!(f, "plain:[INSECURE-DEFAULT]")
				} else {
					write!(f, "plain:[REDACTED]")
				}
			},
		}
	}
}

/// Helper trait for easy conversion from strings in config
impl From<&str> for ConfigurableValue {
	fn from(value: &str) -> Self {
		// If the string starts with "env:", treat it as an environment variable
		if let Some(env_var) = value.strip_prefix("env:") {
			Self::from_env(env_var)
		} else {
			Self::from_plain(value)
		}
	}
}

impl From<String> for ConfigurableValue {
	fn from(value: String) -> Self {
		ConfigurableValue::from(value.as_str())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::env;

	#[test]
	fn test_plain_value() {
		let config = ConfigurableValue::from_plain("test-secret");
		assert_eq!(config.value_type, ValueType::Plain);
		assert_eq!(config.value, "test-secret");
		assert_eq!(config.resolve().unwrap(), "test-secret");
	}

	#[test]
	fn test_env_value() {
		env::set_var("TEST_SECRET", "secret-from-env");

		let config = ConfigurableValue::from_env("TEST_SECRET");
		assert_eq!(config.value_type, ValueType::Env);
		assert_eq!(config.value, "TEST_SECRET");
		assert_eq!(config.resolve().unwrap(), "secret-from-env");

		env::remove_var("TEST_SECRET");
	}

	#[test]
	fn test_env_value_not_found() {
		let config = ConfigurableValue::from_env("NON_EXISTENT_VAR");
		assert!(config.resolve().is_err());
	}

	#[test]
	fn test_from_string_conversion() {
		let plain_config = ConfigurableValue::from("plain-value");
		assert_eq!(plain_config.value_type, ValueType::Plain);
		assert_eq!(plain_config.value, "plain-value");

		let env_config = ConfigurableValue::from("env:MY_SECRET");
		assert_eq!(env_config.value_type, ValueType::Env);
		assert_eq!(env_config.value, "MY_SECRET");
	}

	#[test]
	fn test_secret_resolution() {
		let config = ConfigurableValue::from_plain("test-secret");
		let secret_value = config.resolve_for_secret().unwrap();
		assert_eq!(secret_value.expose_secret(), "test-secret");
	}

	#[test]
	fn test_insecure_default_detection() {
		let insecure = ConfigurableValue::from_plain("WARNING-INSECURE-DEFAULT-test");
		assert!(insecure.is_insecure_default());

		let secure = ConfigurableValue::from_plain("secure-value");
		assert!(!secure.is_insecure_default());

		let env_config = ConfigurableValue::from_env("MY_SECRET");
		assert!(!env_config.is_insecure_default());
	}

	#[test]
	fn test_description() {
		let plain_config = ConfigurableValue::from_plain("secret");
		assert_eq!(plain_config.description(), "configured plain value");

		let insecure_config = ConfigurableValue::from_plain("WARNING-INSECURE-DEFAULT-test");
		assert_eq!(insecure_config.description(), "insecure default value");

		let env_config = ConfigurableValue::from_env("MY_SECRET");
		assert_eq!(env_config.description(), "environment variable 'MY_SECRET'");
	}

	#[test]
	fn test_serde_serialization() {
		let config = ConfigurableValue {
			value_type: ValueType::Env,
			value: "MY_SECRET".to_string(),
		};

		let json = serde_json::to_string(&config).unwrap();
		assert!(json.contains("\"type\":\"env\""));
		assert!(json.contains("\"value\":\"MY_SECRET\""));

		let deserialized: ConfigurableValue = serde_json::from_str(&json).unwrap();
		assert_eq!(deserialized.value_type, ValueType::Env);
		assert_eq!(deserialized.value, "MY_SECRET");
	}
}
