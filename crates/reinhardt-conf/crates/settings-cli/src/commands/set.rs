//! Set command

use crate::output;
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub(crate) struct SetArgs {
	/// Configuration file to modify
	#[arg(value_name = "FILE")]
	pub file: PathBuf,

	/// Key to set (dot notation supported, e.g., database.host)
	#[arg(short, long)]
	pub key: String,

	/// Value to set
	#[arg(short, long)]
	pub value: String,

	/// Create file if it doesn't exist
	#[arg(short, long)]
	pub create: bool,

	/// Backup original file before modifying
	#[arg(short, long, default_value = "true")]
	pub backup: bool,
}
/// Maximum configuration file size for set command (50 MB).
const MAX_CONFIG_FILE_SIZE: u64 = 50 * 1024 * 1024;

/// Set a configuration value in a file
pub(crate) async fn execute(args: SetArgs) -> anyhow::Result<()> {
	output::info(&format!("Setting configuration value in: {:?}", args.file));

	// Try to read file metadata to check existence and size (TOCTOU mitigation)
	match std::fs::metadata(&args.file) {
		Ok(metadata) => {
			if metadata.len() > MAX_CONFIG_FILE_SIZE {
				return Err(anyhow::anyhow!(
					"Configuration file exceeds maximum size ({} bytes, limit {} bytes)",
					metadata.len(),
					MAX_CONFIG_FILE_SIZE
				));
			}
		}
		Err(_) => {
			if args.create {
				output::info("Creating new configuration file");
				// Create with empty object/map using atomic create
				let extension = args.file.extension().and_then(|s| s.to_str());
				match extension {
					Some("json") => {
						std::fs::write(&args.file, "{}")?;
					}
					Some("toml") => {
						std::fs::write(&args.file, "")?;
					}
					Some("env") => {
						std::fs::write(&args.file, "")?;
					}
					_ => {
						output::error("Unsupported file format");
						return Err(anyhow::anyhow!("Unsupported file format"));
					}
				}
			} else {
				output::error("Configuration file not found. Use --create to create it.");
				return Err(anyhow::anyhow!("File not found: {:?}", args.file));
			}
		}
	}

	// Create backup if requested
	if args.backup && args.file.exists() {
		let backup_path = args.file.with_extension(
			format!(
				"{}.bak",
				args.file.extension().and_then(|s| s.to_str()).unwrap_or("")
			)
			.trim_start_matches('.'),
		);
		std::fs::copy(&args.file, &backup_path)?;

		// Set restrictive permissions on backup file (owner read/write only)
		#[cfg(unix)]
		{
			use std::os::unix::fs::PermissionsExt;
			let permissions = std::fs::Permissions::from_mode(0o600);
			std::fs::set_permissions(&backup_path, permissions)?;
		}

		output::info(&format!("Backup created: {:?}", backup_path));
	}

	// Determine file type and modify
	let extension = args.file.extension().and_then(|s| s.to_str());

	match extension {
		Some("toml") => {
			let content = std::fs::read_to_string(&args.file)?;
			let mut toml_value: toml::Value =
				toml::from_str(&content).map_err(|e| anyhow::anyhow!("Invalid TOML: {}", e))?;

			// Navigate to the key and set value
			set_nested_value(&mut toml_value, &args.key, &args.value)?;

			// Write back
			let toml_str = toml::to_string_pretty(&toml_value)?;
			std::fs::write(&args.file, toml_str)?;
			output::success(&format!("Set {} = {}", args.key, args.value));
		}
		Some("json") => {
			let content = std::fs::read_to_string(&args.file)?;
			let mut json_value: serde_json::Value = serde_json::from_str(&content)
				.map_err(|e| anyhow::anyhow!("Invalid JSON: {}", e))?;

			// Navigate to the key and set value
			set_nested_json_value(&mut json_value, &args.key, &args.value)?;

			// Write back
			let json_str = serde_json::to_string_pretty(&json_value)?;
			std::fs::write(&args.file, json_str)?;
			output::success(&format!("Set {} = {}", args.key, args.value));
		}
		Some("env") => {
			let content = std::fs::read_to_string(&args.file)?;
			let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

			// Find and replace or append
			let new_line = format!("{}={}", args.key, args.value);

			let mut found = false;
			for line in &mut lines {
				let trimmed = line.trim();
				if trimmed.is_empty() || trimmed.starts_with('#') {
					continue;
				}
				// Split on first '=' and trim spaces around key/value
				if let Some((key, _)) = trimmed.split_once('=') {
					if key.trim() == args.key {
						*line = new_line.clone();
						found = true;
						break;
					}
				}
			}

			if !found {
				lines.push(new_line);
			}

			// Write back
			let new_content = lines.join("\n");
			std::fs::write(&args.file, new_content)?;
			output::success(&format!("Set {} = {}", args.key, args.value));
		}
		_ => {
			output::error("Unsupported file format");
			return Err(anyhow::anyhow!("Unsupported file format: {:?}", extension));
		}
	}

	Ok(())
}

fn set_nested_value(value: &mut toml::Value, key: &str, new_value: &str) -> anyhow::Result<()> {
	let parts: Vec<&str> = key.split('.').collect();

	if parts.is_empty() {
		return Err(anyhow::anyhow!("Invalid key"));
	}

	// Navigate to parent
	let mut current = value;
	for part in &parts[..parts.len() - 1] {
		if !current.is_table() {
			return Err(anyhow::anyhow!("Cannot navigate to key: {}", key));
		}

		current = current
			.as_table_mut()
			.unwrap()
			.entry(*part)
			.or_insert(toml::Value::Table(toml::map::Map::new()));
	}

	// Set the final value
	let final_key = parts[parts.len() - 1];

	// Try to parse as different types
	let parsed_value = if let Ok(b) = new_value.parse::<bool>() {
		toml::Value::Boolean(b)
	} else if let Ok(i) = new_value.parse::<i64>() {
		toml::Value::Integer(i)
	} else if let Ok(f) = new_value.parse::<f64>() {
		toml::Value::Float(f)
	} else {
		toml::Value::String(new_value.to_string())
	};

	if let Some(table) = current.as_table_mut() {
		table.insert(final_key.to_string(), parsed_value);
	} else {
		return Err(anyhow::anyhow!("Cannot set value at key: {}", key));
	}

	Ok(())
}

fn set_nested_json_value(
	value: &mut serde_json::Value,
	key: &str,
	new_value: &str,
) -> anyhow::Result<()> {
	let parts: Vec<&str> = key.split('.').collect();

	if parts.is_empty() {
		return Err(anyhow::anyhow!("Invalid key"));
	}

	// Navigate to parent
	let mut current = value;
	for part in &parts[..parts.len() - 1] {
		if !current.is_object() {
			return Err(anyhow::anyhow!("Cannot navigate to key: {}", key));
		}

		current = current
			.as_object_mut()
			.unwrap()
			.entry(*part)
			.or_insert(serde_json::Value::Object(serde_json::Map::new()));
	}

	// Set the final value
	let final_key = parts[parts.len() - 1];

	// Try to parse as different types
	let parsed_value = if let Ok(b) = new_value.parse::<bool>() {
		serde_json::Value::Bool(b)
	} else if let Ok(i) = new_value.parse::<i64>() {
		serde_json::Value::Number(i.into())
	} else if let Ok(f) = new_value.parse::<f64>() {
		serde_json::Number::from_f64(f)
			.map(serde_json::Value::Number)
			.unwrap_or_else(|| serde_json::Value::String(new_value.to_string()))
	} else {
		serde_json::Value::String(new_value.to_string())
	};

	if let Some(obj) = current.as_object_mut() {
		obj.insert(final_key.to_string(), parsed_value);
	} else {
		return Err(anyhow::anyhow!("Cannot set value at key: {}", key));
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;
	use tempfile::TempDir;

	#[cfg(unix)]
	#[rstest]
	#[tokio::test]
	async fn backup_file_has_restrictive_permissions() {
		// Arrange
		let tmp_dir = TempDir::new().unwrap();
		let config_path = tmp_dir.path().join("config.toml");
		std::fs::write(&config_path, "[database]\nhost = \"localhost\"\n").unwrap();

		let args = SetArgs {
			file: config_path.clone(),
			key: "database.port".to_string(),
			value: "5432".to_string(),
			create: false,
			backup: true,
		};

		// Act
		execute(args).await.unwrap();

		// Assert
		let backup_path = config_path.with_extension("toml.bak");
		assert!(backup_path.exists());
		use std::os::unix::fs::PermissionsExt;
		let metadata = std::fs::metadata(&backup_path).unwrap();
		let mode = metadata.permissions().mode() & 0o777;
		assert_eq!(mode, 0o600, "Backup file should have 0600 permissions");
	}
}
