//! Plugin system error types.
//!
//! This module defines all error types used throughout the Dentdelion plugin system.

use semver::Version;
use thiserror::Error;

/// Result type for plugin operations.
pub type PluginResult<T> = Result<T, PluginError>;

/// Plugin system errors.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PluginError {
	/// Plugin not found in registry.
	#[error("plugin not found: {0}")]
	NotFound(String),

	/// Plugin already installed.
	#[error("plugin already installed: {0}")]
	AlreadyInstalled(String),

	/// Plugin already registered in registry.
	#[error("plugin already registered: {0}")]
	AlreadyRegistered(String),

	/// Version conflict between plugins.
	#[error("version conflict for plugin '{plugin}': existing {existing}, new {new}")]
	VersionConflict {
		/// Plugin name.
		plugin: String,
		/// Existing version.
		existing: Version,
		/// New (conflicting) version.
		new: Version,
	},

	/// Missing required dependency.
	#[error("plugin '{plugin}' requires missing dependency: {dependency}")]
	MissingDependency {
		/// Plugin that has the dependency.
		plugin: String,
		/// Missing dependency name.
		dependency: String,
	},

	/// Incompatible dependency version.
	#[error("plugin '{plugin}' requires {dependency} {required}, but {actual} is installed")]
	IncompatibleVersion {
		/// Plugin that has the requirement.
		plugin: String,
		/// Dependency name.
		dependency: String,
		/// Required version specification.
		required: String,
		/// Actual installed version.
		actual: Version,
	},

	/// Circular dependency detected.
	#[error("circular dependency detected in plugin graph")]
	CircularDependency,

	/// Missing required capability.
	#[error("plugin '{plugin}' requires capability '{capability}' which is not provided")]
	MissingCapability {
		/// Plugin that requires the capability.
		plugin: String,
		/// Missing capability name.
		capability: String,
	},

	/// Plugin lifecycle error.
	#[error("plugin '{plugin}' failed during {phase}: {message}")]
	LifecycleError {
		/// Plugin name.
		plugin: String,
		/// Lifecycle phase (load, enable, disable, unload).
		phase: String,
		/// Error message.
		message: String,
	},

	/// Invalid plugin state transition.
	#[error(
		"invalid state transition for plugin '{plugin}': cannot transition from {from:?} to {to:?}"
	)]
	InvalidStateTransition {
		/// Plugin name.
		plugin: String,
		/// Current state.
		from: PluginState,
		/// Attempted target state.
		to: PluginState,
	},

	/// Manifest parsing error.
	#[error("failed to parse manifest: {0}")]
	ManifestParseError(String),

	/// Manifest file not found.
	#[error("manifest file not found: {0}")]
	ManifestNotFound(String),

	/// Invalid manifest format.
	#[error("invalid manifest format: {0}")]
	InvalidManifest(String),

	/// IO error.
	#[error("IO error: {0}")]
	Io(#[from] std::io::Error),

	/// TOML parsing error.
	#[error("TOML parse error: {0}")]
	TomlParse(String),

	/// TOML editing error.
	#[error("TOML edit error: {0}")]
	TomlEdit(String),

	/// Invalid version string.
	#[error("invalid version: {0}")]
	InvalidVersion(String),

	/// Invalid version requirement.
	#[error("invalid version requirement: {0}")]
	InvalidVersionReq(String),

	/// WASM loading error (when wasm feature is enabled).
	#[cfg(feature = "wasm")]
	#[error("WASM load error: {0}")]
	WasmLoadError(String),

	/// WASM execution error (when wasm feature is enabled).
	#[cfg(feature = "wasm")]
	#[error("WASM execution error: {0}")]
	WasmExecutionError(String),

	/// WASM file not found.
	#[cfg(feature = "wasm")]
	#[error("WASM file not found for version: {0}")]
	WasmFileNotFound(String),

	/// Invalid WASM binary.
	#[cfg(feature = "wasm")]
	#[error("invalid WASM binary: missing magic bytes")]
	InvalidWasmBinary,

	/// Plugin execution timeout.
	#[error("plugin execution timeout")]
	ExecutionTimeout,

	/// Generic plugin error with custom message.
	#[error("{0}")]
	Custom(String),

	/// Configuration error.
	#[error("configuration error: {0}")]
	ConfigError(String),

	/// Network error (for crates.io API calls).
	#[cfg(feature = "cli")]
	#[error("network error: {0}")]
	Network(String),

	/// Runtime execution error.
	#[error("runtime error: {0}")]
	RuntimeError(String),
}

/// Plugin lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginState {
	/// Plugin is registered but not yet loaded.
	Registered,
	/// Plugin is loaded but not enabled.
	Loaded,
	/// Plugin is enabled and active.
	Enabled,
	/// Plugin is disabled but still loaded.
	Disabled,
	/// Plugin failed during a lifecycle phase.
	Failed,
}

impl std::fmt::Display for PluginState {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Registered => write!(f, "registered"),
			Self::Loaded => write!(f, "loaded"),
			Self::Enabled => write!(f, "enabled"),
			Self::Disabled => write!(f, "disabled"),
			Self::Failed => write!(f, "failed"),
		}
	}
}

impl From<toml::de::Error> for PluginError {
	fn from(err: toml::de::Error) -> Self {
		Self::TomlParse(err.to_string())
	}
}

impl From<toml_edit::TomlError> for PluginError {
	fn from(err: toml_edit::TomlError) -> Self {
		Self::TomlEdit(err.to_string())
	}
}

impl From<semver::Error> for PluginError {
	fn from(err: semver::Error) -> Self {
		Self::InvalidVersion(err.to_string())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	// ==========================================================================
	// PluginError Display Tests
	// ==========================================================================

	#[test]
	fn test_not_found_error_display() {
		let err = PluginError::NotFound("test-delion".to_string());
		assert_eq!(err.to_string(), "plugin not found: test-delion");
	}

	#[test]
	fn test_already_installed_error_display() {
		let err = PluginError::AlreadyInstalled("auth-delion".to_string());
		assert_eq!(err.to_string(), "plugin already installed: auth-delion");
	}

	#[test]
	fn test_already_registered_error_display() {
		let err = PluginError::AlreadyRegistered("cache-delion".to_string());
		assert_eq!(err.to_string(), "plugin already registered: cache-delion");
	}

	#[test]
	fn test_version_conflict_error_display() {
		let err = PluginError::VersionConflict {
			plugin: "test-delion".to_string(),
			existing: Version::new(1, 0, 0),
			new: Version::new(2, 0, 0),
		};
		let msg = err.to_string();
		assert!(msg.contains("version conflict"));
		assert!(msg.contains("test-delion"));
		assert!(msg.contains("1.0.0"));
		assert!(msg.contains("2.0.0"));
	}

	#[test]
	fn test_missing_dependency_error_display() {
		let err = PluginError::MissingDependency {
			plugin: "auth-delion".to_string(),
			dependency: "core-delion".to_string(),
		};
		let msg = err.to_string();
		assert!(msg.contains("auth-delion"));
		assert!(msg.contains("core-delion"));
		assert!(msg.contains("missing dependency"));
	}

	#[test]
	fn test_incompatible_version_error_display() {
		let err = PluginError::IncompatibleVersion {
			plugin: "auth-delion".to_string(),
			dependency: "core-delion".to_string(),
			required: "^1.0.0".to_string(),
			actual: Version::new(2, 0, 0),
		};
		let msg = err.to_string();
		assert!(msg.contains("auth-delion"));
		assert!(msg.contains("core-delion"));
		assert!(msg.contains("^1.0.0"));
		assert!(msg.contains("2.0.0"));
	}

	#[test]
	fn test_circular_dependency_error_display() {
		let err = PluginError::CircularDependency;
		assert_eq!(
			err.to_string(),
			"circular dependency detected in plugin graph"
		);
	}

	#[test]
	fn test_missing_capability_error_display() {
		let err = PluginError::MissingCapability {
			plugin: "auth-delion".to_string(),
			capability: "Database".to_string(),
		};
		let msg = err.to_string();
		assert!(msg.contains("auth-delion"));
		assert!(msg.contains("Database"));
		assert!(msg.contains("capability"));
	}

	#[test]
	fn test_lifecycle_error_display() {
		let err = PluginError::LifecycleError {
			plugin: "test-delion".to_string(),
			phase: "load".to_string(),
			message: "initialization failed".to_string(),
		};
		let msg = err.to_string();
		assert!(msg.contains("test-delion"));
		assert!(msg.contains("load"));
		assert!(msg.contains("initialization failed"));
	}

	#[test]
	fn test_invalid_state_transition_error_display() {
		let err = PluginError::InvalidStateTransition {
			plugin: "test-delion".to_string(),
			from: PluginState::Enabled,
			to: PluginState::Registered,
		};
		let msg = err.to_string();
		assert!(msg.contains("test-delion"));
		assert!(msg.contains("invalid state transition"));
	}

	#[test]
	fn test_manifest_parse_error_display() {
		let err = PluginError::ManifestParseError("invalid TOML".to_string());
		assert_eq!(err.to_string(), "failed to parse manifest: invalid TOML");
	}

	#[test]
	fn test_manifest_not_found_error_display() {
		let err = PluginError::ManifestNotFound("/path/to/dentdelion.toml".to_string());
		assert_eq!(
			err.to_string(),
			"manifest file not found: /path/to/dentdelion.toml"
		);
	}

	#[test]
	fn test_invalid_manifest_error_display() {
		let err = PluginError::InvalidManifest("missing plugins section".to_string());
		assert_eq!(
			err.to_string(),
			"invalid manifest format: missing plugins section"
		);
	}

	#[test]
	fn test_toml_parse_error_display() {
		let err = PluginError::TomlParse("unexpected token".to_string());
		assert_eq!(err.to_string(), "TOML parse error: unexpected token");
	}

	#[test]
	fn test_toml_edit_error_display() {
		let err = PluginError::TomlEdit("edit failed".to_string());
		assert_eq!(err.to_string(), "TOML edit error: edit failed");
	}

	#[test]
	fn test_invalid_version_error_display() {
		let err = PluginError::InvalidVersion("not a valid semver".to_string());
		assert_eq!(err.to_string(), "invalid version: not a valid semver");
	}

	#[test]
	fn test_invalid_version_req_error_display() {
		let err = PluginError::InvalidVersionReq("invalid requirement".to_string());
		assert_eq!(
			err.to_string(),
			"invalid version requirement: invalid requirement"
		);
	}

	#[test]
	fn test_execution_timeout_error_display() {
		let err = PluginError::ExecutionTimeout;
		assert_eq!(err.to_string(), "plugin execution timeout");
	}

	#[test]
	fn test_custom_error_display() {
		let err = PluginError::Custom("custom error message".to_string());
		assert_eq!(err.to_string(), "custom error message");
	}

	#[cfg(feature = "cli")]
	#[test]
	fn test_network_error_display() {
		let err = PluginError::Network("connection refused".to_string());
		assert_eq!(err.to_string(), "network error: connection refused");
	}

	// ==========================================================================
	// PluginState Tests
	// ==========================================================================

	#[test]
	fn test_plugin_state_display() {
		assert_eq!(PluginState::Registered.to_string(), "registered");
		assert_eq!(PluginState::Loaded.to_string(), "loaded");
		assert_eq!(PluginState::Enabled.to_string(), "enabled");
		assert_eq!(PluginState::Disabled.to_string(), "disabled");
		assert_eq!(PluginState::Failed.to_string(), "failed");
	}

	#[test]
	fn test_plugin_state_equality() {
		assert_eq!(PluginState::Enabled, PluginState::Enabled);
		assert_eq!(PluginState::Disabled, PluginState::Disabled);
		assert_ne!(PluginState::Enabled, PluginState::Disabled);
		assert_ne!(PluginState::Registered, PluginState::Loaded);
	}

	#[test]
	fn test_plugin_state_clone() {
		let state = PluginState::Enabled;
		let cloned = state;
		assert_eq!(state, cloned);
	}

	#[test]
	fn test_plugin_state_debug() {
		let debug_str = format!("{:?}", PluginState::Enabled);
		assert_eq!(debug_str, "Enabled");
	}

	#[test]
	fn test_plugin_state_hash() {
		use std::collections::HashSet;
		let mut set = HashSet::new();
		set.insert(PluginState::Enabled);
		set.insert(PluginState::Disabled);
		set.insert(PluginState::Enabled); // duplicate

		assert_eq!(set.len(), 2);
		assert!(set.contains(&PluginState::Enabled));
		assert!(set.contains(&PluginState::Disabled));
	}

	// ==========================================================================
	// From Trait Conversion Tests
	// ==========================================================================

	#[test]
	fn test_io_error_conversion() {
		let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
		let plugin_err: PluginError = io_err.into();
		assert!(matches!(plugin_err, PluginError::Io(_)));
		assert!(plugin_err.to_string().contains("file not found"));
	}

	#[test]
	fn test_semver_error_conversion() {
		let result: Result<Version, _> = "invalid".parse();
		let semver_err = result.unwrap_err();
		let plugin_err: PluginError = semver_err.into();
		assert!(matches!(plugin_err, PluginError::InvalidVersion(_)));
	}

	// ==========================================================================
	// PluginError Debug Tests
	// ==========================================================================

	#[test]
	fn test_plugin_error_debug() {
		let err = PluginError::NotFound("test".to_string());
		let debug_str = format!("{:?}", err);
		assert!(debug_str.contains("NotFound"));
		assert!(debug_str.contains("test"));
	}

	#[test]
	fn test_version_conflict_error_debug() {
		let err = PluginError::VersionConflict {
			plugin: "test".to_string(),
			existing: Version::new(1, 0, 0),
			new: Version::new(2, 0, 0),
		};
		let debug_str = format!("{:?}", err);
		assert!(debug_str.contains("VersionConflict"));
	}
}
