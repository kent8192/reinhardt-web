//! WebView abstraction for mobile platforms.
//!
//! Provides a unified interface for WebView operations across
//! Android and iOS platforms.

use crate::{MobileConfig, MobileError, MobileResult, TargetPlatform};

/// WebView configuration.
#[derive(Debug, Clone)]
pub struct WebViewConfig {
	/// Initial URL to load
	pub initial_url: Option<String>,

	/// Enable developer tools
	pub devtools: bool,

	/// Allow file access
	pub file_access: bool,

	/// Custom user agent
	pub user_agent: Option<String>,

	/// Background color
	pub background_color: Option<(u8, u8, u8, u8)>,

	/// Enable JavaScript
	pub javascript_enabled: bool,
}

impl Default for WebViewConfig {
	fn default() -> Self {
		Self {
			initial_url: None,
			devtools: cfg!(debug_assertions),
			file_access: false,
			user_agent: None,
			background_color: Some((255, 255, 255, 255)),
			javascript_enabled: true,
		}
	}
}

/// Mobile WebView wrapper.
pub struct MobileWebView {
	// Config is stored for platform-specific WebView initialization
	#[allow(dead_code)]
	config: WebViewConfig,
	platform: TargetPlatform,
	initialized: bool,
}

impl MobileWebView {
	/// Creates a new MobileWebView with the given configuration.
	pub fn new(config: WebViewConfig, platform: TargetPlatform) -> Self {
		Self {
			config,
			platform,
			initialized: false,
		}
	}

	/// Creates from mobile config.
	pub fn from_mobile_config(mobile_config: &MobileConfig) -> Self {
		Self::new(WebViewConfig::default(), mobile_config.platform)
	}

	/// Initializes the WebView.
	pub fn initialize(&mut self) -> MobileResult<()> {
		// TODO: Platform-specific initialization
		self.initialized = true;
		Ok(())
	}

	/// Returns the protocol scheme for the current platform.
	pub fn protocol_scheme(&self) -> &'static str {
		match self.platform {
			TargetPlatform::Android => "http://wry",
			TargetPlatform::Ios => "wry",
		}
	}

	/// Loads a URL in the WebView.
	pub fn load_url(&self, url: &str) -> MobileResult<()> {
		if !self.initialized {
			return Err(MobileError::WebViewInit(
				"WebView not initialized".to_string(),
			));
		}
		// TODO: Platform-specific URL loading
		let _ = url;
		Ok(())
	}

	/// Evaluates JavaScript in the WebView.
	pub fn evaluate_script(&self, script: &str) -> MobileResult<()> {
		if !self.initialized {
			return Err(MobileError::WebViewInit(
				"WebView not initialized".to_string(),
			));
		}
		// TODO: Platform-specific script evaluation
		let _ = script;
		Ok(())
	}

	/// Returns whether the WebView is initialized.
	pub fn is_initialized(&self) -> bool {
		self.initialized
	}
}
