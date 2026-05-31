//! Settings fragment for the deeplink subsystem.
//!
//! This module defines the [`DeeplinkSettings`] fragment, which is the
//! settings-based replacement for the deprecated [`crate::config::DeeplinkConfig`].
//!
//! The fragment integrates with `reinhardt-conf`'s layered settings system and
//! can be loaded from configuration files, environment variables, and other
//! sources via the `#[settings]` macro.

// The conversions in this module bridge the `#[settings]` fragments into the
// deprecated `DeeplinkConfig` for backward compatibility during the 0.2 window.
// Remove this allowance once `DeeplinkConfig` is deleted.
#![allow(deprecated)]

use reinhardt_core::macros::settings;
use serde::{Deserialize, Serialize};

use crate::config::{
	AndroidConfig, AndroidConfigBuilder, CustomScheme, DeeplinkConfig, IosConfig, IosConfigBuilder,
};

/// iOS Universal Links configuration for the [`DeeplinkSettings`] fragment.
///
/// This is a nested value object embedded in [`DeeplinkSettings`]; it is never
/// loaded from its own configuration section. Its fields mirror the inputs
/// accepted by [`IosConfigBuilder`], and it is converted into an [`IosConfig`]
/// through that builder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct IosSettings {
	/// The app identifier in the format `TEAMID.BUNDLEID`.
	#[serde(default)]
	pub app_id: String,
	/// URL paths that should open the app.
	#[serde(default)]
	pub paths: Vec<String>,
	/// URL paths that should NOT open the app.
	#[serde(default)]
	pub exclude_paths: Vec<String>,
	/// App Clip bundle identifiers (usually ending in `.Clip`).
	#[serde(default)]
	pub app_clips: Vec<String>,
	/// Whether to enable web credentials (password autofill) for the app id.
	#[serde(default)]
	pub web_credentials: bool,
}

/// Android App Links configuration for the [`DeeplinkSettings`] fragment.
///
/// This is a nested value object embedded in [`DeeplinkSettings`]; it is never
/// loaded from its own configuration section. Its fields mirror the inputs
/// accepted by [`AndroidConfigBuilder`], and it is converted into an
/// [`AndroidConfig`] through that builder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AndroidSettings {
	/// The Android app package name (e.g., `com.example.app`).
	#[serde(default)]
	pub package_name: String,
	/// SHA-256 certificate fingerprints.
	#[serde(default)]
	pub sha256_cert_fingerprints: Vec<String>,
}

/// A single custom URL scheme for the [`DeeplinkSettings`] fragment.
///
/// This is a nested value object embedded in [`DeeplinkSettings`]; it is never
/// loaded from its own configuration section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CustomSchemeSettings {
	/// The scheme name (e.g., `myapp`).
	#[serde(default)]
	pub name: String,
	/// Allowed hosts for the scheme.
	#[serde(default)]
	pub hosts: Vec<String>,
	/// Allowed paths for the scheme.
	#[serde(default)]
	pub paths: Vec<String>,
}

/// Settings fragment for configuring the deeplink subsystem.
///
/// This fragment unifies the iOS, Android, and custom-scheme configuration into
/// a single loadable section and replaces the deprecated
/// [`crate::config::DeeplinkConfig`].
#[settings(fragment = true, section = "deeplink_app")]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeeplinkSettings {
	/// iOS Universal Links configuration (optional).
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub ios: Option<IosSettings>,
	/// Android App Links configuration (optional).
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub android: Option<AndroidSettings>,
	/// Custom URL scheme configurations.
	#[serde(default)]
	pub custom_schemes: Vec<CustomSchemeSettings>,
}

impl IosSettings {
	/// Builds an [`IosConfig`] from these settings via [`IosConfigBuilder`].
	fn to_config(&self) -> IosConfig {
		let path_refs: Vec<&str> = self.paths.iter().map(String::as_str).collect();
		let exclude_refs: Vec<&str> = self.exclude_paths.iter().map(String::as_str).collect();

		let mut builder = IosConfigBuilder::new()
			.app_id(self.app_id.clone())
			.paths(&path_refs)
			.exclude_paths(&exclude_refs);

		for app_clip in &self.app_clips {
			builder = builder.app_clip(app_clip.clone());
		}

		if self.web_credentials {
			builder = builder.with_web_credentials();
		}

		builder.build()
	}
}

impl AndroidSettings {
	/// Builds an [`AndroidConfig`] from these settings via
	/// [`AndroidConfigBuilder`].
	///
	/// This uses the builder's unchecked path; fingerprint and package-name
	/// validation are performed by the settings/configuration layer rather than
	/// at conversion time.
	fn to_config(&self) -> AndroidConfig {
		let fingerprint_refs: Vec<&str> = self
			.sha256_cert_fingerprints
			.iter()
			.map(String::as_str)
			.collect();

		AndroidConfigBuilder::new()
			.package_name(self.package_name.clone())
			.sha256_fingerprints(&fingerprint_refs)
			.build_unchecked()
	}
}

impl From<&CustomSchemeSettings> for CustomScheme {
	fn from(settings: &CustomSchemeSettings) -> Self {
		CustomScheme {
			name: settings.name.clone(),
			hosts: settings.hosts.clone(),
			paths: settings.paths.clone(),
		}
	}
}

impl From<&DeeplinkSettings> for DeeplinkConfig {
	fn from(settings: &DeeplinkSettings) -> Self {
		DeeplinkConfig {
			ios: settings.ios.as_ref().map(IosSettings::to_config),
			android: settings.android.as_ref().map(AndroidSettings::to_config),
			custom_schemes: settings
				.custom_schemes
				.iter()
				.map(CustomScheme::from)
				.collect(),
		}
	}
}

/// Creates a deeplink configuration from the settings fragment.
pub fn create_deeplink_config_from_settings(settings: &DeeplinkSettings) -> DeeplinkConfig {
	DeeplinkConfig::from(settings)
}

#[cfg(test)]
mod tests {
	use rstest::rstest;

	use super::*;

	const VALID_FINGERPRINT: &str = "FA:C6:17:45:DC:09:03:78:6F:B9:ED:E6:2A:96:2B:39:9F:73:48:F0:BB:6F:89:9B:83:32:66:75:91:03:3B:9C";

	#[rstest]
	fn test_default_settings() {
		let settings = DeeplinkSettings::default();
		assert!(settings.ios.is_none());
		assert!(settings.android.is_none());
		assert!(settings.custom_schemes.is_empty());
	}

	#[rstest]
	fn test_default_settings_bridge_to_empty_config() {
		let settings = DeeplinkSettings::default();
		let config = create_deeplink_config_from_settings(&settings);
		assert!(!config.is_configured());
	}

	#[rstest]
	fn test_ios_bridge() {
		let settings = DeeplinkSettings {
			ios: Some(IosSettings {
				app_id: "TEAM.com.example".to_string(),
				paths: vec!["/products/*".to_string()],
				exclude_paths: vec!["/api/*".to_string()],
				app_clips: vec!["TEAM.com.example.Clip".to_string()],
				web_credentials: true,
			}),
			..Default::default()
		};
		let config = DeeplinkConfig::from(&settings);
		assert!(config.has_ios());
		let ios = config.ios.expect("ios config present");
		let json = serde_json::to_string(&ios).expect("serialize ios config");
		assert!(json.contains("TEAM.com.example"));
		assert!(json.contains("/products/*"));
		assert!(json.contains("/api/*"));
		assert!(json.contains("webcredentials"));
		assert!(json.contains("appclips"));
	}

	#[rstest]
	fn test_android_bridge() {
		let settings = DeeplinkSettings {
			android: Some(AndroidSettings {
				package_name: "com.example.app".to_string(),
				sha256_cert_fingerprints: vec![VALID_FINGERPRINT.to_string()],
			}),
			..Default::default()
		};
		let config = DeeplinkConfig::from(&settings);
		assert!(config.has_android());
		let android = config.android.expect("android config present");
		assert_eq!(android.statements.len(), 1);
		assert_eq!(android.statements[0].target.package_name, "com.example.app");
		assert_eq!(
			android.statements[0].target.sha256_cert_fingerprints,
			vec![VALID_FINGERPRINT.to_string()]
		);
	}

	#[rstest]
	fn test_custom_schemes_bridge() {
		let settings = DeeplinkSettings {
			custom_schemes: vec![CustomSchemeSettings {
				name: "myapp".to_string(),
				hosts: vec!["open".to_string()],
				paths: vec!["/products/*".to_string()],
			}],
			..Default::default()
		};
		let config = DeeplinkConfig::from(&settings);
		assert!(config.has_custom_schemes());
		assert_eq!(config.custom_schemes.len(), 1);
		assert_eq!(config.custom_schemes[0].name, "myapp");
		assert_eq!(config.custom_schemes[0].hosts, vec!["open".to_string()]);
		assert_eq!(
			config.custom_schemes[0].paths,
			vec!["/products/*".to_string()]
		);
	}

	#[rstest]
	fn test_settings_roundtrip_serde() {
		let settings = DeeplinkSettings {
			ios: Some(IosSettings {
				app_id: "TEAM.com.example".to_string(),
				paths: vec!["/".to_string()],
				exclude_paths: Vec::new(),
				app_clips: Vec::new(),
				web_credentials: false,
			}),
			android: None,
			custom_schemes: Vec::new(),
		};
		let json = serde_json::to_string(&settings).expect("serialize settings");
		let parsed: DeeplinkSettings = serde_json::from_str(&json).expect("deserialize settings");
		assert_eq!(settings, parsed);
	}
}
