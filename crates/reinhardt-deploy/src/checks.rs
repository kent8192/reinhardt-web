//! Pre-flight check system for deployment validation.
//!
//! Provides tools to verify that required CLI tools and services
//! are available before starting a deployment.

use crate::providers::traits::PreflightCheck;

/// Result of a single pre-flight check execution.
#[derive(Debug, Clone)]
pub struct PreflightCheckResult {
	pub name: String,
	pub passed: bool,
	pub message: String,
}

/// Runner for executing pre-flight checks before deployment.
pub struct PreflightRunner;

impl PreflightRunner {
	/// Checks if a tool exists in PATH by building a check result.
	///
	/// Returns a `PreflightCheckResult` indicating whether the tool
	/// was found. This builds the result structure without actually
	/// executing the tool.
	pub fn check_tool_exists(tool_name: &str) -> PreflightCheckResult {
		match which::which(tool_name) {
			Ok(path) => PreflightCheckResult {
				name: tool_name.to_string(),
				passed: true,
				message: format!("found at {}", path.display()),
			},
			Err(_) => PreflightCheckResult {
				name: tool_name.to_string(),
				passed: false,
				message: format!("{} not found in PATH", tool_name),
			},
		}
	}

	/// Builds check results from a list of `PreflightCheck` definitions.
	///
	/// This is a pure function that converts check definitions into
	/// pending result structures without executing any commands.
	pub fn build_check_results(checks: &[PreflightCheck]) -> Vec<PreflightCheckResult> {
		checks
			.iter()
			.map(|check| PreflightCheckResult {
				name: check.name.clone(),
				passed: true,
				message: format!("check defined: {}", check.description),
			})
			.collect()
	}

	/// Returns true if all checks in the results passed.
	pub fn all_passed(results: &[PreflightCheckResult]) -> bool {
		results.iter().all(|r| r.passed)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn preflight_check_result_pass() {
		// Arrange
		let check = PreflightCheckResult {
			name: "terraform_version".to_string(),
			passed: true,
			message: "Terraform v1.11.2".to_string(),
		};

		// Act & Assert
		assert!(check.passed);
		assert_eq!(check.name, "terraform_version");
	}

	#[rstest]
	fn preflight_check_result_fail() {
		// Arrange
		let check = PreflightCheckResult {
			name: "docker_daemon".to_string(),
			passed: false,
			message: "Docker daemon is not running".to_string(),
		};

		// Act & Assert
		assert!(!check.passed);
	}

	#[rstest]
	fn all_checks_passed_returns_true() {
		// Arrange
		let results = vec![
			PreflightCheckResult {
				name: "terraform".to_string(),
				passed: true,
				message: "OK".to_string(),
			},
			PreflightCheckResult {
				name: "docker".to_string(),
				passed: true,
				message: "OK".to_string(),
			},
		];

		// Act
		let result = PreflightRunner::all_passed(&results);

		// Assert
		assert!(result);
	}

	#[rstest]
	fn all_checks_passed_returns_false() {
		// Arrange
		let results = vec![
			PreflightCheckResult {
				name: "terraform".to_string(),
				passed: true,
				message: "OK".to_string(),
			},
			PreflightCheckResult {
				name: "docker".to_string(),
				passed: false,
				message: "Not running".to_string(),
			},
		];

		// Act
		let result = PreflightRunner::all_passed(&results);

		// Assert
		assert!(!result);
	}

	#[rstest]
	fn empty_checks_returns_true() {
		// Arrange
		let results: Vec<PreflightCheckResult> = vec![];

		// Act
		let result = PreflightRunner::all_passed(&results);

		// Assert
		assert!(result);
	}

	#[rstest]
	fn check_tool_exists_finds_cargo() {
		// Arrange & Act
		let result = PreflightRunner::check_tool_exists("cargo");

		// Assert
		assert!(result.passed);
		assert_eq!(result.name, "cargo");
		assert!(result.message.starts_with("found at "));
	}

	#[rstest]
	fn check_tool_exists_missing_tool() {
		// Arrange & Act
		let result = PreflightRunner::check_tool_exists("nonexistent_tool_xyz_12345");

		// Assert
		assert!(!result.passed);
		assert_eq!(result.name, "nonexistent_tool_xyz_12345");
		assert_eq!(
			result.message,
			"nonexistent_tool_xyz_12345 not found in PATH"
		);
	}

	#[rstest]
	fn build_check_results_from_definitions() {
		// Arrange
		let checks = vec![
			PreflightCheck {
				name: "docker_check".to_string(),
				description: "Verify Docker is running".to_string(),
				command: "docker info".to_string(),
				expected_exit_code: 0,
			},
			PreflightCheck {
				name: "terraform_check".to_string(),
				description: "Verify Terraform is installed".to_string(),
				command: "terraform version".to_string(),
				expected_exit_code: 0,
			},
		];

		// Act
		let results = PreflightRunner::build_check_results(&checks);

		// Assert
		assert_eq!(results.len(), 2);
		assert_eq!(results[0].name, "docker_check");
		assert!(results[0].passed);
		assert_eq!(
			results[0].message,
			"check defined: Verify Docker is running"
		);
		assert_eq!(results[1].name, "terraform_check");
		assert!(results[1].passed);
		assert_eq!(
			results[1].message,
			"check defined: Verify Terraform is installed"
		);
	}

	#[rstest]
	fn build_check_results_empty_list() {
		// Arrange
		let checks: Vec<PreflightCheck> = vec![];

		// Act
		let results = PreflightRunner::build_check_results(&checks);

		// Assert
		assert!(results.is_empty());
	}
}
