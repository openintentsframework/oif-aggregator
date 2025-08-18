//! Storage traits for pluggable storage implementations

// Re-export the storage traits from types crate
pub use oif_types::storage::{
	OrderStorageTrait as OrderStorage, QuoteStorageTrait as QuoteStorage,
	SolverStorageTrait as SolverStorage, StorageError, StorageResult, StorageTrait as Storage,
};
