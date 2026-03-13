//! WASM-specific deployment configuration and utilities
//!
//! Provides [`WasmBuildOptions`] for configuring trunk-based WASM builds
//! and utilities for detecting WASM feature requirements.

use serde::{Deserialize, Serialize};

use crate::config::FrontendConfig;

/// WASM build options for trunk-based frontend applications
///
/// Controls the Rust toolchain versions, optimization level,
/// and runtime configuration for WASM deployments.
///
/// # Examples
///
/// ```rust
/// use reinhardt_deploy::wasm::WasmBuildOptions;
///
/// let opts = WasmBuildOptions::default();
/// assert_eq!(opts.target, "wasm32-unknown-unknown");
/// assert_eq!(opts.optimization_level, "z");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmBuildOptions {
	/// WASM compilation target triple
	pub target: String,

	/// Rust toolchain version for the build stage
	pub rust_version: String,

	/// Trunk CLI version
	pub trunk_version: String,

	/// wasm-bindgen-cli version
	pub wasm_bindgen_version: String,

	/// WASM optimization level (`s` for size, `z` for aggressive size, `3` for speed)
	pub optimization_level: String,

	/// Port for the nginx server in the final stage
	pub nginx_port: u16,
}

impl Default for WasmBuildOptions {
	fn default() -> Self {
		Self {
			target: "wasm32-unknown-unknown".to_owned(),
			rust_version: "1.85".to_owned(),
			trunk_version: "0.21.12".to_owned(),
			wasm_bindgen_version: "0.2.100".to_owned(),
			optimization_level: "z".to_owned(),
			nginx_port: 80,
		}
	}
}

impl WasmBuildOptions {
	/// Create build options from a [`FrontendConfig`]
	///
	/// Uses the WASM target from the config if provided,
	/// otherwise falls back to defaults.
	pub fn from_config(config: &FrontendConfig) -> Self {
		let mut opts = Self::default();
		if let Some(ref target) = config.wasm_target {
			opts.target = target.clone();
		}
		opts
	}

	/// Set the Rust toolchain version
	pub fn with_rust_version(mut self, version: impl Into<String>) -> Self {
		self.rust_version = version.into();
		self
	}

	/// Set the trunk CLI version
	pub fn with_trunk_version(mut self, version: impl Into<String>) -> Self {
		self.trunk_version = version.into();
		self
	}

	/// Set the wasm-bindgen-cli version
	pub fn with_wasm_bindgen_version(mut self, version: impl Into<String>) -> Self {
		self.wasm_bindgen_version = version.into();
		self
	}

	/// Set the WASM optimization level
	pub fn with_optimization_level(mut self, level: impl Into<String>) -> Self {
		self.optimization_level = level.into();
		self
	}

	/// Set the nginx port
	pub fn with_nginx_port(mut self, port: u16) -> Self {
		self.nginx_port = port;
		self
	}
}

/// Detect whether a Cargo.toml content indicates WASM target usage
///
/// Checks for common WASM-related dependencies and target configurations
/// that suggest the project is a WASM frontend application.
///
/// # Examples
///
/// ```rust
/// use reinhardt_deploy::wasm::detect_wasm_project;
///
/// let cargo_toml = r#"
/// [dependencies]
/// wasm-bindgen = "0.2"
/// web-sys = "0.3"
/// "#;
/// assert!(detect_wasm_project(cargo_toml));
///
/// let non_wasm = r#"
/// [dependencies]
/// tokio = "1"
/// "#;
/// assert!(!detect_wasm_project(non_wasm));
/// ```
pub fn detect_wasm_project(cargo_toml_content: &str) -> bool {
	const WASM_INDICATORS: &[&str] = &[
		"wasm-bindgen",
		"web-sys",
		"js-sys",
		"wasm32-unknown-unknown",
		"trunk",
		"reinhardt-pages",
		"target_arch = \"wasm32\"",
	];

	WASM_INDICATORS
		.iter()
		.any(|indicator| cargo_toml_content.contains(indicator))
}
