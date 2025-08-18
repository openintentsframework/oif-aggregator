//! Storage-related types and traits

pub mod traits;
pub use traits::*;

pub mod errors;
pub use errors::*;

pub type StorageResult<T> = Result<T, StorageError>;
