//! OIF Storage
//!
//! Storage implementations for the Open Order Framework Aggregator.
//! Supports multiple backends including memory and Redis.

pub mod memory_store;
pub mod traits;

#[cfg(feature = "redis")]
pub mod redis_store;

pub use memory_store::MemoryStore;
pub use traits::Storage;

#[cfg(feature = "redis")]
pub use redis_store::RedisStore;
