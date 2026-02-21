//! Output formatting utilities

use colored::Colorize;
use serde::Serialize;
use serde_json::Value;

/// Output format for displaying values
#[derive(Debug, Clone, Copy)]
pub(crate) enum OutputFormat {
	Text,
	Json,
	Toml,
}

/// Redacted placeholder for sensitive values
pub(crate) const REDACTED: &str = "[REDACTED]";

/// Sensitive key name patterns used to detect credentials and secrets
const SENSITIVE_PATTERNS: &[&str] = &[
	"password",
	"passwd",
	"pwd",
	"secret",
	"token",
	"api_key",
	"apikey",
	"credential",
	"private_key",
	"connection_string",
	"database_url",
	"auth",
];

/// Check whether a key name indicates a sensitive value
///
/// Checks the last segment of a dotted key path (e.g., "database.password"
/// checks "password") against known sensitive patterns.
pub(crate) fn is_sensitive_key(key: &str) -> bool {
	let lower = key.to_lowercase();
	// Check the last segment of a dotted key path
	let segment = lower.rsplit('.').next().unwrap_or(&lower);
	SENSITIVE_PATTERNS
		.iter()
		.any(|pattern| segment.contains(pattern))
}

/// Recursively redact sensitive values in a JSON value tree
///
/// Object keys matching sensitive patterns will have their values replaced
/// with `[REDACTED]`.
pub(crate) fn redact_sensitive_values(value: &Value) -> Value {
	match value {
		Value::Object(map) => {
			let mut new_map = serde_json::Map::new();
			for (key, val) in map {
				if is_sensitive_key(key) {
					new_map.insert(key.clone(), Value::String(REDACTED.to_string()));
				} else {
					new_map.insert(key.clone(), redact_sensitive_values(val));
				}
			}
			Value::Object(new_map)
		}
		Value::Array(arr) => Value::Array(arr.iter().map(redact_sensitive_values).collect()),
		other => other.clone(),
	}
}

// Note: OutputFormat::parse() was removed as clap's ValueEnum is used instead
// See OutputFormatArg in commands/show.rs
/// Print a success message
///
pub(crate) fn success(msg: &str) {
	println!("{} {}", "✓".green().bold(), msg);
}
/// Print an error message
///
pub(crate) fn error(msg: &str) {
	eprintln!("{} {}", "✗".red().bold(), msg);
}
/// Print a warning message
///
pub(crate) fn warning(msg: &str) {
	println!("{} {}", "⚠".yellow().bold(), msg);
}
/// Print an info message
///
pub(crate) fn info(msg: &str) {
	println!("{} {}", "ℹ".blue().bold(), msg);
}
// Note: key_value(), table_header(), and table_row() were removed as unused
// These were placeholder utility functions that were never integrated into any command
/// Format and print a value based on the output format
///
pub(crate) fn print_value<T: Serialize>(value: &T, format: OutputFormat) -> anyhow::Result<()> {
	match format {
		OutputFormat::Json => {
			let json = serde_json::to_string_pretty(value)?;
			println!("{}", json);
		}
		OutputFormat::Toml => {
			let toml_str = toml::to_string_pretty(value)?;
			println!("{}", toml_str);
		}
		OutputFormat::Text => {
			// For text format, try to convert to a pretty-printed JSON first
			let json = serde_json::to_value(value)?;
			print_value_text(&json, 0);
		}
	}
	Ok(())
}

fn print_value_text(value: &Value, indent: usize) {
	let indent_str = "  ".repeat(indent);
	match value {
		Value::Object(map) => {
			for (key, val) in map {
				match val {
					Value::Object(_) | Value::Array(_) => {
						println!("{}{}:", indent_str, key.cyan().bold());
						print_value_text(val, indent + 1);
					}
					_ => {
						print!("{}{}: ", indent_str, key.cyan().bold());
						print_value_text(val, 0);
					}
				}
			}
		}
		Value::Array(arr) => {
			for val in arr {
				print!("{}- ", indent_str);
				print_value_text(val, indent + 1);
			}
		}
		Value::String(s) => println!("{}", s.green()),
		Value::Number(n) => println!("{}", n.to_string().yellow()),
		Value::Bool(b) => println!("{}", b.to_string().blue()),
		Value::Null => println!("{}", "null".dimmed()),
	}
}
/// Print a diff between two values
///
pub(crate) fn print_diff(key: &str, old_value: Option<&str>, new_value: Option<&str>) {
	match (old_value, new_value) {
		(Some(old), Some(new)) if old != new => {
			println!(
				"{} {} {} → {}",
				"~".yellow().bold(),
				key.cyan(),
				old.red().strikethrough(),
				new.green()
			);
		}
		(None, Some(new)) => {
			println!("{} {} {}", "+".green().bold(), key.cyan(), new.green());
		}
		(Some(old), None) => {
			println!(
				"{} {} {}",
				"-".red().bold(),
				key.cyan(),
				old.red().strikethrough()
			);
		}
		_ => {}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	#[case("password", true)]
	#[case("database.password", true)]
	#[case("secret_key", true)]
	#[case("api_key", true)]
	#[case("apikey", true)]
	#[case("auth_token", true)]
	#[case("private_key", true)]
	#[case("database_url", true)]
	#[case("connection_string", true)]
	#[case("credential", true)]
	#[case("DATABASE_PASSWORD", true)]
	#[case("host", false)]
	#[case("database.host", false)]
	#[case("port", false)]
	#[case("name", false)]
	#[case("debug", false)]
	fn is_sensitive_key_detects_sensitive_patterns(#[case] key: &str, #[case] expected: bool) {
		// Act
		let result = is_sensitive_key(key);

		// Assert
		assert_eq!(result, expected, "key '{}' sensitivity mismatch", key);
	}

	#[rstest]
	fn redact_sensitive_values_replaces_secret_fields() {
		// Arrange
		let value = serde_json::json!({
			"database": {
				"host": "localhost",
				"port": 5432,
				"password": "super_secret"
			},
			"api_key": "ak_12345",
			"debug": true
		});

		// Act
		let redacted = redact_sensitive_values(&value);

		// Assert
		assert_eq!(
			redacted["database"]["host"],
			serde_json::Value::String("localhost".to_string())
		);
		assert_eq!(
			redacted["database"]["port"],
			serde_json::Value::Number(5432.into())
		);
		assert_eq!(
			redacted["database"]["password"],
			serde_json::Value::String(REDACTED.to_string())
		);
		assert_eq!(
			redacted["api_key"],
			serde_json::Value::String(REDACTED.to_string())
		);
		assert_eq!(redacted["debug"], serde_json::Value::Bool(true));
	}

	#[rstest]
	fn redact_sensitive_values_handles_nested_objects() {
		// Arrange
		let value = serde_json::json!({
			"services": {
				"db": {
					"connection_string": "postgres://user:pass@host/db"
				}
			}
		});

		// Act
		let redacted = redact_sensitive_values(&value);

		// Assert
		assert_eq!(
			redacted["services"]["db"]["connection_string"],
			serde_json::Value::String(REDACTED.to_string())
		);
	}
}
