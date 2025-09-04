//! Shared domain models used across adapters, solvers, and other components

pub mod asset;
pub mod asset_route;
pub mod health;
pub mod interop_address;
pub mod lock;
pub mod network;
pub mod secret_string;
pub mod u256;

pub use asset::Asset;
pub use asset_route::{AssetRoute, AssetRouteResponse};
pub use health::{HealthResponse, SolverStats, StorageHealthInfo};
pub use interop_address::InteropAddress;
pub use lock::Lock;
pub use network::Network;
pub use secret_string::SecretString;
pub use u256::U256;

/// Data returned by adapters for supported assets/routes (without source info)
#[derive(Debug, Clone, PartialEq)]
pub enum SupportedAssetsData {
	/// Asset-based: supports any-to-any within asset list (including same-chain)
	Assets(Vec<Asset>),
	/// Route-based: supports specific origin->destination pairs
	Routes(Vec<AssetRoute>),
}
