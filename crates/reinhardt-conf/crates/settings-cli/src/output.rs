//! Output formatting utilities

use colored::*;
use serde::Serialize;
use serde_json::Value;

/// Output format for displaying values
#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
	Text,
	Json,
	Toml,
}

impl OutputFormat {
	/// Documentation for `from_str`
	///
	#[allow(dead_code)]
	pub fn from_str(s: &str) -> Self {
		match s.to_lowercase().as_str() {
			"json" => Self::Json,
			"toml" => Self::Toml,
			_ => Self::Text,
		}
	}
}
/// Print a success message
///
pub fn success(msg: &str) {
	println!("{} {}", "✓".green().bold(), msg);
}
/// Print an error message
///
pub fn error(msg: &str) {
	eprintln!("{} {}", "✗".red().bold(), msg);
}
/// Print a warning message
///
pub fn warning(msg: &str) {
	println!("{} {}", "⚠".yellow().bold(), msg);
}
/// Print an info message
///
pub fn info(msg: &str) {
	println!("{} {}", "ℹ".blue().bold(), msg);
}
/// Print a key-value pair
///
#[allow(dead_code)]
pub fn key_value(key: &str, value: &str) {
	println!("{}: {}", key.cyan().bold(), value);
}
/// Print a table header
///
#[allow(dead_code)]
pub fn table_header(columns: &[&str]) {
	let header = columns
		.iter()
		.map(|c| c.bold().to_string())
		.collect::<Vec<_>>()
		.join(" | ");
	println!("{}", header);
	println!("{}", "-".repeat(header.len()));
}
/// Print a table row
///
#[allow(dead_code)]
pub fn table_row(values: &[&str]) {
	println!("{}", values.join(" | "));
}
/// Format and print a value based on the output format
///
pub fn print_value<T: Serialize>(value: &T, format: OutputFormat) -> anyhow::Result<()> {
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
pub fn print_diff(key: &str, old_value: Option<&str>, new_value: Option<&str>) {
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
