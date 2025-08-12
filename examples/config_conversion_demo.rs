//! Configuration Conversion Demo
//!
//! This example demonstrates how to convert configuration types from `oif_config`
//! to domain types in `oif_types` without creating circular dependencies.

use oif_config::settings::SolverConfig as SettingsSolverConfig;
use oif_types::SolverConfig as DomainSolverConfig;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("🔄 Configuration Conversion Demo");
	println!("==============================");

	// Example 1: Direct conversion from settings config to domain config
	println!("\n📋 Example 1: Settings -> Domain Conversion");

	let settings_config = SettingsSolverConfig {
		solver_id: "example-solver".to_string(),
		adapter_id: "oif".to_string(),
		endpoint: "https://api.example.com/v1".to_string(),
		timeout_ms: 5000,
		enabled: true,
		max_retries: 3,
		headers: Some({
			let mut headers = HashMap::new();
			headers.insert("Authorization".to_string(), "Bearer token".to_string());
			headers.insert("Content-Type".to_string(), "application/json".to_string());
			headers
		}),
		name: Some("Example Solver".to_string()),
		description: Some("A demonstration solver for the conversion example".to_string()),
		supported_networks: None,
		supported_assets: None,
	};

	println!("Settings Config:");
	println!("  - ID: {}", settings_config.solver_id);
	println!("  - Adapter: {}", settings_config.adapter_id);
	println!("  - Endpoint: {}", settings_config.endpoint);
	println!("  - Timeout: {}ms", settings_config.timeout_ms);
	println!("  - Max Retries: {}", settings_config.max_retries);

	// Convert to domain config using the From trait (defined in oif_config::conversions)
	let domain_config: DomainSolverConfig = settings_config.into();

	println!("\nDomain Config (after conversion):");
	println!("  - ID: {}", domain_config.solver_id);
	println!("  - Adapter: {}", domain_config.adapter_id);
	println!("  - Endpoint: {}", domain_config.endpoint);
	println!("  - Timeout: {}ms", domain_config.timeout_ms);
	println!("  - Max Retries: {}", domain_config.max_retries);
	println!("  - Name: {:?}", domain_config.name);
	println!("  - Description: {:?}", domain_config.description);
	println!(
		"  - Headers count: {}",
		domain_config.headers.as_ref().map_or(0, |h| h.len())
	);

	// Domain-specific fields that are None after conversion
	println!("  - Version: {:?}", domain_config.version);
	println!(
		"  - Supported Networks: {:?}",
		domain_config.supported_networks.as_ref().map(|n| n.len())
	);
	println!(
		"  - Supported Assets: {:?}",
		domain_config.supported_assets.as_ref().map(|a| a.len())
	);
	println!(
		"  - Config: {:?}",
		domain_config.config.as_ref().map(|c| c.len())
	);

	// Example 2: Loading from actual settings file (if available)
	println!("\n📁 Example 2: Loading from Settings");

	match oif_config::load_config() {
		Ok(settings) => {
			let enabled_solvers = settings.enabled_solvers();
			println!(
				"Loaded {} solver(s) from configuration:",
				enabled_solvers.len()
			);

			for (id, solver_config) in enabled_solvers {
				println!("\n  Solver: {}", id);

				// Convert each settings solver config to domain config
				let domain_config: DomainSolverConfig = solver_config.clone().into();

				println!("    - Adapter: {}", domain_config.adapter_id);
				println!("    - Endpoint: {}", domain_config.endpoint);
				println!("    - Enabled: {}", domain_config.enabled);
				println!("    - Timeout: {}ms", domain_config.timeout_ms);

				if let Some(name) = &domain_config.name {
					println!("    - Name: {}", name);
				}
				if let Some(description) = &domain_config.description {
					println!("    - Description: {}", description);
				}
			}
		},
		Err(e) => {
			println!("No configuration file found or error loading: {}", e);
			println!("This is expected if config/config.json doesn't exist");
		},
	}

	println!("\n✅ Conversion demo completed successfully!");
	println!("\n💡 Key Benefits:");
	println!("   - No circular dependencies between oif-config and oif-types");
	println!("   - Clean separation of concerns");
	println!("   - Type-safe conversions using From trait");
	println!("   - Settings config stays in oif-config");
	println!("   - Domain config available in oif-types");

	Ok(())
}
