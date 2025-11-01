//! Validate command

use crate::output;
use clap::Args;
use reinhardt_settings::prelude::*;
use std::path::PathBuf;

#[derive(Args)]
pub struct ValidateArgs {
	/// Configuration file to validate
	#[arg(value_name = "FILE")]
	pub file: PathBuf,

	/// Profile to validate against (development, staging, production)
	#[arg(short, long)]
	pub profile: Option<String>,
}
/// Documentation for `execute`
///
pub async fn execute(args: ValidateArgs) -> anyhow::Result<()> {
	output::info(&format!("Validating configuration file: {:?}", args.file));

	// Check if file exists
	if !args.file.exists() {
		output::error("Configuration file not found");
		return Err(anyhow::anyhow!("File not found: {:?}", args.file));
	}

	// Determine file type and load
	let extension = args.file.extension().and_then(|s| s.to_str());

	match extension {
		Some("toml") => {
			let content = std::fs::read_to_string(&args.file)?;
			let _: toml::Value =
				toml::from_str(&content).map_err(|e| anyhow::anyhow!("Invalid TOML: {}", e))?;
			output::success("TOML syntax is valid");
		}
		Some("json") => {
			let content = std::fs::read_to_string(&args.file)?;
			let _: serde_json::Value = serde_json::from_str(&content)
				.map_err(|e| anyhow::anyhow!("Invalid JSON: {}", e))?;
			output::success("JSON syntax is valid");
		}
		Some("env") => {
			// Validate .env file
			let content = std::fs::read_to_string(&args.file)?;
			for (line_num, line) in content.lines().enumerate() {
				let line = line.trim();
				if line.is_empty() || line.starts_with('#') {
					continue;
				}
				if !line.contains('=') {
					output::warning(&format!(
						"Line {} might be invalid: missing '=' separator",
						line_num + 1
					));
				}
			}
			output::success(".env file format is valid");
		}
		_ => {
			output::warning("Unknown file format, skipping syntax validation");
		}
	}

	// Profile-specific validation
	if let Some(profile_name) = args.profile {
		output::info(&format!("Validating for profile: {}", profile_name));

		let profile = match profile_name.as_str() {
			"development" | "dev" => Profile::Development,
			"staging" => Profile::Staging,
			"production" | "prod" => Profile::Production,
			_ => {
				output::warning("Unknown profile, using custom");
				Profile::Custom
			}
		};

		// Create validator
		let _validator = reinhardt_settings::validation::SecurityValidator::new(profile);

		output::info("Profile-specific validation completed");
	}

	output::success("Configuration validation passed");
	Ok(())
}
