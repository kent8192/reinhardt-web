//! Validate command

use crate::output;
use clap::Args;
use reinhardt_conf::settings::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Args)]
pub(crate) struct ValidateArgs {
	/// Configuration file to validate
	#[arg(value_name = "FILE")]
	pub file: PathBuf,

	/// Profile to validate against (development, staging, production)
	#[arg(short, long)]
	pub profile: Option<String>,
}
/// Maximum configuration file size for validate command (50 MB).
const MAX_CONFIG_FILE_SIZE: u64 = 50 * 1024 * 1024;

/// Validate a configuration file for syntax and profile-specific rules
pub(crate) async fn execute(args: ValidateArgs) -> anyhow::Result<()> {
	output::info(&format!("Validating configuration file: {:?}", args.file));

	// Check file existence and size in one operation (TOCTOU mitigation)
	let metadata = std::fs::metadata(&args.file).map_err(|e| {
		output::error("Configuration file not found or inaccessible");
		anyhow::anyhow!("Cannot access file {:?}: {}", args.file, e)
	})?;

	if metadata.len() > MAX_CONFIG_FILE_SIZE {
		return Err(anyhow::anyhow!(
			"Configuration file exceeds maximum size ({} bytes, limit {} bytes)",
			metadata.len(),
			MAX_CONFIG_FILE_SIZE
		));
	}

	// Determine file type and load
	let extension = args.file.extension().and_then(|s| s.to_str());

	// Store parsed settings for profile-specific validation
	let settings_opt: Option<HashMap<String, serde_json::Value>> = match extension {
		Some("toml") => {
			let content = std::fs::read_to_string(&args.file)?;
			let toml_value: toml::Value =
				toml::from_str(&content).map_err(|e| anyhow::anyhow!("Invalid TOML: {}", e))?;
			output::success("TOML syntax is valid");
			Some(toml_to_hashmap(&toml_value))
		}
		Some("json") => {
			let content = std::fs::read_to_string(&args.file)?;
			let json_value: serde_json::Value = serde_json::from_str(&content)
				.map_err(|e| anyhow::anyhow!("Invalid JSON: {}", e))?;
			output::success("JSON syntax is valid");
			Some(json_to_hashmap(&json_value))
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
			None
		}
		_ => {
			output::warning("Unknown file format, skipping syntax validation");
			None
		}
	};

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

		// Run profile-specific security validation
		if let Some(settings) = settings_opt {
			let validator = reinhardt_conf::settings::validation::SecurityValidator::new(profile);
			validator
				.validate_settings(&settings)
				.map_err(|e| anyhow::anyhow!("Profile validation failed: {}", e))?;
			output::success("Profile-specific validation passed");
		} else {
			output::warning(
				"Profile-specific validation is only supported for TOML and JSON files",
			);
		}
	}

	output::success("Configuration validation passed");
	Ok(())
}

/// Convert TOML value to HashMap for validation
fn toml_to_hashmap(toml: &toml::Value) -> HashMap<String, serde_json::Value> {
	let mut map = HashMap::new();
	if let toml::Value::Table(table) = toml {
		for (key, value) in table {
			map.insert(key.clone(), toml_to_json_value(value));
		}
	}
	map
}

/// Convert TOML value to JSON value
fn toml_to_json_value(toml: &toml::Value) -> serde_json::Value {
	match toml {
		toml::Value::String(s) => serde_json::Value::String(s.clone()),
		toml::Value::Integer(i) => serde_json::Value::Number((*i).into()),
		toml::Value::Float(f) => {
			serde_json::Value::Number(serde_json::Number::from_f64(*f).unwrap_or_else(|| 0.into()))
		}
		toml::Value::Boolean(b) => serde_json::Value::Bool(*b),
		toml::Value::Array(arr) => {
			serde_json::Value::Array(arr.iter().map(toml_to_json_value).collect())
		}
		toml::Value::Table(table) => serde_json::Value::Object(
			table
				.iter()
				.map(|(k, v)| (k.clone(), toml_to_json_value(v)))
				.collect(),
		),
		toml::Value::Datetime(dt) => serde_json::Value::String(dt.to_string()),
	}
}

/// Convert JSON object to HashMap for validation
fn json_to_hashmap(json: &serde_json::Value) -> HashMap<String, serde_json::Value> {
	let mut map = HashMap::new();
	if let serde_json::Value::Object(obj) = json {
		for (key, value) in obj {
			map.insert(key.clone(), value.clone());
		}
	}
	map
}
