//! Xcode project integration.
//!
//! Provides utilities for Xcode project configuration.

use super::config::IosConfig;

/// Xcode project integration helper.
// Used for Xcode project setup and documentation
#[allow(dead_code)]
pub(crate) struct XcodeProject;

#[allow(dead_code)]
impl XcodeProject {
	/// Returns instructions for Xcode project setup.
	pub(crate) fn setup_instructions() -> &'static str {
		r#"
# Xcode Project Setup for reinhardt-mobile

## Prerequisites
- macOS with Xcode 14+ installed
- Rust toolchain with iOS targets:
  ```bash
  rustup target add aarch64-apple-ios
  rustup target add aarch64-apple-ios-sim
  rustup target add x86_64-apple-ios
  ```

## Using cargo-mobile2

1. Install cargo-mobile2:
   ```bash
   cargo install --git https://github.com/tauri-apps/cargo-mobile2
   ```

2. Initialize the project:
   ```bash
   cargo mobile init
   ```

3. Build for iOS:
   ```bash
   cargo apple build
   ```

4. Run on simulator:
   ```bash
   cargo apple run
   ```

## Manual Setup

1. Create Xcode project from template
2. Add Rust library as dependency
3. Configure build phases for Rust compilation
4. Set up code signing

## Code Signing

For development:
- Use automatic signing with your Apple Developer account

For distribution:
- Configure provisioning profiles
- Set up appropriate entitlements
"#
	}

	/// Generates Xcode build settings.
	pub(crate) fn generate_build_settings(config: &IosConfig) -> String {
		format!(
			r#"// Xcode Build Settings
PRODUCT_BUNDLE_IDENTIFIER = {bundle_id}
MARKETING_VERSION = {version}
CURRENT_PROJECT_VERSION = {build}
IPHONEOS_DEPLOYMENT_TARGET = {min_ios}
TARGETED_DEVICE_FAMILY = {device_family}
CODE_SIGN_STYLE = Automatic
"#,
			bundle_id = config.bundle_identifier,
			version = config.short_version_string,
			build = config.bundle_version,
			min_ios = config.minimum_ios_version,
			device_family = config
				.device_family
				.iter()
				.map(|d| d.to_string())
				.collect::<Vec<_>>()
				.join(","),
		)
	}

	/// Returns the entitlements template.
	pub(crate) fn entitlements_template() -> &'static str {
		r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>com.apple.security.app-sandbox</key>
	<true/>
	<key>com.apple.security.network.client</key>
	<true/>
</dict>
</plist>
"#
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_setup_instructions() {
		let instructions = XcodeProject::setup_instructions();
		assert!(instructions.contains("cargo-mobile2"));
		assert!(instructions.contains("aarch64-apple-ios"));
	}

	#[test]
	fn test_build_settings_generation() {
		let config = IosConfig::default();
		let settings = XcodeProject::generate_build_settings(&config);

		assert!(settings.contains("PRODUCT_BUNDLE_IDENTIFIER"));
		assert!(settings.contains("com.example.reinhardt"));
	}
}
