//! Storage module for data management with pluggable backends

pub mod memory_store;
pub mod redis_store;
pub mod traits;

pub use memory_store::MemoryStore;
pub use redis_store::{RedisConfig, RedisStore};
pub use traits::{
    OrderStorage, QuoteStorage, SolverStorage, Storage, StorageError, StorageResult, StorageStats,
};
