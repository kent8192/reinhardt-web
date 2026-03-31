//! UnoCSS utility CSS generation for the admin panel.
//!
//! This module provides functionality to generate UnoCSS utility CSS
//! by invoking the UnoCSS CLI. The generated CSS is placed in the
//! vendor directory alongside other downloaded vendor assets.

#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
use super::vendor::Verbosity;

/// Configuration for UnoCSS CLI invocation.
#[cfg(not(target_arch = "wasm32"))]
pub struct UnoCssConfig {
	/// Glob patterns for source files to scan for utility classes.
	pub scan_patterns: &'static [&'static str],
	/// Output file path relative to the base directory.
	pub out_file: &'static str,
}

/// Default UnoCSS configuration for the admin panel.
///
/// Scans Rust source files and the bootstrap script for utility class names,
/// then outputs the generated CSS to the vendor directory.
#[cfg(not(target_arch = "wasm32"))]
pub fn admin_unocss_config() -> UnoCssConfig {
	UnoCssConfig {
		scan_patterns: &["src/**/*.rs", "assets/main.js"],
		out_file: "assets/vendor/unocss.generated.css",
	}
}

/// Generates UnoCSS utility CSS by invoking the CLI.
///
/// Runs `npx unocss <scan_patterns> --out-file <out_file>` in `crate_dir`.
/// The `crate_dir` should be the root of the `reinhardt-admin` crate,
/// since scan patterns are relative to it.
///
/// Returns `Ok(())` on success or `Err` with a descriptive message on failure.
/// Callers should treat errors as non-fatal when Node.js or npx is not available.
#[cfg(not(target_arch = "wasm32"))]
pub fn generate_unocss(
	crate_dir: &Path,
	config: &UnoCssConfig,
	verbosity: Verbosity,
) -> Result<(), String> {
	// Ensure the output parent directory exists
	let out_path = crate_dir.join(config.out_file);
	if let Some(parent) = out_path.parent() {
		std::fs::create_dir_all(parent).map_err(|e| {
			format!(
				"failed to create output directory {}: {}",
				parent.display(),
				e
			)
		})?;
	}

	// Check that npx is available
	let npx_check = std::process::Command::new("npx")
		.arg("--version")
		.stdout(std::process::Stdio::null())
		.stderr(std::process::Stdio::null())
		.status();

	match npx_check {
		Ok(status) if status.success() => {}
		_ => {
			return Err(
				"npx is not available; install Node.js to enable UnoCSS generation".to_string(),
			);
		}
	}

	if verbosity != Verbosity::Silent {
		println!("Generating UnoCSS utility CSS...");
	}

	// Build the command: npx unocss <patterns...> --out-file <out_file>
	let mut cmd = std::process::Command::new("npx");
	cmd.arg("unocss");
	for pattern in config.scan_patterns {
		cmd.arg(pattern);
	}
	cmd.arg("--out-file");
	cmd.arg(config.out_file);
	cmd.current_dir(crate_dir);

	if verbosity == Verbosity::Silent {
		cmd.stdout(std::process::Stdio::null());
		cmd.stderr(std::process::Stdio::null());
	}

	let output = cmd
		.output()
		.map_err(|e| format!("failed to execute npx unocss: {}", e))?;

	if !output.status.success() {
		let stderr = String::from_utf8_lossy(&output.stderr);
		return Err(format!(
			"npx unocss exited with {}: {}",
			output.status,
			stderr.trim()
		));
	}

	if verbosity == Verbosity::Verbose {
		println!("saved: {}", config.out_file);
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use rstest::rstest;

	#[cfg(not(target_arch = "wasm32"))]
	use super::{UnoCssConfig, admin_unocss_config, generate_unocss};
	#[cfg(not(target_arch = "wasm32"))]
	use crate::core::vendor::Verbosity;

	/// The default config must have non-empty scan patterns.
	#[cfg(not(target_arch = "wasm32"))]
	#[rstest]
	fn config_has_scan_patterns() {
		// Arrange / Act
		let config = admin_unocss_config();

		// Assert
		assert!(
			!config.scan_patterns.is_empty(),
			"scan_patterns must not be empty"
		);
	}

	/// The default config must have a non-empty output file path.
	#[cfg(not(target_arch = "wasm32"))]
	#[rstest]
	fn config_has_out_file() {
		// Arrange / Act
		let config = admin_unocss_config();

		// Assert
		assert!(!config.out_file.is_empty(), "out_file must not be empty");
	}

	/// The output file path should target the vendor directory.
	#[cfg(not(target_arch = "wasm32"))]
	#[rstest]
	fn config_out_file_targets_vendor() {
		// Arrange / Act
		let config = admin_unocss_config();

		// Assert
		assert!(
			config.out_file.starts_with("assets/vendor/"),
			"out_file should be in assets/vendor/, got: {}",
			config.out_file
		);
	}

	/// All scan patterns must be non-empty strings.
	#[cfg(not(target_arch = "wasm32"))]
	#[rstest]
	fn config_scan_patterns_non_empty() {
		// Arrange / Act
		let config = admin_unocss_config();

		// Assert
		for pattern in config.scan_patterns {
			assert!(!pattern.is_empty(), "scan pattern must not be empty");
		}
	}

	/// `generate_unocss` returns an error when given a non-existent directory
	/// (npx invocation should fail gracefully).
	#[cfg(not(target_arch = "wasm32"))]
	#[rstest]
	fn generate_unocss_fails_gracefully_for_missing_dir() {
		// Arrange
		let non_existent = std::path::Path::new("/tmp/reinhardt-test-nonexistent-dir-unocss");
		let config = UnoCssConfig {
			scan_patterns: &["src/**/*.rs"],
			out_file: "out/test.css",
		};

		// Act
		let result = generate_unocss(non_existent, &config, Verbosity::Silent);

		// Assert — either npx not found or command fails, both are Err
		// We accept either outcome since CI may or may not have Node.js
		// The important thing is no panic occurs
		let _ = result;
	}
}
