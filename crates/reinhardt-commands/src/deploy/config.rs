//! Deployment configuration

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Main deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployConfig {
	/// Default deployment provider
	pub provider: String,

	/// Fly.io specific configuration
	#[serde(default)]
	pub fly: FlyConfig,

	/// AWS specific configuration
	#[serde(default)]
	pub aws: AwsConfig,

	/// GCP specific configuration
	#[serde(default)]
	pub gcp: GcpConfig,
}

impl Default for DeployConfig {
	fn default() -> Self {
		Self {
			provider: "fly.io".to_string(),
			fly: FlyConfig::default(),
			aws: AwsConfig::default(),
			gcp: GcpConfig::default(),
		}
	}
}

/// Fly.io deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FlyConfig {
	pub app_name: Option<String>,
	pub region: Option<String>,
}

/// AWS deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AwsConfig {
	pub region: Option<String>,
	pub service: Option<String>,
}

/// GCP deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GcpConfig {
	pub project: Option<String>,
	pub region: Option<String>,
	pub service: Option<String>,
}

/// Load deployment configuration from file
pub fn load_config(path: &Path) -> Result<DeployConfig, crate::CommandError> {
	let content = std::fs::read_to_string(path).map_err(crate::CommandError::IoError)?;

	toml::from_str(&content).map_err(|e| crate::CommandError::ParseError(e.to_string()))
}

/// Save deployment configuration to file
pub fn save_config(path: &Path, config: &DeployConfig) -> Result<(), crate::CommandError> {
	let content = toml::to_string_pretty(config)
		.map_err(|e| crate::CommandError::ParseError(e.to_string()))?;

	std::fs::write(path, content).map_err(crate::CommandError::IoError)
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_default_config_provider() {
		// Arrange & Act
		let config = DeployConfig::default();

		// Assert
		assert_eq!(config.provider, "fly.io");
	}

	#[rstest]
	fn test_config_serialization() {
		// Arrange
		let config = DeployConfig::default();

		// Act
		let toml = toml::to_string_pretty(&config).unwrap();

		// Assert
		assert!(toml.contains("provider = \"fly.io\""));
	}

	#[rstest]
	fn test_fly_config_default() {
		// Arrange & Act
		let config = FlyConfig::default();

		// Assert
		assert!(config.app_name.is_none());
		assert!(config.region.is_none());
	}

	#[rstest]
	fn test_aws_config_default() {
		// Arrange & Act
		let config = AwsConfig::default();

		// Assert
		assert!(config.region.is_none());
		assert!(config.service.is_none());
	}

	#[rstest]
	fn test_gcp_config_default() {
		// Arrange & Act
		let config = GcpConfig::default();

		// Assert
		assert!(config.project.is_none());
		assert!(config.region.is_none());
		assert!(config.service.is_none());
	}

	#[rstest]
	fn test_deploy_config_has_all_providers() {
		// Arrange & Act
		let config = DeployConfig::default();

		// Assert
		assert!(!config.provider.is_empty());
	}
}
