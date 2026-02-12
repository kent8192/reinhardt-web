//! iOS platform implementation.

mod config;
mod plist;
mod xcode;

pub(super) use config::IosConfig;

use crate::{MobileConfig, MobileError, MobileResult};

/// iOS platform implementation.
pub struct IosPlatform {
	config: Option<IosConfig>,
	initialized: bool,
}

impl IosPlatform {
	/// Creates a new iOS platform instance.
	pub fn new() -> Self {
		Self {
			config: None,
			initialized: false,
		}
	}
}

impl Default for IosPlatform {
	fn default() -> Self {
		Self::new()
	}
}

impl super::MobilePlatform for IosPlatform {
	type Config = IosConfig;

	fn protocol_scheme(&self) -> &'static str {
		// iOS uses wry:// scheme
		"wry"
	}

	fn name(&self) -> &'static str {
		"ios"
	}

	fn initialize(&mut self, config: &MobileConfig) -> MobileResult<()> {
		self.config = Some(IosConfig::from_mobile_config(config));
		self.initialized = true;
		Ok(())
	}

	fn is_available() -> bool {
		cfg!(target_os = "ios")
	}
}

/// iOS asset loader implementation.
// Will be used when asset loading is fully implemented
#[allow(dead_code)]
pub(super) struct IosAssetLoader {
	bundle_path: String,
}

#[allow(dead_code)]
impl IosAssetLoader {
	/// Creates a new iOS asset loader.
	pub(super) fn new(bundle_path: String) -> Self {
		Self { bundle_path }
	}
}

impl super::AssetLoader for IosAssetLoader {
	fn load(&self, path: &str) -> MobileResult<Vec<u8>> {
		// TODO: Implement actual asset loading from bundle
		Err(MobileError::AssetLoad {
			path: path.to_string(),
			source: std::io::Error::new(std::io::ErrorKind::NotFound, "Not implemented"),
		})
	}

	fn base_url(&self) -> String {
		format!("wry://{}", self.bundle_path)
	}

	fn exists(&self, _path: &str) -> bool {
		// TODO: Implement asset existence check
		false
	}
}
