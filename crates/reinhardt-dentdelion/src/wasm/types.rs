//! WIT Type Mappings
//!
//! This module provides Rust type definitions that correspond to the WIT interface
//! types defined in `wit/dentdelion.wit`. These types are used for serialization
//! and deserialization across the WASM boundary.

use crate::capability::{Capability, PluginCapability};
use crate::error::PluginError;
use crate::metadata::PluginMetadata;

use serde::{Deserialize, Serialize};

/// WIT plugin-metadata record mapped to Rust.
///
/// This struct corresponds to the `plugin-metadata` record in the WIT interface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WitPluginMetadata {
	/// Unique identifier for the plugin
	pub name: String,
	/// Semantic version string
	pub version: String,
	/// Human-readable description
	pub description: Option<String>,
	/// List of authors
	pub authors: Vec<String>,
	/// SPDX license identifier
	pub license: Option<String>,
	/// Repository URL
	pub repository: Option<String>,
	/// Homepage URL
	pub homepage: Option<String>,
}

impl WitPluginMetadata {
	/// Convert to the internal `PluginMetadata` type.
	///
	/// # Errors
	///
	/// Returns an error if the version string is invalid.
	pub fn to_plugin_metadata(&self) -> Result<PluginMetadata, PluginError> {
		let mut builder = PluginMetadata::builder(&self.name, &self.version);

		if let Some(ref desc) = self.description {
			builder = builder.description(desc);
		}

		for author in &self.authors {
			builder = builder.author(author);
		}

		if let Some(ref license) = self.license {
			builder = builder.license(license);
		}

		if let Some(ref repo) = self.repository {
			builder = builder.repository(repo);
		}

		if let Some(ref homepage) = self.homepage {
			builder = builder.homepage(homepage);
		}

		builder.build()
	}
}

impl From<&PluginMetadata> for WitPluginMetadata {
	fn from(meta: &PluginMetadata) -> Self {
		Self {
			name: meta.name.clone(),
			version: meta.version.to_string(),
			description: if meta.description.is_empty() {
				None
			} else {
				Some(meta.description.clone())
			},
			authors: meta.authors.clone(),
			license: if meta.license.is_empty() {
				None
			} else {
				Some(meta.license.clone())
			},
			repository: meta.repository.clone(),
			homepage: meta.homepage.clone(),
		}
	}
}

/// WIT capability variant mapped to Rust.
///
/// This enum corresponds to the `capability` variant in the WIT interface.
/// Note: `Models` is intentionally excluded as it's not WASM-compatible.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum WitCapability {
	/// HTTP middleware
	Middleware,
	/// CLI commands
	Commands,
	/// REST API ViewSets
	ViewSets,
	/// Custom signals
	Signals,
	/// DI service registration
	Services,
	/// Authentication backends
	Auth,
	/// Template engines
	Templates,
	/// Static file handlers
	StaticFiles,
	/// URL routing
	Routing,
	/// Signal receivers
	SignalReceivers,
	/// HTTP handlers
	Handlers,
	/// Network/HTTP access
	NetworkAccess,
	/// Database access
	DatabaseAccess,
	/// Custom capability
	Custom(String),
}

impl WitCapability {
	/// Convert to the internal `Capability` type.
	pub fn to_capability(&self) -> Capability {
		match self {
			Self::Middleware => Capability::Core(PluginCapability::Middleware),
			Self::Commands => Capability::Core(PluginCapability::Commands),
			Self::ViewSets => Capability::Core(PluginCapability::ViewSets),
			Self::Signals => Capability::Core(PluginCapability::Signals),
			Self::Services => Capability::Core(PluginCapability::Services),
			Self::Auth => Capability::Core(PluginCapability::Auth),
			Self::Templates => Capability::Core(PluginCapability::Templates),
			Self::StaticFiles => Capability::Core(PluginCapability::StaticFiles),
			Self::Routing => Capability::Core(PluginCapability::Routing),
			Self::SignalReceivers => Capability::Core(PluginCapability::SignalReceivers),
			Self::Handlers => Capability::Core(PluginCapability::Handlers),
			Self::NetworkAccess => Capability::Core(PluginCapability::NetworkAccess),
			Self::DatabaseAccess => Capability::Core(PluginCapability::DatabaseAccess),
			Self::Custom(name) => Capability::Custom(name.clone()),
		}
	}

	/// Create from the internal `Capability` type.
	///
	/// # Returns
	///
	/// `None` if the capability is `Models` (not WASM-compatible).
	pub fn from_capability(cap: &Capability) -> Option<Self> {
		match cap {
			Capability::Core(core) => match core {
				PluginCapability::Middleware => Some(Self::Middleware),
				PluginCapability::Commands => Some(Self::Commands),
				PluginCapability::ViewSets => Some(Self::ViewSets),
				PluginCapability::Signals => Some(Self::Signals),
				PluginCapability::Services => Some(Self::Services),
				PluginCapability::Auth => Some(Self::Auth),
				PluginCapability::Templates => Some(Self::Templates),
				PluginCapability::StaticFiles => Some(Self::StaticFiles),
				PluginCapability::Routing => Some(Self::Routing),
				PluginCapability::SignalReceivers => Some(Self::SignalReceivers),
				PluginCapability::Handlers => Some(Self::Handlers),
				PluginCapability::NetworkAccess => Some(Self::NetworkAccess),
				PluginCapability::DatabaseAccess => Some(Self::DatabaseAccess),
				// BuildToolIntegration is WASM-compatible
				PluginCapability::BuildToolIntegration => {
					Some(Self::Custom("build_tool_integration".to_string()))
				}
				// These capabilities are NOT WASM-compatible (require compile-time integration or TypeScript runtime)
				PluginCapability::Models => None,
				PluginCapability::StaticSiteGeneration => None,
				PluginCapability::FrontendSsr => None,
				PluginCapability::FrontendHydration => None,
				PluginCapability::TypeScriptRuntime => None,
				PluginCapability::HotModuleReplacement => None,
			},
			Capability::Custom(name) => Some(Self::Custom(name.clone())),
		}
	}
}

/// WIT config-value variant mapped to Rust.
///
/// This enum corresponds to the `config-value` variant in the WIT interface.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ConfigValue {
	/// String value
	StringVal(String),
	/// Integer value (i64)
	IntVal(i64),
	/// Floating point value (f64)
	FloatVal(f64),
	/// Boolean value
	BoolVal(bool),
	/// List of config values
	ListVal(Vec<ConfigValue>),
	/// Map of string keys to config values
	MapVal(Vec<(String, ConfigValue)>),
}

impl ConfigValue {
	/// Try to get the value as a string.
	pub fn as_string(&self) -> Option<&str> {
		match self {
			Self::StringVal(s) => Some(s),
			_ => None,
		}
	}

	/// Try to get the value as an integer.
	pub fn as_int(&self) -> Option<i64> {
		match self {
			Self::IntVal(i) => Some(*i),
			_ => None,
		}
	}

	/// Try to get the value as a float.
	pub fn as_float(&self) -> Option<f64> {
		match self {
			Self::FloatVal(f) => Some(*f),
			_ => None,
		}
	}

	/// Try to get the value as a boolean.
	pub fn as_bool(&self) -> Option<bool> {
		match self {
			Self::BoolVal(b) => Some(*b),
			_ => None,
		}
	}

	/// Try to get the value as a list.
	pub fn as_list(&self) -> Option<&[ConfigValue]> {
		match self {
			Self::ListVal(l) => Some(l),
			_ => None,
		}
	}

	/// Try to get the value as a map.
	pub fn as_map(&self) -> Option<&[(String, ConfigValue)]> {
		match self {
			Self::MapVal(m) => Some(m),
			_ => None,
		}
	}
}

impl From<String> for ConfigValue {
	fn from(s: String) -> Self {
		Self::StringVal(s)
	}
}

impl From<&str> for ConfigValue {
	fn from(s: &str) -> Self {
		Self::StringVal(s.to_string())
	}
}

impl From<i64> for ConfigValue {
	fn from(i: i64) -> Self {
		Self::IntVal(i)
	}
}

impl From<f64> for ConfigValue {
	fn from(f: f64) -> Self {
		Self::FloatVal(f)
	}
}

impl From<bool> for ConfigValue {
	fn from(b: bool) -> Self {
		Self::BoolVal(b)
	}
}

/// WIT plugin-error record mapped to Rust.
///
/// This struct corresponds to the `plugin-error` record in the WIT interface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WitPluginError {
	/// Error code (0 = success, non-zero = error)
	pub code: u32,
	/// Human-readable error message
	pub message: String,
	/// Optional additional details
	pub details: Option<String>,
}

impl WitPluginError {
	/// Create a new error with the given code and message.
	pub fn new(code: u32, message: impl Into<String>) -> Self {
		Self {
			code,
			message: message.into(),
			details: None,
		}
	}

	/// Create a new error with details.
	pub fn with_details(code: u32, message: impl Into<String>, details: impl Into<String>) -> Self {
		Self {
			code,
			message: message.into(),
			details: Some(details.into()),
		}
	}

	/// Create an error from a `PluginError`.
	pub fn from_plugin_error(error: &PluginError) -> Self {
		Self {
			code: error_to_code(error),
			message: error.to_string(),
			details: None,
		}
	}

	/// Convert to a `PluginError`.
	pub fn to_plugin_error(&self) -> PluginError {
		PluginError::WasmExecutionError(format!(
			"[{}] {}{}",
			self.code,
			self.message,
			self.details
				.as_ref()
				.map(|d| format!(": {}", d))
				.unwrap_or_default()
		))
	}
}

/// WIT http-response record mapped to Rust.
///
/// This struct corresponds to the `http-response` record in the WIT interface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WitHttpResponse {
	/// HTTP status code
	pub status: u16,
	/// Response headers
	pub headers: Vec<(String, String)>,
	/// Response body
	pub body: Vec<u8>,
}

/// Convert a `PluginError` to an error code.
fn error_to_code(error: &PluginError) -> u32 {
	match error {
		PluginError::NotFound(_) => 1,
		PluginError::AlreadyInstalled(_) => 2,
		PluginError::AlreadyRegistered(_) => 3,
		PluginError::VersionConflict { .. } => 4,
		PluginError::MissingDependency { .. } => 5,
		PluginError::IncompatibleVersion { .. } => 6,
		PluginError::CircularDependency => 7,
		PluginError::MissingCapability { .. } => 8,
		PluginError::LifecycleError { .. } => 9,
		PluginError::InvalidStateTransition { .. } => 10,
		PluginError::WasmLoadError(_) => 100,
		PluginError::WasmExecutionError(_) => 101,
		PluginError::WasmFileNotFound(_) => 102,
		PluginError::InvalidWasmBinary => 103,
		PluginError::ExecutionTimeout => 104,
		_ => 999,
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_wit_capability_roundtrip() {
		let wit_cap = WitCapability::Middleware;
		let cap = wit_cap.to_capability();
		let back = WitCapability::from_capability(&cap).unwrap();
		assert_eq!(wit_cap, back);
	}

	#[rstest]
	fn test_models_not_wasm_compatible() {
		let cap = Capability::Core(PluginCapability::Models);
		assert!(WitCapability::from_capability(&cap).is_none());
	}

	#[rstest]
	fn test_config_value_accessors() {
		let string_val = ConfigValue::StringVal("test".to_string());
		assert_eq!(string_val.as_string(), Some("test"));
		assert!(string_val.as_int().is_none());

		let int_val = ConfigValue::IntVal(42);
		assert_eq!(int_val.as_int(), Some(42));
		assert!(int_val.as_string().is_none());

		let bool_val = ConfigValue::BoolVal(true);
		assert_eq!(bool_val.as_bool(), Some(true));
	}

	#[rstest]
	fn test_wit_plugin_error() {
		let error = WitPluginError::new(1, "Test error");
		assert_eq!(error.code, 1);
		assert_eq!(error.message, "Test error");
		assert!(error.details.is_none());

		let error_with_details = WitPluginError::with_details(2, "Error", "More info");
		assert_eq!(error_with_details.code, 2);
		assert!(error_with_details.details.is_some());
	}
}
