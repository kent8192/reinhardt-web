//! Platform abstraction for reinhardt-mobile.
//!
//! Provides traits and types for platform-specific mobile implementations.

mod android;
mod ios;

pub use android::AndroidPlatform;
pub use ios::IosPlatform;

use crate::{MobileConfig, MobileResult};

/// Trait for mobile platform implementations.
pub trait MobilePlatform {
	/// The configuration type for this platform.
	type Config;

	/// Returns the protocol scheme for this platform.
	///
	/// - Android: `http://wry`
	/// - iOS: `wry`
	fn protocol_scheme(&self) -> &'static str;

	/// Returns the platform name.
	fn name(&self) -> &'static str;

	/// Initializes the platform with the given configuration.
	fn initialize(&mut self, config: &MobileConfig) -> MobileResult<()>;

	/// Returns whether the platform is available on the current system.
	fn is_available() -> bool;
}

/// Asset loader trait for loading assets from the bundle.
pub trait AssetLoader {
	/// Loads an asset by path.
	fn load(&self, path: &str) -> MobileResult<Vec<u8>>;

	/// Returns the base URL for assets.
	fn base_url(&self) -> String;

	/// Checks if an asset exists.
	fn exists(&self, path: &str) -> bool;
}
