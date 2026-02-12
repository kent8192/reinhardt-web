//! iOS Info.plist generation.

use super::config::IosConfig;

/// iOS Info.plist generator.
// Used for Info.plist generation during build process
#[allow(dead_code)]
pub(crate) struct InfoPlist;

#[allow(dead_code)]
impl InfoPlist {
	/// Generates Info.plist content from configuration.
	pub(crate) fn generate(config: &IosConfig) -> String {
		let device_family = config
			.device_family
			.iter()
			.map(|d| format!("\t\t<integer>{}</integer>", d))
			.collect::<Vec<_>>()
			.join("\n");

		let orientations = config
			.supported_orientations
			.iter()
			.map(|o| format!("\t\t<string>{}</string>", o))
			.collect::<Vec<_>>()
			.join("\n");

		let capabilities = config
			.required_capabilities
			.iter()
			.map(|c| format!("\t\t<string>{}</string>", c))
			.collect::<Vec<_>>()
			.join("\n");

		format!(
			r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleDevelopmentRegion</key>
	<string>en</string>
	<key>CFBundleDisplayName</key>
	<string>{display_name}</string>
	<key>CFBundleExecutable</key>
	<string>$(EXECUTABLE_NAME)</string>
	<key>CFBundleIdentifier</key>
	<string>{bundle_id}</string>
	<key>CFBundleInfoDictionaryVersion</key>
	<string>6.0</string>
	<key>CFBundleName</key>
	<string>$(PRODUCT_NAME)</string>
	<key>CFBundlePackageType</key>
	<string>APPL</string>
	<key>CFBundleShortVersionString</key>
	<string>{short_version}</string>
	<key>CFBundleVersion</key>
	<string>{bundle_version}</string>
	<key>LSRequiresIPhoneOS</key>
	<true/>
	<key>MinimumOSVersion</key>
	<string>{min_ios}</string>
	<key>UIDeviceFamily</key>
	<array>
{device_family}
	</array>
	<key>UILaunchStoryboardName</key>
	<string>LaunchScreen</string>
	<key>UIRequiredDeviceCapabilities</key>
	<array>
{capabilities}
	</array>
	<key>UISupportedInterfaceOrientations</key>
	<array>
{orientations}
	</array>
</dict>
</plist>
"#,
			display_name = config.display_name,
			bundle_id = config.bundle_identifier,
			short_version = config.short_version_string,
			bundle_version = config.bundle_version,
			min_ios = config.minimum_ios_version,
			device_family = device_family,
			capabilities = capabilities,
			orientations = orientations,
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_plist_generation() {
		let config = IosConfig::default();
		let plist = InfoPlist::generate(&config);

		assert!(plist.contains("com.example.reinhardt"));
		assert!(plist.contains("13.0"));
		assert!(plist.contains("CFBundleIdentifier"));
	}
}
