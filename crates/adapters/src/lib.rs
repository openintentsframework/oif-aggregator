//! OIF Adapters
//!
//! Solver-specific adapters for the Open Intent Framework Aggregator.

pub mod lifi_adapter;
pub mod oif_adapter;

pub use lifi_adapter::LifiAdapter;
pub use oif_adapter::OifAdapter;
pub use oif_types::{AdapterError, AdapterResult, SolverAdapter};

/// Simple registry for solver adapters
pub struct AdapterFactory {
	adapters: std::collections::HashMap<String, Box<dyn SolverAdapter>>,
}

impl AdapterFactory {
	pub fn new() -> Self {
		Self {
			adapters: std::collections::HashMap::new(),
		}
	}

	/// Create factory with default OIF and LiFi adapters
	pub fn with_defaults() -> Self {
		let mut factory = Self::new();

		// Add default OIF adapter
		if let Ok(oif_adapter) = OifAdapter::default() {
			let _ = factory.register(Box::new(oif_adapter));
		}

		// Add default LiFi adapter
		if let Ok(lifi_adapter) = LifiAdapter::default() {
			let _ = factory.register(Box::new(lifi_adapter));
		}

		factory
	}

	/// Get all registered adapter IDs
	pub fn get_adapter_ids(&self) -> Vec<String> {
		self.adapters.keys().cloned().collect()
	}

	/// Validate that all solver adapter IDs exist in the registry
	pub fn validate_solvers(&self, solvers: &[oif_types::Solver]) -> Result<(), String> {
		for solver in solvers {
			if !self.adapters.contains_key(&solver.adapter_id) {
				return Err(format!(
					"Solver '{}' references unknown adapter '{}'",
					solver.solver_id, solver.adapter_id
				));
			}
		}
		Ok(())
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
