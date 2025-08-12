//! Authentication and authorization types and traits

pub mod errors;
pub mod traits;

pub use errors::*;
pub use traits::*;

/// Result types for auth operations
pub type AuthResult<T> = Result<T, AuthError>;
pub type RateLimitResult<T> = Result<T, RateLimitError>;
