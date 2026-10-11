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
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use wasmtime::component::{Component, HasSelf, Linker};
use wasmtime::{CallHook, Config, Engine, ResourceLimiter, Store, StoreLimits, StoreLimitsBuilder};
use wasmtime_wasi::p2;

use super::host::HostState;

/// Aggregate linear-memory accounting retained for the lifetime of one store.
/// Wasmtime invokes the limiter before each non-shared memory allocation/growth.
pub(super) struct StoreMemoryLimits {
	limits: StoreLimits,
	budget: usize,
	allocated: usize,
	last_growth: usize,
}

impl StoreMemoryLimits {
	fn new(budget: usize) -> Self {
		Self {
			limits: StoreLimitsBuilder::new()
				.memory_size(budget)
				.trap_on_grow_failure(true)
				.build(),
			budget,
			allocated: 0,
			last_growth: 0,
		}
	}
}

impl ResourceLimiter for StoreMemoryLimits {
	fn memory_growing(
		&mut self,
		current: usize,
		desired: usize,
		maximum: Option<usize>,
	) -> anyhow::Result<bool> {
		self.last_growth = 0;
		if !self.limits.memory_growing(current, desired, maximum)? {
			return Ok(false);
		}
		let growth = desired.saturating_sub(current);
		if growth > self.budget.saturating_sub(self.allocated) {
			anyhow::bail!("WASM store linear-memory budget exceeded");
		}
		self.allocated += growth;
		self.last_growth = growth;
		Ok(true)
	}

	fn memory_grow_failed(&mut self, error: anyhow::Error) -> anyhow::Result<()> {
		// A permitted allocation can still fail in the OS; release its reservation.
		self.allocated -= self.last_growth;
		self.last_growth = 0;
		self.limits.memory_grow_failed(error)
	}

	fn table_growing(
		&mut self,
		current: usize,
		desired: usize,
		maximum: Option<usize>,
	) -> anyhow::Result<bool> {
		self.limits.table_growing(current, desired, maximum)
	}
	fn table_grow_failed(&mut self, error: anyhow::Error) -> anyhow::Result<()> {
		self.limits.table_grow_failed(error)
	}
	fn instances(&self) -> usize {
		self.limits.instances()
	}
	fn tables(&self) -> usize {
		self.limits.tables()
	}
	fn memories(&self) -> usize {
		self.limits.memories()
	}
}

/// Default memory limit in megabytes.
pub(super) const DEFAULT_MEMORY_LIMIT_MB: u32 = 128;

/// Default execution timeout in seconds.
pub(super) const DEFAULT_TIMEOUT_SECS: u32 = 30;

/// Configuration for the WASM runtime.
#[derive(Debug, Clone)]
pub struct WasmRuntimeConfig {
	/// Aggregate linear-memory limit in megabytes for one store.
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
	/// Stops the epoch ticker when the runtime is dropped.
	stop_epoch_ticker: Arc<AtomicBool>,
	epoch_ticker: Option<std::thread::JoinHandle<()>>,
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
		// ResourceLimiter cannot account shared memories; guest threads are unsupported.
		wasmtime_config.wasm_threads(false);
		wasmtime_config.epoch_interruption(true);

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

		let stop_epoch_ticker = Arc::new(AtomicBool::new(false));
		let ticker_engine = engine.clone();
		let ticker_stop = Arc::clone(&stop_epoch_ticker);
		let epoch_ticker = std::thread::spawn(move || {
			while !ticker_stop.load(Ordering::Relaxed) {
				std::thread::sleep(Duration::from_millis(10));
				ticker_engine.increment_epoch();
			}
		});

		Ok(Self {
			engine,
			config,
			linker: Arc::new(linker),
			stop_epoch_ticker,
			epoch_ticker: Some(epoch_ticker),
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
	pub fn create_store(&self, mut host_state: HostState) -> Store<HostState> {
		host_state.store_limits = Some(StoreMemoryLimits::new(
			(self.config.memory_limit_mb as usize).saturating_mul(1024 * 1024),
		));
		let mut store = Store::new(&self.engine, host_state);
		store.limiter(|state| {
			state
				.store_limits
				.as_mut()
				.expect("store limits configured")
		});
		let ticks = self.config.timeout.as_millis().div_ceil(10).max(1);
		store.set_epoch_deadline(u64::try_from(ticks).unwrap_or(u64::MAX));
		store.epoch_deadline_trap();
		let ticks = u64::try_from(ticks).unwrap_or(u64::MAX);
		// Reset only on a new host-to-guest call. Returning from a host import
		// must not extend the budget of an already-running guest invocation.
		store.call_hook(move |mut context, transition| {
			if matches!(transition, CallHook::CallingWasm) {
				context.set_epoch_deadline(ticks);
			}
			Ok(())
		});

		// Configure fuel if enabled
		if self.config.fuel_metering {
			store.set_fuel(self.config.initial_fuel).ok();
		}

		store
	}

	/// Bound a lifecycle invocation, including its asynchronous host imports.
	pub(super) async fn with_execution_timeout<T>(
		&self,
		invocation: impl std::future::Future<Output = PluginResult<T>>,
	) -> PluginResult<T> {
		tokio::time::timeout(self.config.timeout, invocation)
			.await
			.map_err(|_| {
				PluginError::WasmExecutionError(format!(
					"WASM execution timed out after {:?}",
					self.config.timeout
				))
			})?
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

impl Drop for WasmRuntime {
	fn drop(&mut self) {
		self.stop_epoch_ticker.store(true, Ordering::Relaxed);
		if let Some(ticker) = self.epoch_ticker.take() {
			let _ = ticker.join();
		}
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

	#[test]
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

	#[test]
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

	#[test]
	fn test_runtime_creation() {
		let config = WasmRuntimeConfig::default();
		let result = WasmRuntime::new(config);
		assert!(result.is_ok());
	}

	#[test]
	fn test_invalid_wasm_bytes() {
		let config = WasmRuntimeConfig::default();
		let runtime = WasmRuntime::new(config).unwrap();

		let invalid_bytes = vec![0x00, 0x00, 0x00, 0x00];
		let result = runtime.load_component_from_bytes(&invalid_bytes);

		assert!(matches!(result, Err(PluginError::InvalidWasmBinary)));
	}

	#[rstest::rstest]
	#[tokio::test]
	async fn store_memory_budget_counts_all_memories_and_instances() {
		// Arrange: a one-MiB store budget, with two distinct linear memories.
		let runtime = WasmRuntime::new(
			WasmRuntimeConfigBuilder::new()
				.memory_limit_mb(1)
				.fuel_metering(false)
				.build(),
		)
		.unwrap();
		let module = wasmtime::Module::new(
			runtime.engine(),
			r#"(module
			(memory $a 1) (memory $b 1)
			(func (export "grow_a") (result i32) i32.const 7 memory.grow $a)
			(func (export "grow_b") (result i32) i32.const 7 memory.grow $b)
			(func (export "grow_again") (result i32) i32.const 1 memory.grow $a)
		)"#,
		)
		.unwrap();
		let mut store = runtime.create_store(HostState::new("aggregate-memories"));
		let instance = wasmtime::Instance::new_async(&mut store, &module, &[])
			.await
			.unwrap();
		let grow_a = instance
			.get_typed_func::<(), i32>(&mut store, "grow_a")
			.unwrap();
		let grow_b = instance
			.get_typed_func::<(), i32>(&mut store, "grow_b")
			.unwrap();
		let grow_again = instance
			.get_typed_func::<(), i32>(&mut store, "grow_again")
			.unwrap();

		// Act: each memory reaches half a MiB, exhausting the shared budget.
		assert_eq!(grow_a.call_async(&mut store, ()).await.unwrap(), 1);
		assert_eq!(grow_b.call_async(&mut store, ()).await.unwrap(), 1);

		// Assert: a per-memory-valid growth still traps when the aggregate is full.
		assert!(grow_again.call_async(&mut store, ()).await.is_err());
		let another = wasmtime::Module::new(runtime.engine(), "(module (memory 1))").unwrap();
		assert!(
			wasmtime::Instance::new_async(&mut store, &another, &[])
				.await
				.is_err()
		);
	}

	#[rstest::rstest]
	#[tokio::test]
	async fn store_memory_budget_rejects_oversized_initial_multi_memory_allocation() {
		// Arrange: individually valid one-MiB memories exceed the total budget together.
		let runtime =
			WasmRuntime::new(WasmRuntimeConfigBuilder::new().memory_limit_mb(1).build()).unwrap();
		let module =
			wasmtime::Module::new(runtime.engine(), "(module (memory 16) (memory 16))").unwrap();
		let mut store = runtime.create_store(HostState::new("initial-memories"));

		// Act / Assert: initialization is metered as well as later memory.grow.
		assert!(
			wasmtime::Instance::new_async(&mut store, &module, &[])
				.await
				.is_err()
		);
	}

	#[rstest::rstest]
	fn failed_memory_allocation_releases_its_budget_reservation() {
		// Arrange: simulate an OS allocation failure after the limiter permits growth.
		let mut limiter = StoreMemoryLimits::new(65536);
		assert!(limiter.memory_growing(0, 65536, None).unwrap());

		// Act: the Wasmtime failure callback rolls back the unfulfilled reservation.
		assert!(
			limiter
				.memory_grow_failed(anyhow::anyhow!("allocation failed"))
				.is_err()
		);

		// Assert: a later real allocation can still use the full budget.
		assert!(limiter.memory_growing(0, 65536, None).unwrap());
	}

	#[rstest::rstest]
	fn shared_memories_cannot_bypass_store_accounting() {
		// Arrange: shared memories are outside Wasmtime's ResourceLimiter callbacks.
		let runtime = WasmRuntime::new(WasmRuntimeConfig::default()).unwrap();

		// Act / Assert: guest validation rejects them before any allocation.
		assert!(wasmtime::Module::new(runtime.engine(), "(module (memory 1 2 shared))").is_err());
	}

	#[rstest::rstest]
	#[tokio::test]
	async fn store_memory_limit_allows_small_growth_and_traps_excess() {
		let config = WasmRuntimeConfigBuilder::new()
			.memory_limit_mb(1)
			.fuel_metering(false)
			.build();
		let runtime = WasmRuntime::new(config).unwrap();
		let module = wasmtime::Module::new(
			runtime.engine(),
			r#"(module
				(memory (export "memory") 1)
				(func (export "grow_one") (result i32) i32.const 1 memory.grow)
				(func (export "grow_past_limit") (result i32) i32.const 15 memory.grow)
			)"#,
		)
		.unwrap();
		let mut store = runtime.create_store(HostState::new("memory-test"));
		let instance = wasmtime::Instance::new_async(&mut store, &module, &[])
			.await
			.unwrap();

		let grow_one = instance
			.get_typed_func::<(), i32>(&mut store, "grow_one")
			.unwrap();
		assert_eq!(grow_one.call_async(&mut store, ()).await.unwrap(), 1);
		let grow_past_limit = instance
			.get_typed_func::<(), i32>(&mut store, "grow_past_limit")
			.unwrap();
		assert!(grow_past_limit.call_async(&mut store, ()).await.is_err());
	}

	#[rstest::rstest]
	#[tokio::test]
	async fn execution_deadline_cancels_a_delayed_host_import() {
		// Arrange: the guest awaits a host future much longer than its runtime budget.
		let runtime = WasmRuntime::new(
			WasmRuntimeConfigBuilder::new()
				.timeout(Duration::from_millis(30))
				.fuel_metering(false)
				.build(),
		)
		.unwrap();
		let module = wasmtime::Module::new(
			runtime.engine(),
			r#"(module
			(import "test" "delay" (func $delay))
			(func (export "run") call $delay)
		)"#,
		)
		.unwrap();
		let cancelled = Arc::new(AtomicBool::new(false));
		struct ImportGuard(Arc<AtomicBool>);
		impl Drop for ImportGuard {
			fn drop(&mut self) {
				self.0.store(true, Ordering::Release);
			}
		}
		let mut linker = wasmtime::Linker::<HostState>::new(runtime.engine());
		let flag = Arc::clone(&cancelled);
		linker
			.func_wrap_async("test", "delay", move |_caller, (): ()| {
				let flag = Arc::clone(&flag);
				Box::new(async move {
					let _guard = ImportGuard(flag);
					tokio::time::sleep(Duration::from_secs(1)).await;
					Ok(())
				})
			})
			.unwrap();
		let mut store = runtime.create_store(HostState::new("delayed-import"));
		let instance = linker.instantiate_async(&mut store, &module).await.unwrap();
		let function = instance
			.get_typed_func::<(), ()>(&mut store, "run")
			.unwrap();
		let started = std::time::Instant::now();

		// Act: use the same whole-invocation bound as the plugin lifecycle methods.
		let result = runtime
			.with_execution_timeout(async {
				function
					.call_async(&mut store, ())
					.await
					.map_err(|error| PluginError::WasmExecutionError(error.to_string()))
			})
			.await;

		// Assert: expiration drops the suspended import rather than waiting for it.
		assert!(
			matches!(result, Err(PluginError::WasmExecutionError(message)) if message.contains("timed out"))
		);
		assert!(started.elapsed() < Duration::from_millis(500));
		assert!(cancelled.load(Ordering::Acquire));
	}

	#[rstest::rstest]
	#[tokio::test]
	async fn store_epoch_deadline_interrupts_busy_loop() {
		let config = WasmRuntimeConfigBuilder::new()
			.timeout(Duration::from_millis(30))
			.fuel_metering(false)
			.build();
		let runtime = WasmRuntime::new(config).unwrap();
		let module = wasmtime::Module::new(
			runtime.engine(),
			r#"(module
				(func (export "noop"))
				(func (export "spin") (loop (br 0)))
			)"#,
		)
		.unwrap();
		let mut store = runtime.create_store(HostState::new("timeout-test"));
		let instance = wasmtime::Instance::new_async(&mut store, &module, &[])
			.await
			.unwrap();
		let noop = instance
			.get_typed_func::<(), ()>(&mut store, "noop")
			.unwrap();
		assert!(noop.call_async(&mut store, ()).await.is_ok());
		tokio::time::sleep(Duration::from_millis(60)).await;
		// A reused store gets a fresh execution budget after idle time.
		assert!(noop.call_async(&mut store, ()).await.is_ok());
		let spin = instance
			.get_typed_func::<(), ()>(&mut store, "spin")
			.unwrap();

		assert!(spin.call_async(&mut store, ()).await.is_err());
	}
}
