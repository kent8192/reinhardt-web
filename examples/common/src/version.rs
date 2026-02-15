//! Version verification utilities

use semver::{Version, VersionReq};
use std::sync::OnceLock;

static REINHARDT_VERSION: OnceLock<Option<String>> = OnceLock::new();

/// Get reinhardt version
pub fn get_reinhardt_version() -> &'static str {
	REINHARDT_VERSION
		.get_or_init(|| {
			// Get version through multiple methods
			get_version_from_cargo_metadata().or_else(get_version_from_cargo_lock)
		})
		.as_ref()
		.map(|s| s.as_str())
		.unwrap_or("unknown")
}

/// Check version with Cargo version specifier
///
/// Supports the same syntax as Cargo.toml:
/// - "0.1.0" - Exact version
/// - "^0.1" - Caret requirement (0.1.x)
/// - "~0.1.2" - Tilde requirement (0.1.2 <= version < 0.2.0)
/// - ">=0.1, <0.2" - Range specification
/// - "*" - Wildcard (latest)
pub fn check_version(version_spec: &str) -> bool {
	let actual_version = get_reinhardt_version();

	if actual_version == "unknown" {
		eprintln!("⚠️  Warning: Could not determine reinhardt version");
		return false;
	}

	// "*" means latest version (always true)
	if version_spec == "*" {
		return true;
	}

	// Check with semantic versioning
	match (
		Version::parse(actual_version),
		VersionReq::parse(version_spec),
	) {
		(Ok(version), Ok(req)) => {
			let matches = req.matches(&version);
			if !matches {
				eprintln!(
					"   Version mismatch: required {}, found {}",
					version_spec, actual_version
				);
			}
			matches
		}
		(Err(e), _) => {
			eprintln!(
				"❌ Failed to parse actual version '{}': {}",
				actual_version, e
			);
			false
		}
		(_, Err(e)) => {
			eprintln!(
				"❌ Failed to parse version requirement '{}': {}",
				version_spec, e
			);
			false
		}
	}
}

/// Get reinhardt version from cargo metadata
fn get_version_from_cargo_metadata() -> Option<String> {
	use std::process::Command;

	let output = Command::new("cargo")
		.args(["metadata", "--format-version", "1", "--no-deps"])
		.output()
		.ok()?;

	if !output.status.success() {
		return None;
	}

	let metadata: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
	let packages = metadata.get("packages")?.as_array()?;

	for package in packages {
		if package.get("name")?.as_str()? == "reinhardt" {
			return Some(package.get("version")?.as_str()?.to_string());
		}
	}

	None
}

/// Get reinhardt version from Cargo.lock
fn get_version_from_cargo_lock() -> Option<String> {
	use std::fs;

	let cargo_lock = fs::read_to_string("../Cargo.lock").ok()?;

	// Simple string search to avoid TOML parsing
	let mut found_reinhardt = false;
	for line in cargo_lock.lines() {
		if line.trim() == "name = \"reinhardt\"" {
			found_reinhardt = true;
			continue;
		}

		if found_reinhardt {
			if let Some(version_line) = line.trim().strip_prefix("version = \"")
				&& let Some(version) = version_line.strip_suffix("\"") {
					return Some(version.to_string());
				}
			// If no version after name, search until next name
			if line.trim().starts_with("name = ") {
				found_reinhardt = false;
			}
		}
	}

	None
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_version_spec_parsing() {
		// Verify version specifier parsing works correctly
		let specs = vec![
			// Stable version patterns
			"^0.1",
			"~0.1.2",
			">=0.1, <0.2",
			"*",
			// Pre-release version patterns (alpha)
			"0.1.0-alpha.1",
			">=0.1.0-alpha.1",
			">=0.1.0-alpha.1, <0.1.0",
			"^0.1.0-alpha",
		];

		for spec in specs {
			if spec != "*" {
				assert!(
					VersionReq::parse(spec).is_ok(),
					"Failed to parse version spec: {}",
					spec
				);
			}
		}
	}

	#[test]
	fn test_wildcard_always_passes() {
		// Wildcard always returns true
		// However, it returns false if version is unknown
		if get_reinhardt_version() != "unknown" {
			assert!(check_version("*"));
		}
	}
}
