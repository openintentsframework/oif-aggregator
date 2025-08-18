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
use std::collections::HashSet;
use tracing::{debug, info};

/// Fixed weight for unknown solvers in weighted sampling (1%)
const UNKNOWN_SOLVER_WEIGHT: f64 = 0.01;

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
		let required_assets = Self::extract_assets(request);

		if solver.metadata.supported_assets.is_empty() {
			return Compatibility::Unknown;
		}

		self.check_asset_compatibility(solver, &required_assets)
	}
}

impl CompatibilityAnalyzer {
	fn check_asset_compatibility(
		&self,
		solver: &Solver,
		required_assets: &HashSet<(u64, String)>,
	) -> Compatibility {
		for (chain_id, address) in required_assets {
			if !solver.supports_asset_on_chain(*chain_id, address) {
				return Compatibility::Incompatible;
			}
		}

		Compatibility::Compatible
	}

	fn extract_assets(request: &QuoteRequest) -> HashSet<(u64, String)> {
		let mut assets = HashSet::new();

		for input in &request.available_inputs {
			if let Ok(chain_id) = input.asset.extract_chain_id() {
				let address = input.asset.extract_address();
				assets.insert((chain_id, address));
			}
		}

		for output in &request.requested_outputs {
			if let Ok(chain_id) = output.asset.extract_chain_id() {
				let address = output.asset.extract_address();
				assets.insert((chain_id, address));
			}
		}

		assets
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
					base_weight * (-0.3 * i as f64 / remaining.len() as f64).exp()
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
				debug!("Solver '{}': {:?}", solver.solver_id, compatibility);
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
		quotes::request::SolverOptions, solvers::SolverMetadata, AvailableInput, InteropAddress,
		QuoteRequest, RequestedOutput, Solver, U256,
	};
	use std::collections::HashMap;

	// Helper functions for creating test data
	fn create_test_solver(id: &str, assets: Vec<(u64, &str)>) -> Solver {
		use oif_types::chrono::Utc;
		use oif_types::models::Asset;
		use oif_types::solvers::{SolverMetrics, SolverStatus};

		let supported_assets: Vec<Asset> = assets
			.into_iter()
			.map(|(chain_id, address)| Asset {
				name: format!("Asset {}", address),
				symbol: format!("SYM{}", chain_id),
				address: address.to_string(),
				chain_id,
				decimals: 18,
			})
			.collect();

		Solver {
			solver_id: id.to_string(),
			adapter_id: format!("{}_adapter", id),
			endpoint: format!("https://{}.example.com", id),
			timeout_ms: 2000,
			status: SolverStatus::Active,
			metadata: SolverMetadata {
				name: Some(format!("Test Solver {}", id)),
				description: Some(format!("Test solver {}", id)),
				version: Some("1.0.0".to_string()),
				supported_assets,
				max_retries: 3,
				headers: Some(HashMap::new()),
				config: HashMap::new(),
			},
			created_at: Utc::now(),
			last_seen: Some(Utc::now()),
			metrics: SolverMetrics::default(),
		}
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

	mod compatibility_analyzer_tests {
		use super::*;

		#[test]
		fn test_full_compatibility() {
			let analyzer = CompatibilityAnalyzer;
			// Solver supports BOTH input and output assets for full compatibility
			let solver = create_test_solver(
				"solver1",
				vec![
					(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B"), // Input asset
					(1, "0xB0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1C"), // Output asset
				],
			);
			let request = create_test_request(
				vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")], // Input
				vec![(1, "0xB0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1C")], // Output
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
			let solver = create_test_solver(
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
			let solver = create_test_solver(
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
			// Solver supports different assets but same network - strict mode = incompatible
			let solver = create_test_solver(
				"solver1",
				vec![
					(1, "0xDifferentAsset1111111111111111111111111"),
					(1, "0xDifferentAsset2222222222222222222222222"),
				],
			);
			let request = create_test_request(
				vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")], // Input (asset not supported, but network is)
				vec![(1, "0xB0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1C")], // Output (asset not supported, but network is)
			);

			let result = analyzer.analyze(&solver, &request);
			assert_eq!(result, Compatibility::Incompatible);
		}

		#[test]
		fn test_complete_incompatibility() {
			let analyzer = CompatibilityAnalyzer;
			// Solver supports different network entirely
			let solver = create_test_solver(
				"solver1",
				vec![(2, "0xDifferentAsset1111111111111111111111111")],
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
			let solver = create_test_solver("solver1", vec![]); // No metadata
			let request = create_test_request(
				vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")],
				vec![(1, "0xB0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1C")],
			);

			let result = analyzer.analyze(&solver, &request);
			assert_eq!(result, Compatibility::Unknown);
		}

		#[test]
		fn test_no_asset_metadata_unknown() {
			let analyzer = CompatibilityAnalyzer;
			// Solver has no asset metadata - should return Unknown
			let solver = create_test_solver(
				"solver1",
				vec![], // No asset metadata
			);
			let request = create_test_request(
				vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")], // Input on network 1
				vec![(1, "0xB0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1C")], // Output on network 1
			);

			let result = analyzer.analyze(&solver, &request);
			assert_eq!(result, Compatibility::Unknown);
		}

		#[test]
		fn test_strict_cross_network_incompatibility() {
			let analyzer = CompatibilityAnalyzer;
			// Solver only has assets on networks 1,2 but request needs network 42
			let solver = create_test_solver(
				"solver1",
				vec![
					(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B"),
					(2, "0xSomeAssetOnNetwork2111111111111111111111"),
				],
			);
			let request = create_test_request(
				vec![
					(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B"), // Supported
					(42, "0xB0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1C"), // NOT supported (wrong network)
				],
				vec![],
			);

			let result = analyzer.analyze(&solver, &request);
			assert_eq!(result, Compatibility::Incompatible); // Strict: must support ALL assets
		}

		#[test]
		fn test_multi_network_usdc_base_to_ethereum() {
			let analyzer = CompatibilityAnalyzer;

			// Create a solver that supports both Base (8453) and Ethereum (1) with USDC on both
			let cross_chain_solver = create_test_solver(
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
			let ethereum_only_solver = create_test_solver(
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
			let incomplete_solver = create_test_solver(
				"incomplete_solver",
				vec![
					(8453, "0xDifferentTokenOnBase11111111111111111111"), // Different token on Base
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

			// Create a solver that supports the same address but only on network 1, not network 2
			let single_network_solver = create_test_solver(
				"single_network_solver",
				vec![
					(1, "0x1234567890123456789012345678901234567890"), // Same address on network 1
				],
			);

			// Request that needs the same address but on network 2 (not supported)
			let cross_network_request = create_test_request(
				vec![(2, "0x1234567890123456789012345678901234567890")], // Same address but network 2
				vec![],
			);

			let result = analyzer.analyze(&single_network_solver, &cross_network_request);
			assert_eq!(result, Compatibility::Incompatible); // Same address but wrong network

			// Now test the opposite: solver supports network 2, request needs network 1
			let network2_solver = create_test_solver(
				"network2_solver",
				vec![
					(2, "0x1234567890123456789012345678901234567890"), // Same address on network 2
				],
			);

			let network1_request = create_test_request(
				vec![(1, "0x1234567890123456789012345678901234567890")], // Same address but network 1
				vec![],
			);

			let result = analyzer.analyze(&network2_solver, &network1_request);
			assert_eq!(result, Compatibility::Incompatible); // Same address but wrong network
		}
	}

	mod solver_selector_tests {
		use super::*;

		#[test]
		fn test_select_all_strategy() {
			let selector = SolverSelector::new();
			let solvers = vec![
				(create_test_solver("s1", vec![]), Compatibility::Compatible),
				(create_test_solver("s2", vec![]), Compatibility::Compatible),
				(create_test_solver("s3", vec![]), Compatibility::Unknown),
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
					create_test_solver("s1", vec![]),
					Compatibility::Compatible, // score = 1.0
				),
				(
					create_test_solver("s2", vec![]),
					Compatibility::Unknown, // score = 0.0
				),
				(
					create_test_solver("s3", vec![]),
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
					create_test_solver("s1", vec![]),
					Compatibility::Compatible, // score = 1.0
				),
				(
					create_test_solver("s2", vec![]),
					Compatibility::Compatible, // score = 1.0
				),
				(
					create_test_solver("s3", vec![]),
					Compatibility::Compatible, // score = 1.0
				),
				(
					create_test_solver("s4", vec![]),
					Compatibility::Unknown, // score = 0.0
				),
				(
					create_test_solver("s5", vec![]),
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
				(create_test_solver("s1", vec![]), Compatibility::Compatible),
				(create_test_solver("s2", vec![]), Compatibility::Compatible),
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
				create_test_solver(
					"compatible",
					vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")],
				),
				create_test_solver(
					"incompatible",
					vec![(2, "0xB0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1C")],
				),
				create_test_solver("unknown", vec![]),
			];

			let request = create_test_request(
				vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")],
				vec![],
			);
			let options = SolverOptions::default();
			let config = default_config();

			let result = service
				.filter_solvers(&solvers, &request, &options, &config)
				.await;

			// Should include compatible and unknown (with default config)
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
				create_test_solver(
					"compatible",
					vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")],
				),
				create_test_solver("unknown", vec![]),
			];

			let request = create_test_request(
				vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")],
				vec![],
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
				create_test_solver("solver1", vec![]),
				create_test_solver("solver2", vec![]),
				create_test_solver("solver3", vec![]),
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
			let config = default_config();

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
				create_test_solver(
					"incompatible1",
					vec![(2, "0xDifferentAsset1111111111111111111111111")],
				),
				create_test_solver(
					"incompatible2",
					vec![(3, "0xDifferentAsset2222222222222222222222222")],
				),
			];

			let request = create_test_request(
				vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")], // Network 1 asset
				vec![],
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
				create_test_solver(
					"compatible",
					vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")],
				),
				create_test_solver(
					"incompatible",
					vec![(2, "0xB0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1C")],
				), // Different network
				create_test_solver("network_only", vec![]), // No asset metadata
			];

			let request = create_test_request(
				vec![(1, "0xA0b86a33E6842d3c5d5b8c5e8d6e77d6Cc9e7a1B")],
				vec![],
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
					create_test_solver("known", vec![]),
					Compatibility::Compatible,
				),
				(
					create_test_solver("unknown", vec![]),
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
