//! OIF Adapters
//!
//! Solver-specific adapters for the Open Intent Framework Aggregator.

pub mod lifi_adapter;
pub mod oif_adapter;

pub use lifi_adapter::LifiAdapter;
pub use oif_adapter::OifAdapter;
pub use oif_types::{AdapterError, AdapterResult, SolverAdapter};

/// Simple registry for solver adapters
pub struct AdapterRegistry {
	adapters: std::collections::HashMap<String, Box<dyn SolverAdapter>>,
}

impl AdapterRegistry {
	pub fn new() -> Self {
		Self {
			adapters: std::collections::HashMap::new(),
		}
	}

	/// Create registry with default OIF and LiFi adapters
	pub fn with_defaults() -> Self {
		let mut registry = Self::new();

		// Add default OIF adapter
		if let Ok(oif_adapter) = OifAdapter::default() {
			let _ = registry.register(Box::new(oif_adapter));
		}

		// Add default LiFi adapter
		if let Ok(lifi_adapter) = LifiAdapter::default() {
			let _ = registry.register(Box::new(lifi_adapter));
		}

		registry
	}

	/// Get all registered adapter IDs
	pub fn get_adapter_ids(&self) -> Vec<String> {
		self.adapters.keys().cloned().collect()
	}

	/// Register an adapter (uses the adapter's own ID)
	pub fn register(&mut self, adapter: Box<dyn SolverAdapter>) -> Result<(), String> {
		let adapter_id = adapter.adapter_id().to_string();

		// Check for duplicate IDs
		if self.adapters.contains_key(&adapter_id) {
			return Err(format!(
				"Adapter with ID '{}' already registered",
				adapter_id
			));
		}

		self.adapters.insert(adapter_id, adapter);
		Ok(())
	}

	pub fn get(&self, id: &str) -> Option<&Box<dyn SolverAdapter>> {
		self.adapters.get(id)
	}

	pub fn get_all(&self) -> &std::collections::HashMap<String, Box<dyn SolverAdapter>> {
		&self.adapters
	}
}
