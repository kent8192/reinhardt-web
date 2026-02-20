//! Deployment provider trait and types

use super::config::DeployConfig;
use async_trait::async_trait;

/// Result type for deployment operations
pub type DeployResult<T> = Result<T, DeployError>;

/// Deployment error types
#[derive(Debug, thiserror::Error)]
pub enum DeployError {
	#[error("Authentication failed: {0}")]
	AuthenticationFailed(String),

	#[error("Deployment failed: {0}")]
	DeploymentFailed(String),

	#[error("Provider not configured: {0}")]
	NotConfigured(String),

	#[error("IO error: {0}")]
	Io(#[from] std::io::Error),
}

/// Deployment status
#[derive(Debug, Clone)]
pub struct DeployStatus {
	pub deployed: bool,
	pub url: Option<String>,
	pub version: Option<String>,
}

/// Deployment result
#[derive(Debug, Clone)]
pub struct DeployResultInfo {
	pub url: String,
	pub version: String,
}

/// Trait for deployment providers
#[async_trait]
pub trait DeployProvider: Send + Sync {
	/// Provider name
	fn name(&self) -> &str;

	/// Check if provider is authenticated
	async fn is_authenticated(&self) -> bool;

	/// Authenticate with the provider
	async fn authenticate(&self) -> DeployResult<()>;

	/// Deploy the application
	async fn deploy(&self, config: &DeployConfig) -> DeployResult<DeployResultInfo>;

	/// Get deployment status
	async fn status(&self) -> DeployResult<DeployStatus>;

	/// View application logs
	async fn logs(&self, follow: bool) -> DeployResult<()>;

	/// Destroy the deployment
	async fn destroy(&self) -> DeployResult<()>;
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_deploy_error_display_authentication_failed() {
		// Arrange
		let err = DeployError::AuthenticationFailed("test".to_string());

		// Act & Assert
		assert!(err.to_string().contains("Authentication failed"));
		assert!(err.to_string().contains("test"));
	}

	#[rstest]
	fn test_deploy_error_display_deployment_failed() {
		// Arrange
		let err = DeployError::DeploymentFailed("test".to_string());

		// Act & Assert
		assert!(err.to_string().contains("Deployment failed"));
		assert!(err.to_string().contains("test"));
	}

	#[rstest]
	fn test_deploy_error_display_not_configured() {
		// Arrange
		let err = DeployError::NotConfigured("test".to_string());

		// Act & Assert
		assert!(err.to_string().contains("Provider not configured"));
		assert!(err.to_string().contains("test"));
	}

	#[rstest]
	fn test_deploy_status_all_fields() {
		// Arrange & Act
		let status = DeployStatus {
			deployed: true,
			url: Some("https://example.com".to_string()),
			version: Some("v1.0.0".to_string()),
		};

		// Assert
		assert!(status.deployed);
		assert!(status.url.is_some());
		assert!(status.version.is_some());
	}

	#[rstest]
	fn test_deploy_status_no_url() {
		// Arrange & Act
		let status = DeployStatus {
			deployed: false,
			url: None,
			version: None,
		};

		// Assert
		assert!(!status.deployed);
		assert!(status.url.is_none());
		assert!(status.version.is_none());
	}

	#[rstest]
	fn test_deploy_result_info_has_fields() {
		// Arrange & Act
		let result = DeployResultInfo {
			url: "https://example.com".to_string(),
			version: "v1.0.0".to_string(),
		};

		// Assert
		assert_eq!(result.url, "https://example.com");
		assert_eq!(result.version, "v1.0.0");
	}
}
