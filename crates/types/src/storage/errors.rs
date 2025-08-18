//! Error types for storage operations

use thiserror::Error;

/// Storage error type
#[derive(Debug, Error)]
pub enum StorageError {
	#[error("Item not found: {id}")]
	NotFound { id: String },
	#[error("Connection error: {message}")]
	Connection { message: String },
	#[error("Serialization error: {message}")]
	Serialization { message: String },
	#[error("Storage operation failed: {message}")]
	Operation { message: String },
}
