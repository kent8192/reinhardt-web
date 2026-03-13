use thiserror::Error;

/// Errors that can occur during deployment configuration and generation
#[derive(Debug, Error)]
pub enum DeployError {
	/// Template rendering error
	#[error("template rendering failed: {0}")]
	TemplateRender(#[from] tera::Error),

	/// Invalid configuration
	#[error("invalid configuration: {0}")]
	InvalidConfig(String),
}
