//! Solver filtering and selection service with async architecture and intelligent scoring
//!
//! This module provides intelligent solver filtering and selection based on network/asset
//! compatibility, confidence levels, and request requirements. It implements a sophisticated
//! multi-dimensional scoring system that distinguishes between explicitly incompatible,
//! unknown, and compatible solvers with configurable behavior and async support.
//!
//! ## Architecture
//!
//! The service is composed of three main components:
//! - **CompatibilityCalculator**: Scores solvers with detailed analysis and confidence levels
//! - **SelectionEngine**: Applies configurable selection strategies with advanced algorithms
//! - **SolverFilterService**: Orchestrates the async filtering pipeline with tunable configuration
//!
//! ## Key Features
//!
//! - **Async Architecture**: All operations are async, enabling future database/network integration
//! - **Detailed Scoring**: Multi-dimensional compatibility analysis with confidence levels (0.1-1.0)
//! - **Adaptive Sampling**: Sample size adjusts based on request complexity
//! - **Configurable Filtering**: Tunable minimum score thresholds and sampling strategies
//!
//! ## Three-Tier Scoring System
//!
//! | Score | Meaning | Solver State | Action |
//! |-------|---------|-------------|--------|
//! | **-1.0** | Explicitly incompatible | Has defined networks/assets but doesn't match | Filter out completely |
//! | **0.0** | Unknown compatibility | Empty/undefined network/asset lists | Very low probability (1%) |
//! | **0.5-1.0** | Partial to full compatibility | Matches some/all requirements | Normal weighted probability |
//!
//! ## Filtering Pipeline
//!
//! 1. **Compatibility Scoring**: Calculate scores for all solvers based on request requirements
//! 2. **Sorting**: Order solvers by compatibility (highest first)
//! 3. **Hard Filtering**: Remove explicitly incompatible solvers (score < 0)
//! 4. **Include/Exclude Filtering**: Apply user-specified include/exclude lists
//! 5. **Selection Strategy**: Apply the chosen selection strategy
//!
//! ## Selection Strategies
//!
//! ### All Strategy
//! Returns all compatible solvers in compatibility order.
//!
//! ### Sampled Strategy
//! Uses **score-aware weighted sampling** where:
//! - Higher compatibility scores get exponentially higher selection probability
//! - 0.0 score solvers get very low probability (1% of normal weight)
//! - Maintains diversity while strongly favoring quality
//!
//! ### Priority Strategy
//! Filters solvers by priority threshold (placeholder implementation).
//!
//! ## Compatibility Scoring Logic
//!
//! For each solver, the system:
//! 1. Extracts required networks and assets from the quote request
//! 2. Checks if solver has defined supported networks/assets
//! 3. Calculates compatibility based on matches:
//!    - **Full asset support**: +1.0 per asset
//!    - **Network support only**: +0.5 per asset
//!    - **No support**: +0.0
//! 4. Returns -1.0 if solver has definitions but supports nothing
//! 5. Returns 0.0 if solver has no definitions (unknown capability)
//! 6. Returns normalized score (0.5-1.0) for partial/full compatibility
//!
//! ## Benefits
//!
//! - **Prevents guaranteed failures**: Filters out explicitly incompatible solvers
//! - **Maximizes success rates**: Prioritizes known compatible solvers
//! - **Maintains diversity**: Gives small chance to unknown solvers (might be universal)
//! - **Resource efficiency**: Focuses calls on viable solvers
//! - **Quality bias**: 99% probability goes to compatible solvers, 1% to unknown
//!
//! ## Example
//!
//! For an ETH swap on Ethereum:
//! ```text
//! SolverA [Ethereum + ETH]     → Score: 1.0 → 60% selection probability
//! SolverB [Polygon + USDC]     → Score: -1.0 → Filtered out
//! SolverC [Empty lists]        → Score: 0.0 → 1% selection probability  
//! SolverD [Ethereum + USDC]    → Score: 0.5 → 35% selection probability
//! ```

use async_trait::async_trait;
use oif_types::constants::limits::{DEFAULT_PRIORITY_THRESHOLD, DEFAULT_SAMPLE_SIZE};
use oif_types::quotes::request::{SolverOptions, SolverSelection};
use oif_types::{QuoteRequest, Solver};
use std::collections::HashSet;
use tracing::{debug, info};

/// Enhanced compatibility score with detailed breakdown
#[derive(Debug, Clone)]
pub struct CompatibilityScore {
	pub network_score: f64,
	pub asset_score: f64,
	pub overall_score: f64,
	pub confidence_level: f64,
	pub supported_networks: Vec<u64>,
	pub supported_assets: Vec<String>,
}

/// Configuration for filtering behavior
#[derive(Debug, Clone)]
pub struct FilterConfig {
	pub min_solver_score: f64,
	pub adaptive_sample_size: bool,
	pub enhanced_sampling: bool,
}

impl Default for FilterConfig {
	fn default() -> Self {
		Self {
			min_solver_score: 0.0, // Keep unknown compatibility solvers
			adaptive_sample_size: true,
			enhanced_sampling: true,
		}
	}
}

/// Trait for solver filtering operations
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait SolverFilterTrait: Send + Sync {
	/// Filter and select solvers based on request requirements and solver options
	async fn filter_solvers(
		&self,
		available_solvers: &[Solver],
		request: &QuoteRequest,
		options: &SolverOptions,
	) -> Vec<Solver>;

	/// Get detailed compatibility analysis for a solver
	async fn analyze_solver_compatibility(
		&self,
		solver: &Solver,
		request: &QuoteRequest,
	) -> CompatibilityScore;
}

/// Enhanced helper for calculating solver compatibility with detailed analysis
pub struct CompatibilityCalculator {
	config: FilterConfig,
}

impl CompatibilityCalculator {
	pub fn new(config: FilterConfig) -> Self {
		Self { config }
	}

	/// Calculate detailed compatibility scores with confidence levels
	pub async fn analyze_compatibility(
		&self,
		solver: &Solver,
		request: &QuoteRequest,
	) -> CompatibilityScore {
		let required_networks = Self::extract_required_networks(request);
		let required_assets = Self::extract_required_assets(request);

		let network_analysis = self.analyze_network_compatibility(solver, &required_networks);
		let asset_analysis = self.analyze_asset_compatibility(solver, &required_assets);

		let overall_score = self.calculate_composite_compatibility(
			network_analysis.0,
			asset_analysis.0,
			&required_networks,
			&required_assets,
		);

		// Calculate confidence based on how much information we have about the solver
		let confidence_level =
			self.calculate_confidence_level(solver, &required_networks, &required_assets);

		CompatibilityScore {
			network_score: network_analysis.0,
			asset_score: asset_analysis.0,
			overall_score,
			confidence_level,
			supported_networks: network_analysis.1,
			supported_assets: asset_analysis.1,
		}
	}

	fn analyze_network_compatibility(
		&self,
		solver: &Solver,
		required_networks: &HashSet<u64>,
	) -> (f64, Vec<u64>) {
		if required_networks.is_empty() {
			return (
				1.0,
				solver
					.metadata
					.supported_networks
					.iter()
					.map(|n| n.chain_id)
					.collect(),
			);
		}

		if solver.metadata.supported_networks.is_empty() {
			return (0.0, Vec::new()); // Unknown compatibility
		}

		let supported_networks: HashSet<u64> = solver
			.metadata
			.supported_networks
			.iter()
			.map(|n| n.chain_id)
			.collect();
		let intersection: HashSet<u64> = required_networks
			.intersection(&supported_networks)
			.copied()
			.collect();

		if intersection.is_empty() {
			(
				-1.0,
				solver
					.metadata
					.supported_networks
					.iter()
					.map(|n| n.chain_id)
					.collect(),
			) // Explicitly incompatible
		} else {
			let score = intersection.len() as f64 / required_networks.len() as f64;
			(
				score,
				solver
					.metadata
					.supported_networks
					.iter()
					.map(|n| n.chain_id)
					.collect(),
			)
		}
	}

	fn analyze_asset_compatibility(
		&self,
		solver: &Solver,
		required_assets: &HashSet<(u64, String)>,
	) -> (f64, Vec<String>) {
		if required_assets.is_empty() {
			return (
				1.0,
				solver
					.metadata
					.supported_assets
					.iter()
					.map(|a| a.address.clone())
					.collect(),
			);
		}

		if solver.metadata.supported_assets.is_empty() {
			// Check if solver at least supports the networks for these assets
			let required_networks: HashSet<u64> = required_assets
				.iter()
				.map(|(chain_id, _)| *chain_id)
				.collect();
			let network_score = self
				.analyze_network_compatibility(solver, &required_networks)
				.0;
			return (network_score * 0.5, Vec::new()); // Partial score based on network support
		}

		let mut supported_count = 0;
		let mut partial_count = 0;

		for (chain_id, address) in required_assets {
			if solver.supports_asset_address(address) {
				supported_count += 1;
			} else if solver.supports_chain(*chain_id) {
				partial_count += 1;
			}
		}

		if supported_count == 0 && partial_count == 0 {
			(
				-1.0,
				solver
					.metadata
					.supported_assets
					.iter()
					.map(|a| a.address.clone())
					.collect(),
			) // Explicitly incompatible
		} else {
			// Calculate score with full credit for exact matches, half credit for network-only matches
			let score = (supported_count as f64 + partial_count as f64 * 0.5)
				/ required_assets.len() as f64;
			(
				score,
				solver
					.metadata
					.supported_assets
					.iter()
					.map(|a| a.address.clone())
					.collect(),
			)
		}
	}

	fn calculate_composite_compatibility(
		&self,
		network_score: f64,
		asset_score: f64,
		required_networks: &HashSet<u64>,
		required_assets: &HashSet<(u64, String)>,
	) -> f64 {
		// If either is explicitly incompatible, overall is incompatible
		if network_score < 0.0 || asset_score < 0.0 {
			return -1.0;
		}

		// Weight based on what's more important for this request
		let network_weight = if required_networks.is_empty() {
			0.0
		} else {
			0.6
		};
		let asset_weight = if required_assets.is_empty() { 0.0 } else { 0.4 };

		if network_weight == 0.0 && asset_weight == 0.0 {
			return 1.0; // No requirements
		}

		let total_weight = network_weight + asset_weight;
		(network_score * network_weight + asset_score * asset_weight) / total_weight
	}

	fn calculate_confidence_level(
		&self,
		solver: &Solver,
		_required_networks: &HashSet<u64>,
		_required_assets: &HashSet<(u64, String)>,
	) -> f64 {
		let has_network_info = !solver.metadata.supported_networks.is_empty();
		let has_asset_info = !solver.metadata.supported_assets.is_empty();

		match (has_network_info, has_asset_info) {
			(true, true) => 1.0,   // High confidence
			(true, false) => 0.7,  // Medium-high confidence
			(false, true) => 0.6,  // Medium confidence
			(false, false) => 0.1, // Very low confidence
		}
	}
	/// Enhanced compatibility scoring with detailed analysis
	pub async fn score_and_sort_solvers_with_details(
		&self,
		solvers: &[Solver],
		request: &QuoteRequest,
	) -> Vec<(Solver, f64, CompatibilityScore)> {
		let mut solver_scores = Vec::new();

		for solver in solvers {
			let compatibility = self.analyze_compatibility(solver, request).await;
			let weighted_score = compatibility.overall_score * compatibility.confidence_level;
			solver_scores.push((solver.clone(), weighted_score, compatibility));
		}

		// Sort by weighted score (highest first)
		solver_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

		// Filter out explicitly incompatible solvers
		solver_scores.retain(|(_, score, _)| *score >= self.config.min_solver_score);

		info!(
			"Enhanced compatibility scoring: {} viable solvers from {} total",
			solver_scores.len(),
			solvers.len()
		);

		solver_scores
	}

	/// Calculate compatibility scores and sort solvers by compatibility (highest first) - Legacy method
	pub fn score_and_sort_solvers(
		&self,
		solvers: &[Solver],
		request: &QuoteRequest,
	) -> Vec<Solver> {
		let required_networks = Self::extract_required_networks(request);
		let required_assets = Self::extract_required_assets(request);

		info!(
			"Request requires networks: {:?}, assets: {} unique",
			required_networks,
			required_assets.len()
		);

		// Calculate compatibility scores for all solvers
		let mut solver_compatibility: Vec<(Solver, f64)> = solvers
			.iter()
			.map(|solver| {
				let compatibility = Self::calculate_solver_compatibility(
					solver,
					&required_networks,
					&required_assets,
				);
				(solver.clone(), compatibility)
			})
			.collect();

		// Sort by compatibility score (highest first)
		solver_compatibility
			.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

		// Log compatibility information
		for (solver, score) in &solver_compatibility {
			if score > &0.0 {
				info!(
					"Solver {} compatibility: {:.2} (supports: {} networks, {} assets)",
					solver.solver_id,
					score,
					solver.metadata.supported_networks.len(),
					solver.metadata.supported_assets.len()
				);
			}
		}

		// Filter out explicitly incompatible solvers (score < 0) and return sorted by compatibility
		// Keep unknown compatibility solvers (score = 0.0) for potential use with very low probability
		let compatible_solvers: Vec<Solver> = solver_compatibility
			.into_iter()
			.filter_map(|(solver, score)| {
				if score >= 0.0 {
					Some(solver)
				} else {
					debug!(
						"Filtering out explicitly incompatible solver '{}' (score: {:.2})",
						solver.solver_id, score
					);
					None
				}
			})
			.collect();

		info!(
			"Compatibility filtering: {} usable solvers from {} total (filtered {} explicitly incompatible)",
			compatible_solvers.len(),
			solvers.len(),
			solvers.len() - compatible_solvers.len()
		);

		compatible_solvers
	}

	/// Score and sort solvers, returning both solvers and their scores for advanced processing
	pub fn score_and_sort_solvers_with_scores(
		&self,
		solvers: &[Solver],
		request: &QuoteRequest,
	) -> Vec<(Solver, f64)> {
		let required_networks = Self::extract_required_networks(request);
		let required_assets = Self::extract_required_assets(request);

		info!(
			"Request requires networks: {:?}, assets: {} unique",
			required_networks,
			required_assets.len()
		);

		// Calculate compatibility scores for all solvers
		let mut solver_compatibility: Vec<(Solver, f64)> = solvers
			.iter()
			.map(|solver| {
				let compatibility = Self::calculate_solver_compatibility(
					solver,
					&required_networks,
					&required_assets,
				);
				(solver.clone(), compatibility)
			})
			.collect();

		// Sort by compatibility score (highest first)
		solver_compatibility
			.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

		// Log compatibility information
		for (solver, score) in &solver_compatibility {
			if score >= &0.0 {
				let compatibility_type = if *score == 0.0 {
					"unknown"
				} else if *score < 0.0 {
					"incompatible"
				} else {
					"compatible"
				};
				debug!(
					"Solver {} compatibility: {:.2} ({}) - supports: {} networks, {} assets",
					solver.solver_id,
					score,
					compatibility_type,
					solver.metadata.supported_networks.len(),
					solver.metadata.supported_assets.len()
				);
			}
		}

		// Filter out explicitly incompatible solvers (score < 0)
		solver_compatibility
			.into_iter()
			.filter(|(solver, score)| {
				if *score >= 0.0 {
					true
				} else {
					debug!(
						"Filtering out explicitly incompatible solver '{}' (score: {:.2})",
						solver.solver_id, score
					);
					false
				}
			})
			.collect()
	}

	/// Extract required networks from QuoteRequest
	fn extract_required_networks(request: &QuoteRequest) -> HashSet<u64> {
		let mut networks = HashSet::new();

		// Extract networks from available inputs
		for input in &request.available_inputs {
			if let Ok(chain_id) = input.asset.extract_chain_id() {
				networks.insert(chain_id);
			}
		}

		// Extract networks from requested outputs
		for output in &request.requested_outputs {
			if let Ok(chain_id) = output.asset.extract_chain_id() {
				networks.insert(chain_id);
			}
		}

		networks
	}

	/// Extract required asset addresses from QuoteRequest
	fn extract_required_assets(request: &QuoteRequest) -> HashSet<(u64, String)> {
		let mut assets = HashSet::new();

		// Extract assets from available inputs (chain_id, address)
		for input in &request.available_inputs {
			if let Ok(chain_id) = input.asset.extract_chain_id() {
				let address = input.asset.extract_address();
				assets.insert((chain_id, address));
			}
		}

		// Extract assets from requested outputs
		for output in &request.requested_outputs {
			if let Ok(chain_id) = output.asset.extract_chain_id() {
				let address = output.asset.extract_address();
				assets.insert((chain_id, address));
			}
		}

		assets
	}

	/// Check if solver supports the required networks and assets from the request
	/// Returns:
	/// -1.0: Explicitly incompatible (has defined networks/assets but doesn't match)
	///  0.0: Unknown compatibility (empty/undefined network/asset lists)
	///  0.5-1.0: Partial to full compatibility
	fn calculate_solver_compatibility(
		solver: &Solver,
		required_networks: &HashSet<u64>,
		required_assets: &HashSet<(u64, String)>,
	) -> f64 {
		let total_requirements = required_networks.len() + required_assets.len();

		if total_requirements == 0 {
			return 1.0; // If no specific requirements, all solvers are equally compatible
		}

		// Check if solver has defined networks/assets or if they're empty
		let has_defined_networks = !solver.metadata.supported_networks.is_empty();
		let has_defined_assets = !solver.metadata.supported_assets.is_empty();

		let mut score = 0.0;
		let mut explicit_incompatibilities = 0;

		// Check network compatibility
		for &network_id in required_networks {
			if has_defined_networks {
				if solver.supports_chain(network_id) {
					score += 1.0; // Explicit support
				} else {
					explicit_incompatibilities += 1; // Explicit non-support
				}
			}
			// If no defined networks, we don't add to score (unknown = neutral)
		}

		// Check asset compatibility
		for (chain_id, address) in required_assets {
			if has_defined_assets {
				// Check if solver supports this specific asset address
				if solver.supports_asset_address(address) {
					score += 1.0; // Full asset support
				} else if solver.supports_chain(*chain_id) {
					// Partial score if solver supports the network but not the specific asset
					score += 0.5; // Partial support
				} else {
					explicit_incompatibilities += 1; // Explicit non-support
				}
			} else if has_defined_networks && solver.supports_chain(*chain_id) {
				// No asset list but network is supported - partial compatibility
				score += 0.5;
			}
			// If no defined assets or networks, we don't add to score (unknown = neutral)
		}

		// If solver has defined lists but supports none of the requirements -> explicitly incompatible
		if (has_defined_networks || has_defined_assets)
			&& explicit_incompatibilities > 0
			&& score == 0.0
		{
			return -1.0; // Explicitly incompatible
		}

		// If solver has no defined networks or assets -> unknown compatibility
		if !has_defined_networks && !has_defined_assets {
			return 0.0; // Unknown compatibility (might work)
		}

		// Normalize score for partial/full compatibility
		score / total_requirements as f64
	}
}

/// Enhanced selection engine with advanced algorithms
pub struct SelectionEngine {
	config: FilterConfig,
}

impl SelectionEngine {
	pub fn new(config: FilterConfig) -> Self {
		Self { config }
	}
	/// Selection strategy with detailed compatibility scores
	pub async fn apply_strategy(
		&self,
		solver_scores: Vec<(Solver, f64, CompatibilityScore)>,
		options: &SolverOptions,
		request: &QuoteRequest,
	) -> Vec<Solver> {
		match options
			.solver_selection
			.as_ref()
			.unwrap_or(&SolverSelection::All)
		{
			SolverSelection::All => {
				info!("Using all {} enhanced-scored solvers", solver_scores.len());
				solver_scores
					.into_iter()
					.map(|(solver, _, _)| solver)
					.collect()
			},
			SolverSelection::Sampled => {
				let mut sample_size = options.sample_size.unwrap_or(DEFAULT_SAMPLE_SIZE) as usize;

				// Adaptive sample size based on request complexity
				if self.config.adaptive_sample_size {
					let complexity =
						request.available_inputs.len() + request.requested_outputs.len();
					sample_size = if complexity > 4 {
						sample_size + 2 // More complex requests get more solvers
					} else {
						sample_size
					};
				}

				if solver_scores.len() > sample_size {
					let solver_count = solver_scores.len();
					let selected = if self.config.enhanced_sampling {
						self.enhanced_weighted_sample(solver_scores, sample_size)
							.await
					} else {
						self.simple_sample(solver_scores, sample_size)
					};
					info!(
						"Enhanced sampling: selected {} solvers from {} available (adaptive size: {})",
						selected.len(), solver_count, sample_size
					);
					selected
				} else {
					solver_scores
						.into_iter()
						.map(|(solver, _, _)| solver)
						.collect()
				}
			},
			SolverSelection::Priority => {
				let threshold = options
					.priority_threshold
					.unwrap_or(DEFAULT_PRIORITY_THRESHOLD) as f64
					/ 100.0;
				let filtered: Vec<Solver> = solver_scores
					.into_iter()
					.filter_map(|(solver, score, _)| {
						if score >= threshold {
							Some(solver)
						} else {
							None
						}
					})
					.collect();

				info!(
					"Priority filtering: {} solvers meet threshold {:.1}%",
					filtered.len(),
					threshold * 100.0
				);
				filtered
			},
		}
	}

	/// Enhanced weighted sampling with provider diversity consideration
	async fn enhanced_weighted_sample(
		&self,
		solver_scores: Vec<(Solver, f64, CompatibilityScore)>,
		sample_size: usize,
	) -> Vec<Solver> {
		use rand::Rng;

		if solver_scores.len() <= sample_size {
			return solver_scores
				.into_iter()
				.map(|(solver, _, _)| solver)
				.collect();
		}

		let mut rng = rand::thread_rng();
		let mut selected = Vec::with_capacity(sample_size);
		let mut remaining = solver_scores;
		let mut selected_providers = HashSet::new();

		// Create enhanced weights considering diversity and confidence
		let create_enhanced_weights = |solvers: &[(Solver, f64, CompatibilityScore)]| -> Vec<f64> {
			solvers
				.iter()
				.enumerate()
				.map(|(i, (_solver, score, compat))| {
					let mut weight = if *score <= 0.0 {
						0.0 // Incompatible solvers get zero weight
					} else if *score < 0.3 {
						0.01 // Very low compatibility
					} else if *score < 0.7 {
						0.5 // Moderate compatibility
					} else {
						1.0 // High compatibility
					};

					// Apply exponential decay for position (later = lower weight)
					weight *= (-0.3 * i as f64 / solvers.len() as f64).exp();

					// Confidence bonus
					weight *= 0.5 + (compat.confidence_level * 0.5);

					weight
				})
				.collect()
		};

		// Selection loop with provider diversity tracking
		for _ in 0..sample_size {
			if remaining.is_empty() {
				break;
			}

			let weights = create_enhanced_weights(&remaining);
			let total_weight: f64 = weights.iter().sum();

			if total_weight == 0.0 {
				break; // No more viable solvers
			}

			// Select based on weights
			let mut random_weight = rng.gen::<f64>() * total_weight;
			let mut selected_index = 0;

			for (i, &weight) in weights.iter().enumerate() {
				random_weight -= weight;
				if random_weight <= 0.0 {
					selected_index = i;
					break;
				}
			}

			let (solver, score, compat) = remaining.remove(selected_index);

			// Track selected solvers
			selected_providers.insert(solver.solver_id.clone());

			debug!(
				"Enhanced selected '{}': score={:.3}, compat={:.3}, confidence={:.2}",
				solver.solver_id, score, compat.overall_score, compat.confidence_level
			);

			selected.push(solver);
		}

		info!(
			"Sampling completed: {} solvers selected",
			selected_providers.len()
		);

		selected
	}

	/// Simple sampling fallback
	fn simple_sample(
		&self,
		solver_scores: Vec<(Solver, f64, CompatibilityScore)>,
		sample_size: usize,
	) -> Vec<Solver> {
		solver_scores
			.into_iter()
			.take(sample_size)
			.map(|(solver, _, _)| solver)
			.collect()
	}

	/// Apply the specified selection strategy to the filtered solvers - Legacy method
	pub fn apply_strategy_legacy(
		&self,
		mut solvers: Vec<Solver>,
		options: &SolverOptions,
	) -> Vec<Solver> {
		match options
			.solver_selection
			.as_ref()
			.unwrap_or(&SolverSelection::All)
		{
			SolverSelection::All => {
				// Use all available solvers after filtering
				debug!("Using all {} available solvers", solvers.len());
				solvers
			},
			SolverSelection::Sampled => {
				// Sample a subset of solvers using weighted selection based on compatibility
				let sample_size = options.sample_size.unwrap_or(DEFAULT_SAMPLE_SIZE) as usize;
				if solvers.len() > sample_size {
					// Use weighted sampling: higher compatibility = higher selection probability
					// This utilizes the pre-sorted order from compatibility scoring
					let total_available = solvers.len();
					let selected = Self::weighted_sample_solvers(solvers, sample_size);
					info!(
						"Weighted-sampled {} solvers from {} available (prioritizing compatibility)",
						selected.len(),
						total_available
					);
					selected
				} else {
					solvers
				}
			},
			SolverSelection::Priority => {
				// Filter solvers by priority threshold
				let threshold = options
					.priority_threshold
					.unwrap_or(DEFAULT_PRIORITY_THRESHOLD);
				// For now, we'll use a placeholder priority logic
				// In a real implementation, you'd have solver scores/priorities stored
				solvers.retain(|_solver| {
					// Placeholder: all solvers meet threshold for now
					// In practice, you'd check: solver.priority_score >= threshold
					true
				});

				if threshold > 0 {
					info!(
						"Filtered to solvers with priority >= {}, {} remaining",
						threshold,
						solvers.len()
					);
				}
				solvers
			},
		}
	}

	/// Perform weighted sampling of solvers based on their position in the sorted list
	/// Solvers are assumed to be pre-sorted by compatibility score (highest first)
	/// Earlier positions (higher compatibility) get higher selection probability
	fn weighted_sample_solvers(solvers: Vec<Solver>, sample_size: usize) -> Vec<Solver> {
		use rand::Rng;

		if solvers.len() <= sample_size {
			return solvers;
		}

		let mut rng = rand::thread_rng();
		let mut selected = Vec::with_capacity(sample_size);
		let mut remaining = solvers;

		// Create weights: higher index (better compatibility) = higher weight
		// Use exponential decay: early solvers get much higher probability
		let create_weights = |len: usize| -> Vec<f64> {
			(0..len)
				.map(|i| {
					// Higher weight for earlier positions (better compatibility)
					// Weight decreases exponentially: first solver gets weight ~2.7, last gets weight ~0.37
					// This gives strong preference to compatible solvers while maintaining randomness
					(-0.5 * i as f64 / len as f64).exp()
				})
				.collect()
		};

		// Select solvers one by one using weighted selection
		for _ in 0..sample_size {
			if remaining.is_empty() {
				break;
			}

			let weights = create_weights(remaining.len());

			// Select index based on weights
			let selected_index = {
				let total_weight: f64 = weights.iter().sum();
				let mut random_weight = rng.gen::<f64>() * total_weight;

				let mut selected_idx = 0;
				for (i, &weight) in weights.iter().enumerate() {
					random_weight -= weight;
					if random_weight <= 0.0 {
						selected_idx = i;
						break;
					}
				}
				selected_idx.min(remaining.len() - 1)
			};

			// Move selected solver to result
			selected.push(remaining.remove(selected_index));
		}

		debug!(
			"Weighted sampling: selected {} solvers with exponential compatibility preference",
			selected.len()
		);

		selected
	}

	/// Apply the specified selection strategy to the filtered solvers with scores
	/// This version is score-aware and can give very low probability to 0.0 score solvers
	pub fn apply_strategy_with_scores(
		&self,
		solver_scores: Vec<(Solver, f64)>,
		options: &SolverOptions,
	) -> Vec<Solver> {
		match options
			.solver_selection
			.as_ref()
			.unwrap_or(&SolverSelection::All)
		{
			SolverSelection::All => {
				// Use all available solvers after filtering
				debug!("Using all {} available solvers", solver_scores.len());
				solver_scores
					.into_iter()
					.map(|(solver, _)| solver)
					.collect()
			},
			SolverSelection::Sampled => {
				// Sample a subset of solvers using score-aware weighted selection
				let sample_size = options.sample_size.unwrap_or(DEFAULT_SAMPLE_SIZE) as usize;
				if solver_scores.len() > sample_size {
					let total_available = solver_scores.len();
					let selected = Self::score_aware_weighted_sample(solver_scores, sample_size);
					info!(
						"Score-aware weighted-sampled {} solvers from {} available (prioritizing compatibility)",
						selected.len(),
						total_available
					);
					selected
				} else {
					solver_scores
						.into_iter()
						.map(|(solver, _)| solver)
						.collect()
				}
			},
			SolverSelection::Priority => {
				// Filter solvers by priority threshold
				let threshold = options
					.priority_threshold
					.unwrap_or(DEFAULT_PRIORITY_THRESHOLD);
				let filtered_solvers: Vec<Solver> = solver_scores
					.into_iter()
					.filter_map(|(solver, _score)| {
						// For now, we'll use a placeholder priority logic
						// In a real implementation, you'd have solver scores/priorities stored
						// Placeholder: all solvers meet threshold for now
						// In practice, you'd check: solver.priority_score >= threshold
						Some(solver)
					})
					.collect();

				if threshold > 0 {
					info!(
						"Filtered to solvers with priority >= {}, {} remaining",
						threshold,
						filtered_solvers.len()
					);
				}
				filtered_solvers
			},
		}
	}

	/// Perform score-aware weighted sampling of solvers
	/// Gives very low probability to 0.0 score solvers while favoring higher scores
	fn score_aware_weighted_sample(
		solver_scores: Vec<(Solver, f64)>,
		sample_size: usize,
	) -> Vec<Solver> {
		use rand::Rng;

		if solver_scores.len() <= sample_size {
			return solver_scores
				.into_iter()
				.map(|(solver, _)| solver)
				.collect();
		}

		let mut rng = rand::thread_rng();
		let mut selected = Vec::with_capacity(sample_size);
		let mut remaining = solver_scores;

		// Create score-aware weights
		let create_weights = |solver_scores: &[(Solver, f64)]| -> Vec<f64> {
			solver_scores
				.iter()
				.enumerate()
				.map(|(i, (_solver, score))| {
					if *score == 0.0 {
						// Very low probability for unknown compatibility (1% of normal weight)
						0.01
					} else {
						// Normal exponential decay based on position (higher compatibility first)
						(-0.5 * i as f64 / solver_scores.len() as f64).exp()
					}
				})
				.collect()
		};

		// Select solvers one by one using score-aware weighted selection
		for _ in 0..sample_size {
			if remaining.is_empty() {
				break;
			}

			let weights = create_weights(&remaining);

			// Select index based on weights
			let selected_index = {
				let total_weight: f64 = weights.iter().sum();
				let mut random_weight = rng.gen::<f64>() * total_weight;

				let mut selected_idx = 0;
				for (i, &weight) in weights.iter().enumerate() {
					random_weight -= weight;
					if random_weight <= 0.0 {
						selected_idx = i;
						break;
					}
				}
				selected_idx.min(remaining.len() - 1)
			};

			// Move selected solver to result
			let (solver, score) = remaining.remove(selected_index);
			debug!(
				"Selected solver '{}' with score {:.2} for weighted sampling",
				solver.solver_id, score
			);
			selected.push(solver);
		}

		debug!(
			"Score-aware sampling: selected {} solvers with strong bias toward compatibility",
			selected.len()
		);

		selected
	}
}

/// Enhanced main service for filtering and selecting solvers
pub struct SolverFilterService {
	compatibility_calculator: CompatibilityCalculator,
	selection_engine: SelectionEngine,
	config: FilterConfig,
}

impl SolverFilterService {
	/// Create a new enhanced solver filter service
	pub fn new() -> Self {
		Self::with_config(FilterConfig::default())
	}

	/// Create a new enhanced solver filter service with custom configuration
	pub fn with_config(config: FilterConfig) -> Self {
		let compatibility_calculator = CompatibilityCalculator::new(config.clone());
		let selection_engine = SelectionEngine::new(config.clone());

		Self {
			compatibility_calculator,
			selection_engine,
			config,
		}
	}
}

#[async_trait]
impl SolverFilterTrait for SolverFilterService {
	/// Filter and select solvers based on request requirements and solver options
	async fn filter_solvers(
		&self,
		available_solvers: &[Solver],
		request: &QuoteRequest,
		options: &SolverOptions,
	) -> Vec<Solver> {
		debug!(
			"Starting enhanced solver filtering with {} available solvers",
			available_solvers.len()
		);

		// Step 1: Enhanced compatibility scoring with detailed analysis
		let mut solver_scores = self
			.compatibility_calculator
			.score_and_sort_solvers_with_details(available_solvers, request)
			.await;

		// Step 2: Apply include/exclude filters
		solver_scores = self.apply_include_filter_enhanced(solver_scores, options);
		solver_scores = self.apply_exclude_filter_enhanced(solver_scores, options);

		// Step 3: Apply selection strategy
		let selected_solvers = self
			.selection_engine
			.apply_strategy(solver_scores, options, request)
			.await;

		info!(
			"Enhanced filtering completed: {} solvers selected from {} available",
			selected_solvers.len(),
			available_solvers.len()
		);

		selected_solvers
	}

	/// Get detailed compatibility analysis for a solver
	async fn analyze_solver_compatibility(
		&self,
		solver: &Solver,
		request: &QuoteRequest,
	) -> CompatibilityScore {
		self.compatibility_calculator
			.analyze_compatibility(solver, request)
			.await
	}
}

impl SolverFilterService {
	/// Apply inclusion filter with enhanced scores
	fn apply_include_filter_enhanced(
		&self,
		mut solver_scores: Vec<(Solver, f64, CompatibilityScore)>,
		options: &SolverOptions,
	) -> Vec<(Solver, f64, CompatibilityScore)> {
		if let Some(include_list) = &options.include_solvers {
			if !include_list.is_empty() {
				solver_scores.retain(|(solver, _, _)| include_list.contains(&solver.solver_id));
				info!(
					"Filtered to {} included solvers with enhanced scores: {:?}",
					solver_scores.len(),
					include_list
				);
			}
		}
		solver_scores
	}

	/// Apply exclusion filter with enhanced scores
	fn apply_exclude_filter_enhanced(
		&self,
		mut solver_scores: Vec<(Solver, f64, CompatibilityScore)>,
		options: &SolverOptions,
	) -> Vec<(Solver, f64, CompatibilityScore)> {
		if let Some(exclude_list) = &options.exclude_solvers {
			if !exclude_list.is_empty() {
				solver_scores.retain(|(solver, _, _)| !exclude_list.contains(&solver.solver_id));
				info!(
					"Excluded {} solvers with enhanced scores: {:?}",
					exclude_list.len(),
					exclude_list
				);
			}
		}
		solver_scores
	}

	/// Get current configuration
	pub fn get_config(&self) -> &FilterConfig {
		&self.config
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
	use oif_types::{AvailableInput, InteropAddress, RequestedOutput, U256};

	// Helper function to create a valid quote request for testing
	fn create_test_quote_request() -> QuoteRequest {
		let user = InteropAddress::from_text("eip155:1:0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266")
			.unwrap();
		let asset =
			InteropAddress::from_text("eip155:1:0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0")
				.unwrap();

		QuoteRequest {
			user: user.clone(),
			available_inputs: vec![AvailableInput {
				user: user.clone(),
				asset: asset.clone(),
				amount: U256::from(1000u64),
				lock: None,
			}],
			requested_outputs: vec![RequestedOutput {
				receiver: user,
				asset,
				amount: U256::from(500u64),
				calldata: None,
			}],
			min_valid_until: None,
			preference: None,
			solver_options: None,
		}
	}

	// Helper function to create test solvers with basic network/asset support
	fn create_test_solvers(count: usize) -> Vec<Solver> {
		use oif_types::models::{Asset, Network};

		// Create basic test network and asset that matches our test request
		let test_network = Network {
			chain_id: 1,
			name: "Ethereum".to_string(),
			is_testnet: false,
		};
		let test_asset = Asset {
			name: "Test Token".to_string(),
			symbol: "TEST".to_string(),
			address: "0xA0b86a33E6417a77C9A0C65f8E69b8b6e2b0c4A0".to_string(),
			decimals: 18,
			chain_id: 1,
		};

		let mut solvers = Vec::new();
		for i in 1..=count {
			let mut solver = Solver::new(
				format!("solver{}", i),
				format!("adapter{}", i),
				format!("http://localhost:800{}", i),
				5000,
			);
			// Add basic network and asset support so solvers aren't filtered out
			solver.metadata.supported_networks = vec![test_network.clone()];
			solver.metadata.supported_assets = vec![test_asset.clone()];
			solvers.push(solver);
		}
		solvers
	}

	#[tokio::test]
	async fn test_mock_solver_filter_trait() {
		// Test that the trait can be implemented properly
		// (mocking async traits is complex, so we test the real service)
		let service = SolverFilterService::new();
		let solvers = create_test_solvers(5);
		let request = create_test_quote_request();
		let options = SolverOptions {
			include_solvers: Some(vec!["solver1".to_string(), "solver3".to_string()]),
			exclude_solvers: None,
			timeout: None,
			solver_timeout: None,
			min_quotes: None,
			solver_selection: Some(SolverSelection::All),
			sample_size: None,
			priority_threshold: None,
		};

		let result = service.filter_solvers(&solvers, &request, &options).await;
		assert_eq!(result.len(), 2);
	}

	#[test]
	fn test_solver_filter_service_creation() {
		let _service = SolverFilterService::new();
		// Just verify it can be created without panicking
		assert!(true);
	}

	#[tokio::test]
	async fn test_filter_solvers_no_filters() {
		let service = SolverFilterService::new();
		let solvers = create_test_solvers(5);
		let request = create_test_quote_request();
		let options = SolverOptions {
			include_solvers: None,
			exclude_solvers: None,
			timeout: None,
			solver_timeout: None,
			min_quotes: None,
			solver_selection: Some(SolverSelection::All),
			sample_size: None,
			priority_threshold: None,
		};

		let filtered = service.filter_solvers(&solvers, &request, &options).await;
		assert_eq!(filtered.len(), 5); // All solvers should be returned
	}

	#[tokio::test]
	async fn test_filter_solvers_include_specific() {
		let service = SolverFilterService::new();
		let solvers = create_test_solvers(5);
		let request = create_test_quote_request();
		let options = SolverOptions {
			include_solvers: Some(vec!["solver1".to_string(), "solver3".to_string()]),
			exclude_solvers: None,
			timeout: None,
			solver_timeout: None,
			min_quotes: None,
			solver_selection: Some(SolverSelection::All),
			sample_size: None,
			priority_threshold: None,
		};

		let filtered = service.filter_solvers(&solvers, &request, &options).await;
		assert_eq!(filtered.len(), 2);

		let solver_ids: Vec<String> = filtered.iter().map(|s| s.solver_id.clone()).collect();
		assert!(solver_ids.contains(&"solver1".to_string()));
		assert!(solver_ids.contains(&"solver3".to_string()));
	}

	#[tokio::test]
	async fn test_filter_solvers_exclude_specific() {
		let service = SolverFilterService::new();
		let solvers = create_test_solvers(5);
		let request = create_test_quote_request();
		let options = SolverOptions {
			include_solvers: None,
			exclude_solvers: Some(vec!["solver2".to_string(), "solver4".to_string()]),
			timeout: None,
			solver_timeout: None,
			min_quotes: None,
			solver_selection: Some(SolverSelection::All),
			sample_size: None,
			priority_threshold: None,
		};

		let filtered = service.filter_solvers(&solvers, &request, &options).await;
		assert_eq!(filtered.len(), 3);

		let solver_ids: Vec<String> = filtered.iter().map(|s| s.solver_id.clone()).collect();
		assert!(solver_ids.contains(&"solver1".to_string()));
		assert!(solver_ids.contains(&"solver3".to_string()));
		assert!(solver_ids.contains(&"solver5".to_string()));
		assert!(!solver_ids.contains(&"solver2".to_string()));
		assert!(!solver_ids.contains(&"solver4".to_string()));
	}

	#[tokio::test]
	async fn test_filter_solvers_sampled_selection() {
		let service = SolverFilterService::new();
		let solvers = create_test_solvers(10);
		let request = create_test_quote_request();
		let options = SolverOptions {
			include_solvers: None,
			exclude_solvers: None,
			timeout: None,
			solver_timeout: None,
			min_quotes: None,
			solver_selection: Some(SolverSelection::Sampled),
			sample_size: Some(3),
			priority_threshold: None,
		};

		let filtered = service.filter_solvers(&solvers, &request, &options).await;
		assert_eq!(filtered.len(), 3); // Should be limited to sample size
	}

	#[test]
	fn test_compatibility_calculator_extract_networks() {
		let request = create_test_quote_request();
		let networks = CompatibilityCalculator::extract_required_networks(&request);

		// The test request uses chain ID 1 (Ethereum)
		assert!(networks.contains(&1));
		assert_eq!(networks.len(), 1);
	}

	#[test]
	fn test_compatibility_calculator_extract_assets() {
		let request = create_test_quote_request();
		let assets = CompatibilityCalculator::extract_required_assets(&request);

		// Should have 1 unique asset (input and output use the same asset in test)
		assert_eq!(assets.len(), 1);

		// All assets should be on chain ID 1
		for (chain_id, _) in &assets {
			assert_eq!(*chain_id, 1);
		}
	}

	#[test]
	fn test_selection_engine_all_strategy() {
		let engine = SelectionEngine::new(FilterConfig::default());
		let solvers = create_test_solvers(5);
		let options = SolverOptions {
			include_solvers: None,
			exclude_solvers: None,
			timeout: None,
			solver_timeout: None,
			min_quotes: None,
			solver_selection: Some(SolverSelection::All),
			sample_size: None,
			priority_threshold: None,
		};

		let result = engine.apply_strategy_legacy(solvers, &options);
		assert_eq!(result.len(), 5); // All solvers should be returned
	}

	#[test]
	fn test_selection_engine_sampled_strategy() {
		let engine = SelectionEngine::new(FilterConfig::default());
		let solvers = create_test_solvers(10);
		let options = SolverOptions {
			include_solvers: None,
			exclude_solvers: None,
			timeout: None,
			solver_timeout: None,
			min_quotes: None,
			solver_selection: Some(SolverSelection::Sampled),
			sample_size: Some(3),
			priority_threshold: None,
		};

		let result = engine.apply_strategy_legacy(solvers, &options);
		assert_eq!(result.len(), 3); // Should be limited to sample size
	}

	#[test]
	fn test_selection_engine_priority_strategy() {
		let engine = SelectionEngine::new(FilterConfig::default());
		let solvers = create_test_solvers(5);
		let options = SolverOptions {
			include_solvers: None,
			exclude_solvers: None,
			timeout: None,
			solver_timeout: None,
			min_quotes: None,
			solver_selection: Some(SolverSelection::Priority),
			sample_size: None,
			priority_threshold: Some(50),
		};

		let result = engine.apply_strategy_legacy(solvers, &options);
		// Currently all solvers pass priority filter (placeholder implementation)
		assert_eq!(result.len(), 5);
	}

	#[test]
	fn test_weighted_sample_solvers_biases_toward_early_positions() {
		// Test that weighted sampling favors solvers in early positions (higher compatibility)
		let solvers = create_test_solvers(10);
		let sample_size = 3;

		// Run sampling many times to test the bias
		let mut selection_counts = std::collections::HashMap::new();
		let iterations = 1000;

		for _ in 0..iterations {
			let selected = SelectionEngine::weighted_sample_solvers(solvers.clone(), sample_size);
			assert_eq!(selected.len(), sample_size);

			for solver in selected {
				*selection_counts.entry(solver.solver_id).or_insert(0) += 1;
			}
		}

		// Verify bias: earlier solvers (solver1, solver2) should be selected more often
		// than later solvers (solver9, solver10)
		let early_selections = selection_counts.get("solver1").unwrap_or(&0)
			+ selection_counts.get("solver2").unwrap_or(&0);
		let late_selections = selection_counts.get("solver9").unwrap_or(&0)
			+ selection_counts.get("solver10").unwrap_or(&0);

		assert!(
			early_selections > late_selections,
			"Early solvers should be selected more often than late solvers. Early: {}, Late: {}",
			early_selections,
			late_selections
		);

		// solver1 should be selected most frequently due to highest weight
		let solver1_count = *selection_counts.get("solver1").unwrap_or(&0);
		let solver10_count = *selection_counts.get("solver10").unwrap_or(&0);

		assert!(
			solver1_count > solver10_count,
			"solver1 (first/best) should be selected more than solver10 (last/worst). solver1: {}, solver10: {}",
			solver1_count, solver10_count
		);
	}

	#[test]
	fn test_weighted_sample_solvers_handles_edge_cases() {
		// Test with sample size larger than available solvers
		let solvers = create_test_solvers(3);
		let result = SelectionEngine::weighted_sample_solvers(solvers.clone(), 10);
		assert_eq!(result.len(), 3); // Should return all available

		// Test with empty solvers
		let empty_solvers = vec![];
		let result = SelectionEngine::weighted_sample_solvers(empty_solvers, 5);
		assert_eq!(result.len(), 0);

		// Test with sample size 0
		let result = SelectionEngine::weighted_sample_solvers(solvers, 0);
		assert_eq!(result.len(), 0);
	}

	#[test]
	fn test_score_and_sort_filters_incompatible_solvers() {
		use oif_types::models::{Asset, Network};
		use oif_types::{AvailableInput, RequestedOutput, U256};

		// Create solvers with different network/asset support
		let ethereum_network = Network {
			chain_id: 1,
			name: "Ethereum".to_string(),
			is_testnet: false,
		};
		let polygon_network = Network {
			chain_id: 137,
			name: "Polygon".to_string(),
			is_testnet: false,
		};
		let eth_asset = Asset {
			name: "Ethereum".to_string(),
			symbol: "ETH".to_string(),
			address: "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2".to_string(),
			decimals: 18,
			chain_id: 1,
		};
		let usdc_asset = Asset {
			name: "USD Coin".to_string(),
			symbol: "USDC".to_string(),
			address: "0xa0b86a33e6417a77c9a0c65f8e69b8b6e2b0c4a0".to_string(),
			decimals: 6,
			chain_id: 137,
		};

		// Solver 1: Supports Ethereum + ETH (should match)
		let mut solver1 = Solver::new(
			"solver1".to_string(),
			"adapter1".to_string(),
			"http://localhost:8001".to_string(),
			5000,
		);
		solver1.metadata.supported_networks = vec![ethereum_network.clone()];
		solver1.metadata.supported_assets = vec![eth_asset.clone()];

		// Solver 2: Supports only Polygon (should be filtered out)
		let mut solver2 = Solver::new(
			"solver2".to_string(),
			"adapter2".to_string(),
			"http://localhost:8002".to_string(),
			5000,
		);
		solver2.metadata.supported_networks = vec![polygon_network.clone()];
		solver2.metadata.supported_assets = vec![usdc_asset.clone()];

		// Solver 3: Supports Ethereum but not ETH (should get partial score)
		let mut solver3 = Solver::new(
			"solver3".to_string(),
			"adapter3".to_string(),
			"http://localhost:8003".to_string(),
			5000,
		);
		solver3.metadata.supported_networks = vec![ethereum_network.clone()];
		solver3.metadata.supported_assets = vec![usdc_asset.clone()];

		let solvers = vec![solver1, solver2, solver3];

		// Create a request for ETH on Ethereum
		let user = InteropAddress::from_text("eip155:1:0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266")
			.unwrap();
		let eth_address =
			InteropAddress::from_text("eip155:1:0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2")
				.unwrap();

		let request = QuoteRequest {
			user: user.clone(),
			available_inputs: vec![AvailableInput {
				user: user.clone(),
				asset: eth_address.clone(),
				amount: U256::from(1000u64),
				lock: None,
			}],
			requested_outputs: vec![RequestedOutput {
				receiver: user,
				asset: eth_address,
				amount: U256::from(900u64),
				calldata: None,
			}],
			min_valid_until: None,
			preference: None,
			solver_options: None,
		};

		let calculator = CompatibilityCalculator::new(FilterConfig::default());
		let result = calculator.score_and_sort_solvers(&solvers, &request);

		// Should only return compatible solvers (score > 0.0)
		// solver1: full compatibility (1.0), solver3: partial (0.5), solver2: none (0.0 - filtered out)
		assert_eq!(result.len(), 2, "Should filter out incompatible solvers");

		// Verify the returned solvers are the right ones
		let solver_ids: Vec<String> = result.iter().map(|s| s.solver_id.clone()).collect();
		assert!(
			solver_ids.contains(&"solver1".to_string()),
			"solver1 should be included (full compatibility)"
		);
		assert!(
			solver_ids.contains(&"solver3".to_string()),
			"solver3 should be included (partial compatibility)"
		);
		assert!(
			!solver_ids.contains(&"solver2".to_string()),
			"solver2 should be filtered out (no compatibility)"
		);

		// Verify ordering (highest compatibility first)
		assert_eq!(
			result[0].solver_id, "solver1",
			"solver1 should be first (highest compatibility)"
		);
		assert_eq!(
			result[1].solver_id, "solver3",
			"solver3 should be second (partial compatibility)"
		);
	}

	#[tokio::test]
	async fn test_enhanced_scoring_system_with_explicit_vs_unknown_compatibility() {
		use oif_types::models::{Asset, Network};
		use oif_types::{AvailableInput, RequestedOutput, U256};

		let ethereum_network = Network {
			chain_id: 1,
			name: "Ethereum".to_string(),
			is_testnet: false,
		};
		let polygon_network = Network {
			chain_id: 137,
			name: "Polygon".to_string(),
			is_testnet: false,
		};
		let eth_asset = Asset {
			name: "Ethereum".to_string(),
			symbol: "ETH".to_string(),
			address: "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2".to_string(),
			decimals: 18,
			chain_id: 1,
		};

		// Solver 1: Explicitly supports Ethereum + ETH (should get 1.0)
		let mut solver1 = Solver::new(
			"explicit_compatible".to_string(),
			"adapter1".to_string(),
			"http://localhost:8001".to_string(),
			5000,
		);
		solver1.metadata.supported_networks = vec![ethereum_network.clone()];
		solver1.metadata.supported_assets = vec![eth_asset.clone()];

		// Solver 2: Explicitly supports only Polygon (should get -1.0 and be filtered out)
		let mut solver2 = Solver::new(
			"explicit_incompatible".to_string(),
			"adapter2".to_string(),
			"http://localhost:8002".to_string(),
			5000,
		);
		solver2.metadata.supported_networks = vec![polygon_network.clone()];
		solver2.metadata.supported_assets = vec![];

		// Solver 3: No defined networks or assets (should get 0.0 - unknown compatibility)
		let mut solver3 = Solver::new(
			"unknown_compatibility".to_string(),
			"adapter3".to_string(),
			"http://localhost:8003".to_string(),
			5000,
		);
		solver3.metadata.supported_networks = vec![];
		solver3.metadata.supported_assets = vec![];

		let solvers = vec![solver1, solver2, solver3];

		// Create a request for ETH on Ethereum
		let user = InteropAddress::from_text("eip155:1:0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266")
			.unwrap();
		let eth_address =
			InteropAddress::from_text("eip155:1:0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2")
				.unwrap();

		let request = QuoteRequest {
			user: user.clone(),
			available_inputs: vec![AvailableInput {
				user: user.clone(),
				asset: eth_address.clone(),
				amount: U256::from(1000u64),
				lock: None,
			}],
			requested_outputs: vec![RequestedOutput {
				receiver: user,
				asset: eth_address,
				amount: U256::from(900u64),
				calldata: None,
			}],
			min_valid_until: None,
			preference: None,
			solver_options: None,
		};

		let calculator = CompatibilityCalculator::new(FilterConfig::default());
		let solver_scores = calculator
			.score_and_sort_solvers_with_details(&solvers, &request)
			.await;

		// Should return 2 solvers (explicit_compatible and unknown_compatibility)
		// explicit_incompatible should be filtered out
		assert_eq!(
			solver_scores.len(),
			2,
			"Should keep compatible and unknown solvers, filter out explicitly incompatible"
		);

		// Verify the scores and ordering
		let scores_map: std::collections::HashMap<String, f64> = solver_scores
			.iter()
			.map(|(solver, score, _compat)| (solver.solver_id.clone(), *score))
			.collect();

		assert_eq!(
			scores_map.get("explicit_compatible").unwrap(),
			&1.0,
			"Explicit compatibility should get score 1.0"
		);
		assert_eq!(
			scores_map.get("unknown_compatibility").unwrap(),
			&0.0,
			"Unknown compatibility should get score 0.0"
		);
		assert!(
			!scores_map.contains_key("explicit_incompatible"),
			"Explicitly incompatible solver should be filtered out"
		);

		// Verify ordering (highest compatibility first)
		assert_eq!(
			solver_scores[0].0.solver_id, "explicit_compatible",
			"Explicit compatibility should be first"
		);
		assert_eq!(
			solver_scores[1].0.solver_id, "unknown_compatibility",
			"Unknown compatibility should be second"
		);
	}

	#[test]
	fn test_score_aware_weighted_sampling_prioritizes_known_compatibility() {
		use oif_types::models::{Asset, Network};

		let ethereum_network = Network {
			chain_id: 1,
			name: "Ethereum".to_string(),
			is_testnet: false,
		};
		let eth_asset = Asset {
			name: "Ethereum".to_string(),
			symbol: "ETH".to_string(),
			address: "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2".to_string(),
			decimals: 18,
			chain_id: 1,
		};

		// Create mix of solvers with different compatibility scores
		let mut solvers_with_scores = Vec::new();

		// 3 high-compatibility solvers (score 1.0)
		for i in 1..=3 {
			let mut solver = Solver::new(
				format!("high_compat_{}", i),
				"adapter".to_string(),
				"http://localhost".to_string(),
				5000,
			);
			solver.metadata.supported_networks = vec![ethereum_network.clone()];
			solver.metadata.supported_assets = vec![eth_asset.clone()];
			solvers_with_scores.push((solver, 1.0));
		}

		// 2 unknown-compatibility solvers (score 0.0)
		for i in 1..=2 {
			let mut solver = Solver::new(
				format!("unknown_compat_{}", i),
				"adapter".to_string(),
				"http://localhost".to_string(),
				5000,
			);
			solver.metadata.supported_networks = vec![];
			solver.metadata.supported_assets = vec![];
			solvers_with_scores.push((solver, 0.0));
		}

		// Test weighted sampling multiple times to verify bias
		let mut high_compat_selections = 0;
		let mut unknown_compat_selections = 0;
		let iterations = 1000;
		let sample_size = 3;

		for _ in 0..iterations {
			let selected = SelectionEngine::score_aware_weighted_sample(
				solvers_with_scores.clone(),
				sample_size,
			);

			for solver in selected {
				if solver.solver_id.starts_with("high_compat_") {
					high_compat_selections += 1;
				} else if solver.solver_id.starts_with("unknown_compat_") {
					unknown_compat_selections += 1;
				}
			}
		}

		// High compatibility solvers should be selected much more often
		// With 1% weight for unknown solvers, we expect roughly 99:1 ratio
		let ratio = high_compat_selections as f64 / unknown_compat_selections as f64;
		assert!(
			ratio > 10.0,
			"High compatibility solvers should be selected much more often than unknown compatibility. Ratio: {:.2} (high: {}, unknown: {})",
			ratio, high_compat_selections, unknown_compat_selections
		);

		// Both types should still be selected sometimes (unknown should not be 0)
		assert!(
			unknown_compat_selections > 0,
			"Unknown compatibility solvers should still be selected occasionally"
		);
	}
}
