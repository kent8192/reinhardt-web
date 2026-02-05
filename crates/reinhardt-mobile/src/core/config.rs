//! Mobile application configuration.

use serde::{Deserialize, Serialize};

/// Configuration for mobile application build.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileConfig {
	/// Application name
	pub app_name: String,

	/// Application identifier (e.g., com.example.myapp)
	pub app_id: String,

	/// Application version
	pub version: String,

	/// Target platform
	pub platform: TargetPlatform,

	/// Build configuration
	pub build: BuildConfig,

	/// Security settings
	pub security: SecurityConfig,
}

/// Target platform for build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetPlatform {
	/// Android platform
	Android,
	/// iOS platform
	Ios,
}

/// Build configuration options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
	/// Debug or release build
	pub release: bool,

	/// Minimum API level (Android) or iOS version
	pub min_api_level: u32,

	/// Target API level (Android only)
	pub target_api_level: Option<u32>,

	/// Enable experimental features
	pub experimental: bool,
}

/// Security configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
	/// Allow remote navigation
	pub allow_remote_navigation: bool,

	/// Allowed origins for IPC
	pub allowed_ipc_origins: Vec<String>,

	/// Custom protocol scheme
	pub protocol_scheme: String,
}

impl Default for MobileConfig {
	fn default() -> Self {
		Self {
			app_name: String::from("ReinhardtApp"),
			app_id: String::from("com.example.reinhardt"),
			version: String::from("1.0.0"),
			platform: TargetPlatform::Android,
			build: BuildConfig::default(),
			security: SecurityConfig::default(),
		}
	}
}

impl Default for BuildConfig {
	fn default() -> Self {
		Self {
			release: false,
			min_api_level: 26,
			target_api_level: Some(33),
			experimental: true,
		}
	}
}

impl Default for SecurityConfig {
	fn default() -> Self {
		Self {
			allow_remote_navigation: false,
			allowed_ipc_origins: vec![],
			protocol_scheme: String::from("reinhardt"),
		}
	}
}
