//! Android platform implementation.

mod config;
mod jni;
mod manifest;

pub(super) use config::AndroidConfig;

use crate::{MobileConfig, MobileError, MobileResult};

/// Android platform implementation.
pub struct AndroidPlatform {
	config: Option<AndroidConfig>,
	initialized: bool,
}

impl AndroidPlatform {
	/// Creates a new Android platform instance.
	pub fn new() -> Self {
		Self {
			config: None,
			initialized: false,
		}
	}
}

impl Default for AndroidPlatform {
	fn default() -> Self {
		Self::new()
	}
}

impl super::MobilePlatform for AndroidPlatform {
	type Config = AndroidConfig;

	fn protocol_scheme(&self) -> &'static str {
		// Android uses HTTP protocol with wry domain
		"http://wry"
	}

	fn name(&self) -> &'static str {
		"android"
	}

	fn initialize(&mut self, config: &MobileConfig) -> MobileResult<()> {
		self.config = Some(AndroidConfig::from_mobile_config(config));
		self.initialized = true;
		Ok(())
	}

	fn is_available() -> bool {
		cfg!(target_os = "android")
	}
}

/// Android asset loader implementation.
// Will be used when asset loading is fully implemented
#[allow(dead_code)]
pub(super) struct AndroidAssetLoader {
	base_path: String,
}

#[allow(dead_code)]
impl AndroidAssetLoader {
	/// Creates a new Android asset loader.
	pub(super) fn new(base_path: String) -> Self {
		Self { base_path }
	}
}

impl super::AssetLoader for AndroidAssetLoader {
	fn load(&self, path: &str) -> MobileResult<Vec<u8>> {
		// TODO: Implement actual asset loading via JNI
		Err(MobileError::AssetLoad {
			path: path.to_string(),
			source: std::io::Error::new(std::io::ErrorKind::NotFound, "Not implemented"),
		})
	}

	fn base_url(&self) -> String {
		format!("http://wry.{}", self.base_path)
	}

	fn exists(&self, _path: &str) -> bool {
		// TODO: Implement asset existence check
		false
	}
}
