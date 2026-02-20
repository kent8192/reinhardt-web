//! Show command

use crate::output::{self, OutputFormat};
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub(crate) struct ShowArgs {
	/// Configuration file to read
	#[arg(value_name = "FILE")]
	pub file: PathBuf,

	/// Key to show (shows all if not specified)
	#[arg(short, long)]
	pub key: Option<String>,

	/// Output format (text, json, toml)
	#[arg(short = 'f', long, value_enum, default_value = "text")]
	pub format: OutputFormatArg,

	/// Show sensitive values without redaction (passwords, keys, tokens)
	#[arg(long)]
	pub show_secrets: bool,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub(crate) enum OutputFormatArg {
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
/// Maximum configuration file size for show command (50 MB).
const MAX_CONFIG_FILE_SIZE: u64 = 50 * 1024 * 1024;

/// Display configuration file contents
pub(crate) async fn execute(args: ShowArgs) -> anyhow::Result<()> {
	output::info(&format!("Reading configuration file: {:?}", args.file));

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
						serde_json::Value::String(strip_env_quotes(value.trim()).to_string()),
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

		// Redact sensitive values unless --show-secrets is used
		let display_value = if args.show_secrets || !output::is_sensitive_key(key) {
			current.clone()
		} else {
			serde_json::Value::String(output::REDACTED.to_string())
		};

		output::info(&format!("Value for key '{}':", key));
		output::print_value(&display_value, args.format.into())?;
	} else {
		// Redact sensitive values unless --show-secrets is used
		let display_value = if args.show_secrets {
			value
		} else {
			output::redact_sensitive_values(&value)
		};

		// Show all values
		output::info("Configuration values:");
		output::print_value(&display_value, args.format.into())?;
	}

	Ok(())
}

/// Strip surrounding quotes (double or single) from an .env file value.
fn strip_env_quotes(raw: &str) -> &str {
	let raw = raw.trim();
	if raw.len() >= 2
		&& ((raw.starts_with('"') && raw.ends_with('"'))
			|| (raw.starts_with('\'') && raw.ends_with('\'')))
	{
		&raw[1..raw.len() - 1]
	} else {
		raw
	}
}
