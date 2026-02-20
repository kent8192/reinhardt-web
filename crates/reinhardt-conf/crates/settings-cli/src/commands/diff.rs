//! Diff command

use crate::output;
use clap::Args;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Args)]
pub(crate) struct DiffArgs {
	/// First configuration file
	#[arg(value_name = "FILE1")]
	pub file1: PathBuf,

	/// Second configuration file
	#[arg(value_name = "FILE2")]
	pub file2: PathBuf,

	/// Show only differences
	#[arg(short, long)]
	pub only_differences: bool,

	/// Show values (otherwise just show keys)
	#[arg(short = 'v', long)]
	pub show_values: bool,

	/// Show sensitive values without redaction (passwords, keys, tokens)
	#[arg(long)]
	pub show_secrets: bool,
}
/// Documentation for `execute`
///
pub(crate) fn execute(args: DiffArgs) -> anyhow::Result<()> {
	output::info("Comparing configuration files");

	// Load both files
	let value1 = load_config_file(&args.file1)?;
	let value2 = load_config_file(&args.file2)?;

	// Flatten both configs into maps
	let map1 = flatten_value("", &value1);
	let map2 = flatten_value("", &value2);

	// Collect all keys
	let mut all_keys: std::collections::BTreeSet<String> = map1.keys().cloned().collect();
	all_keys.extend(map2.keys().cloned());

	let mut differences = 0;
	let mut additions = 0;
	let mut deletions = 0;

	for key in all_keys {
		let val1 = map1.get(&key);
		let val2 = map2.get(&key);
		let is_sensitive = !args.show_secrets && output::is_sensitive_key(&key);

		match (val1, val2) {
			(Some(v1), Some(v2)) if v1 == v2 => {
				// Same value
				if !args.only_differences {
					if args.show_values {
						if is_sensitive {
							output::print_diff(
								&key,
								Some(output::REDACTED),
								Some(output::REDACTED),
							);
						} else {
							output::print_diff(&key, Some(v1), Some(v2));
						}
					} else {
						output::print_diff(&key, None, None);
					}
				}
			}
			(Some(v1), Some(v2)) => {
				// Different values
				differences += 1;
				if args.show_values {
					if is_sensitive {
						output::print_diff(&key, Some(output::REDACTED), Some(output::REDACTED));
					} else {
						output::print_diff(&key, Some(v1), Some(v2));
					}
				} else {
					output::print_diff(&key, Some("*"), Some("*"));
				}
			}
			(Some(v1), None) => {
				// Only in file1 (deleted in file2)
				deletions += 1;
				if args.show_values {
					if is_sensitive {
						output::print_diff(&key, Some(output::REDACTED), None);
					} else {
						output::print_diff(&key, Some(v1), None);
					}
				} else {
					output::print_diff(&key, Some("*"), None);
				}
			}
			(None, Some(v2)) => {
				// Only in file2 (added in file2)
				additions += 1;
				if args.show_values {
					if is_sensitive {
						output::print_diff(&key, None, Some(output::REDACTED));
					} else {
						output::print_diff(&key, None, Some(v2));
					}
				} else {
					output::print_diff(&key, None, Some("*"));
				}
			}
			(None, None) => unreachable!(),
		}
	}

	// Summary
	println!();
	output::info(&format!("Total differences: {}", differences));
	output::info(&format!("Additions: {}", additions));
	output::info(&format!("Deletions: {}", deletions));

	if differences + additions + deletions == 0 {
		output::success("Files are identical");
	}

	Ok(())
}

/// Maximum configuration file size for diff command (50 MB).
const MAX_CONFIG_FILE_SIZE: u64 = 50 * 1024 * 1024;

fn load_config_file(path: &PathBuf) -> anyhow::Result<serde_json::Value> {
	// Check file existence and size in one operation (TOCTOU mitigation)
	let metadata = std::fs::metadata(path)
		.map_err(|e| anyhow::anyhow!("Cannot access file {:?}: {}", path, e))?;

	if metadata.len() > MAX_CONFIG_FILE_SIZE {
		return Err(anyhow::anyhow!(
			"Configuration file {:?} exceeds maximum size ({} bytes, limit {} bytes)",
			path,
			metadata.len(),
			MAX_CONFIG_FILE_SIZE
		));
	}

	let extension = path.extension().and_then(|s| s.to_str());

	match extension {
		Some("toml") => {
			let content = std::fs::read_to_string(path)?;
			let toml_value: toml::Value =
				toml::from_str(&content).map_err(|e| anyhow::anyhow!("Invalid TOML: {}", e))?;
			Ok(serde_json::to_value(toml_value)?)
		}
		Some("json") => {
			let content = std::fs::read_to_string(path)?;
			serde_json::from_str(&content).map_err(|e| anyhow::anyhow!("Invalid JSON: {}", e))
		}
		Some("env") => {
			let content = std::fs::read_to_string(path)?;
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
			Ok(serde_json::Value::Object(env_map))
		}
		_ => Err(anyhow::anyhow!("Unsupported file format: {:?}", extension)),
	}
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

fn flatten_value(prefix: &str, value: &serde_json::Value) -> BTreeMap<String, String> {
	let mut map = BTreeMap::new();

	match value {
		serde_json::Value::Object(obj) => {
			for (key, val) in obj {
				let new_prefix = if prefix.is_empty() {
					key.clone()
				} else {
					format!("{}.{}", prefix, key)
				};
				map.extend(flatten_value(&new_prefix, val));
			}
		}
		serde_json::Value::Array(arr) => {
			for (i, val) in arr.iter().enumerate() {
				let new_prefix = format!("{}[{}]", prefix, i);
				map.extend(flatten_value(&new_prefix, val));
			}
		}
		_ => {
			map.insert(prefix.to_string(), value_to_string(value));
		}
	}

	map
}

fn value_to_string(value: &serde_json::Value) -> String {
	match value {
		serde_json::Value::String(s) => s.clone(),
		serde_json::Value::Number(n) => n.to_string(),
		serde_json::Value::Bool(b) => b.to_string(),
		serde_json::Value::Null => "null".to_string(),
		_ => value.to_string(),
	}
}
