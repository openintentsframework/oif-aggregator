//! # OpenAPI Specification Generator
//!
//! This utility generates an OpenAPI specification JSON file from the
//! OIF Aggregator API definitions. It doesn't require starting the full server
//! and can be used as part of documentation or CI/CD workflows.
//!
//! ## Usage
//!
//! Run the utility with optional output path parameter:
//!
//! ```bash
//! # Generate to default location (docs/api/openapi.json)
//! cargo run --bin generate_openapi --features openapi
//!
//! # Generate to custom location
//! cargo run --bin generate_openapi --features openapi -- custom/path/openapi.json
//! ```
//!
//! ## Features
//!
//! - Generates a complete OpenAPI specification from code annotations
//! - Includes all API endpoints and schemas
//! - Creates output directories automatically if they don't exist
//! - Pretty-prints the JSON for better readability
//!
//! ## Integration
//!
//! This utility is commonly used in CI/CD pipelines to generate up-to-date API documentation
//! whenever the API changes. The generated file can be committed to the repository
//! or published to API documentation platforms.
#[cfg(feature = "openapi")]
use std::env;
#[cfg(feature = "openapi")]
use std::fs;
#[cfg(feature = "openapi")]
use std::path::Path;

#[cfg(feature = "openapi")]
use oif_api::openapi::ApiDoc;
#[cfg(feature = "openapi")]
use utoipa::OpenApi;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	#[cfg(not(feature = "openapi"))]
	{
		eprintln!("Error: The 'openapi' feature must be enabled to generate OpenAPI specs.");
		eprintln!("Run with: cargo run --bin generate_openapi --features openapi");
		std::process::exit(1);
	}

	#[cfg(feature = "openapi")]
	{
		let args: Vec<String> = env::args().collect();
		let output_path = args
			.get(1)
			.map(|s| s.as_str())
			.unwrap_or("docs/api/openapi.json");

		if let Some(parent) = Path::new(output_path).parent() {
			if !parent.exists() {
				fs::create_dir_all(parent)?;
			}
		}

		println!("🚀 Generating OpenAPI specification to {}", output_path);

		let openapi = ApiDoc::openapi();

		let json = serde_json::to_string_pretty(&openapi)?;

		fs::write(output_path, json)?;

		println!("✅ OpenAPI specification successfully generated!");
	}

	#[allow(unreachable_code)]
	Ok(())
}
