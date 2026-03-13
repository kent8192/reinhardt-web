use serde::{Deserialize, Serialize};

/// Frontend deployment configuration
///
/// Defines the build and deployment settings for the frontend
/// portion of a Reinhardt application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendConfig {
	/// Application name used in container labels and build artifacts
	pub app_name: String,

	/// Path to the frontend source directory relative to the project root
	pub source_dir: String,

	/// Output directory for built assets
	pub output_dir: String,

	/// Whether this frontend targets WASM compilation
	#[cfg(feature = "wasm-deploy")]
	pub wasm: bool,

	/// WASM compilation target triple (e.g., `wasm32-unknown-unknown`)
	#[cfg(feature = "wasm-deploy")]
	pub wasm_target: Option<String>,
}

impl FrontendConfig {
	/// Create a new frontend configuration with the given application name
	pub fn new(app_name: impl Into<String>) -> Self {
		Self {
			app_name: app_name.into(),
			source_dir: "frontend".to_owned(),
			output_dir: "dist".to_owned(),
			#[cfg(feature = "wasm-deploy")]
			wasm: false,
			#[cfg(feature = "wasm-deploy")]
			wasm_target: None,
		}
	}

	/// Set the source directory
	pub fn with_source_dir(mut self, dir: impl Into<String>) -> Self {
		self.source_dir = dir.into();
		self
	}

	/// Set the output directory
	pub fn with_output_dir(mut self, dir: impl Into<String>) -> Self {
		self.output_dir = dir.into();
		self
	}

	/// Enable WASM deployment with the default target
	#[cfg(feature = "wasm-deploy")]
	pub fn with_wasm(mut self) -> Self {
		self.wasm = true;
		if self.wasm_target.is_none() {
			self.wasm_target = Some("wasm32-unknown-unknown".to_owned());
		}
		self
	}

	/// Set the WASM target triple
	#[cfg(feature = "wasm-deploy")]
	pub fn with_wasm_target(mut self, target: impl Into<String>) -> Self {
		self.wasm = true;
		self.wasm_target = Some(target.into());
		self
	}
}
