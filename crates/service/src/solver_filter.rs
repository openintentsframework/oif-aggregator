//! Simplified solver filtering and selection service
//!
//! This module provides intelligent solver filtering based on network/asset compatibility
//! with a clean, maintainable architecture focused on the essential functionality.
//!
//! ## Core Philosophy
//! - **Strict compatibility**: Binary compatible/incompatible/unknown states (no partial scoring)
//! - **Clear separation**: Distinct phases for filtering and selection  
//! - **Minimal configuration**: Sensible defaults with targeted customization
//! - **Predictable behavior**: Straightforward logic that's easy to test and debug

use async_trait::async_trait;
use oif_config::AggregationConfig;
use oif_types::constants::limits::{DEFAULT_PRIORITY_THRESHOLD, DEFAULT_SAMPLE_SIZE};
use oif_types::quotes::request::{SolverOptions, SolverSelection};
use oif_types::{QuoteRequest, Solver};

use tracing::{debug, info};

/// Fixed weight for unknown solvers in weighted sampling (1%)
/// We assign a low weight (1%) to unknown solvers to allow them to be sampled
/// occasionally, without significantly impacting the selection of known compatible solvers.
const UNKNOWN_SOLVER_WEIGHT: f64 = 0.01;

/// Exponential decay rate for solver selection (0.3)
const SOLVER_EXPONENTIAL_DECAY_RATE: f64 = 0.3;

/// Strict compatibility result - Compatible means solver supports ALL requirements
#[derive(Debug, Clone, PartialEq)]
pub enum Compatibility {
	Compatible,   // Solver supports ALL inputs and ALL outputs
	Incompatible, // Explicitly doesn't support requirements
	Unknown,      // No information available - might work
}

impl Compatibility {
	pub fn score(&self) -> f64 {
		match self {
			Self::Compatible => 1.0,
			Self::Unknown => 0.0,
			Self::Incompatible => -1.0,
		}
	}

	pub fn is_viable(&self) -> bool {
		!matches!(self, Self::Incompatible)
	}
}

/// Trait for analyzing solver compatibility
#[cfg_attr(test, mockall::automock)]
pub trait CompatibilityAnalyzerTrait: Send + Sync {
	fn analyze(&self, solver: &Solver, request: &QuoteRequest) -> Compatibility;
}

/// Trait for selecting solvers based on strategy
#[cfg_attr(test, mockall::automock)]
pub trait SolverSelectorTrait: Send + Sync {
	fn select_solvers(
		&self,
		solvers: Vec<(Solver, Compatibility)>,
		options: &SolverOptions,
	) -> Vec<Solver>;
}

/// Main trait for solver filtering
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait SolverFilterTrait: Send + Sync {
	async fn filter_solvers(
		&self,
		available_solvers: &[Solver],
		request: &QuoteRequest,
		options: &SolverOptions,
		config: &AggregationConfig,
	) -> Vec<Solver>;
}

/// Core compatibility analyzer
pub struct CompatibilityAnalyzer;

impl CompatibilityAnalyzerTrait for CompatibilityAnalyzer {
	fn analyze(&self, solver: &Solver, request: &QuoteRequest) -> Compatibility {
		use oif_types::solvers::{AssetSource, SupportedAssets};

		// Check what type of support information this solver has
		match &solver.metadata.supported_assets {
			SupportedAssets::Routes { routes, source } => {
				// For routes mode, check if we have route data
				if routes.is_empty() && source == &AssetSource::AutoDiscovered {
					// Empty routes with auto-discovery means data hasn't been fetched yet
					return Compatibility::Unknown;
				}
				self.check_route_compatibility(solver, request)
			},
			SupportedAssets::Assets { assets, source } => {
				// For assets mode, check if we have asset data
				if assets.is_empty() && source == &AssetSource::AutoDiscovered {
					// Empty assets with auto-discovery means data hasn't been fetched yet
					return Compatibility::Unknown;
				}
				self.check_asset_compatibility(solver, request)
			},
		}
	}
}

impl CompatibilityAnalyzer {
	/// Check asset-based compatibility
	///
	/// For assets mode, solver supports any-to-any within its asset list (including same-chain swaps).
	/// Verify that all required outputs can be satisfied using available inputs.
	fn check_asset_compatibility(&self, solver: &Solver, request: &QuoteRequest) -> Compatibility {
		if request.requested_outputs.is_empty() {
			debug!("No requested outputs in request");
			return Compatibility::Unknown;
		}

		// For each requested output, check if ANY available input can reach it via asset support
		for output in &request.requested_outputs {
			let mut output_satisfiable = false;

			for input in &request.available_inputs {
				// For assets mode: check if solver supports the route from input to output asset
				// The supports_route method handles assets mode correctly by checking both assets individually
				if solver.supports_route(&input.asset, &output.asset) {
					debug!(
						"Solver '{}' (assets mode) can satisfy output {} via input {}",
						solver.solver_id, output.asset, input.asset
					);
					output_satisfiable = true;
					break; // Found at least one path to this output
				}
			}

			if !output_satisfiable {
				debug!(
					"Solver '{}' (assets mode) cannot satisfy output {} with any available input",
					solver.solver_id, output.asset
				);
				return Compatibility::Incompatible;
			}
		}

		debug!(
			"Solver '{}' (assets mode) can satisfy all {} requested outputs",
			solver.solver_id,
			request.requested_outputs.len()
		);
		Compatibility::Compatible
	}

	/// Check route-based compatibility using output-centric approach
	///
	/// For each requested output, verify that at least one available input can reach it.
	/// This correctly handles multiple inputs as options (OR logic) and multiple outputs as requirements (AND logic).
	fn check_route_compatibility(&self, solver: &Solver, request: &QuoteRequest) -> Compatibility {
		if request.requested_outputs.is_empty() {
			debug!("No requested outputs in request");
			return Compatibility::Unknown;
		}

		// For each requested output, check if ANY available input can reach it
		for output in &request.requested_outputs {
			let mut output_satisfiable = false;

			for input in &request.available_inputs {
				// In routes mode, check if solver explicitly supports this route
				// (including same-asset same-chain if explicitly defined)
				if solver.supports_route(&input.asset, &output.asset) {
					debug!(
						"Solver '{}' can satisfy output {} via input {}",
						solver.solver_id, output.asset, input.asset
					);
					output_satisfiable = true;
					break; // Found at least one path to this output
				}
			}

			if !output_satisfiable {
				debug!(
					"Solver '{}' cannot satisfy output {} with any available input",
					solver.solver_id, output.asset
				);
				return Compatibility::Incompatible;
			}
		}

		debug!(
			"Solver '{}' can satisfy all {} requested outputs",
			solver.solver_id,
			request.requested_outputs.len()
		);
		Compatibility::Compatible
	}
}

/// Solver selection strategies
#[derive(Default)]
pub struct SolverSelector;

impl SolverSelector {
	pub fn new() -> Self {
		Self
	}
}

impl SolverSelectorTrait for SolverSelector {
	fn select_solvers(
		&self,
		mut solvers: Vec<(Solver, Compatibility)>,
		options: &SolverOptions,
	) -> Vec<Solver> {
		solvers.sort_by(|a, b| {
			b.1.score()
				.partial_cmp(&a.1.score())
				.unwrap_or(std::cmp::Ordering::Equal)
		});

		match options
			.solver_selection
			.as_ref()
			.unwrap_or(&SolverSelection::All)
		{
			SolverSelection::All => solvers.into_iter().map(|(solver, _)| solver).collect(),
			SolverSelection::Sampled => {
				let sample_size = options.sample_size.unwrap_or(DEFAULT_SAMPLE_SIZE) as usize;
				self.weighted_sample(solvers, sample_size)
			},
			SolverSelection::Priority => {
				let threshold = options
					.priority_threshold
					.unwrap_or(DEFAULT_PRIORITY_THRESHOLD) as f64
					/ 100.0;
				solvers
					.into_iter()
					.filter(|(_, compat)| compat.score() >= threshold)
					.map(|(solver, _)| solver)
					.collect()
			},
		}
	}
}

impl SolverSelector {
	fn weighted_sample(
		&self,
		solvers: Vec<(Solver, Compatibility)>,
		sample_size: usize,
	) -> Vec<Solver> {
		use rand::Rng;

		if solvers.len() <= sample_size {
			return solvers.into_iter().map(|(solver, _)| solver).collect();
		}

		let mut rng = rand::rng();
		let mut selected = Vec::with_capacity(sample_size);
		let mut remaining = solvers;

		for _ in 0..sample_size {
			if remaining.is_empty() {
				break;
			}

			let weights: Vec<f64> = remaining
				.iter()
				.enumerate()
				.map(|(i, (_, compat))| {
					let base_weight = match compat {
						Compatibility::Compatible => 1.0,
						Compatibility::Unknown => UNKNOWN_SOLVER_WEIGHT,
						Compatibility::Incompatible => 0.0,
					};
					base_weight
						* (-SOLVER_EXPONENTIAL_DECAY_RATE * i as f64 / remaining.len() as f64).exp()
				})
				.collect();

			let total_weight: f64 = weights.iter().sum();
			if total_weight == 0.0 {
				break;
			}

			let mut random_weight = rng.random::<f64>() * total_weight;
			let mut selected_index = 0;

			for (i, &weight) in weights.iter().enumerate() {
				random_weight -= weight;
				if random_weight <= 0.0 {
					selected_index = i;
					break;
				}
			}

			let (solver, _) = remaining.remove(selected_index);
			selected.push(solver);
		}

		selected
	}
}

/// Main solver filter service
pub struct SolverFilterService {
	analyzer: CompatibilityAnalyzer,
	selector: SolverSelector,
}

impl SolverFilterService {
	pub fn new() -> Self {
		Self {
			analyzer: CompatibilityAnalyzer,
			selector: SolverSelector::new(),
		}
	}

	/// Filter solvers by include/exclude lists
	fn apply_filters(
		&self,
		solvers: Vec<(Solver, Compatibility)>,
		options: &SolverOptions,
	) -> Vec<(Solver, Compatibility)> {
		let mut filtered = solvers;

		// Apply include filter
		if let Some(include_list) = &options.include_solvers {
			if !include_list.is_empty() {
				filtered.retain(|(solver, _)| include_list.contains(&solver.solver_id));
				info!("Applied include filter: {} solvers", filtered.len());
			}
		}

		// Apply exclude filter
		if let Some(exclude_list) = &options.exclude_solvers {
			if !exclude_list.is_empty() {
				filtered.retain(|(solver, _)| !exclude_list.contains(&solver.solver_id));
				info!(
					"Applied exclude filter: {} solvers remaining",
					filtered.len()
				);
			}
		}

		filtered
	}
}

#[async_trait]
impl SolverFilterTrait for SolverFilterService {
	async fn filter_solvers(
		&self,
		available_solvers: &[Solver],
		request: &QuoteRequest,
		options: &SolverOptions,
		config: &AggregationConfig,
	) -> Vec<Solver> {
		info!(
			"Starting solver filtering: {} available",
			available_solvers.len()
		);

		// Step 1: Analyze compatibility
		let mut solver_compatibility: Vec<(Solver, Compatibility)> = available_solvers
			.iter()
			.map(|solver| {
				let compatibility = self.analyzer.analyze(solver, request);
				(solver.clone(), compatibility)
			})
			.collect();

		// Step 2: Remove incompatible solvers
		let initial_count = solver_compatibility.len();
		solver_compatibility.retain(|(_, compat)| compat.is_viable());

		if !config.include_unknown_compatibility {
			solver_compatibility.retain(|(_, compat)| !matches!(compat, Compatibility::Unknown));
		}

		info!(
			"Compatibility filtering: {} viable from {} total",
			solver_compatibility.len(),
			initial_count
		);

		// Step 3: Apply include/exclude filters
		solver_compatibility = self.apply_filters(solver_compatibility, options);

		// Step 4: Apply selection strategy
		let selected = self.selector.select_solvers(solver_compatibility, options);

		info!("Final selection: {} solvers", selected.len());
		selected
	}
}

impl Default for SolverFilterService {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use oif_config::AggregationConfig;
	use oif_types::{
		quotes::request::SolverOptions, AssetRoute, AvailableInput, InteropAddress, QuoteRequest,
		RequestedOutput, Solver, U256,
	};
	use std::collections::HashMap;

	// Helper functions for creating test data
	fn create_test_solver_with_routes(id: &str, assets: Vec<(u64, &str)>) -> Solver {
		// Convert assets to cross-chain routes directly
		let mut routes = Vec::new();

		if assets.is_empty() {
			// No assets = no routes (will be Unknown compatibility)
		} else if assets.len() == 1 {
			// Single asset: create route to/from chain 999 for cross-chain testing
			let (chain_id, address) = &assets[0];
			let origin =
				InteropAddress::from_text(&format!("eip155:{}:{}", chain_id, address)).unwrap();
			let destination =
				InteropAddress::from_text("eip155:999:0x9999999999999999999999999999999999999999")
					.unwrap();

			routes.push(AssetRoute::with_symbols(
				origin.clone(),
				format!("SYM{}", chain_id),
				destination.clone(),
				"COMP".to_string(),
			));
			routes.push(AssetRoute::with_symbols(
				destination,
				"COMP".to_string(),
				origin,
				format!("SYM{}", chain_id),
			));
		} else {
			// Multiple assets: create all cross-chain combinations
			for (i, (origin_chain, origin_addr)) in assets.iter().enumerate() {
				for (dest_chain, dest_addr) in assets.iter().skip(i + 1) {
					if origin_chain != dest_chain {
						let origin = InteropAddress::from_text(&format!(
							"eip155:{}:{}",
							origin_chain, origin_addr
						))
						.unwrap();
						let destination = InteropAddress::from_text(&format!(
							"eip155:{}:{}",
							dest_chain, dest_addr
						))
						.unwrap();

						routes.push(AssetRoute::with_symbols(
							origin.clone(),
							format!("SYM{}", origin_chain),
							destination.clone(),
							format!("SYM{}", dest_chain),
						));
						routes.push(AssetRoute::with_symbols(
							destination,
							format!("SYM{}", dest_chain),
							origin,
							format!("SYM{}", origin_chain),
						));
					}
				}
			}
		}

		Solver::new(
			id.to_string(),
			format!("{}_adapter", id),
			format!("https://{}.example.com", id),
		)
		.with_name(format!("Test Solver {}", id))
		.with_description(format!("Test solver {}", id))
		.with_version("1.0.0".to_string())
		.with_routes(routes)
		.with_headers(HashMap::new())
	}

	fn create_test_solver_with_assets(id: &str, assets: Vec<(u64, &str)>) -> Solver {
		use oif_types::{models::Asset, InteropAddress};

		// Convert to Asset structs
		let asset_list: Vec<Asset> = assets
			.into_iter()
			.map(|(chain_id, address)| {
				let interop_addr =
					InteropAddress::from_text(&format!("eip155:{}:{}", chain_id, address)).unwrap();
				Asset::new(
					interop_addr,
					format!("SYM{}", chain_id),
					format!("Symbol {}", chain_id),
					18,
				)
			})
			.collect();

		Solver::new(
			id.to_string(),
			format!("{}_adapter", id),
			format!("https://{}.example.com", id),
		)
		.with_name(format!("Test Solver {}", id))
		.with_description(format!("Test solver {}", id))
		.with_version("1.0.0".to_string())
		.with_assets(asset_list)
		.with_headers(HashMap::new())
	}

	fn create_test_request(
		input_assets: Vec<(u64, &str)>,
		output_assets: Vec<(u64, &str)>,
	) -> QuoteRequest {
		QuoteRequest {
			user: InteropAddress::from_text("eip155:1:0x742d35Cc6675C88b1C6e3c0c61b2e9a3D0C3F123")
				.unwrap(),
			available_inputs: input_assets
				.into_iter()
				.map(|(chain_id, address)| AvailableInput {
					user: InteropAddress::from_text(
						"eip155:1:0x742d35Cc6675C88b1C6e3c0c61b2e9a3D0C3F123",
					)
					.unwrap(),
					asset: InteropAddress::from_text(&format!("eip155:{}:{}", chain_id, address))
						.unwrap(),
					amount: U256::new("1000000000000000000".to_string()), // 1 ETH in wei
					lock: None,
				})
				.collect(),
			requested_outputs: output_assets
				.into_iter()
				.map(|(chain_id, address)| RequestedOutput {
					receiver: InteropAddress::from_text(
						"eip155:1:0x742d35Cc6675C88b1C6e3c0c61b2e9a3D0C3F123",
					)
					.unwrap(),
					asset: InteropAddress::from_text(&format!("eip155:{}:{}", chain_id, address))
						.unwrap(),
					amount: U256::new("2500000000".to_string()), // 2500 USDC
					calldata: None,
				})
				.collect(),
			min_valid_until: None,
			preference: None,
			solver_options: None,
			metadata: None,
		}
	}

	fn default_config() -> AggregationConfig {
		AggregationConfig::default()
	}

	fn config_exclude_unknown() -> AggregationConfig {
		AggregationConfig {
			include_unknown_compatibility: false,
			..Default::default()
		}
	}

	fn config_include_unknown() -> AggregationConfig {
		AggregationConfig {
			include_unknown_compatibility: true,
			..Default::default()
		}
	}

	mod compatibility_analyzer_tests {
		use super::*;

		#[test]
		fn test_full_compatibility() {
			let analyzer = CompatibilityAnalyzer;
			// Solver supports BOTH input and output assets for full compatibility
			let solver = create_test_solver_with_routes(
				"solver1",
				vec![
					(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B"), // Input asset on Ethereum
					(10, "0xB0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1C"), // Output asset on Optimism
				],
			);
			let request = create_test_request(
				vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")], // Input on Ethereum
				vec![(10, "0xB0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1C")], // Output on Optimism
			);

			let result = analyzer.analyze(&solver, &request);
			println!("result: {:?}", result);
			assert!(matches!(result, Compatibility::Compatible));
			assert!(result.score() == 1.0);
		}

		#[test]
		fn test_strict_incompatibility_missing_output() {
			let analyzer = CompatibilityAnalyzer;
			// Solver only supports INPUT asset, NOT output asset
			let solver = create_test_solver_with_routes(
				"solver1",
				vec![
					(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B"), // Only input asset
					                                                   // NOT supporting: (1, "0xB0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1C")
				],
			);

			// Strict compatibility: Missing output asset = Incompatible
			let request = create_test_request(
				vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")], // Input (supported)
				vec![(1, "0xB0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1C")], // Output (NOT supported)
			);

			let result = analyzer.analyze(&solver, &request);
			assert_eq!(result, Compatibility::Incompatible);
		}

		#[test]
		fn test_network_incompatibility() {
			let analyzer = CompatibilityAnalyzer;
			let solver = create_test_solver_with_routes(
				"solver1",
				vec![(1, "0x0000000000000000000000000000000000000000")],
			);
			let request = create_test_request(
				vec![(42, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")], // Different network
				vec![(4, "0xB0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1C")],  // Not supported
			);

			let result = analyzer.analyze(&solver, &request);
			assert_eq!(result, Compatibility::Incompatible);
		}

		#[test]
		fn test_strict_asset_incompatibility() {
			let analyzer = CompatibilityAnalyzer;
			// Solver supports cross-chain routes but not the specific assets requested
			let solver = create_test_solver_with_routes(
				"solver1",
				vec![
					(1, "0x3333333333333333333333333333333333333333"), // Chain 1
					(10, "0x4444444444444444444444444444444444444444"), // Chain 10
				],
			);
			let request = create_test_request(
				vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")], // Different asset on chain 1
				vec![(10, "0xB0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1C")], // Different asset on chain 10
			);

			let result = analyzer.analyze(&solver, &request);
			assert_eq!(result, Compatibility::Incompatible);
		}

		#[test]
		fn test_complete_incompatibility() {
			let analyzer = CompatibilityAnalyzer;
			// Solver supports different network entirely
			let solver = create_test_solver_with_routes(
				"solver1",
				vec![(2, "0x2222222222222222222222222222222222222222")],
			);
			let request = create_test_request(
				vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")], // Chain 1 (not supported)
				vec![(1, "0xB0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1C")], // Chain 1 (not supported)
			);

			let result = analyzer.analyze(&solver, &request);
			assert_eq!(result, Compatibility::Incompatible); // Truly incompatible - different networks
		}

		#[test]
		fn test_unknown_compatibility() {
			let analyzer = CompatibilityAnalyzer;
			// Create solver with empty routes but auto-discovery source (data not fetched yet)
			let mut solver = create_test_solver_with_routes("solver1", vec![]); // No metadata
																	   // Override to simulate auto-discovery with empty data
			solver.metadata.supported_assets = oif_types::solvers::SupportedAssets::Routes {
				routes: vec![],
				source: oif_types::solvers::AssetSource::AutoDiscovered,
			};

			let request = create_test_request(
				vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")],
				vec![(1, "0xB0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1C")],
			);

			let result = analyzer.analyze(&solver, &request);
			assert_eq!(result, Compatibility::Unknown);
		}

		// Asset-based compatibility tests
		#[test]
		fn test_asset_mode_full_compatibility() {
			let analyzer = CompatibilityAnalyzer;
			// Solver supports BOTH input and output assets for full compatibility
			let solver = create_test_solver_with_assets(
				"solver1",
				vec![
					(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B"), // Input asset on Ethereum
					(10, "0xB0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1C"), // Output asset on Optimism
				],
			);
			let request = create_test_request(
				vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")], // Input on Ethereum
				vec![(10, "0xB0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1C")], // Output on Optimism
			);

			let result = analyzer.analyze(&solver, &request);
			assert!(matches!(result, Compatibility::Compatible));
			assert_eq!(result.score(), 1.0);
		}

		#[test]
		fn test_asset_mode_same_chain_swap() {
			let analyzer = CompatibilityAnalyzer;
			// Solver supports same-chain swaps in asset mode
			let solver = create_test_solver_with_assets(
				"solver1",
				vec![
					(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B"), // USDC on Ethereum
					(1, "0xB0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1C"), // WETH on Ethereum
				],
			);
			let request = create_test_request(
				vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")], // Input: USDC on Ethereum
				vec![(1, "0xB0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1C")], // Output: WETH on Ethereum
			);

			let result = analyzer.analyze(&solver, &request);
			assert!(matches!(result, Compatibility::Compatible));
		}

		#[test]
		fn test_asset_mode_incompatible() {
			let analyzer = CompatibilityAnalyzer;
			// Solver only supports Ethereum assets, request needs Polygon output
			let solver = create_test_solver_with_assets(
				"solver1",
				vec![
					(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B"), // USDC on Ethereum
					(1, "0xB0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1C"), // WETH on Ethereum
				],
			);
			let request = create_test_request(
				vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")], // Input: USDC on Ethereum
				vec![(137, "0xC0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1D")], // Output: Token on Polygon
			);

			let result = analyzer.analyze(&solver, &request);
			assert!(matches!(result, Compatibility::Incompatible));
		}

		#[test]
		fn test_asset_mode_unknown_empty_autodiscovery() {
			let analyzer = CompatibilityAnalyzer;
			// Create a solver with empty assets but auto-discovery source (data not fetched yet)
			let mut solver = create_test_solver_with_assets("solver1", vec![]);
			// Override to simulate auto-discovery with empty data
			solver.metadata.supported_assets = oif_types::solvers::SupportedAssets::Assets {
				assets: vec![],
				source: oif_types::solvers::AssetSource::AutoDiscovered,
			};

			let request = create_test_request(
				vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")],
				vec![(10, "0xB0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1C")],
			);

			let result = analyzer.analyze(&solver, &request);
			assert_eq!(result, Compatibility::Unknown);
		}
	}

	mod additional_tests {
		use super::*;

		#[test]
		fn test_strict_cross_network_incompatibility() {
			let analyzer = CompatibilityAnalyzer;
			// Solver only has assets on networks 1,2 but request needs network 42
			let solver = create_test_solver_with_routes(
				"solver1",
				vec![
					(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B"),
					(2, "0x6666666666666666666666666666666666666666"),
				],
			);
			let request = create_test_request(
				vec![(42, "0xB0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1C")], // Input on unsupported network 42
				vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")],  // Output on supported network 1
			);

			let result = analyzer.analyze(&solver, &request);
			assert_eq!(result, Compatibility::Incompatible); // Strict: must support ALL assets
		}

		#[test]
		fn test_multi_network_usdc_base_to_ethereum() {
			let analyzer = CompatibilityAnalyzer;

			// Create a solver that supports both Base (8453) and Ethereum (1) with USDC on both
			let cross_chain_solver = create_test_solver_with_routes(
				"cross_chain_solver",
				vec![
					(8453, "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"), // Base USDC
					(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B"),    // Ethereum USDC
				],
			);

			// USDC transfer from Base to Ethereum
			let usdc_cross_chain_request = create_test_request(
				vec![(8453, "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913")], // Input: Base USDC
				vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")],    // Output: Ethereum USDC
			);

			let result = analyzer.analyze(&cross_chain_solver, &usdc_cross_chain_request);
			assert_eq!(result, Compatibility::Compatible); // Should be compatible
			assert_eq!(result.score(), 1.0);
		}

		#[test]
		fn test_multi_network_incompatible_missing_base_support() {
			let analyzer = CompatibilityAnalyzer;

			// Create a solver that only supports Ethereum, not Base
			let ethereum_only_solver = create_test_solver_with_routes(
				"ethereum_only_solver",
				vec![
					(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B"), // Ethereum USDC
				],
			);

			// USDC transfer from Base to Ethereum (but solver doesn't support Base)
			let usdc_cross_chain_request = create_test_request(
				vec![(8453, "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913")], // Input: Base USDC
				vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")],    // Output: Ethereum USDC
			);

			let result = analyzer.analyze(&ethereum_only_solver, &usdc_cross_chain_request);
			assert_eq!(result, Compatibility::Incompatible); // Missing Base network support
		}

		#[test]
		fn test_multi_network_incompatible_missing_usdc_on_base() {
			let analyzer = CompatibilityAnalyzer;

			// Create a solver that supports both networks but missing Base USDC
			let incomplete_solver = create_test_solver_with_routes(
				"incomplete_solver",
				vec![
					(8453, "0x5555555555555555555555555555555555555555"), // Different token on Base
					(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B"),    // Ethereum USDC (correct)
				],
			);

			// USDC transfer from Base to Ethereum
			let usdc_cross_chain_request = create_test_request(
				vec![(8453, "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913")], // Input: Base USDC
				vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")],    // Output: Ethereum USDC
			);

			let result = analyzer.analyze(&incomplete_solver, &usdc_cross_chain_request);
			assert_eq!(result, Compatibility::Incompatible); // Missing specific Base USDC asset
		}

		#[test]
		fn test_same_address_different_networks() {
			let analyzer = CompatibilityAnalyzer;

			// Create a solver that supports cross-chain routes involving network 1 but not network 2
			let solver_with_network1 = create_test_solver_with_routes(
				"solver_with_network1",
				vec![
					(1, "0x1234567890123456789012345678901234567890"), // Address on network 1
					(10, "0x7777777777777777777777777777777777777777"), // Different address on network 10
				],
			);

			// Request that needs the same address but on network 2 (not supported)
			let cross_network_request = create_test_request(
				vec![(2, "0x1234567890123456789012345678901234567890")], // Same address but network 2
				vec![(10, "0x7777777777777777777777777777777777777777")], // Supported destination
			);

			let result = analyzer.analyze(&solver_with_network1, &cross_network_request);
			assert_eq!(result, Compatibility::Incompatible); // Same address but wrong network

			// Now test the opposite: solver supports network 2, request needs network 1
			let solver_with_network2 = create_test_solver_with_routes(
				"solver_with_network2",
				vec![
					(2, "0x1234567890123456789012345678901234567890"), // Address on network 2
					(10, "0x7777777777777777777777777777777777777777"), // Different address on network 10
				],
			);

			let network1_request = create_test_request(
				vec![(1, "0x1234567890123456789012345678901234567890")], // Same address but network 1
				vec![(10, "0x7777777777777777777777777777777777777777")], // Supported destination
			);

			let result = analyzer.analyze(&solver_with_network2, &network1_request);
			assert_eq!(result, Compatibility::Incompatible); // Same address but wrong network
		}

		#[test]
		fn test_multiple_inputs_single_output_or_logic() {
			let analyzer = CompatibilityAnalyzer;

			// Solver only supports ETH→USDC route, not MATIC→USDC
			let solver = create_test_solver_with_routes(
				"partial_solver",
				vec![
					(1, "0x1111111111111111111111111111111111111111"), // ETH on Ethereum
					(137, "0x2222222222222222222222222222222222222222"), // USDC on Polygon
				],
			);

			// Request with multiple input options but single output
			let request = create_test_request(
				vec![
					(1, "0x1111111111111111111111111111111111111111"), // ETH on Ethereum (supported)
					(137, "0x3333333333333333333333333333333333333333"), // MATIC on Polygon (not supported)
				],
				vec![(137, "0x2222222222222222222222222222222222222222")], // USDC on Polygon
			);

			let result = analyzer.analyze(&solver, &request);
			// Should be Compatible because ETH(Ethereum)→USDC(Polygon) route exists
			// Even though MATIC(Polygon)→USDC(Polygon) doesn't exist
			assert_eq!(result, Compatibility::Compatible);
		}

		#[test]
		fn test_single_input_multiple_outputs_and_logic() {
			let analyzer = CompatibilityAnalyzer;

			// Solver supports ETH→USDC but not ETH→USDT
			let solver = create_test_solver_with_routes(
				"partial_solver",
				vec![
					(1, "0x1111111111111111111111111111111111111111"), // ETH on Ethereum
					(137, "0x2222222222222222222222222222222222222222"), // USDC on Polygon
					                                                   // Missing: USDT on Arbitrum
				],
			);

			// Request with single input but multiple outputs
			let request = create_test_request(
				vec![(1, "0x1111111111111111111111111111111111111111")], // ETH on Ethereum
				vec![
					(137, "0x2222222222222222222222222222222222222222"), // USDC on Polygon (supported)
					(42161, "0x3333333333333333333333333333333333333333"), // USDT on Arbitrum (not supported)
				],
			);

			let result = analyzer.analyze(&solver, &request);
			// Should be Incompatible because ETH→USDT route doesn't exist
			// ALL outputs must be satisfiable
			assert_eq!(result, Compatibility::Incompatible);
		}

		#[test]
		fn test_multiple_inputs_multiple_outputs_mixed_logic() {
			let analyzer = CompatibilityAnalyzer;

			// Solver supports some but not all routes
			let solver = create_test_solver_with_routes(
				"mixed_solver",
				vec![
					(1, "0x1111111111111111111111111111111111111111"), // ETH on Ethereum
					(137, "0x2222222222222222222222222222222222222222"), // USDC on Polygon
					(10, "0x3333333333333333333333333333333333333333"), // USDT on Optimism
					                                                   // Missing: MATIC on Polygon, USDT on Arbitrum
				],
			);

			// Complex request: multiple inputs and outputs
			let request = create_test_request(
				vec![
					(1, "0x1111111111111111111111111111111111111111"), // ETH on Ethereum
					(137, "0x4444444444444444444444444444444444444444"), // MATIC on Polygon (not supported)
				],
				vec![
					(137, "0x2222222222222222222222222222222222222222"), // USDC on Polygon
					(10, "0x3333333333333333333333333333333333333333"),  // USDT on Optimism
				],
			);

			let result = analyzer.analyze(&solver, &request);
			// Should be Compatible because:
			// - USDC(Polygon) can be satisfied by ETH(Ethereum)→USDC(Polygon)
			// - USDT(Optimism) can be satisfied by ETH(Ethereum)→USDT(Optimism)
			// Even though MATIC inputs cannot satisfy these outputs
			assert_eq!(result, Compatibility::Compatible);
		}

		#[test]
		fn test_no_viable_input_for_output() {
			let analyzer = CompatibilityAnalyzer;

			// Solver supports limited routes
			let solver = create_test_solver_with_routes(
				"limited_solver",
				vec![
					(1, "0x1111111111111111111111111111111111111111"), // ETH on Ethereum
					(137, "0x2222222222222222222222222222222222222222"), // USDC on Polygon
				],
			);

			// Request where no input can reach the desired output
			let request = create_test_request(
				vec![
					(1, "0x1111111111111111111111111111111111111111"), // ETH on Ethereum
					(137, "0x2222222222222222222222222222222222222222"), // USDC on Polygon
				],
				vec![(42161, "0x3333333333333333333333333333333333333333")], // USDT on Arbitrum (unreachable)
			);

			let result = analyzer.analyze(&solver, &request);
			// Should be Incompatible because no input can reach USDT on Arbitrum
			assert_eq!(result, Compatibility::Incompatible);
		}

		#[test]
		fn test_same_asset_different_chains_supported() {
			let analyzer = CompatibilityAnalyzer;

			// Solver with same asset on different chains (valid bridging scenario)
			let solver = create_test_solver_with_routes(
				"bridging_solver",
				vec![
					(1, "0x1111111111111111111111111111111111111111"), // USDC on Ethereum
					(137, "0x1111111111111111111111111111111111111111"), // USDC on Polygon (same asset address)
				],
			);

			// Request for USDC bridging (same asset, different chains)
			let request = create_test_request(
				vec![(1, "0x1111111111111111111111111111111111111111")], // USDC on Ethereum
				vec![(137, "0x1111111111111111111111111111111111111111")], // USDC on Polygon
			);

			let result = analyzer.analyze(&solver, &request);
			// Should be Compatible because USDC(Ethereum)→USDC(Polygon) is a valid cross-chain route
			// Same asset bridging is a common use case
			assert_eq!(result, Compatibility::Compatible);
		}

		#[test]
		fn test_explicit_same_chain_route_respected() {
			let analyzer = CompatibilityAnalyzer;

			// Create solver with explicit same-asset same-chain route
			let usdc_addr =
				InteropAddress::from_text("eip155:1:0x1111111111111111111111111111111111111111")
					.unwrap();
			let same_chain_route = AssetRoute::with_symbols(
				usdc_addr.clone(),
				"USDC".to_string(),
				usdc_addr.clone(), // Same asset, same chain
				"USDC".to_string(),
			);

			let solver = Solver::new(
				"same_chain_solver".to_string(),
				"test_adapter".to_string(),
				"https://test.example.com".to_string(),
			)
			.with_routes(vec![same_chain_route]);

			// Request with same asset on same chain
			let request = create_test_request(
				vec![(1, "0x1111111111111111111111111111111111111111")], // USDC on Ethereum
				vec![(1, "0x1111111111111111111111111111111111111111")], // USDC on Ethereum (same asset, same chain)
			);

			let result = analyzer.analyze(&solver, &request);
			// Should be Compatible because the route is explicitly defined
			assert_eq!(result, Compatibility::Compatible);
		}

		#[test]
		fn test_real_world_scenario_local_vs_testnet() {
			let analyzer = CompatibilityAnalyzer;

			// Local anvil solver with local chain routes (exactly like your data)
			let local_solver = create_test_solver_with_routes(
				"example-solver", // Same ID as your local solver
				vec![
					(31337, "0x5fbdb2315678afecb367f032d93f642f64180aa3"), // TOKA on local chain
					(31338, "0xe7f1725e7734ce288f8367e1bb143e90bb3f0512"), // TOKB on local chain
				],
			);

			// Request with testnet tokens (exactly like your request)
			let testnet_request = create_test_request(
				vec![(11155420, "0x4200000000000000000000000000000000000006")], // ETH on testnet
				vec![(129399, "0x17b8ee96e3bcb3b04b3e8334de4524520c51cab4")],   // ETH on different testnet
			);

			let result = analyzer.analyze(&local_solver, &testnet_request);

			// Should be Incompatible because local solver has no testnet routes
			assert_eq!(result, Compatibility::Incompatible);
		}
	}

	mod solver_selector_tests {
		use super::*;

		#[test]
		fn test_select_all_strategy() {
			let selector = SolverSelector::new();
			let solvers = vec![
				(
					create_test_solver_with_routes("s1", vec![]),
					Compatibility::Compatible,
				),
				(
					create_test_solver_with_routes("s2", vec![]),
					Compatibility::Compatible,
				),
				(
					create_test_solver_with_routes("s3", vec![]),
					Compatibility::Unknown,
				),
			];

			let options = SolverOptions {
				solver_selection: Some(SolverSelection::All),
				..Default::default()
			};

			let result = selector.select_solvers(solvers, &options);
			assert_eq!(result.len(), 3); // All solvers returned

			// Should be sorted by score (highest first)
			assert_eq!(result[0].solver_id, "s1");
			assert_eq!(result[1].solver_id, "s2");
			assert_eq!(result[2].solver_id, "s3");
		}

		#[test]
		fn test_priority_strategy() {
			let selector = SolverSelector::new();
			let solvers = vec![
				(
					create_test_solver_with_routes("s1", vec![]),
					Compatibility::Compatible, // score = 1.0
				),
				(
					create_test_solver_with_routes("s2", vec![]),
					Compatibility::Unknown, // score = 0.0
				),
				(
					create_test_solver_with_routes("s3", vec![]),
					Compatibility::Incompatible, // score = -1.0
				),
			];

			let options = SolverOptions {
				solver_selection: Some(SolverSelection::Priority),
				priority_threshold: Some(50), // 0.5 threshold
				..Default::default()
			};

			let result = selector.select_solvers(solvers, &options);
			assert_eq!(result.len(), 1); // Only s1 above 0.5 threshold (score = 1.0)
			assert_eq!(result[0].solver_id, "s1");
		}

		#[test]
		fn test_sampled_strategy() {
			let selector = SolverSelector::new();
			let solvers = vec![
				(
					create_test_solver_with_routes("s1", vec![]),
					Compatibility::Compatible, // score = 1.0
				),
				(
					create_test_solver_with_routes("s2", vec![]),
					Compatibility::Compatible, // score = 1.0
				),
				(
					create_test_solver_with_routes("s3", vec![]),
					Compatibility::Compatible, // score = 1.0
				),
				(
					create_test_solver_with_routes("s4", vec![]),
					Compatibility::Unknown, // score = 0.0
				),
				(
					create_test_solver_with_routes("s5", vec![]),
					Compatibility::Unknown, // score = 0.0
				),
			];

			let options = SolverOptions {
				solver_selection: Some(SolverSelection::Sampled),
				sample_size: Some(3),
				..Default::default()
			};

			let result = selector.select_solvers(solvers, &options);
			assert_eq!(result.len(), 3); // Exactly sample_size

			// Compatible solvers should be heavily preferred in sampling
			let selected_ids: Vec<String> = result.iter().map(|s| s.solver_id.clone()).collect();
			assert!(selected_ids.len() == 3);
		}

		#[test]
		fn test_sample_size_larger_than_available() {
			let selector = SolverSelector::new();
			let solvers = vec![
				(
					create_test_solver_with_routes("s1", vec![]),
					Compatibility::Compatible,
				),
				(
					create_test_solver_with_routes("s2", vec![]),
					Compatibility::Compatible,
				),
			];

			let options = SolverOptions {
				solver_selection: Some(SolverSelection::Sampled),
				sample_size: Some(5), // More than available
				..Default::default()
			};

			let result = selector.select_solvers(solvers, &options);
			assert_eq!(result.len(), 2); // Returns all available solvers
		}
	}

	mod solver_filter_service_tests {
		use super::*;

		#[tokio::test]
		async fn test_basic_filtering() {
			let service = SolverFilterService::new();
			let solvers = vec![
				create_test_solver_with_routes(
					"compatible",
					vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")],
				),
				create_test_solver_with_routes(
					"incompatible",
					vec![(2, "0xB0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1C")],
				),
				{
					// Create solver with auto-discovery source but empty routes (data not fetched yet)
					let mut unknown_solver = create_test_solver_with_routes("unknown", vec![]);
					unknown_solver.metadata.supported_assets =
						oif_types::solvers::SupportedAssets::Routes {
							routes: vec![],
							source: oif_types::solvers::AssetSource::AutoDiscovered,
						};
					unknown_solver
				},
			];

			let request = create_test_request(
				vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")], // Input on chain 1
				vec![(999, "0x9999999999999999999999999999999999999999")], // Output on chain 999
			);
			let options = SolverOptions::default();
			let config = config_include_unknown();

			let result = service
				.filter_solvers(&solvers, &request, &options, &config)
				.await;

			// Should include compatible and unknown (with include_unknown config for tests)
			assert_eq!(result.len(), 2);
			let result_ids: Vec<String> = result.iter().map(|s| s.solver_id.clone()).collect();
			assert!(result_ids.contains(&"compatible".to_string()));
			assert!(result_ids.contains(&"unknown".to_string()));
			assert!(!result_ids.contains(&"incompatible".to_string()));
		}

		#[tokio::test]
		async fn test_exclude_unknown_compatibility() {
			let service = SolverFilterService::new();
			let solvers = vec![
				create_test_solver_with_routes(
					"compatible",
					vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")],
				),
				{
					// Create solver with auto-discovery source but empty routes (data not fetched yet)
					let mut unknown_solver = create_test_solver_with_routes("unknown", vec![]);
					unknown_solver.metadata.supported_assets =
						oif_types::solvers::SupportedAssets::Routes {
							routes: vec![],
							source: oif_types::solvers::AssetSource::AutoDiscovered,
						};
					unknown_solver
				},
			];

			let request = create_test_request(
				vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")], // Input on chain 1
				vec![(999, "0x9999999999999999999999999999999999999999")], // Output on chain 999
			);
			let options = SolverOptions::default();
			let config = config_exclude_unknown();

			let result = service
				.filter_solvers(&solvers, &request, &options, &config)
				.await;

			// Should only include compatible (unknown excluded by config)
			assert_eq!(result.len(), 1);
			assert_eq!(result[0].solver_id, "compatible");
		}

		#[tokio::test]
		async fn test_include_exclude_filters() {
			let service = SolverFilterService::new();
			let solvers = vec![
				create_test_solver_with_routes("solver1", vec![]),
				create_test_solver_with_routes("solver2", vec![]),
				create_test_solver_with_routes("solver3", vec![]),
			];

			let request = create_test_request(
				vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")],
				vec![],
			);
			let options = SolverOptions {
				include_solvers: Some(vec!["solver1".to_string(), "solver2".to_string()]),
				exclude_solvers: Some(vec!["solver2".to_string()]),
				..Default::default()
			};
			let config = config_include_unknown();

			let result = service
				.filter_solvers(&solvers, &request, &options, &config)
				.await;

			// Should include only solver1 (solver2 excluded, solver3 not in include list)
			assert_eq!(result.len(), 1);
			assert_eq!(result[0].solver_id, "solver1");
		}

		#[tokio::test]
		async fn test_no_viable_solvers() {
			let service = SolverFilterService::new();
			let solvers = vec![
				create_test_solver_with_routes(
					"incompatible1",
					vec![(2, "0x3333333333333333333333333333333333333333")],
				),
				create_test_solver_with_routes(
					"incompatible2",
					vec![(3, "0x4444444444444444444444444444444444444444")],
				),
			];

			let request = create_test_request(
				vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")], // Input on chain 1
				vec![(42, "0x5555555555555555555555555555555555555555")], // Output on unsupported chain 42
			);
			let options = SolverOptions::default();
			let config = default_config();

			let result = service
				.filter_solvers(&solvers, &request, &options, &config)
				.await;
			assert_eq!(result.len(), 0); // No compatible solvers (wrong networks/assets)
		}

		#[tokio::test]
		async fn test_solver_selection_integration() {
			let service = SolverFilterService::new();
			let solvers = vec![
				create_test_solver_with_routes(
					"compatible",
					vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")],
				),
				create_test_solver_with_routes(
					"incompatible",
					vec![(2, "0xB0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1C")],
				), // Different network
				create_test_solver_with_routes("network_only", vec![]), // No asset metadata
			];

			let request = create_test_request(
				vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")], // Input on chain 1
				vec![(999, "0x9999999999999999999999999999999999999999")], // Output on chain 999
			);
			let options = SolverOptions {
				solver_selection: Some(SolverSelection::Priority),
				priority_threshold: Some(90), // Very high threshold to filter out unknown solvers
				..Default::default()
			};
			let config = default_config();

			let result = service
				.filter_solvers(&solvers, &request, &options, &config)
				.await;

			// Should have solvers that are compatible enough to pass the high threshold
			assert!(result.len() >= 1);
			assert!(result.len() <= 2); // At most the compatible and maybe network_only solver

			// Should include the fully compatible solver
			let result_ids: Vec<String> = result.iter().map(|s| s.solver_id.clone()).collect();
			assert!(result_ids.contains(&"compatible".to_string()));

			// Should not include the incompatible solver
			assert!(!result_ids.contains(&"incompatible".to_string()));
		}
	}

	mod compatibility_edge_cases {
		use super::*;

		#[test]
		fn test_compatibility_score_bounds() {
			let compatible = Compatibility::Compatible;
			let unknown = Compatibility::Unknown;
			let incompatible = Compatibility::Incompatible;

			assert_eq!(compatible.score(), 1.0);
			assert_eq!(unknown.score(), 0.0);
			assert_eq!(incompatible.score(), -1.0);

			assert!(compatible.is_viable());
			assert!(unknown.is_viable());
			assert!(!incompatible.is_viable());
		}

		#[test]
		fn test_weighted_sampling_with_unknown_solvers() {
			let selector = SolverSelector::new();
			let solvers = vec![
				(
					create_test_solver_with_routes("known", vec![]),
					Compatibility::Compatible,
				),
				(
					create_test_solver_with_routes("unknown", vec![]),
					Compatibility::Unknown,
				),
			];

			let options = SolverOptions {
				solver_selection: Some(SolverSelection::Sampled),
				sample_size: Some(1),
				..Default::default()
			};

			// Run multiple times to test probabilistic behavior
			let mut known_selected = 0;
			let mut unknown_selected = 0;

			for _ in 0..100 {
				let result = selector.select_solvers(solvers.clone(), &options);
				if result.len() == 1 {
					if result[0].solver_id == "known" {
						known_selected += 1;
					} else if result[0].solver_id == "unknown" {
						unknown_selected += 1;
					}
				}
			}

			// Known solver should be selected much more often due to higher weight
			assert!(known_selected > unknown_selected);
			assert!(known_selected > 50); // Should be selected in majority of cases
		}
	}
}
