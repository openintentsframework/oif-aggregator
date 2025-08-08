//! Service startup logging for the OIF Aggregator
//!
//! This module provides comprehensive logging functionality for service startup,
//! including service information, environment details, and system information.

use std::env;
use tracing::info;

/// Logs comprehensive service information at startup
pub fn log_service_info() {
	// Use the root package name and version, not the current crate
	let service_name = "oif-aggregator";
	let service_version = env!("CARGO_PKG_VERSION");

	info!("=== OIF Aggregator Service Starting ===");
	info!("🚀 Service: {} v{}", service_name, service_version);

	// Log Rust information
	if let Ok(rust_version) = env::var("RUSTC_VERSION") {
		info!("🦀 Rust Version: {}", rust_version);
	} else {
		// Fallback to edition info
		info!("🦀 Rust Edition: 2021");
	}

	// Log build information
	if let Ok(profile) = env::var("CARGO_PKG_PROFILE") {
		info!("🔧 Build Profile: {}", profile);
	}

	// Log target information
	info!("💻 Platform: {}", env::consts::OS);
	info!("🏗️ Architecture: {}", env::consts::ARCH);

	// Log current working directory
	if let Ok(cwd) = env::current_dir() {
		info!("📁 Working Directory: {}", cwd.display());
	}

	// Log important environment variables if present
	if let Ok(rust_log) = env::var("RUST_LOG") {
		info!("🔧 Log Level: {}", rust_log);
	}

	if let Ok(run_mode) = env::var("RUN_MODE") {
		info!("🌍 Run Mode: {}", run_mode);
	}

	if let Ok(config_path) = env::var("CONFIG_PATH") {
		info!("📋 Config Path: {}", config_path);
	}

	// Log startup timestamp
	info!(
		"🕒 Started at: {}",
		chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
	);

	info!("🎯 Starting aggregator initialization...");
}

/// Logs service shutdown information
pub fn log_service_shutdown() {
	info!("🛑 OIF Aggregator Service Shutting Down");
	info!(
		"🕒 Shutdown at: {}",
		chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
	);
}

/// Logs additional startup completion information
pub fn log_startup_complete(bind_address: &str) {
	info!("✅ OIF Aggregator Service Started Successfully");
	info!("🌐 Server listening on: {}", bind_address);
	info!("📡 Ready to accept requests");
}
