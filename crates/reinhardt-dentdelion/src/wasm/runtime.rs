//! WASM Runtime Management
//!
//! This module provides the `WasmRuntime` struct for managing the wasmtime
//! Engine, Store, and Linker. It handles Component Model setup and resource limits.
//!
//! # Constraints
//!
//! - Memory limit: 128MB default (configurable)
//! - Execution timeout: 30 seconds default (configurable)

// Generate Rust bindings from WIT definition
// Configure async support for both imports and exports:
// - exports: async enables .await on plugin's exported functions
// - imports: async | trappable enables async host functions and safe ResourceTable interaction
wasmtime::component::bindgen!({
	world: "dentdelion-plugin",
	path: "wit",
	exports: { default: async },
	imports: { default: async | trappable },
});

// Re-export generated module for use in other modules
// Currently accessed via full path (crate::wasm::runtime::reinhardt::dentdelion)
// but this re-export enables shorter import paths in the future
#[allow(unused_imports)]
pub(super) use self::reinhardt::dentdelion;

use crate::error::{PluginError, PluginResult};

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use wasmtime::component::{Component, HasSelf, Linker};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::p2;

use super::host::HostState;

/// Default memory limit in megabytes.
pub(super) const DEFAULT_MEMORY_LIMIT_MB: u32 = 128;

/// Default execution timeout in seconds.
pub(super) const DEFAULT_TIMEOUT_SECS: u32 = 30;

/// Configuration for the WASM runtime.
#[derive(Debug, Clone)]
pub struct WasmRuntimeConfig {
	/// Memory limit in megabytes.
	pub memory_limit_mb: u32,
	/// Execution timeout.
	pub timeout: Duration,
	/// Enable async support.
	pub async_support: bool,
	/// Enable fuel-based metering for CPU limits.
	pub fuel_metering: bool,
	/// Initial fuel amount (if fuel_metering is enabled).
	pub initial_fuel: u64,
}

impl Default for WasmRuntimeConfig {
	fn default() -> Self {
		Self {
			memory_limit_mb: DEFAULT_MEMORY_LIMIT_MB,
			timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS as u64),
			async_support: true,
			fuel_metering: true,
			initial_fuel: 100_000_000, // ~100M instructions
		}
	}
}

/// Builder for `WasmRuntimeConfig`.
#[derive(Debug, Default)]
pub struct WasmRuntimeConfigBuilder {
	config: WasmRuntimeConfig,
}

impl WasmRuntimeConfigBuilder {
	/// Create a new builder with default configuration.
	pub fn new() -> Self {
		Self::default()
	}

	/// Set the memory limit in megabytes.
	pub fn memory_limit_mb(mut self, mb: u32) -> Self {
		self.config.memory_limit_mb = mb;
		self
	}

	/// Set the execution timeout.
	pub fn timeout(mut self, timeout: Duration) -> Self {
		self.config.timeout = timeout;
		self
	}

	/// Set the execution timeout in seconds.
	pub fn timeout_secs(mut self, secs: u32) -> Self {
		self.config.timeout = Duration::from_secs(secs as u64);
		self
	}

	/// Enable or disable async support.
	pub fn async_support(mut self, enabled: bool) -> Self {
		self.config.async_support = enabled;
		self
	}

	/// Enable or disable fuel-based metering.
	pub fn fuel_metering(mut self, enabled: bool) -> Self {
		self.config.fuel_metering = enabled;
		self
	}

	/// Set the initial fuel amount.
	pub fn initial_fuel(mut self, fuel: u64) -> Self {
		self.config.initial_fuel = fuel;
		self
	}

	/// Build the configuration.
	pub fn build(self) -> WasmRuntimeConfig {
		self.config
	}
}

/// WASM Runtime for managing plugin execution.
///
/// This struct encapsulates the wasmtime Engine and provides methods for
/// loading and instantiating WASM components.
pub struct WasmRuntime {
	/// The wasmtime engine.
	engine: Engine,
	/// Runtime configuration.
	config: WasmRuntimeConfig,
	/// Component linker (thread-safe, shareable).
	linker: Arc<Linker<HostState>>,
}

impl WasmRuntime {
	/// Create a new WASM runtime with the given configuration.
	///
	/// # Arguments
	///
	/// * `config` - Runtime configuration
	///
	/// # Errors
	///
	/// Returns an error if the engine cannot be created.
	pub fn new(config: WasmRuntimeConfig) -> PluginResult<Self> {
		let mut wasmtime_config = Config::new();

		// Enable async support if configured
		if config.async_support {
			wasmtime_config.async_support(true);
		}

		// Enable fuel-based metering if configured
		if config.fuel_metering {
			wasmtime_config.consume_fuel(true);
		}

		// Enable Component Model
		wasmtime_config.wasm_component_model(true);

		// Create the engine
		let engine = Engine::new(&wasmtime_config)
			.map_err(|e| PluginError::WasmLoadError(format!("Failed to create engine: {}", e)))?;

		// Create the linker and add host functions
		let mut linker = Linker::new(&engine);

		// Add WASI P2 interfaces to linker (wasi:cli/environment, etc.)
		// This is required for plugins that use WASI P2 imports
		if config.async_support {
			p2::add_to_linker_async(&mut linker).map_err(|e| {
				PluginError::WasmLoadError(format!("Failed to add WASI P2 to linker: {}", e))
			})?;
		} else {
			p2::add_to_linker_sync(&mut linker).map_err(|e| {
				PluginError::WasmLoadError(format!("Failed to add WASI P2 to linker: {}", e))
			})?;
		}

		// Add host functions from WIT definition
		// Use the generated DentdelionPlugin::add_to_linker function
		// HasSelf<_> is a type-level marker that indicates HostState directly implements Host traits
		// The closure simply returns the state itself (no wrapping needed)
		DentdelionPlugin::add_to_linker::<_, HasSelf<_>>(&mut linker, |state| state).map_err(
			|e| {
				PluginError::WasmLoadError(format!("Failed to add host functions to linker: {}", e))
			},
		)?;

		Ok(Self {
			engine,
			config,
			linker: Arc::new(linker),
		})
	}

	/// Get a reference to the engine.
	pub fn engine(&self) -> &Engine {
		&self.engine
	}

	/// Get the runtime configuration.
	pub fn config(&self) -> &WasmRuntimeConfig {
		&self.config
	}

	/// Get a reference to the linker.
	pub fn linker(&self) -> &Linker<HostState> {
		&self.linker
	}

	/// Load a WASM component from a file.
	///
	/// # Arguments
	///
	/// * `path` - Path to the `.wasm` file
	///
	/// # Errors
	///
	/// Returns an error if the file cannot be read or parsed.
	pub async fn load_component<P: AsRef<Path>>(&self, path: P) -> PluginResult<Component> {
		let path = path.as_ref();

		// Read the file
		let bytes = tokio::fs::read(path).await.map_err(|e| {
			if e.kind() == std::io::ErrorKind::NotFound {
				PluginError::WasmFileNotFound(path.display().to_string())
			} else {
				PluginError::WasmLoadError(format!("Failed to read {}: {}", path.display(), e))
			}
		})?;

		// Validate magic bytes
		if !super::is_valid_wasm(&bytes) {
			return Err(PluginError::InvalidWasmBinary);
		}

		// Compile the component
		let component = Component::from_binary(&self.engine, &bytes).map_err(|e| {
			PluginError::WasmLoadError(format!(
				"Failed to compile component {}: {}",
				path.display(),
				e
			))
		})?;

		Ok(component)
	}

	/// Load a WASM component from bytes.
	///
	/// # Arguments
	///
	/// * `bytes` - The WASM binary bytes
	///
	/// # Errors
	///
	/// Returns an error if the bytes are not valid WASM.
	pub fn load_component_from_bytes(&self, bytes: &[u8]) -> PluginResult<Component> {
		// Validate magic bytes
		if !super::is_valid_wasm(bytes) {
			return Err(PluginError::InvalidWasmBinary);
		}

		// Compile the component
		let component = Component::from_binary(&self.engine, bytes).map_err(|e| {
			PluginError::WasmLoadError(format!("Failed to compile component: {}", e))
		})?;

		Ok(component)
	}

	/// Create a new store with the given host state.
	///
	/// The store is configured with resource limits based on the runtime configuration.
	///
	/// # Arguments
	///
	/// * `host_state` - The host state for this store
	pub fn create_store(&self, host_state: HostState) -> Store<HostState> {
		let mut store = Store::new(&self.engine, host_state);

		// Configure fuel if enabled
		if self.config.fuel_metering {
			store.set_fuel(self.config.initial_fuel).ok();
		}

		store
	}

	/// Add fuel to a store.
	///
	/// # Arguments
	///
	/// * `store` - The store to add fuel to
	/// * `fuel` - Amount of fuel to add
	pub fn add_fuel(&self, store: &mut Store<HostState>, fuel: u64) -> PluginResult<()> {
		store
			.set_fuel(fuel)
			.map_err(|e| PluginError::WasmExecutionError(format!("Failed to set fuel: {}", e)))
	}

	/// Get remaining fuel in a store.
	///
	/// # Arguments
	///
	/// * `store` - The store to check
	///
	/// # Returns
	///
	/// The remaining fuel, or `None` if fuel metering is disabled.
	pub fn remaining_fuel(&self, store: &Store<HostState>) -> Option<u64> {
		store.get_fuel().ok()
	}
}

impl std::fmt::Debug for WasmRuntime {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("WasmRuntime")
			.field("config", &self.config)
			.finish_non_exhaustive()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_default_config() {
		let config = WasmRuntimeConfig::default();
		assert_eq!(config.memory_limit_mb, DEFAULT_MEMORY_LIMIT_MB);
		assert_eq!(
			config.timeout,
			Duration::from_secs(DEFAULT_TIMEOUT_SECS as u64)
		);
		assert!(config.async_support);
		assert!(config.fuel_metering);
	}

	#[rstest]
	fn test_config_builder() {
		let config = WasmRuntimeConfigBuilder::new()
			.memory_limit_mb(256)
			.timeout_secs(60)
			.async_support(false)
			.fuel_metering(false)
			.build();

		assert_eq!(config.memory_limit_mb, 256);
		assert_eq!(config.timeout, Duration::from_secs(60));
		assert!(!config.async_support);
		assert!(!config.fuel_metering);
	}

	#[rstest]
	fn test_runtime_creation() {
		let config = WasmRuntimeConfig::default();
		let result = WasmRuntime::new(config);
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_invalid_wasm_bytes() {
		let config = WasmRuntimeConfig::default();
		let runtime = WasmRuntime::new(config).unwrap();

		let invalid_bytes = vec![0x00, 0x00, 0x00, 0x00];
		let result = runtime.load_component_from_bytes(&invalid_bytes);

		assert!(matches!(result, Err(PluginError::InvalidWasmBinary)));
	}
}
