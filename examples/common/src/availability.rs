//! crates.io availability check utilities

use std::sync::OnceLock;

#[cfg(feature = "build-check")]
use semver::{Version, VersionReq};

static REINHARDT_AVAILABLE: OnceLock<bool> = OnceLock::new();

/// Check if reinhardt is available from crates.io
pub fn is_reinhardt_available() -> bool {
	*REINHARDT_AVAILABLE.get_or_init(check_reinhardt_availability)
}

fn check_reinhardt_availability() -> bool {
	// 1. Check Cargo.lock existence
	let cargo_lock_exists = std::path::Path::new("../Cargo.lock").exists();
	if !cargo_lock_exists {
		eprintln!("⚠️  Cargo.lock not found - dependency resolution may have failed");
		return false;
	}

	// 2. Verify reinhardt existence with cargo tree
	use std::process::Command;

	let output = Command::new("cargo")
		.args(["tree", "-p", "reinhardt", "--depth", "0"])
		.output();

	match output {
		Ok(output) => {
			if output.status.success() {
				let stdout = String::from_utf8_lossy(&output.stdout);
				let available = stdout.contains("reinhardt");

				if !available {
					eprintln!("⚠️  reinhardt not found in dependency tree");
					eprintln!("   This is expected if reinhardt is not yet published to crates.io");
				}

				available
			} else {
				eprintln!(
					"⚠️  cargo tree failed: {}",
					String::from_utf8_lossy(&output.stderr)
				);
				false
			}
		}
		Err(e) => {
			eprintln!("⚠️  Failed to run cargo tree: {}", e);
			false
		}
	}
}

/// Availability check before tests (executed only once)
pub fn ensure_reinhardt_available() -> Result<(), String> {
	if is_reinhardt_available() {
		Ok(())
	} else {
		Err("reinhardt is not available from crates.io".to_string())
	}
}

// ============================================================================
// Build-time crates.io API availability checking
// ============================================================================

#[cfg(feature = "build-check")]
#[derive(serde::Deserialize)]
struct CratesIoResponse {
	versions: Vec<CrateVersion>,
}

#[cfg(feature = "build-check")]
#[derive(serde::Deserialize)]
struct CrateVersion {
	num: String,
	yanked: bool,
}

/// Check if a specific version of a crate exists on crates.io (for build.rs)
///
/// This function directly queries the crates.io API and checks if any version
/// matching the given requirement exists and is not yanked.
///
/// # Arguments
/// * `crate_name` - The name of the crate to check
/// * `version_req` - Version requirement string (e.g., "^0.1.0-alpha.1")
///
/// # Returns
/// * `Ok(true)` - Matching version found on crates.io
/// * `Ok(false)` - No matching version found
/// * `Err(_)` - Network error or API error
#[cfg(feature = "build-check")]
pub fn check_crates_io_availability(
	crate_name: &str,
	version_req: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
	// 1. Query crates.io API
	let url = format!("https://crates.io/api/v1/crates/{}", crate_name);

	let response: CratesIoResponse = match ureq::get(&url)
		.timeout(std::time::Duration::from_secs(10))
		.call()
	{
		Ok(resp) => resp.into_json()?,
		Err(ureq::Error::Status(404, _)) => {
			// Crate not found
			return Ok(false);
		}
		Err(e) => return Err(Box::new(e)),
	};

	// 2. Parse version requirement
	let req = VersionReq::parse(version_req)?;

	// 3. Check if any non-yanked version matches the requirement
	for ver in response.versions {
		if ver.yanked {
			continue; // Skip yanked versions
		}

		if let Ok(version) = Version::parse(&ver.num)
			&& req.matches(&version) {
				return Ok(true); // Found matching version
			}
	}

	Ok(false) // No matching version found
}

/// Verify reinhardt availability for build.rs
///
/// This is a helper function specifically for use in build.rs scripts.
/// It checks if the required version of reinhardt is available on crates.io.
///
/// # Arguments
/// * `version_req` - Version requirement string (e.g., "^0.1.0-alpha.1")
///
/// # Returns
/// * `Ok(())` - reinhardt is available
/// * `Err(String)` - reinhardt is not available (with error message)
#[cfg(feature = "build-check")]
pub fn verify_reinhardt_for_build(version_req: &str) -> Result<(), String> {
	match check_crates_io_availability("reinhardt", version_req) {
		Ok(true) => Ok(()),
		Ok(false) => Err(format!(
			"No matching version found on crates.io for requirement '{}'",
			version_req
		)),
		Err(e) => {
			// Network or API error - treat as unavailable but provide details
			Err(format!("Failed to check crates.io: {}", e))
		}
	}
}
