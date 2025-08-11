//! OIF Storage
//!
//! Storage implementations for the Open Order Framework Aggregator.

pub mod memory_store;
pub mod traits;

pub use memory_store::MemoryStore;
pub use traits::Storage;
