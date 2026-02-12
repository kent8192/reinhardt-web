//! iOS-specific configuration.

use crate::MobileConfig;
use serde::{Deserialize, Serialize};

/// iOS-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IosConfig {
	/// Bundle identifier (e.g., com.example.myapp)
	pub bundle_identifier: String,

	/// Minimum iOS version
	pub minimum_ios_version: String,

	/// Display name
	pub display_name: String,

	/// Bundle version (build number)
	pub bundle_version: String,

	/// Short version string (marketing version)
	pub short_version_string: String,

	/// Development team ID
	pub development_team: Option<String>,

	/// Code signing identity
	pub code_sign_identity: Option<String>,

	/// Provisioning profile
	pub provisioning_profile: Option<String>,

	/// Device family (1 = iPhone, 2 = iPad, 1,2 = Universal)
	pub device_family: Vec<u8>,

	/// Supported orientations
	pub supported_orientations: Vec<String>,

	/// Enable background modes
	pub background_modes: Vec<String>,

	/// Required device capabilities
	pub required_capabilities: Vec<String>,
}

impl IosConfig {
	/// Creates IosConfig from MobileConfig.
	pub fn from_mobile_config(config: &MobileConfig) -> Self {
		Self {
			bundle_identifier: config.app_id.clone(),
			minimum_ios_version: format!("{}.0", config.build.min_api_level.max(13)),
			display_name: config.app_name.clone(),
			bundle_version: "1".to_string(),
			short_version_string: config.version.clone(),
			development_team: None,
			code_sign_identity: None,
			provisioning_profile: None,
			device_family: vec![1, 2], // Universal
			supported_orientations: vec![
				"UIInterfaceOrientationPortrait".to_string(),
				"UIInterfaceOrientationLandscapeLeft".to_string(),
				"UIInterfaceOrientationLandscapeRight".to_string(),
			],
			background_modes: vec![],
			required_capabilities: vec!["arm64".to_string()],
		}
	}
}

impl Default for IosConfig {
	fn default() -> Self {
		Self {
			bundle_identifier: "com.example.reinhardt".to_string(),
			minimum_ios_version: "13.0".to_string(),
			display_name: "Reinhardt App".to_string(),
			bundle_version: "1".to_string(),
			short_version_string: "1.0.0".to_string(),
			development_team: None,
			code_sign_identity: None,
			provisioning_profile: None,
			device_family: vec![1, 2],
			supported_orientations: vec![
				"UIInterfaceOrientationPortrait".to_string(),
				"UIInterfaceOrientationLandscapeLeft".to_string(),
				"UIInterfaceOrientationLandscapeRight".to_string(),
			],
			background_modes: vec![],
			required_capabilities: vec!["arm64".to_string()],
		}
	}
}
