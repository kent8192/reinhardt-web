//! Android manifest generation.

use super::config::AndroidConfig;

/// Android manifest generator.
// Used for AndroidManifest.xml generation during build process
#[allow(dead_code)]
pub(crate) struct AndroidManifest;

#[allow(dead_code)]
impl AndroidManifest {
	/// Generates AndroidManifest.xml content from configuration.
	pub(crate) fn generate(config: &AndroidConfig) -> String {
		let permissions = config
			.permissions
			.iter()
			.map(|p| format!(r#"	<uses-permission android:name="{}" />"#, p))
			.collect::<Vec<_>>()
			.join("\n");

		format!(
			r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
	package="{package}">

{permissions}

	<application
		android:label="{label}"
		android:icon="@mipmap/ic_launcher"
		android:roundIcon="@mipmap/ic_launcher_round"
		android:theme="@style/Theme.AppCompat.Light.NoActionBar"
		android:hardwareAccelerated="{hw_accel}"
		android:debuggable="{debug}">

		<activity
			android:name=".MainActivity"
			android:exported="true"
			android:configChanges="orientation|keyboardHidden|screenSize"
			android:windowSoftInputMode="adjustResize">
			<intent-filter>
				<action android:name="android.intent.action.MAIN" />
				<category android:name="android.intent.category.LAUNCHER" />
			</intent-filter>
		</activity>
	</application>
</manifest>
"#,
			package = config.package_name,
			permissions = permissions,
			label = config.app_label,
			hw_accel = config.hardware_accelerated,
			debug = config.debuggable,
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_manifest_generation() {
		let config = AndroidConfig::default();
		let manifest = AndroidManifest::generate(&config);

		assert!(manifest.contains("com.example.reinhardt"));
		assert!(manifest.contains("android.permission.INTERNET"));
		assert!(manifest.contains("MainActivity"));
	}
}
