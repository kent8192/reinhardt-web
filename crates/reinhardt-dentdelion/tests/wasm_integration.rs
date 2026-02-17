//! WASM Plugin System Integration Tests
//!
//! These tests require actual WASM components implementing the dentdelion-plugin world.
//! Sample plugins must be compiled to WASM Component Model format using tools like:
//! - `cargo component` for Rust plugins
//! - `wit-bindgen` for other languages
//!
//! # Test Fixtures Required
//!
//! 1. `tests/fixtures/minimal_plugin.wasm` - Basic plugin implementing all lifecycle functions
//! 2. `tests/fixtures/logging_plugin.wasm` - Plugin that uses host logging APIs
//! 3. `tests/fixtures/config_plugin.wasm` - Plugin that accesses configuration
//! 4. `tests/fixtures/network_plugin.wasm` - Plugin with NetworkAccess capability
//! 5. `tests/fixtures/database_plugin.wasm` - Plugin with DatabaseAccess capability

use rstest::rstest;
#[cfg(feature = "wasm")]
mod wasm_tests {
	use reinhardt_dentdelion::{
		context::PluginContext,
		plugin::PluginLifecycle,
		wasm::{WasmPluginLoader, WasmRuntime, WasmRuntimeConfig},
	};
	use rstest::rstest;
	use std::path::PathBuf;
	use std::sync::Arc;

	/// Get the path to the test fixtures directory
	fn fixtures_dir() -> PathBuf {
		PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
	}

	/// Test full plugin lifecycle (load -> enable -> disable -> unload)
	#[rstest]
	#[tokio::test]
	async fn test_full_plugin_lifecycle() {
		let wasm_path = fixtures_dir().join("minimal_plugin.wasm");
		if !wasm_path.exists() {
			eprintln!(
				"Skipping test: minimal_plugin.wasm not found at {:?}",
				wasm_path
			);
			return;
		}

		// Create runtime with default configuration
		let runtime = Arc::new(
			WasmRuntime::new(WasmRuntimeConfig::default()).expect("Failed to create runtime"),
		);

		// Create loader
		let loader = WasmPluginLoader::new(fixtures_dir(), runtime);

		// Load plugin from path
		let instance = loader
			.load_from_path(&wasm_path)
			.await
			.expect("Failed to load minimal plugin");

		// Test lifecycle
		let ctx = PluginContext::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")));

		// on_load
		instance
			.on_load(&ctx)
			.await
			.expect("on_load should succeed");

		// on_enable
		instance
			.on_enable(&ctx)
			.await
			.expect("on_enable should succeed");

		// on_disable
		instance
			.on_disable(&ctx)
			.await
			.expect("on_disable should succeed");

		// on_unload
		instance
			.on_unload(&ctx)
			.await
			.expect("on_unload should succeed");
	}

	/// Test that invalid state transitions are rejected
	#[rstest]
	#[tokio::test]
	async fn test_invalid_state_transitions() {
		let wasm_path = fixtures_dir().join("minimal_plugin.wasm");
		if !wasm_path.exists() {
			eprintln!(
				"Skipping test: minimal_plugin.wasm not found at {:?}",
				wasm_path
			);
			return;
		}

		let runtime = Arc::new(
			WasmRuntime::new(WasmRuntimeConfig::default()).expect("Failed to create runtime"),
		);
		let loader = WasmPluginLoader::new(fixtures_dir(), runtime);
		let instance = loader
			.load_from_path(&wasm_path)
			.await
			.expect("Failed to load plugin");

		let ctx = PluginContext::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")));

		// Try to enable before loading - should fail
		let result = instance.on_enable(&ctx).await;
		assert!(
			result.is_err(),
			"Should not be able to enable before loading"
		);

		// Try to disable before enabling - should fail after loading
		instance
			.on_load(&ctx)
			.await
			.expect("on_load should succeed");
		let result = instance.on_disable(&ctx).await;
		assert!(
			result.is_err(),
			"Should not be able to disable before enabling"
		);
	}

	/// Test that plugin can be re-enabled after disabling
	#[rstest]
	#[tokio::test]
	async fn test_reenable_after_disable() {
		let wasm_path = fixtures_dir().join("minimal_plugin.wasm");
		if !wasm_path.exists() {
			eprintln!(
				"Skipping test: minimal_plugin.wasm not found at {:?}",
				wasm_path
			);
			return;
		}

		let runtime = Arc::new(
			WasmRuntime::new(WasmRuntimeConfig::default()).expect("Failed to create runtime"),
		);
		let loader = WasmPluginLoader::new(fixtures_dir(), runtime);
		let instance = loader
			.load_from_path(&wasm_path)
			.await
			.expect("Failed to load plugin");

		let ctx = PluginContext::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")));

		// Full cycle
		instance
			.on_load(&ctx)
			.await
			.expect("on_load should succeed");
		instance
			.on_enable(&ctx)
			.await
			.expect("on_enable should succeed");
		instance
			.on_disable(&ctx)
			.await
			.expect("on_disable should succeed");

		// Re-enable should work from disabled state
		instance
			.on_enable(&ctx)
			.await
			.expect("on_enable after disable should succeed");
	}

	/// Test host logging from WASM plugin
	#[rstest]
	#[tokio::test]
	async fn test_host_logging_from_wasm() {
		let plugins_dir = fixtures_dir().join("plugins/logging");
		let wasm_path = plugins_dir.join("target/wasm32-wasip1/release/logging_plugin.wasm");

		// Check if the built wasm exists, if not try alternative paths
		let wasm_path = if wasm_path.exists() {
			wasm_path
		} else {
			// Try the direct fixtures path
			let alt_path = fixtures_dir().join("logging_plugin.wasm");
			if !alt_path.exists() {
				eprintln!(
					"Skipping test: logging_plugin.wasm not found at {:?} or {:?}",
					wasm_path, alt_path
				);
				return;
			}
			alt_path
		};

		let runtime = Arc::new(
			WasmRuntime::new(WasmRuntimeConfig::default()).expect("Failed to create runtime"),
		);
		let loader = WasmPluginLoader::new(fixtures_dir(), runtime);
		let instance = loader
			.load_from_path(&wasm_path)
			.await
			.expect("Failed to load logging plugin");

		let ctx = PluginContext::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")));

		// Logging plugin should call host logging APIs during lifecycle
		// The test passes if no errors occur during the lifecycle calls
		instance
			.on_load(&ctx)
			.await
			.expect("on_load with logging should succeed");
		instance
			.on_enable(&ctx)
			.await
			.expect("on_enable with logging should succeed");
		instance
			.on_disable(&ctx)
			.await
			.expect("on_disable with logging should succeed");
		instance
			.on_unload(&ctx)
			.await
			.expect("on_unload with logging should succeed");
	}

	/// Test host config access from WASM plugin
	#[rstest]
	#[tokio::test]
	async fn test_host_config_from_wasm() {
		let plugins_dir = fixtures_dir().join("plugins/config");
		let wasm_path = plugins_dir.join("target/wasm32-wasip1/release/config_plugin.wasm");

		// Check if the built wasm exists
		let wasm_path = if wasm_path.exists() {
			wasm_path
		} else {
			let alt_path = fixtures_dir().join("config_plugin.wasm");
			if !alt_path.exists() {
				eprintln!(
					"Skipping test: config_plugin.wasm not found at {:?} or {:?}",
					wasm_path, alt_path
				);
				return;
			}
			alt_path
		};

		let runtime = Arc::new(
			WasmRuntime::new(WasmRuntimeConfig::default()).expect("Failed to create runtime"),
		);
		let loader = WasmPluginLoader::new(fixtures_dir(), runtime);
		let instance = loader
			.load_from_path(&wasm_path)
			.await
			.expect("Failed to load config plugin");

		let ctx = PluginContext::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")));

		// Config plugin should call host config APIs during lifecycle
		instance
			.on_load(&ctx)
			.await
			.expect("on_load with config should succeed");
		instance
			.on_enable(&ctx)
			.await
			.expect("on_enable should succeed");
	}

	/// Test that loading invalid WASM bytes fails gracefully
	#[rstest]
	#[tokio::test]
	async fn test_invalid_wasm_bytes_error() {
		let runtime = Arc::new(
			WasmRuntime::new(WasmRuntimeConfig::default()).expect("Failed to create runtime"),
		);

		// Try to load from a non-existent path
		let loader = WasmPluginLoader::new(fixtures_dir(), runtime);
		let result = loader.load_from_path("nonexistent_plugin.wasm").await;

		assert!(result.is_err(), "Loading non-existent plugin should fail");
	}

	/// Test plugin discovery in fixtures directory
	#[rstest]
	#[tokio::test]
	async fn test_plugin_discovery() {
		let runtime = Arc::new(
			WasmRuntime::new(WasmRuntimeConfig::default()).expect("Failed to create runtime"),
		);
		let loader = WasmPluginLoader::new(fixtures_dir(), runtime);

		let discovered = loader.discover().await;

		// Should not error even if no plugins are found
		assert!(
			discovered.is_ok(),
			"Discovery should not fail: {:?}",
			discovered.err()
		);
	}
}
