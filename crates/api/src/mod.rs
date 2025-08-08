//! API module for handling HTTP routes and request/response logic

pub mod handlers;
pub mod router;
pub mod security;
pub mod state;
pub mod pagination;

pub use router::create_router;
pub use state::AppState;
pub mod security;
