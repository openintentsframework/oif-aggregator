use serde::Serialize;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Error response format shared by handlers
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub timestamp: i64,
}


