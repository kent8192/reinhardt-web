//! Host API Implementation
//!
//! This module provides the host state and functions that are exposed to WASM plugins.
//! It implements the `host` interface defined in the WIT file.
//!
//! # Capabilities
//!
//! Some host functions require specific capabilities:
//! - HTTP functions: Require network access permission
//! - Database functions: Require database access permission

use crate::capability::Capability;
use crate::error::PluginResult;

use parking_lot::RwLock;
use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::types::{ConfigValue, WitHttpResponse, WitPluginError};

#[cfg(feature = "wasm")]
use reinhardt_backends::connection::DatabaseConnection;

#[cfg(feature = "wasm")]
use wasmtime::component::ResourceTable;
#[cfg(feature = "wasm")]
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

/// Host state that is passed to WASM instances.
///
/// This struct contains all the state that plugins can access through host functions.
pub struct HostState {
	/// Plugin name (for logging context)
	pub plugin_name: String,
	/// Configuration values accessible to the plugin
	config: RwLock<HashMap<String, ConfigValue>>,
	/// Registered services (name -> MessagePack-serialized data)
	services: RwLock<HashMap<String, Vec<u8>>>,
	/// Type-safe services (for internal use)
	typed_services: RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
	/// Granted capabilities for this plugin
	capabilities: HashSet<Capability>,
	/// HTTP client for making external requests (optional)
	http_client: Option<reqwest::Client>,
	/// Database connection for SQL queries (optional, requires DatabaseAccess capability)
	#[cfg(feature = "wasm")]
	db_connection: Option<Arc<DatabaseConnection>>,
	#[cfg(not(feature = "wasm"))]
	db_connection: Option<()>,
	/// WASI context for WASI P2 interface support
	/// Wrapped in Mutex to allow interior mutability while maintaining Sync
	#[cfg(feature = "wasm")]
	wasi_ctx: parking_lot::Mutex<WasiCtx>,
	/// Resource table for WASI P2 interface support
	/// Wrapped in Mutex to allow interior mutability while maintaining Sync
	#[cfg(feature = "wasm")]
	resource_table: parking_lot::Mutex<ResourceTable>,
}

impl HostState {
	/// Create a new host state with default configuration.
	pub fn new(plugin_name: impl Into<String>) -> Self {
		Self {
			plugin_name: plugin_name.into(),
			config: RwLock::new(HashMap::new()),
			services: RwLock::new(HashMap::new()),
			typed_services: RwLock::new(HashMap::new()),
			capabilities: HashSet::new(),
			http_client: Some(reqwest::Client::new()),
			db_connection: None,
			#[cfg(feature = "wasm")]
			wasi_ctx: parking_lot::Mutex::new(WasiCtxBuilder::new().build()),
			#[cfg(feature = "wasm")]
			resource_table: parking_lot::Mutex::new(ResourceTable::new()),
		}
	}

	/// Create a builder for host state.
	pub fn builder(plugin_name: impl Into<String>) -> HostStateBuilder {
		HostStateBuilder::new(plugin_name)
	}

	// ===== Configuration API =====

	/// Get a configuration value by key.
	pub fn get_config(&self, key: &str) -> Option<ConfigValue> {
		self.config.read().get(key).cloned()
	}

	/// Set a configuration value.
	pub fn set_config(&self, key: &str, value: ConfigValue) -> PluginResult<()> {
		self.config.write().insert(key.to_string(), value);
		Ok(())
	}

	/// Set multiple configuration values at once.
	pub fn set_config_all(&self, values: HashMap<String, ConfigValue>) {
		let mut config = self.config.write();
		for (key, value) in values {
			config.insert(key, value);
		}
	}

	// ===== Logging API =====

	/// Log a debug message.
	pub fn log_debug(&self, message: &str) {
		tracing::debug!(plugin = %self.plugin_name, "{}", message);
	}

	/// Log an info message.
	pub fn log_info(&self, message: &str) {
		tracing::info!(plugin = %self.plugin_name, "{}", message);
	}

	/// Log a warning message.
	pub fn log_warn(&self, message: &str) {
		tracing::warn!(plugin = %self.plugin_name, "{}", message);
	}

	/// Log an error message.
	pub fn log_error(&self, message: &str) {
		tracing::error!(plugin = %self.plugin_name, "{}", message);
	}

	// ===== Service Registration API =====

	/// Register a service with MessagePack-serialized data.
	pub fn register_service(&self, name: &str, data: Vec<u8>) -> PluginResult<()> {
		self.services.write().insert(name.to_string(), data);
		Ok(())
	}

	/// Get a registered service by name.
	pub fn get_service(&self, name: &str) -> Option<Vec<u8>> {
		self.services.read().get(name).cloned()
	}

	/// Unregister a service by name.
	pub fn unregister_service(&self, name: &str) -> PluginResult<()> {
		self.services.write().remove(name);
		Ok(())
	}

	/// Register a typed service (for internal use).
	pub fn register_typed_service<T: Any + Send + Sync>(&self, service: Arc<T>) {
		let type_id = TypeId::of::<T>();
		self.typed_services.write().insert(type_id, service);
	}

	/// Get a typed service (for internal use).
	pub fn get_typed_service<T: Any + Send + Sync>(&self) -> Option<Arc<T>> {
		let type_id = TypeId::of::<T>();
		self.typed_services
			.read()
			.get(&type_id)
			.and_then(|s| s.clone().downcast::<T>().ok())
	}

	// ===== HTTP Client API =====

	/// Check if HTTP access is allowed.
	fn has_http_capability(&self) -> bool {
		use crate::capability::PluginCapability;
		self.capabilities
			.contains(&Capability::Core(PluginCapability::NetworkAccess))
	}

	/// Perform an HTTP GET request.
	pub async fn http_get(
		&self,
		url: &str,
		headers: &[(String, String)],
	) -> Result<WitHttpResponse, WitPluginError> {
		if !self.has_http_capability() {
			return Err(WitPluginError::new(403, "HTTP access not permitted"));
		}

		let client = self
			.http_client
			.as_ref()
			.ok_or_else(|| WitPluginError::new(500, "HTTP client not available"))?;

		let mut request = client.get(url);

		for (name, value) in headers {
			request = request.header(name, value);
		}

		let response = request
			.send()
			.await
			.map_err(|e| WitPluginError::with_details(500, "HTTP request failed", e.to_string()))?;

		let status = response.status().as_u16();
		let headers: Vec<(String, String)> = response
			.headers()
			.iter()
			.map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
			.collect();
		let body = response.bytes().await.map_err(|e| {
			WitPluginError::with_details(500, "Failed to read response body", e.to_string())
		})?;

		Ok(WitHttpResponse {
			status,
			headers,
			body: body.to_vec(),
		})
	}

	/// Perform an HTTP POST request.
	pub async fn http_post(
		&self,
		url: &str,
		body: &[u8],
		headers: &[(String, String)],
	) -> Result<WitHttpResponse, WitPluginError> {
		if !self.has_http_capability() {
			return Err(WitPluginError::new(403, "HTTP access not permitted"));
		}

		let client = self
			.http_client
			.as_ref()
			.ok_or_else(|| WitPluginError::new(500, "HTTP client not available"))?;

		let mut request = client.post(url).body(body.to_vec());

		for (name, value) in headers {
			request = request.header(name, value);
		}

		let response = request
			.send()
			.await
			.map_err(|e| WitPluginError::with_details(500, "HTTP request failed", e.to_string()))?;

		let status = response.status().as_u16();
		let resp_headers: Vec<(String, String)> = response
			.headers()
			.iter()
			.map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
			.collect();
		let resp_body = response.bytes().await.map_err(|e| {
			WitPluginError::with_details(500, "Failed to read response body", e.to_string())
		})?;

		Ok(WitHttpResponse {
			status,
			headers: resp_headers,
			body: resp_body.to_vec(),
		})
	}

	// ===== Database API =====

	/// Check if database access is allowed.
	fn has_db_capability(&self) -> bool {
		use crate::capability::PluginCapability;
		self.capabilities
			.contains(&Capability::Core(PluginCapability::DatabaseAccess))
	}

	/// Execute a SQL query.
	#[cfg(feature = "wasm")]
	pub async fn db_query(&self, sql: &str, params: &[u8]) -> Result<Vec<u8>, WitPluginError> {
		if !self.has_db_capability() {
			return Err(WitPluginError::new(403, "Database access not permitted"));
		}

		let conn = self
			.db_connection
			.as_ref()
			.ok_or_else(|| WitPluginError::new(500, "Database connection not available"))?;

		// Deserialize MessagePack parameters
		use reinhardt_backends::types::QueryValue;
		let params_vec: Vec<QueryValue> = rmp_serde::from_slice(params).map_err(|e| {
			WitPluginError::with_details(400, "Invalid query parameters", e.to_string())
		})?;

		// Execute query
		let rows = conn.fetch_all(sql, params_vec).await.map_err(|e| {
			WitPluginError::with_details(500, "Database query failed", e.to_string())
		})?;

		// Convert rows to serializable format (Vec<HashMap>)
		let serializable_rows: Vec<HashMap<String, QueryValue>> =
			rows.into_iter().map(|row| row.data).collect();

		// Serialize results to MessagePack
		rmp_serde::to_vec(&serializable_rows).map_err(|e| {
			WitPluginError::with_details(500, "Failed to serialize results", e.to_string())
		})
	}

	#[cfg(not(feature = "wasm"))]
	pub async fn db_query(&self, _sql: &str, _params: &[u8]) -> Result<Vec<u8>, WitPluginError> {
		if !self.has_db_capability() {
			return Err(WitPluginError::new(403, "Database access not permitted"));
		}
		Err(WitPluginError::new(
			501,
			"Database queries not available (wasm feature not enabled)",
		))
	}

	/// Execute a SQL statement (INSERT, UPDATE, DELETE).
	#[cfg(feature = "wasm")]
	pub async fn db_execute(&self, sql: &str, params: &[u8]) -> Result<u64, WitPluginError> {
		if !self.has_db_capability() {
			return Err(WitPluginError::new(403, "Database access not permitted"));
		}

		let conn = self
			.db_connection
			.as_ref()
			.ok_or_else(|| WitPluginError::new(500, "Database connection not available"))?;

		// Deserialize MessagePack parameters
		use reinhardt_backends::types::QueryValue;
		let params_vec: Vec<QueryValue> = rmp_serde::from_slice(params).map_err(|e| {
			WitPluginError::with_details(400, "Invalid query parameters", e.to_string())
		})?;

		// Execute statement
		let result = conn.execute(sql, params_vec).await.map_err(|e| {
			WitPluginError::with_details(500, "Database execution failed", e.to_string())
		})?;

		Ok(result.rows_affected)
	}

	#[cfg(not(feature = "wasm"))]
	pub async fn db_execute(&self, _sql: &str, _params: &[u8]) -> Result<u64, WitPluginError> {
		if !self.has_db_capability() {
			return Err(WitPluginError::new(403, "Database access not permitted"));
		}
		Err(WitPluginError::new(
			501,
			"Database execution not available (wasm feature not enabled)",
		))
	}

	// ===== Capability Management =====

	/// Add a capability to this host state.
	pub fn add_capability(&mut self, capability: Capability) {
		self.capabilities.insert(capability);
	}

	/// Check if this host state has a specific capability.
	pub fn has_capability(&self, capability: &Capability) -> bool {
		self.capabilities.contains(capability)
	}

	/// Get all capabilities.
	pub fn capabilities(&self) -> &HashSet<Capability> {
		&self.capabilities
	}
}

impl std::fmt::Debug for HostState {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("HostState")
			.field("plugin_name", &self.plugin_name)
			.field("config_count", &self.config.read().len())
			.field("service_count", &self.services.read().len())
			.field("capabilities", &self.capabilities)
			.finish_non_exhaustive()
	}
}

/// Builder for `HostState`.
pub struct HostStateBuilder {
	plugin_name: String,
	config: HashMap<String, ConfigValue>,
	capabilities: HashSet<Capability>,
	http_client: Option<reqwest::Client>,
	#[cfg(feature = "wasm")]
	db_connection: Option<Arc<DatabaseConnection>>,
	#[cfg(not(feature = "wasm"))]
	db_connection: Option<()>,
}

impl HostStateBuilder {
	/// Create a new builder.
	pub fn new(plugin_name: impl Into<String>) -> Self {
		Self {
			plugin_name: plugin_name.into(),
			config: HashMap::new(),
			capabilities: HashSet::new(),
			http_client: Some(reqwest::Client::new()),
			db_connection: None,
		}
	}

	/// Add a configuration value.
	pub fn config(mut self, key: impl Into<String>, value: ConfigValue) -> Self {
		self.config.insert(key.into(), value);
		self
	}

	/// Add multiple configuration values.
	pub fn config_all(mut self, config: HashMap<String, ConfigValue>) -> Self {
		self.config.extend(config);
		self
	}

	/// Add a capability.
	pub fn capability(mut self, capability: Capability) -> Self {
		self.capabilities.insert(capability);
		self
	}

	/// Add multiple capabilities.
	pub fn capabilities(mut self, capabilities: impl IntoIterator<Item = Capability>) -> Self {
		self.capabilities.extend(capabilities);
		self
	}

	/// Set a custom HTTP client.
	pub fn http_client(mut self, client: reqwest::Client) -> Self {
		self.http_client = Some(client);
		self
	}

	/// Disable HTTP client.
	pub fn no_http_client(mut self) -> Self {
		self.http_client = None;
		self
	}

	/// Set a database connection.
	#[cfg(feature = "wasm")]
	pub fn db_connection(mut self, connection: Arc<DatabaseConnection>) -> Self {
		self.db_connection = Some(connection);
		self
	}

	/// Build the host state.
	pub fn build(self) -> HostState {
		HostState {
			plugin_name: self.plugin_name,
			config: RwLock::new(self.config),
			services: RwLock::new(HashMap::new()),
			typed_services: RwLock::new(HashMap::new()),
			capabilities: self.capabilities,
			http_client: self.http_client,
			db_connection: self.db_connection,
			#[cfg(feature = "wasm")]
			wasi_ctx: parking_lot::Mutex::new(WasiCtxBuilder::new().build()),
			#[cfg(feature = "wasm")]
			resource_table: parking_lot::Mutex::new(ResourceTable::new()),
		}
	}
}

// ===== Type Aliases for Generated WASM Types =====

#[cfg(feature = "wasm")]
type GeneratedPluginError = crate::wasm::runtime::reinhardt::dentdelion::types::PluginError;

#[cfg(feature = "wasm")]
type GeneratedHttpResponse = crate::wasm::runtime::reinhardt::dentdelion::types::HttpResponse;

// ===== Type Conversion Functions =====

/// Convert WitPluginError to generated PluginError type
#[cfg(feature = "wasm")]
fn to_generated_error(err: WitPluginError) -> GeneratedPluginError {
	GeneratedPluginError {
		code: err.code,
		message: err.message,
		details: err.details,
	}
}

/// Convert WitHttpResponse to generated HttpResponse type
#[cfg(feature = "wasm")]
fn to_generated_http_response(response: WitHttpResponse) -> GeneratedHttpResponse {
	GeneratedHttpResponse {
		status: response.status,
		headers: response.headers,
		body: response.body,
	}
}

// ===== WasiView Trait Implementation =====
//
// Implement WasiView to enable WASI P2 interface support for plugins.
// This provides access to WasiCtx and ResourceTable required by wasmtime-wasi.

#[cfg(feature = "wasm")]
impl WasiView for HostState {
	fn ctx(&mut self) -> WasiCtxView<'_> {
		// Use get_mut() to obtain mutable references without locking
		// This is safe because WasiView::ctx() requires &mut self,
		// ensuring exclusive access to the Mutex internals
		WasiCtxView {
			ctx: self.wasi_ctx.get_mut(),
			table: self.resource_table.get_mut(),
		}
	}
}

// ===== Host Trait Implementation =====
//
// Implement the Host trait generated by bindgen! to expose host functions to WASM plugins.
// The trait is generated at crate::wasm::runtime::reinhardt::dentdelion::host::Host
//
// Note: The generated Host trait uses synchronous functions, but some of our host
// functions are async. We use tokio::runtime::Handle::current().block_on() to bridge them.

// Implement the empty marker trait from types module
#[cfg(feature = "wasm")]
impl crate::wasm::runtime::reinhardt::dentdelion::types::Host for HostState {}

// Implement the actual host functions from host module
// bindgen generates traits with native async fn (RPITIT - Return Position Impl Trait In Traits)
// so we don't use async_trait macro here
#[cfg(feature = "wasm")]
impl crate::wasm::runtime::reinhardt::dentdelion::host::Host for HostState {
	// ===== Configuration Access =====

	async fn get_config(&mut self, key: String) -> Result<Option<Vec<u8>>, anyhow::Error> {
		// Get config value and serialize to MessagePack
		let result = Self::get_config(self, &key).and_then(|v| rmp_serde::to_vec(&v).ok());
		Ok(result)
	}

	async fn set_config(
		&mut self,
		key: String,
		value: Vec<u8>,
	) -> Result<Result<(), GeneratedPluginError>, anyhow::Error> {
		// Deserialize MessagePack to ConfigValue
		let config_value: ConfigValue = rmp_serde::from_slice(&value)
			.map_err(|e| anyhow::anyhow!("Invalid config value: {}", e))?;

		let result = Self::set_config(self, &key, config_value)
			.map_err(|e| to_generated_error(WitPluginError::from_plugin_error(&e)));
		Ok(result)
	}

	// ===== Logging =====

	async fn log_debug(&mut self, message: String) -> Result<(), anyhow::Error> {
		Self::log_debug(self, &message);
		Ok(())
	}

	async fn log_info(&mut self, message: String) -> Result<(), anyhow::Error> {
		Self::log_info(self, &message);
		Ok(())
	}

	async fn log_warn(&mut self, message: String) -> Result<(), anyhow::Error> {
		Self::log_warn(self, &message);
		Ok(())
	}

	async fn log_error(&mut self, message: String) -> Result<(), anyhow::Error> {
		Self::log_error(self, &message);
		Ok(())
	}

	// ===== Service Registration =====

	async fn register_service(
		&mut self,
		name: String,
		data: Vec<u8>,
	) -> Result<Result<(), GeneratedPluginError>, anyhow::Error> {
		let result = Self::register_service(self, &name, data)
			.map_err(|e| to_generated_error(WitPluginError::from_plugin_error(&e)));
		Ok(result)
	}

	async fn get_service(&mut self, name: String) -> Result<Option<Vec<u8>>, anyhow::Error> {
		Ok(Self::get_service(self, &name))
	}

	async fn unregister_service(
		&mut self,
		name: String,
	) -> Result<Result<(), GeneratedPluginError>, anyhow::Error> {
		let result = Self::unregister_service(self, &name)
			.map_err(|e| to_generated_error(WitPluginError::from_plugin_error(&e)));
		Ok(result)
	}

	// ===== HTTP Client =====

	async fn http_get(
		&mut self,
		url: String,
		headers: Vec<(String, String)>,
	) -> Result<Result<GeneratedHttpResponse, GeneratedPluginError>, anyhow::Error> {
		// Call async function directly (no longer need to block)
		let result = Self::http_get(self, &url, &headers)
			.await
			.map(to_generated_http_response)
			.map_err(to_generated_error);
		Ok(result)
	}

	async fn http_post(
		&mut self,
		url: String,
		body: Vec<u8>,
		headers: Vec<(String, String)>,
	) -> Result<Result<GeneratedHttpResponse, GeneratedPluginError>, anyhow::Error> {
		// Call async function directly (no longer need to block)
		let result = Self::http_post(self, &url, &body, &headers)
			.await
			.map(to_generated_http_response)
			.map_err(to_generated_error);
		Ok(result)
	}

	// ===== Database Access =====

	async fn db_query(
		&mut self,
		sql: String,
		params: Vec<u8>,
	) -> Result<Result<Vec<u8>, GeneratedPluginError>, anyhow::Error> {
		// Call async function directly (no longer need to block)
		let result = Self::db_query(self, &sql, &params)
			.await
			.map_err(to_generated_error);
		Ok(result)
	}

	async fn db_execute(
		&mut self,
		sql: String,
		params: Vec<u8>,
	) -> Result<Result<u64, GeneratedPluginError>, anyhow::Error> {
		// Call async function directly (no longer need to block)
		let result = Self::db_execute(self, &sql, &params)
			.await
			.map_err(to_generated_error);
		Ok(result)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_host_state_config() {
		let state = HostState::new("test-plugin");

		// Set and get config
		state
			.set_config("key1", ConfigValue::StringVal("value1".to_string()))
			.unwrap();
		assert_eq!(
			state.get_config("key1"),
			Some(ConfigValue::StringVal("value1".to_string()))
		);

		// Non-existent key
		assert!(state.get_config("nonexistent").is_none());
	}

	#[test]
	fn test_host_state_services() {
		let state = HostState::new("test-plugin");

		// Register and get service
		let data = vec![1, 2, 3, 4];
		state.register_service("my-service", data.clone()).unwrap();
		assert_eq!(state.get_service("my-service"), Some(data));

		// Unregister service
		state.unregister_service("my-service").unwrap();
		assert!(state.get_service("my-service").is_none());
	}

	#[test]
	fn test_host_state_builder() {
		let state = HostStateBuilder::new("test-plugin")
			.config("key1", ConfigValue::IntVal(42))
			.config("key2", ConfigValue::BoolVal(true))
			.build();

		assert_eq!(state.plugin_name, "test-plugin");
		assert_eq!(state.get_config("key1"), Some(ConfigValue::IntVal(42)));
		assert_eq!(state.get_config("key2"), Some(ConfigValue::BoolVal(true)));
	}

	#[test]
	fn test_host_state_capabilities() {
		use crate::capability::PluginCapability;

		let mut state = HostState::new("test-plugin");
		let cap = Capability::Core(PluginCapability::Middleware);

		assert!(!state.has_capability(&cap));

		state.add_capability(cap.clone());
		assert!(state.has_capability(&cap));
	}
}
