//! Show command

use crate::output::{self, OutputFormat};
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct ShowArgs {
	/// Configuration file to read
	#[arg(value_name = "FILE")]
	pub file: PathBuf,

	/// Key to show (shows all if not specified)
	#[arg(short, long)]
	pub key: Option<String>,

	/// Output format (text, json, toml)
	#[arg(short = 'f', long, value_enum, default_value = "text")]
	pub format: OutputFormatArg,

	/// Profile to use
	#[arg(short, long)]
	pub profile: Option<String>,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum OutputFormatArg {
	Text,
	Json,
	Toml,
}

impl From<OutputFormatArg> for OutputFormat {
	fn from(arg: OutputFormatArg) -> Self {
		match arg {
			OutputFormatArg::Text => OutputFormat::Text,
			OutputFormatArg::Json => OutputFormat::Json,
			OutputFormatArg::Toml => OutputFormat::Toml,
		}
	}
}
/// Documentation for `execute`
///
pub async fn execute(args: ShowArgs) -> anyhow::Result<()> {
	output::info(&format!("Reading configuration file: {:?}", args.file));

	// Check if file exists
	if !args.file.exists() {
		output::error("Configuration file not found");
		return Err(anyhow::anyhow!("File not found: {:?}", args.file));
	}

	// Determine file type and load
	let extension = args.file.extension().and_then(|s| s.to_str());

	let value: serde_json::Value = match extension {
		Some("toml") => {
			let content = std::fs::read_to_string(&args.file)?;
			let toml_value: toml::Value =
				toml::from_str(&content).map_err(|e| anyhow::anyhow!("Invalid TOML: {}", e))?;
			serde_json::to_value(toml_value)?
		}
		Some("json") => {
			let content = std::fs::read_to_string(&args.file)?;
			serde_json::from_str(&content).map_err(|e| anyhow::anyhow!("Invalid JSON: {}", e))?
		}
		Some("env") => {
			let content = std::fs::read_to_string(&args.file)?;
			let mut env_map = serde_json::Map::new();

			for line in content.lines() {
				let line = line.trim();
				if line.is_empty() || line.starts_with('#') {
					continue;
				}
				if let Some((key, value)) = line.split_once('=') {
					env_map.insert(
						key.trim().to_string(),
						serde_json::Value::String(value.trim().to_string()),
					);
				}
			}
			serde_json::Value::Object(env_map)
		}
		_ => {
			output::error("Unsupported file format");
			return Err(anyhow::anyhow!("Unsupported file format: {:?}", extension));
		}
	};

	// If a specific key is requested, extract it
	if let Some(key) = &args.key {
		let parts: Vec<&str> = key.split('.').collect();
		let mut current = &value;

		for part in &parts {
			current = current
				.get(part)
				.ok_or_else(|| anyhow::anyhow!("Key not found: {}", key))?;
		}

		output::info(&format!("Value for key '{}':", key));
		output::print_value(current, args.format.into())?;
	} else {
		// Show all values
		output::info("Configuration values:");
		output::print_value(&value, args.format.into())?;
	}

	Ok(())
}
