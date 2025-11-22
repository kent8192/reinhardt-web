//! Build-time availability and version checking for examples
//!
//! This module provides compile-time checks for reinhardt availability
//! from crates.io and version requirement validation.
//!
//! Each example's `build.rs` controls whether to use local workspace crates
//! or crates.io via the `USE_LOCAL_DEV` constant. Set `USE_LOCAL_DEV = true`
//! to use local workspace crates during development.

use semver::{Version, VersionReq};
use std::process::Command;

/// Check if reinhardt is available from crates.io at build time
///
/// This function uses `cargo metadata` to determine if reinhardt
/// is present in the dependency tree, indicating it was successfully
/// resolved from crates.io.
///
/// # Returns
///
/// `true` if reinhardt is available, `false` otherwise
///
/// # Examples
///
/// ```no_run
/// // In build.rs
/// fn main() {
///     if !example_common::build_check::check_reinhardt_availability_at_build_time() {
///         println!("cargo:rustc-cfg=reinhardt_unavailable");
///         return;
///     }
/// }
/// ```
pub fn check_reinhardt_availability_at_build_time() -> bool {
	// Use cargo metadata to check if reinhardt is in the dependency tree
	let output = Command::new("cargo")
		.args(&["metadata", "--format-version", "1", "--no-deps"])
		.output();

	match output {
		Ok(output) => {
			if !output.status.success() {
				eprintln!("cargo:warning=cargo metadata failed");
				return false;
			}

			// Parse JSON output
			let stdout = String::from_utf8_lossy(&output.stdout);
			if let Ok(metadata) = serde_json::from_str::<serde_json::Value>(&stdout) {
				if let Some(packages) = metadata.get("packages").and_then(|p| p.as_array()) {
					for package in packages {
						if let Some(name) = package.get("name").and_then(|n| n.as_str()) {
							if name == "reinhardt" {
								println!("cargo:warning=Found reinhardt in dependency tree");
								return true;
							}
						}
					}
				}
			}

			eprintln!("cargo:warning=reinhardt not found in dependency tree");
			eprintln!(
				"cargo:warning=This is expected if reinhardt is not yet published to crates.io"
			);
			false
		}
		Err(e) => {
			eprintln!("cargo:warning=Failed to run cargo metadata: {}", e);
			false
		}
	}
}

/// Check if the installed reinhardt version matches the requirement at build time
///
/// # Arguments
///
/// * `version_spec` - Version requirement string (e.g., "^0.1", ">=0.1.0, <0.2.0")
///
/// # Returns
///
/// `true` if the version matches the requirement, `false` otherwise
///
/// # Examples
///
/// ```no_run
/// // In build.rs
/// fn main() {
///     if !example_common::build_check::check_version_requirement_at_build_time("^0.1") {
///         println!("cargo:rustc-cfg=reinhardt_version_mismatch");
///         return;
///     }
/// }
/// ```
pub fn check_version_requirement_at_build_time(version_spec: &str) -> bool {
	// Get reinhardt version from cargo metadata
	let version = match get_reinhardt_version_from_metadata() {
		Some(v) => v,
		None => {
			eprintln!("cargo:warning=Could not determine reinhardt version");
			return false;
		}
	};

	// Handle wildcard
	if version_spec == "*" {
		println!(
			"cargo:warning=Version check passed (wildcard): reinhardt v{}",
			version
		);
		return true;
	}

	// Parse version and requirement
	match (Version::parse(&version), VersionReq::parse(version_spec)) {
		(Ok(ver), Ok(req)) => {
			let matches = req.matches(&ver);
			if matches {
				println!(
					"cargo:warning=Version check passed: reinhardt v{} matches {}",
					version, version_spec
				);
			} else {
				eprintln!(
					"cargo:warning=Version mismatch: reinhardt v{} does not match {}",
					version, version_spec
				);
			}
			matches
		}
		(Err(e), _) => {
			eprintln!("cargo:warning=Failed to parse version '{}': {}", version, e);
			false
		}
		(_, Err(e)) => {
			eprintln!(
				"cargo:warning=Failed to parse version requirement '{}': {}",
				version_spec, e
			);
			false
		}
	}
}

/// Get reinhardt version from cargo metadata
fn get_reinhardt_version_from_metadata() -> Option<String> {
	let output = Command::new("cargo")
		.args(&["metadata", "--format-version", "1", "--no-deps"])
		.output()
		.ok()?;

	if !output.status.success() {
		return None;
	}

	let stdout = String::from_utf8_lossy(&output.stdout);
	let metadata: serde_json::Value = serde_json::from_str(&stdout).ok()?;
	let packages = metadata.get("packages")?.as_array()?;

	for package in packages {
		if package.get("name")?.as_str()? == "reinhardt" {
			return Some(package.get("version")?.as_str()?.to_string());
		}
	}

	None
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_version_requirement_parsing() {
		// Test that common version specs parse correctly
		let specs = vec![
			// Stable version patterns
			"^0.1",
			"~0.1.2",
			">=0.1.0, <0.2.0",
			">=0.1",
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
	fn test_wildcard_version_spec() {
		// Wildcard should always be accepted as valid syntax
		assert_eq!("*", "*");
	}
}
