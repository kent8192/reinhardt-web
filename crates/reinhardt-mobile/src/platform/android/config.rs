//! Android-specific configuration.

use crate::MobileConfig;
use serde::{Deserialize, Serialize};

/// Android-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AndroidConfig {
	/// Application package name (e.g., com.example.myapp)
	pub package_name: String,

	/// Minimum SDK version (API level)
	pub min_sdk_version: u32,

	/// Target SDK version (API level)
	pub target_sdk_version: u32,

	/// Compile SDK version
	pub compile_sdk_version: u32,

	/// NDK version
	pub ndk_version: String,

	/// Application label (display name)
	pub app_label: String,

	/// Version code (integer version for store)
	pub version_code: u32,

	/// Version name (display version)
	pub version_name: String,

	/// Enable hardware acceleration
	pub hardware_accelerated: bool,

	/// Enable debugging
	pub debuggable: bool,

	/// Required permissions
	pub permissions: Vec<String>,
}

impl AndroidConfig {
	/// Creates AndroidConfig from MobileConfig.
	pub fn from_mobile_config(config: &MobileConfig) -> Self {
		Self {
			package_name: config.app_id.clone(),
			min_sdk_version: config.build.min_api_level,
			target_sdk_version: config.build.target_api_level.unwrap_or(33),
			compile_sdk_version: 33,
			ndk_version: "25.0.8775105".to_string(),
			app_label: config.app_name.clone(),
			version_code: 1,
			version_name: config.version.clone(),
			hardware_accelerated: true,
			debuggable: !config.build.release,
			permissions: vec!["android.permission.INTERNET".to_string()],
		}
	}
}

impl Default for AndroidConfig {
	fn default() -> Self {
		Self {
			package_name: "com.example.reinhardt".to_string(),
			min_sdk_version: 26,
			target_sdk_version: 33,
			compile_sdk_version: 33,
			ndk_version: "25.0.8775105".to_string(),
			app_label: "Reinhardt App".to_string(),
			version_code: 1,
			version_name: "1.0.0".to_string(),
			hardware_accelerated: true,
			debuggable: true,
			permissions: vec!["android.permission.INTERNET".to_string()],
		}
	}
}
