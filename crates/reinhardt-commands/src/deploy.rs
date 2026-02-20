//! Deployment commands for Reinhardt applications
//!
//! This module provides commands for deploying Reinhardt applications
//! to various cloud providers.
//!
//! # Commands
//!
//! - `reinhardt deploy init` - Initialize deployment configuration
//! - `reinhardt deploy` - Deploy to configured provider
//!
//! # Supported Providers (Phase 1 - Foundation)
//!
//! The foundation is laid for these providers (full implementation in follow-up PRs):
//! - Fly.io
//! - AWS ECS/Fargate
//! - GCP Cloud Run

pub mod config;
pub mod dockerfile;
pub mod provider;

pub use config::DeployConfig;
pub use provider::{DeployError, DeployProvider};

use crate::{BaseCommand, CommandContext, CommandResult};
use std::path::PathBuf;

/// Get the project root directory
///
/// Uses the current directory as the project root.
fn project_root() -> PathBuf {
	std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Deploy command - deploy application to cloud provider
pub struct DeployCommand;

#[async_trait::async_trait]
impl BaseCommand for DeployCommand {
	fn name(&self) -> &str {
		"deploy"
	}

	fn description(&self) -> &str {
		"Deploy the application to a cloud provider"
	}

	fn help(&self) -> &str {
		self.description()
	}

	async fn execute(&self, ctx: &CommandContext) -> CommandResult<()> {
		// Check if initialized
		let config_path = project_root().join("reinhardt.toml");
		if !config_path.exists() {
			ctx.error("Deployment not initialized.");
			ctx.info(&format!(
				"Run {} first.",
				console::style("reinhardt deploy init").cyan()
			));
			return Ok(());
		}

		// Load configuration
		let config = config::load_config(&config_path)?;
		ctx.info("Deploying...");
		ctx.info(&format!("  Provider: {}", config.provider));
		ctx.info("");
		ctx.info("Deployment infrastructure is ready.");
		ctx.info("Provider adapters will be added in future releases.");

		Ok(())
	}

	fn requires_system_checks(&self) -> bool {
		false
	}
}

/// Deploy init command - initialize deployment configuration
pub struct DeployInitCommand;

#[async_trait::async_trait]
impl BaseCommand for DeployInitCommand {
	fn name(&self) -> &str {
		"deploy:init"
	}

	fn description(&self) -> &str {
		"Initialize deployment configuration"
	}

	fn help(&self) -> &str {
		self.description()
	}

	async fn execute(&self, ctx: &CommandContext) -> CommandResult<()> {
		let config_path = project_root().join("reinhardt.toml");

		if config_path.exists() {
			ctx.warning("reinhardt.toml already exists.");
			return Ok(());
		}

		// Create default configuration
		let default_config = DeployConfig::default();
		config::save_config(&config_path, &default_config)?;

		ctx.success(&format!("Created {}", config_path.display()));
		ctx.info("");
		ctx.info("Edit the [deploy] section in reinhardt.toml to configure your deployment.");

		// Generate Dockerfile
		let dockerfile_path = project_root().join("Dockerfile");
		if !dockerfile_path.exists() {
			let dockerfile = dockerfile::generate_dockerfile()
				.map_err(|e| crate::CommandError::ExecutionError(e.to_string()))?;
			std::fs::write(&dockerfile_path, dockerfile)?;
			ctx.info("");
			ctx.success(&format!(
				"Generated Dockerfile at {}",
				dockerfile_path.display()
			));
		}

		Ok(())
	}

	fn requires_system_checks(&self) -> bool {
		false
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_deploy_command_name() {
		let cmd = DeployCommand;
		assert_eq!(cmd.name(), "deploy");
	}

	#[rstest]
	fn test_deploy_command_description() {
		let cmd = DeployCommand;
		assert_eq!(
			cmd.description(),
			"Deploy the application to a cloud provider"
		);
	}

	#[rstest]
	fn test_deploy_init_command_name() {
		let cmd = DeployInitCommand;
		assert_eq!(cmd.name(), "deploy:init");
	}

	#[rstest]
	fn test_deploy_init_command_description() {
		let cmd = DeployInitCommand;
		assert_eq!(cmd.description(), "Initialize deployment configuration");
	}

	#[rstest]
	fn test_deploy_command_no_system_checks() {
		let cmd = DeployCommand;
		assert!(!cmd.requires_system_checks());
	}

	#[rstest]
	fn test_deploy_init_command_no_system_checks() {
		let cmd = DeployInitCommand;
		assert!(!cmd.requires_system_checks());
	}

	#[rstest]
	fn test_project_root_returns_path() {
		// Act
		let root = project_root();

		// Assert
		assert!(root.is_absolute());
	}
}
