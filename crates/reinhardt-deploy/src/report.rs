//! Deployment report generation.
//!
//! Provides structured report types and formatters for deployment results.
//! Supports human-readable terminal output, JSON serialization, and Markdown
//! output suitable for PR comments.

use serde::{Deserialize, Serialize};

/// Report from a single pre-flight check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckReport {
	pub name: String,
	pub passed: bool,
	pub message: String,
}

/// Pre-flight checks summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightReport {
	pub passed: bool,
	pub checks: Vec<CheckReport>,
}

/// Individual Terraform resource change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceChange {
	pub action: String,
	pub resource_type: String,
	pub resource_name: String,
}

/// Terraform plan summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerraformReport {
	pub creates: u32,
	pub updates: u32,
	pub destroys: u32,
	pub unchanged: u32,
	pub drift: bool,
	pub changes: Vec<ResourceChange>,
}

/// Complete deployment report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployReport {
	pub version: String,
	pub timestamp: String,
	pub commit: String,
	pub environment: String,
	pub provider: String,
	pub dry_run: bool,
	pub preflight: PreflightReport,
	pub terraform: TerraformReport,
	pub exit_code: i32,
}

impl DeployReport {
	/// Format the report as human-readable terminal output.
	pub fn format_human(&self) -> String {
		let mut output = String::new();

		if self.dry_run {
			output.push_str("=== DRY-RUN REPORT ===\n");
		} else {
			output.push_str("=== DEPLOY REPORT ===\n");
		}

		output.push_str(&format!("Environment: {}\n", self.environment));
		output.push_str(&format!("Provider:    {}\n", self.provider));
		output.push_str(&format!("Commit:      {}\n", self.commit));
		output.push_str(&format!("Timestamp:   {}\n", self.timestamp));
		output.push('\n');

		// Pre-flight section
		output.push_str("Pre-flight Checks: ");
		if self.preflight.passed {
			output.push_str("PASSED\n");
		} else {
			output.push_str("FAILED\n");
		}
		for check in &self.preflight.checks {
			let status = if check.passed { "OK" } else { "FAIL" };
			output.push_str(&format!(
				"  [{}] {} - {}\n",
				status, check.name, check.message
			));
		}
		output.push('\n');

		// Terraform section
		output.push_str("Terraform Changes:\n");
		output.push_str(&format!("  Create:    {}\n", self.terraform.creates));
		output.push_str(&format!("  Update:    {}\n", self.terraform.updates));
		output.push_str(&format!("  Destroy:   {}\n", self.terraform.destroys));
		output.push_str(&format!("  Unchanged: {}\n", self.terraform.unchanged));

		if self.terraform.drift {
			output.push_str("  WARNING: Drift detected!\n");
		}

		output
	}

	/// Format the report as JSON.
	pub fn format_json(&self) -> String {
		serde_json::to_string_pretty(self)
			.unwrap_or_else(|e| format!("{{\"error\": \"failed to serialize report: {}\"}}", e))
	}

	/// Format the report as Markdown for PR comments.
	pub fn format_markdown(&self) -> String {
		let mut output = String::new();

		let title = if self.dry_run {
			"Dry-Run Report"
		} else {
			"Deploy Report"
		};
		output.push_str(&format!("## {}\n\n", title));

		output.push_str("| Field | Value |\n");
		output.push_str("|-------|-------|\n");
		output.push_str(&format!("| Environment | {} |\n", self.environment));
		output.push_str(&format!("| Provider | {} |\n", self.provider));
		output.push_str(&format!("| Commit | `{}` |\n", self.commit));
		output.push_str(&format!("| Timestamp | {} |\n", self.timestamp));
		output.push('\n');

		// Terraform summary table
		output.push_str("### Terraform Changes\n\n");
		output.push_str("| Action | Count |\n");
		output.push_str("|--------|-------|\n");
		output.push_str(&format!("| Create | {} |\n", self.terraform.creates));
		output.push_str(&format!("| Update | {} |\n", self.terraform.updates));
		output.push_str(&format!("| Destroy | {} |\n", self.terraform.destroys));
		output.push_str(&format!("| Unchanged | {} |\n", self.terraform.unchanged));

		if self.terraform.drift {
			output.push_str("\n> **Warning**: Infrastructure drift detected!\n");
		}

		output
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	fn sample_report(dry_run: bool) -> DeployReport {
		DeployReport {
			version: "1.0".to_string(),
			timestamp: "2026-02-14T10:30:00Z".to_string(),
			commit: "abc1234".to_string(),
			environment: "production".to_string(),
			provider: "docker".to_string(),
			dry_run,
			preflight: PreflightReport {
				passed: true,
				checks: vec![CheckReport {
					name: "terraform".to_string(),
					passed: true,
					message: "v1.11.2".to_string(),
				}],
			},
			terraform: TerraformReport {
				creates: 3,
				updates: 2,
				destroys: 0,
				unchanged: 15,
				drift: false,
				changes: vec![],
			},
			exit_code: 2,
		}
	}

	#[rstest]
	fn report_json_serialization() {
		// Arrange
		let report = sample_report(true);

		// Act
		let json = report.format_json();

		// Assert
		assert!(json.contains("\"version\": \"1.0\""));
		assert!(json.contains("\"dry_run\": true"));
		assert!(json.contains("\"exit_code\": 2"));
	}

	#[rstest]
	fn report_human_readable_dry_run() {
		// Arrange
		let report = sample_report(true);

		// Act
		let output = report.format_human();

		// Assert
		assert!(output.contains("DRY-RUN REPORT"));
		assert!(output.contains("abc1234"));
		assert!(output.contains("production"));
	}

	#[rstest]
	fn report_human_readable_real_deploy() {
		// Arrange
		let report = sample_report(false);

		// Act
		let output = report.format_human();

		// Assert
		assert!(output.contains("DEPLOY REPORT"));
		assert!(!output.contains("DRY-RUN"));
	}

	#[rstest]
	fn report_markdown_format() {
		// Arrange
		let report = sample_report(true);

		// Act
		let md = report.format_markdown();

		// Assert
		assert!(md.contains("## Dry-Run Report"));
		assert!(md.contains("| Create | 3 |"));
		assert!(md.contains("| Update | 2 |"));
		assert!(md.contains("| `abc1234` |"));
	}

	#[rstest]
	fn report_markdown_deploy() {
		// Arrange
		let report = sample_report(false);

		// Act
		let md = report.format_markdown();

		// Assert
		assert!(md.contains("## Deploy Report"));
	}

	#[rstest]
	fn report_human_shows_preflight_checks() {
		// Arrange
		let report = sample_report(true);

		// Act
		let output = report.format_human();

		// Assert
		assert!(output.contains("Pre-flight Checks: PASSED"));
		assert!(output.contains("[OK] terraform"));
	}

	#[rstest]
	fn report_human_shows_failed_preflight() {
		// Arrange
		let mut report = sample_report(false);
		report.preflight.passed = false;
		report.preflight.checks.push(CheckReport {
			name: "docker".to_string(),
			passed: false,
			message: "not running".to_string(),
		});

		// Act
		let output = report.format_human();

		// Assert
		assert!(output.contains("Pre-flight Checks: FAILED"));
		assert!(output.contains("[FAIL] docker"));
	}

	#[rstest]
	fn report_human_shows_drift_warning() {
		// Arrange
		let mut report = sample_report(false);
		report.terraform.drift = true;

		// Act
		let output = report.format_human();

		// Assert
		assert!(output.contains("WARNING: Drift detected!"));
	}

	#[rstest]
	fn report_markdown_shows_drift_warning() {
		// Arrange
		let mut report = sample_report(false);
		report.terraform.drift = true;

		// Act
		let md = report.format_markdown();

		// Assert
		assert!(md.contains("Infrastructure drift detected"));
	}

	#[rstest]
	fn report_json_round_trip() {
		// Arrange
		let report = sample_report(false);

		// Act
		let json = report.format_json();
		let deserialized: DeployReport = serde_json::from_str(&json).unwrap();

		// Assert
		assert_eq!(deserialized.version, "1.0");
		assert_eq!(deserialized.commit, "abc1234");
		assert_eq!(deserialized.terraform.creates, 3);
		assert!(!deserialized.dry_run);
	}
}
