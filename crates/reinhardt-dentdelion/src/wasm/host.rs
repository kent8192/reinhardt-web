//! Host API Implementation
//!
//! This module provides the host state and functions that are exposed to WASM plugins.
//! It implements the `host` interface defined in the WIT file.
//!
//! # Capabilities
//!
//! Some host functions require specific capabilities:
//! - HTTP functions: Require `NetworkAccess` capability
//! - Database functions: Require `DatabaseAccess` capability
//! - Event functions: Require `Signals` capability
//! - SSR rendering: Requires `Verified` or `Trusted` trust level
//! - JavaScript execution: Requires `Trusted` trust level only
//!
//! The capability-based access control pattern ensures plugins can only access
//! resources they have been explicitly granted. Capabilities are preserved
//! across `Clone` operations so that WASM store creation does not lose state.
//!
//! # HTTP Resource Limits
//!
//! HTTP host calls buffer responses in host memory, outside the guest's memory
//! and fuel limits. Every call is therefore bounded by [`HttpResourceLimits`]:
//! a response-body cap, a request-body cap, a total deadline, and a per-plugin
//! concurrency budget.
//!
//! # Database Resource Limits
//!
//! Query results are encoded in host memory before they are copied into the
//! guest, so `db_query` is bounded by [`DatabaseResourceLimits`] in the same
//! way: rows are streamed and encoded one at a time under a row cap and an
//! encoded-byte cap, inside a total deadline and a per-plugin concurrency
//! budget.

use crate::capability::{Capability, TrustLevel};
use crate::error::PluginResult;

use parking_lot::RwLock;
use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, LazyLock};
use std::time::Duration;
use tokio::sync::Semaphore;

use super::events::{Event, EventBus, EventEmitError, SharedEventBus};
use super::models::{ModelRegistry, ModelSchema, SharedModelRegistry, SqlMigration};
use super::ssr::{RenderOptions, RenderResult, SharedSsrProxy, SsrError, SsrProxy};
use super::types::{ConfigValue, WitHttpResponse, WitPluginError};

/// Default maximum decoded response-body size buffered per HTTP host call (10 MiB).
const DEFAULT_HTTP_MAX_RESPONSE_BODY_BYTES: usize = 10 * 1024 * 1024;

/// Default maximum request-body size a plugin may send per HTTP host call (10 MiB).
const DEFAULT_HTTP_MAX_REQUEST_BODY_BYTES: usize = 10 * 1024 * 1024;

/// Default deadline for one HTTP host call, including the full response body.
const DEFAULT_HTTP_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Default number of HTTP host calls one plugin may have in flight.
const DEFAULT_HTTP_MAX_CONCURRENT_REQUESTS: usize = 4;

/// Connection deadline of the default plugin HTTP client factory.
const DEFAULT_HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Read-idle deadline of the default plugin HTTP client factory.
const DEFAULT_HTTP_READ_TIMEOUT: Duration = Duration::from_secs(15);

/// Shared default HTTP client reused across all `HostState` instances.
///
/// Creating a `reqwest::Client` per `HostState` wastes connection pools and
/// may lead to file descriptor exhaustion under high load. This shared client
/// ensures connection pools are reused across all plugins.
static DEFAULT_HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

/// Resource limits applied to capability-authorized HTTP host calls.
///
/// A plugin holding `NetworkAccess` chooses which public URL to fetch, so
/// response size and timing are attacker-influenced. Responses are buffered in
/// host memory before they are copied into the guest, where guest memory and
/// fuel limits do not apply. These limits bound that host-side work:
///
/// - A response whose declared `Content-Length` exceeds
///   [`max_response_body_bytes`](Self::max_response_body_bytes) is rejected
///   before its body is read. Other responses are streamed through a byte
///   counter, and the connection is dropped as soon as the limit is crossed.
/// - A request body above [`max_request_body_bytes`](Self::max_request_body_bytes)
///   is rejected before it is copied or sent.
/// - [`request_timeout`](Self::request_timeout) covers connecting, sending,
///   and reading the complete response body.
/// - At most [`max_concurrent_requests`](Self::max_concurrent_requests) calls
///   per plugin are in flight. Clones of a [`HostState`] share this budget, so
///   the response memory one plugin can hold at once is bounded by
///   `max_concurrent_requests * max_response_body_bytes`.
///
/// # Examples
///
/// ```
/// use reinhardt_dentdelion::wasm::{HostStateBuilder, HttpResourceLimits};
///
/// let mut limits = HttpResourceLimits::default();
/// limits.max_response_body_bytes = 1024 * 1024;
///
/// let state = HostStateBuilder::new("my-plugin").http_limits(limits).build();
/// assert_eq!(state.http_limits().max_response_body_bytes, 1024 * 1024);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct HttpResourceLimits {
	/// Maximum decoded response-body size buffered per call, in bytes.
	pub max_response_body_bytes: usize,
	/// Maximum request-body size a plugin may send per call, in bytes.
	pub max_request_body_bytes: usize,
	/// Deadline covering connection, request, and the complete response body.
	pub request_timeout: Duration,
	/// Maximum number of HTTP calls one plugin may have in flight.
	///
	/// Calls beyond this budget fail immediately instead of queueing.
	/// `0` rejects every HTTP host call.
	pub max_concurrent_requests: usize,
}

impl Default for HttpResourceLimits {
	fn default() -> Self {
		Self {
			max_response_body_bytes: DEFAULT_HTTP_MAX_RESPONSE_BODY_BYTES,
			max_request_body_bytes: DEFAULT_HTTP_MAX_REQUEST_BODY_BYTES,
			request_timeout: DEFAULT_HTTP_REQUEST_TIMEOUT,
			max_concurrent_requests: DEFAULT_HTTP_MAX_CONCURRENT_REQUESTS,
		}
	}
}

/// Create the in-flight permit pool for a set of HTTP limits.
fn new_http_permits(limits: &HttpResourceLimits) -> Arc<Semaphore> {
	// `Semaphore::new` panics above `MAX_PERMITS`; larger budgets are
	// indistinguishable from it in practice.
	Arc::new(Semaphore::new(
		limits.max_concurrent_requests.min(Semaphore::MAX_PERMITS),
	))
}

/// Read a response body, failing once it exceeds `limit` decoded bytes.
///
/// A declared `Content-Length` above the limit is rejected before any body
/// bytes are read. The declared length can be absent or wrong, so every
/// received chunk is still counted. Returning early drops the response, which
/// closes the connection instead of draining the rest of the body.
async fn read_limited_body(
	mut response: reqwest::Response,
	limit: usize,
) -> Result<Vec<u8>, WitPluginError> {
	let declared = response.content_length();
	if let Some(declared) = declared
		&& declared > limit as u64
	{
		return Err(response_body_too_large(limit));
	}

	// `declared` is at most `limit` here, so the conversion cannot truncate.
	let mut body = Vec::with_capacity(declared.map_or(0, |len| len as usize));
	while let Some(chunk) = response.chunk().await.map_err(|e| {
		WitPluginError::with_details(500, "Failed to read response body", e.to_string())
	})? {
		if chunk.len() > limit - body.len() {
			return Err(response_body_too_large(limit));
		}
		// Grow geometrically, but never past the limit, so the allocation
		// itself stays within the budget.
		if chunk.len() > body.capacity() - body.len() {
			let target = body
				.len()
				.saturating_mul(2)
				.max(body.len() + chunk.len())
				.min(limit);
			body.reserve_exact(target - body.len());
		}
		body.extend_from_slice(&chunk);
	}
	Ok(body)
}

/// Error returned when a response body exceeds the configured limit.
fn response_body_too_large(limit: usize) -> WitPluginError {
	WitPluginError::with_details(
		502,
		"HTTP response body exceeds the host limit",
		format!("limit is {limit} bytes"),
	)
}

type HttpClientFactory = Arc<dyn Fn() -> reqwest::ClientBuilder + Send + Sync>;

/// Default factory for pinned plugin HTTP clients.
///
/// The clients carry connect and read-idle deadlines. The total deadline is
/// enforced per call from [`HttpResourceLimits::request_timeout`], so it also
/// applies to clients from a custom [`HostStateBuilder::http_client_factory`].
fn default_http_client_factory() -> HttpClientFactory {
	Arc::new(|| {
		reqwest::Client::builder()
			.connect_timeout(DEFAULT_HTTP_CONNECT_TIMEOUT)
			.read_timeout(DEFAULT_HTTP_READ_TIMEOUT)
	})
}

/// Default maximum number of rows returned per database query host call.
const DEFAULT_DB_MAX_RESULT_ROWS: usize = 10_000;

/// Default maximum encoded result size returned per database query host call (10 MiB).
const DEFAULT_DB_MAX_RESULT_BYTES: usize = 10 * 1024 * 1024;

/// Default deadline for one database query host call, including every row.
const DEFAULT_DB_QUERY_TIMEOUT: Duration = Duration::from_secs(30);

/// Default number of database query host calls one plugin may have in flight.
const DEFAULT_DB_MAX_CONCURRENT_QUERIES: usize = 4;

/// Resource limits applied to capability-authorized database query host calls.
///
/// A plugin holding `DatabaseAccess` chooses the SQL it runs, so the size of
/// a result (including joins that expand into very many rows) is
/// attacker-influenced. Results are encoded in host memory before they are
/// copied into the guest, where guest memory and fuel limits do not apply.
/// These limits bound that host-side work:
///
/// - Rows are streamed from the backend and encoded one at a time. The call
///   fails as soon as the result exceeds
///   [`max_result_rows`](Self::max_result_rows) rows or
///   [`max_result_bytes`](Self::max_result_bytes) encoded bytes, and the row
///   stream is dropped so the host buffers no further rows.
/// - [`query_timeout`](Self::query_timeout) covers running the statement and
///   reading every row.
/// - At most [`max_concurrent_queries`](Self::max_concurrent_queries) query
///   calls per plugin are in flight. Clones of a [`HostState`] share this
///   budget, so the result memory one plugin can hold at once is bounded by
///   `max_concurrent_queries * max_result_bytes`.
///
/// These limits bound host memory and how long a call waits, not the work the
/// database does. An abandoned statement can keep running on the server, and
/// a pooled network connection discards its remaining output before reuse.
/// Configure a server-side statement timeout and a least-privilege role for
/// the plugin's database connection to bound that work.
///
/// The backend must support row streaming. A connection whose backend cannot
/// stream rows rejects every query instead of materializing the full result.
///
/// # Examples
///
/// ```
/// use reinhardt_dentdelion::wasm::{DatabaseResourceLimits, HostStateBuilder};
///
/// let mut limits = DatabaseResourceLimits::default();
/// limits.max_result_rows = 500;
///
/// let state = HostStateBuilder::new("my-plugin").db_limits(limits).build();
/// assert_eq!(state.db_limits().max_result_rows, 500);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DatabaseResourceLimits {
	/// Maximum number of rows returned per call.
	pub max_result_rows: usize,
	/// Maximum MessagePack-encoded size of the returned rows per call, in
	/// bytes. The enclosing array header (at most 5 bytes) is not counted.
	pub max_result_bytes: usize,
	/// Deadline covering the statement and reading every row.
	pub query_timeout: Duration,
	/// Maximum number of query calls one plugin may have in flight.
	///
	/// Calls beyond this budget fail immediately instead of queueing.
	/// `0` rejects every query host call.
	pub max_concurrent_queries: usize,
}

impl Default for DatabaseResourceLimits {
	fn default() -> Self {
		Self {
			max_result_rows: DEFAULT_DB_MAX_RESULT_ROWS,
			max_result_bytes: DEFAULT_DB_MAX_RESULT_BYTES,
			query_timeout: DEFAULT_DB_QUERY_TIMEOUT,
			max_concurrent_queries: DEFAULT_DB_MAX_CONCURRENT_QUERIES,
		}
	}
}

/// Create the in-flight permit pool for a set of database limits.
fn new_db_permits(limits: &DatabaseResourceLimits) -> Arc<Semaphore> {
	// `Semaphore::new` panics above `MAX_PERMITS`; larger budgets are
	// indistinguishable from it in practice.
	Arc::new(Semaphore::new(
		limits.max_concurrent_queries.min(Semaphore::MAX_PERMITS),
	))
}

/// Largest MessagePack array header, reserved before the encoded rows.
#[cfg(feature = "wasm")]
const MAX_MSGPACK_ARRAY_HEADER_LEN: usize = 5;

/// Byte sink that refuses writes once its buffer would exceed `limit` bytes.
///
/// Encoding through it stops as soon as a value would cross the limit, so an
/// oversized row never grows the result past the budget. Buffer growth is
/// capped at the limit as well, so the allocation stays within it.
#[cfg(feature = "wasm")]
struct LimitedWriter<'a> {
	buf: &'a mut Vec<u8>,
	limit: usize,
	exceeded: bool,
}

#[cfg(feature = "wasm")]
impl std::io::Write for LimitedWriter<'_> {
	fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
		if data.len() > self.limit.saturating_sub(self.buf.len()) {
			self.exceeded = true;
			return Err(std::io::Error::other("result byte limit exceeded"));
		}
		// Grow geometrically, but never past the limit.
		if data.len() > self.buf.capacity() - self.buf.len() {
			let target = self
				.buf
				.len()
				.saturating_mul(2)
				.max(self.buf.len() + data.len())
				.min(self.limit);
			self.buf.reserve_exact(target - self.buf.len());
		}
		self.buf.extend_from_slice(data);
		Ok(data.len())
	}

	fn flush(&mut self) -> std::io::Result<()> {
		Ok(())
	}
}

/// Encode a row stream as a MessagePack array, failing once it exceeds `limits`.
///
/// The output is byte-for-byte what `rmp_serde::to_vec` produces for the
/// equivalent `Vec<HashMap<String, QueryValue>>`. Each row is encoded and
/// dropped as soon as it arrives, and encoding stops at the byte limit even
/// within a row. Returning early drops the stream, so no further rows reach
/// host memory. The driver still materializes each row it yields; only the
/// database can bound the size of a single value.
#[cfg(feature = "wasm")]
async fn encode_limited_rows(
	mut rows: futures_util::stream::BoxStream<
		'_,
		reinhardt_db::backends::Result<reinhardt_db::backends::types::Row>,
	>,
	limits: &DatabaseResourceLimits,
) -> Result<Vec<u8>, WitPluginError> {
	use futures_util::StreamExt;

	// The array header's size depends on the final row count, so reserve the
	// largest one and trim the unused prefix at the end.
	let mut encoded = vec![0; MAX_MSGPACK_ARRAY_HEADER_LEN];
	let mut writer = LimitedWriter {
		buf: &mut encoded,
		limit: MAX_MSGPACK_ARRAY_HEADER_LEN.saturating_add(limits.max_result_bytes),
		exceeded: false,
	};
	let mut row_count: usize = 0;
	while let Some(row) = rows.next().await {
		let row = row.map_err(|e| {
			WitPluginError::with_details(500, "Database query failed", e.to_string())
		})?;
		if row_count == limits.max_result_rows {
			return Err(db_result_too_large(format!(
				"limit is {} rows",
				limits.max_result_rows
			)));
		}
		row_count += 1;

		if let Err(error) = rmp_serde::encode::write(&mut writer, &row.data) {
			return Err(if writer.exceeded {
				db_result_too_large(format!("limit is {} bytes", limits.max_result_bytes))
			} else {
				db_serialize_failed(error)
			});
		}
	}

	// MessagePack arrays hold at most `u32::MAX` items.
	let row_count = u32::try_from(row_count)
		.map_err(|_| db_result_too_large(format!("limit is {} rows", u32::MAX)))?;
	let mut header = Vec::with_capacity(MAX_MSGPACK_ARRAY_HEADER_LEN);
	rmp::encode::write_array_len(&mut header, row_count).map_err(db_serialize_failed)?;
	let start = MAX_MSGPACK_ARRAY_HEADER_LEN - header.len();
	encoded[start..MAX_MSGPACK_ARRAY_HEADER_LEN].copy_from_slice(&header);
	encoded.drain(..start);
	Ok(encoded)
}

/// Error returned when a query result exceeds a configured limit.
#[cfg(feature = "wasm")]
fn db_result_too_large(details: String) -> WitPluginError {
	WitPluginError::with_details(413, "Database query result exceeds the host limit", details)
}

/// Error returned when query rows cannot be encoded.
#[cfg(feature = "wasm")]
fn db_serialize_failed(error: impl std::fmt::Display) -> WitPluginError {
	WitPluginError::with_details(500, "Failed to serialize results", error.to_string())
}

#[cfg(feature = "ts")]
use super::ts_runtime::SharedTsRuntime;

#[cfg(feature = "wasm")]
use reinhardt_db::backends::connection::DatabaseConnection;

#[cfg(feature = "wasm")]
use super::runtime::StoreMemoryLimits;
#[cfg(feature = "wasm")]
use wasmtime::component::ResourceTable;
#[cfg(feature = "wasm")]
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

/// Validate a URL to prevent Server-Side Request Forgery (SSRF) attacks.
///
/// Performs the following checks:
/// 1. Only allows `http://` and `https://` schemes
/// 2. Resolves the hostname to an IP address
/// 3. Rejects requests to private, loopback, and link-local IP addresses
async fn validate_url_for_ssrf(url: &str) -> Result<Vec<std::net::SocketAddr>, WitPluginError> {
	validate_url_for_ssrf_with_resolver(url, |domain, port| async move {
		tokio::net::lookup_host((domain, port))
			.await
			.map(Iterator::collect)
	})
	.await
}

async fn validate_url_for_ssrf_with_resolver<R, F>(
	url: &str,
	resolver: R,
) -> Result<Vec<std::net::SocketAddr>, WitPluginError>
where
	R: FnOnce(String, u16) -> F,
	F: std::future::Future<Output = std::io::Result<Vec<std::net::SocketAddr>>>,
{
	// Parse the URL
	let parsed = url::Url::parse(url)
		.map_err(|e| WitPluginError::with_details(400, "Invalid URL", e.to_string()))?;
	if !parsed.username().is_empty() || parsed.password().is_some() {
		return Err(WitPluginError::new(403, "URL userinfo is not allowed"));
	}

	// Validate scheme: only http and https are allowed
	match parsed.scheme() {
		"http" | "https" => {}
		scheme => {
			return Err(WitPluginError::new(
				403,
				format!(
					"URL scheme '{}' is not allowed; only http and https are permitted",
					scheme
				),
			));
		}
	}

	// Extract host
	let host = parsed
		.host()
		.ok_or_else(|| WitPluginError::new(400, "URL must contain a host"))?;

	// Determine port (default to 80 for http, 443 for https)
	let port = parsed.port_or_known_default().unwrap_or(80);

	// Resolve hostname to IP addresses for DNS rebinding protection
	let socket_addrs: Vec<_> = match host {
		url::Host::Domain(domain) => tokio::time::timeout(
			std::time::Duration::from_secs(30),
			resolver(domain.to_owned(), port),
		)
		.await
		.map_err(|_| WitPluginError::new(408, "DNS resolution timed out"))?
		.map_err(|e| {
			WitPluginError::with_details(400, "Failed to resolve hostname", e.to_string())
		})?,
		url::Host::Ipv4(ip) => vec![std::net::SocketAddr::new(ip.into(), port)],
		url::Host::Ipv6(ip) => vec![std::net::SocketAddr::new(ip.into(), port)],
	};

	if socket_addrs.is_empty() {
		return Err(WitPluginError::new(
			400,
			"Hostname resolved to no addresses",
		));
	}

	// Check each resolved IP address against blocklists
	for addr in &socket_addrs {
		if is_private_ip(&addr.ip()) {
			return Err(WitPluginError::new(
				403,
				format!(
					"Requests to private/internal IP address {} are not allowed",
					addr.ip()
				),
			));
		}
	}

	Ok(socket_addrs)
}

/// Construct a one-request client pinned to addresses checked above.
/// A fresh client avoids reusing a pooled connection resolved for another URL.
async fn ssrf_safe_client(
	url: &str,
	factory: &HttpClientFactory,
) -> Result<reqwest::Client, WitPluginError> {
	let addrs = validate_url_for_ssrf(url).await?;
	build_pinned_http_client(url, factory, &addrs)
}

fn build_pinned_http_client(
	url: &str,
	factory: &HttpClientFactory,
	addrs: &[std::net::SocketAddr],
) -> Result<reqwest::Client, WitPluginError> {
	let parsed = url::Url::parse(url)
		.map_err(|e| WitPluginError::with_details(400, "Invalid URL", e.to_string()))?;
	// No client-level total timeout: `HostState::execute_http_request` bounds
	// every call by `HttpResourceLimits::request_timeout`. A second timer of the
	// same length would race it and could report the deadline as a 500.
	let mut builder = factory()
		.redirect(reqwest::redirect::Policy::none())
		.no_proxy();
	if let Some(url::Host::Domain(domain)) = parsed.host() {
		builder = builder.resolve_to_addrs(domain, addrs);
	}
	builder
		.build()
		.map_err(|e| WitPluginError::with_details(500, "HTTP client failed", e.to_string()))
}

/// Check if an IP address is private, loopback, or link-local.
fn is_private_ip(ip: &std::net::IpAddr) -> bool {
	use std::net::IpAddr;

	match ip {
		IpAddr::V4(ipv4) => {
			let o = ipv4.octets();
			// IANA's globally reachable PCP/TURN anycast /32s override the /24 parent.
			// https://www.iana.org/assignments/iana-ipv4-special-registry
			if matches!(o, [192, 0, 0, 9 | 10]) {
				return false;
			}
			// 127.0.0.0/8 - Loopback
			ipv4.is_loopback()
			// 10.0.0.0/8 - Private
			|| ipv4.octets()[0] == 10
			// 172.16.0.0/12 - Private
			|| (ipv4.octets()[0] == 172 && (ipv4.octets()[1] & 0xf0) == 16)
			// 192.168.0.0/16 - Private
			|| (ipv4.octets()[0] == 192 && ipv4.octets()[1] == 168)
			// 169.254.0.0/16 - Link-local
			|| (ipv4.octets()[0] == 169 && ipv4.octets()[1] == 254)
			// 0.0.0.0/8 - Current network
			|| o[0] == 0
			// Shared, reserved, documentation, benchmarking, and multicast ranges
			|| (o[0] == 100 && (o[1] & 0xc0) == 64)
			|| (o[0] == 192 && o[1] == 0 && matches!(o[2], 0 | 2))
			// IANA marks the 6a44 relay anycast /32 as not globally reachable.
			|| matches!(o, [192, 88, 99, 2])
			|| (o[0] == 198 && (o[1] == 18 || o[1] == 19 || (o[1] == 51 && o[2] == 100)))
			|| (o[0] == 203 && o[1] == 0 && o[2] == 113)
			|| o[0] >= 224
		}
		IpAddr::V6(ipv6) => !is_public_ipv6(ipv6),
	}
}

/// Global-unicast locators, excluding non-global and overlay identifier assignments.
fn is_public_ipv6(ip: &std::net::Ipv6Addr) -> bool {
	let segments = ip.segments();
	// The NAT64 well-known prefix 64:ff9b::/96 is globally reachable (IANA) and
	// DNS64 synthesizes it for public IPv4 hosts; judge the embedded IPv4
	// address (RFC 6052 section 2.2) so private targets stay rejected. The
	// local-use prefix 64:ff9b:1::/48 is not global and falls through below.
	if segments[..6] == [0x0064, 0xff9b, 0, 0, 0, 0] {
		let [.., a, b, c, d] = ip.octets();
		return !is_private_ip(&std::net::IpAddr::V4(std::net::Ipv4Addr::new(a, b, c, d)));
	}
	if (segments[0] & 0xe000) != 0x2000
		|| (segments[0] == 0x2001 && segments[1] == 0x0db8)
		|| segments[0] == 0x2002
		|| (segments[0] == 0x3fff && (segments[1] & 0xf000) == 0)
	{
		return false;
	}
	// IANA 2001::/23 is non-global except more-specific assignments:
	// https://www.iana.org/assignments/iana-ipv6-special-registry
	if segments[0] == 0x2001 && (segments[1] & 0xfe00) == 0 {
		// Keep actual anycast, AMT and AS112 locators. ORCHIDv2 and DETs have
		// IANA Global=True, but are non-routable identifiers at the IPv6 layer
		// (RFC 7343 section 1.2/3 and RFC 9374 section 1); HTTP uses locators.
		return (segments[1] == 1 && segments[2..7] == [0; 5] && matches!(segments[7], 1..=3))
			|| segments[1] == 3
			|| (segments[1] == 4 && segments[2] == 0x0112);
	}
	true
}

/// Host state that is passed to WASM instances.
///
/// This struct contains all the state that plugins can access through host functions.
pub struct HostState {
	/// Plugin name (for logging context)
	pub plugin_name: String,
	/// Trust level for this plugin (determines security restrictions)
	trust_level: TrustLevel,
	/// Configuration values accessible to the plugin
	config: RwLock<HashMap<String, ConfigValue>>,
	/// Registered services (name -> MessagePack-serialized data)
	services: RwLock<HashMap<String, Vec<u8>>>,
	/// Type-safe services (for internal use)
	typed_services: RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
	/// Granted capabilities for this plugin
	capabilities: HashSet<Capability>,
	/// Whether plugin HTTP requests are enabled; requests use a fresh pinned client.
	http_client: Option<reqwest::Client>,
	http_client_factory: Option<HttpClientFactory>,
	/// Resource limits applied to HTTP host calls
	http_limits: HttpResourceLimits,
	/// In-flight HTTP call permits, shared with clones of this state
	http_permits: Arc<Semaphore>,
	/// Database connection for SQL queries (optional, requires DatabaseAccess capability)
	#[cfg(feature = "wasm")]
	db_connection: Option<Arc<DatabaseConnection>>,
	#[cfg(not(feature = "wasm"))]
	db_connection: Option<()>,
	/// Resource limits applied to database query host calls
	db_limits: DatabaseResourceLimits,
	/// In-flight database query permits, shared with clones of this state
	db_permits: Arc<Semaphore>,
	/// Event bus for inter-plugin communication
	event_bus: SharedEventBus,
	/// Model registry for schema and migration registration
	model_registry: SharedModelRegistry,
	/// SSR proxy for server-side rendering (requires TypeScript runtime)
	ssr_proxy: SharedSsrProxy,
	/// WASI context for WASI P2 interface support
	/// Wrapped in Mutex to allow interior mutability while maintaining Sync
	#[cfg(feature = "wasm")]
	wasi_ctx: parking_lot::Mutex<WasiCtx>,
	/// Resource table for WASI P2 interface support
	/// Wrapped in Mutex to allow interior mutability while maintaining Sync
	#[cfg(feature = "wasm")]
	resource_table: parking_lot::Mutex<ResourceTable>,
	/// Per-store memory/table limit, installed by `WasmRuntime::create_store`.
	#[cfg(feature = "wasm")]
	pub(super) store_limits: Option<StoreMemoryLimits>,
	/// Absolute invocation deadline, including asynchronous host imports.
	#[cfg(feature = "wasm")]
	pub(super) execution_deadline: Option<std::time::Instant>,
}

impl HostState {
	/// Create a new host state with default configuration.
	pub fn new(plugin_name: impl Into<String>) -> Self {
		Self::with_shared_resources(
			plugin_name,
			Arc::new(EventBus::new()),
			Arc::new(ModelRegistry::new()),
			Arc::new(SsrProxy::new()),
		)
	}

	/// Create a new host state with a shared event bus.
	///
	/// This allows multiple plugin instances to share the same event bus
	/// for inter-plugin communication.
	pub fn with_event_bus(plugin_name: impl Into<String>, event_bus: SharedEventBus) -> Self {
		Self::with_shared_resources(
			plugin_name,
			event_bus,
			Arc::new(ModelRegistry::new()),
			Arc::new(SsrProxy::new()),
		)
	}

	/// Create a new host state with shared event bus and model registry.
	///
	/// This allows multiple plugin instances to share the same event bus,
	/// model registry, and SSR proxy for inter-plugin communication,
	/// schema management, and server-side rendering.
	pub fn with_shared_resources(
		plugin_name: impl Into<String>,
		event_bus: SharedEventBus,
		model_registry: SharedModelRegistry,
		ssr_proxy: SharedSsrProxy,
	) -> Self {
		let http_limits = HttpResourceLimits::default();
		let db_limits = DatabaseResourceLimits::default();
		Self {
			plugin_name: plugin_name.into(),
			trust_level: TrustLevel::default(),
			config: RwLock::new(HashMap::new()),
			services: RwLock::new(HashMap::new()),
			typed_services: RwLock::new(HashMap::new()),
			capabilities: HashSet::new(),
			http_client: Some(DEFAULT_HTTP_CLIENT.clone()),
			http_client_factory: Some(default_http_client_factory()),
			http_permits: new_http_permits(&http_limits),
			http_limits,
			db_connection: None,
			db_permits: new_db_permits(&db_limits),
			db_limits,
			event_bus,
			model_registry,
			ssr_proxy,
			#[cfg(feature = "wasm")]
			wasi_ctx: parking_lot::Mutex::new(WasiCtxBuilder::new().build()),
			#[cfg(feature = "wasm")]
			resource_table: parking_lot::Mutex::new(ResourceTable::new()),
			#[cfg(feature = "wasm")]
			store_limits: None,
			#[cfg(feature = "wasm")]
			execution_deadline: None,
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

	/// Get all configuration values as a cloned HashMap.
	///
	/// This method is primarily used for serializing configuration to pass
	/// to WASM plugins during lifecycle events.
	pub fn get_config_all(&self) -> HashMap<String, ConfigValue> {
		self.config.read().clone()
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

	async fn pinned_http_client(&self, url: &str) -> Result<reqwest::Client, WitPluginError> {
		self.http_client
			.as_ref()
			.ok_or_else(|| WitPluginError::new(500, "HTTP client not available"))?;
		let factory = self.http_client_factory.as_ref().ok_or_else(|| {
			WitPluginError::new(500, "A prebuilt HTTP client cannot be safely pinned; configure HostStateBuilder::http_client_factory instead")
		})?;
		ssrf_safe_client(url, factory).await
	}

	/// Check if HTTP access is allowed.
	fn has_http_capability(&self) -> bool {
		use crate::capability::PluginCapability;
		self.capabilities
			.contains(&Capability::Core(PluginCapability::NetworkAccess))
	}

	/// Get the resource limits applied to HTTP host calls.
	pub fn http_limits(&self) -> &HttpResourceLimits {
		&self.http_limits
	}

	/// Perform an HTTP GET request.
	///
	/// Validates the URL against SSRF attacks before making the request.
	/// Only `http://` and `https://` schemes are allowed, and requests to
	/// private/loopback IP addresses are rejected. The call is bounded by
	/// this state's [`HttpResourceLimits`].
	pub async fn http_get(
		&self,
		url: &str,
		headers: &[(String, String)],
	) -> Result<WitHttpResponse, WitPluginError> {
		let client = self.authorize_http_call(url).await?;

		let mut request = client.get(url);

		for (name, value) in headers {
			request = request.header(name, value);
		}

		self.execute_http_request(request).await
	}

	/// Perform an HTTP POST request.
	///
	/// Validates the URL against SSRF attacks before making the request.
	/// Only `http://` and `https://` schemes are allowed, and requests to
	/// private/loopback IP addresses are rejected. The call is bounded by
	/// this state's [`HttpResourceLimits`]; an oversized body is rejected
	/// before it is copied into the request.
	pub async fn http_post(
		&self,
		url: &str,
		body: &[u8],
		headers: &[(String, String)],
	) -> Result<WitHttpResponse, WitPluginError> {
		let client = self.authorize_http_call(url).await?;
		self.check_http_request_body(body.len())?;
		self.send_http_post(&client, url, body.to_vec(), headers)
			.await
	}

	/// Owned-body form of [`Self::http_post`] for the WIT binding, which
	/// already owns the body lifted from guest memory and would otherwise
	/// copy it a second time.
	async fn http_post_owned(
		&self,
		url: &str,
		body: Vec<u8>,
		headers: &[(String, String)],
	) -> Result<WitHttpResponse, WitPluginError> {
		let client = self.authorize_http_call(url).await?;
		self.check_http_request_body(body.len())?;
		self.send_http_post(&client, url, body, headers).await
	}

	/// Run the checks shared by every HTTP host call and return a client
	/// pinned to the validated destination.
	async fn authorize_http_call(&self, url: &str) -> Result<reqwest::Client, WitPluginError> {
		if !self.has_http_capability() {
			return Err(WitPluginError::new(403, "HTTP access not permitted"));
		}

		self.pinned_http_client(url).await
	}

	/// Reject a plugin-supplied request body larger than the configured limit.
	fn check_http_request_body(&self, len: usize) -> Result<(), WitPluginError> {
		let limit = self.http_limits.max_request_body_bytes;
		if len > limit {
			return Err(WitPluginError::with_details(
				413,
				"HTTP request body exceeds the host limit",
				format!("{len} bytes exceeds the limit of {limit} bytes"),
			));
		}
		Ok(())
	}

	/// Build and execute a POST request with an already size-checked body.
	async fn send_http_post(
		&self,
		client: &reqwest::Client,
		url: &str,
		body: Vec<u8>,
		headers: &[(String, String)],
	) -> Result<WitHttpResponse, WitPluginError> {
		let mut request = client.post(url).body(body);

		for (name, value) in headers {
			request = request.header(name, value);
		}

		self.execute_http_request(request).await
	}

	/// Send a prepared request under this plugin's HTTP resource limits.
	///
	/// The deadline wraps both sending and body reading, so it holds even for
	/// a custom client without timeouts. The concurrency permit is held until
	/// the body has been read, which bounds the response memory one plugin can
	/// buffer at once.
	async fn execute_http_request(
		&self,
		request: reqwest::RequestBuilder,
	) -> Result<WitHttpResponse, WitPluginError> {
		let limits = &self.http_limits;
		let _permit = self.http_permits.try_acquire().map_err(|_| {
			WitPluginError::with_details(
				429,
				"Too many concurrent HTTP requests",
				format!("limit is {} per plugin", limits.max_concurrent_requests),
			)
		})?;

		let exchange = async {
			let response = request.send().await.map_err(|e| {
				WitPluginError::with_details(500, "HTTP request failed", e.to_string())
			})?;

			let status = response.status().as_u16();
			let headers: Vec<(String, String)> = response
				.headers()
				.iter()
				.map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
				.collect();
			let body = read_limited_body(response, limits.max_response_body_bytes).await?;

			Ok(WitHttpResponse {
				status,
				headers,
				body,
			})
		};

		tokio::time::timeout(limits.request_timeout, exchange)
			.await
			.map_err(|_| {
				WitPluginError::with_details(
					504,
					"HTTP request timed out",
					format!("deadline is {:?}", limits.request_timeout),
				)
			})?
	}

	// ===== Database API =====

	/// Check if database access is allowed.
	fn has_db_capability(&self) -> bool {
		use crate::capability::PluginCapability;
		self.capabilities
			.contains(&Capability::Core(PluginCapability::DatabaseAccess))
	}

	/// Get the resource limits applied to database query host calls.
	pub fn db_limits(&self) -> &DatabaseResourceLimits {
		&self.db_limits
	}

	/// Execute a SQL query.
	///
	/// Returns the rows as a MessagePack array of column-name maps. The call
	/// is bounded by this state's [`DatabaseResourceLimits`]: rows are
	/// streamed and encoded one at a time, and the call fails instead of
	/// materializing a result beyond the limits.
	#[cfg(feature = "wasm")]
	pub async fn db_query(&self, sql: &str, params: &[u8]) -> Result<Vec<u8>, WitPluginError> {
		if !self.has_db_capability() {
			return Err(WitPluginError::new(403, "Database access not permitted"));
		}

		// Validate SQL statement to prevent SQL injection
		use super::sql_validator::validate_sql;
		validate_sql(sql).map_err(|e| {
			WitPluginError::with_details(403, "SQL validation failed", e.to_string())
		})?;

		let conn = self
			.db_connection
			.as_ref()
			.ok_or_else(|| WitPluginError::new(500, "Database connection not available"))?;

		// Deserialize MessagePack parameters
		use reinhardt_db::backends::types::QueryValue;
		let params_vec: Vec<QueryValue> = rmp_serde::from_slice(params).map_err(|e| {
			WitPluginError::with_details(400, "Invalid query parameters", e.to_string())
		})?;

		let limits = &self.db_limits;
		let _permit = self.db_permits.try_acquire().map_err(|_| {
			WitPluginError::with_details(
				429,
				"Too many concurrent database queries",
				format!("limit is {} per plugin", limits.max_concurrent_queries),
			)
		})?;

		// Stream the rows instead of `fetch_all`, which would materialize the
		// whole result before any limit could apply. A backend without row
		// streaming fails here rather than falling back to that path.
		let query = async {
			let rows = conn
				.__fetch_stream(sql.to_owned(), params_vec)
				.map_err(|e| {
					WitPluginError::with_details(500, "Database query failed", e.to_string())
				})?;
			encode_limited_rows(rows, limits).await
		};

		tokio::time::timeout(limits.query_timeout, query)
			.await
			.map_err(|_| {
				WitPluginError::with_details(
					504,
					"Database query timed out",
					format!("deadline is {:?}", limits.query_timeout),
				)
			})?
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

		// Validate SQL statement to prevent SQL injection
		use super::sql_validator::validate_sql;
		validate_sql(sql).map_err(|e| {
			WitPluginError::with_details(403, "SQL validation failed", e.to_string())
		})?;

		let conn = self
			.db_connection
			.as_ref()
			.ok_or_else(|| WitPluginError::new(500, "Database connection not available"))?;

		// Deserialize MessagePack parameters
		use reinhardt_db::backends::types::QueryValue;
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

	// ===== Trust Level API =====

	/// Get the trust level for this plugin.
	pub fn trust_level(&self) -> TrustLevel {
		self.trust_level
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

	// ===== Event Bus API =====
	//
	// The public methods below act on the event bus as this plugin without
	// checking its grants, like `event_bus()`. Plugin-originated calls arrive
	// through the WIT `events` interface, which first requires `Signals`.

	/// Get a reference to the shared event bus.
	pub fn event_bus(&self) -> &SharedEventBus {
		&self.event_bus
	}

	/// Check if event bus access is allowed.
	fn has_signals_capability(&self) -> bool {
		use crate::capability::PluginCapability;
		self.capabilities
			.contains(&Capability::Core(PluginCapability::Signals))
	}

	/// Reject a plugin event host call unless `Signals` is granted.
	///
	/// Every WIT `events` call runs this before touching the event bus, so a
	/// plugin without the grant cannot emit, subscribe, unsubscribe, or poll,
	/// and cannot tell whether a subscription ID exists.
	fn authorize_event_call(&self) -> Result<(), WitPluginError> {
		if !self.has_signals_capability() {
			return Err(WitPluginError::new(403, "Event access not permitted"));
		}
		Ok(())
	}

	/// Emit an event to all matching subscribers.
	///
	/// Events that exceed the event bus size limits are not delivered and
	/// `0` is returned; use [`try_emit_event`](Self::try_emit_event) to
	/// observe the rejection.
	///
	/// # Arguments
	///
	/// * `name` - Event name (e.g., "user.created")
	/// * `payload` - MessagePack-serialized event data
	///
	/// # Returns
	///
	/// The number of subscriptions that received the event.
	pub fn emit_event(&self, name: &str, payload: Vec<u8>) -> usize {
		self.event_bus.emit(name, payload, &self.plugin_name)
	}

	/// Emit an event to all matching subscribers, rejecting oversized events.
	///
	/// # Returns
	///
	/// The number of subscriptions that received the event.
	///
	/// # Errors
	///
	/// Returns an error if the event name or payload exceeds the event bus
	/// [`EventByteLimits`](crate::wasm::EventByteLimits).
	pub fn try_emit_event(&self, name: &str, payload: Vec<u8>) -> Result<usize, EventEmitError> {
		self.event_bus.try_emit(name, payload, &self.plugin_name)
	}

	/// Subscribe to events matching a pattern.
	///
	/// # Pattern Syntax
	///
	/// - `*` - Matches all events
	/// - `user.*` - Matches events starting with "user." (e.g., "user.created")
	/// - `user.created` - Matches only "user.created"
	///
	/// # Returns
	///
	/// A unique subscription ID for polling and unsubscribing.
	///
	/// # Errors
	///
	/// Returns an error if the subscription limit has been reached.
	pub fn subscribe_events(
		&self,
		pattern: &str,
	) -> Result<u64, crate::wasm::events::EventBusError> {
		self.event_bus.subscribe(pattern, &self.plugin_name)
	}

	/// Unsubscribe from a subscription.
	pub fn unsubscribe_events(&self, subscription_id: u64) -> bool {
		self.event_bus
			.unsubscribe_for_owner(subscription_id, &self.plugin_name)
	}

	/// Poll for pending events on a subscription.
	///
	/// Events are removed from the queue once polled.
	pub fn poll_pending_events(&self, subscription_id: u64, limit: usize) -> Vec<Event> {
		self.event_bus
			.poll_for_owner(subscription_id, limit, &self.plugin_name)
	}

	// ===== Model Registry API =====

	/// Get a reference to the shared model registry.
	pub fn model_registry(&self) -> &SharedModelRegistry {
		&self.model_registry
	}

	/// Register a model schema.
	///
	/// # Arguments
	///
	/// * `schema` - The model schema to register
	///
	/// # Returns
	///
	/// An error if a schema with the same table name is already registered
	/// by this plugin.
	pub fn register_model_schema(&self, schema: ModelSchema) -> Result<(), String> {
		self.model_registry
			.register_model(&self.plugin_name, schema)
	}

	/// Register a raw SQL migration.
	///
	/// # Arguments
	///
	/// * `migration` - The SQL migration to register
	///
	/// # Returns
	///
	/// An error if a migration with the same version is already registered
	/// by this plugin.
	pub fn register_sql_migration(&self, migration: SqlMigration) -> Result<(), String> {
		self.model_registry
			.register_migration(&self.plugin_name, migration)
	}

	/// List all model table names registered by this plugin.
	pub fn list_registered_models(&self) -> Vec<String> {
		self.model_registry.list_models(&self.plugin_name)
	}

	/// Get a model schema by table name.
	pub fn get_registered_model(&self, table_name: &str) -> Option<ModelSchema> {
		self.model_registry.get_model(&self.plugin_name, table_name)
	}

	/// List all migrations registered by this plugin.
	pub fn list_registered_migrations(&self) -> Vec<SqlMigration> {
		self.model_registry.list_migrations(&self.plugin_name)
	}

	// ===== SSR Proxy API =====

	/// Get a reference to the shared SSR proxy.
	pub fn ssr_proxy(&self) -> &SharedSsrProxy {
		&self.ssr_proxy
	}

	/// Check if SSR is available.
	///
	/// Returns `true` if the host has TypeScript runtime support enabled.
	pub fn is_ssr_available(&self) -> bool {
		self.ssr_proxy.is_available()
	}

	/// Render a React component to HTML.
	///
	/// # Arguments
	///
	/// * `component_path` - Path to the component file (relative to plugin assets)
	/// * `props` - MessagePack-serialized component props
	/// * `options` - Rendering options
	///
	/// # Returns
	///
	/// Rendered HTML and optional extracted assets, or an error if SSR is not available
	/// or the trust level is insufficient.
	///
	/// # Security
	///
	/// Only plugins with `Verified` or `Trusted` trust level can render components.
	pub async fn render_react(
		&self,
		component_path: &str,
		props: &[u8],
		options: RenderOptions,
	) -> Result<RenderResult, SsrError> {
		if !self.trust_level.allows_ssr() {
			return Err(SsrError::PermissionDenied(format!(
				"SSR rendering requires at least Verified trust level, but plugin '{}' has {:?} trust level",
				self.plugin_name, self.trust_level
			)));
		}
		self.ssr_proxy
			.render_react_until(component_path, props, options, self.execution_deadline)
			.await
	}

	/// Execute arbitrary JavaScript code.
	///
	/// # Arguments
	///
	/// * `code` - JavaScript code to execute
	///
	/// # Returns
	///
	/// The result as UTF-8 bytes, or an error if the trust level is insufficient
	/// or SSR is not available.
	///
	/// # Security
	///
	/// Only plugins with `Trusted` trust level can execute arbitrary JavaScript.
	/// This prevents untrusted or verified plugins from accessing the JavaScript
	/// runtime directly.
	pub fn eval_js(&self, code: &str) -> Result<Vec<u8>, SsrError> {
		self.require_js_execution_trust()?;
		self.ssr_proxy.eval_js(code).map(|s| s.into_bytes())
	}

	fn require_js_execution_trust(&self) -> Result<(), SsrError> {
		if !self.trust_level.allows_js_execution() {
			return Err(SsrError::PermissionDenied(format!(
				"JavaScript execution requires Trusted trust level, but plugin '{}' has {:?} trust level",
				self.plugin_name, self.trust_level
			)));
		}
		Ok(())
	}

	/// Render a component with trust level enforcement.
	///
	/// # Arguments
	///
	/// * `component_code` - JavaScript code defining the component
	/// * `props_json` - JSON string representing component props
	/// * `options` - Rendering options
	///
	/// # Security
	///
	/// Only plugins with `Verified` or `Trusted` trust level can render components.
	/// `Untrusted` plugins cannot use SSR as it involves JavaScript execution.
	/// Component code is also validated for dangerous patterns.
	pub fn render_component(
		&self,
		component_code: &str,
		props_json: &str,
		options: RenderOptions,
	) -> Result<RenderResult, SsrError> {
		if !self.trust_level.allows_ssr() {
			return Err(SsrError::PermissionDenied(format!(
				"SSR rendering requires at least Verified trust level, but plugin '{}' has {:?} trust level",
				self.plugin_name, self.trust_level
			)));
		}
		self.ssr_proxy
			.render_component(component_code, props_json, options)
	}
}

// Clone implementation for HostState (needed for store creation).
// Preserves all state (config, services, capabilities, shared resources).
// WASI context and resource table are created fresh since each store
// requires its own isolated WASI state.
impl Clone for HostState {
	fn clone(&self) -> Self {
		let mut builder = HostStateBuilder::new(&self.plugin_name)
			.config_all(self.config.read().clone())
			.capabilities(self.capabilities.clone())
			.event_bus(self.event_bus.clone())
			.model_registry(self.model_registry.clone())
			.ssr_proxy(self.ssr_proxy.clone());

		if let Some(ref client) = self.http_client {
			builder = builder.http_client(client.clone());
		} else {
			builder = builder.no_http_client();
		}
		builder.http_client_factory = self.http_client_factory.clone();

		// Share the permit pool so every store of this plugin draws on one
		// concurrency budget instead of receiving a fresh one per clone.
		builder = builder
			.http_limits(self.http_limits.clone())
			.shared_http_permits(self.http_permits.clone());

		#[cfg(feature = "wasm")]
		if let Some(ref conn) = self.db_connection {
			builder = builder.db_connection(conn.clone());
		}

		// Share the query permit pool for the same reason as the HTTP one.
		builder = builder
			.db_limits(self.db_limits.clone())
			.shared_db_permits(self.db_permits.clone());

		let state = builder.build();

		// Clone serialized services
		let services = self.services.read().clone();
		for (name, data) in services {
			let _ = state.register_service(&name, data);
		}

		// Clone typed services (Arc-based shared references)
		*state.typed_services.write() = self.typed_services.read().clone();

		state
	}
}

impl std::fmt::Debug for HostState {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("HostState")
			.field("plugin_name", &self.plugin_name)
			.field("trust_level", &self.trust_level)
			.field("config_count", &self.config.read().len())
			.field("service_count", &self.services.read().len())
			.field("capabilities", &self.capabilities)
			.finish_non_exhaustive()
	}
}

/// Builder for `HostState`.
pub struct HostStateBuilder {
	plugin_name: String,
	trust_level: TrustLevel,
	config: HashMap<String, ConfigValue>,
	capabilities: HashSet<Capability>,
	http_client: Option<reqwest::Client>,
	http_client_factory: Option<HttpClientFactory>,
	http_limits: HttpResourceLimits,
	http_permits: Option<Arc<Semaphore>>,
	#[cfg(feature = "wasm")]
	db_connection: Option<Arc<DatabaseConnection>>,
	#[cfg(not(feature = "wasm"))]
	db_connection: Option<()>,
	db_limits: DatabaseResourceLimits,
	db_permits: Option<Arc<Semaphore>>,
	event_bus: Option<SharedEventBus>,
	model_registry: Option<SharedModelRegistry>,
	ssr_proxy: Option<SharedSsrProxy>,
}

impl HostStateBuilder {
	/// Create a new builder.
	///
	/// Uses the shared default HTTP client to avoid creating redundant connection pools.
	pub fn new(plugin_name: impl Into<String>) -> Self {
		Self {
			plugin_name: plugin_name.into(),
			trust_level: TrustLevel::default(),
			config: HashMap::new(),
			capabilities: HashSet::new(),
			http_client: Some(DEFAULT_HTTP_CLIENT.clone()),
			http_client_factory: Some(default_http_client_factory()),
			http_limits: HttpResourceLimits::default(),
			http_permits: None,
			db_connection: None,
			db_limits: DatabaseResourceLimits::default(),
			db_permits: None,
			event_bus: None,
			model_registry: None,
			ssr_proxy: None,
		}
	}

	/// Set the trust level for this plugin.
	///
	/// Default is `TrustLevel::Untrusted`.
	pub fn trust_level(mut self, trust_level: TrustLevel) -> Self {
		self.trust_level = trust_level;
		self
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

	/// Set a prebuilt HTTP client.
	///
	/// Requests fail with a migration error because an opaque client cannot
	/// expose its DNS, proxy, redirect, or TLS settings for safe destination
	/// pinning. Use `http_client_factory` to retain custom client settings.
	pub fn http_client(mut self, client: reqwest::Client) -> Self {
		self.http_client = Some(client);
		self.http_client_factory = None;
		self
	}

	/// Configure clients for validated plugin HTTP destinations (native-only, P0).
	///
	/// The trusted factory preserves custom headers, CA roots, and client
	/// certificates. Address pinning and disabled redirects/proxies are applied
	/// after the factory returns, and every call is bounded by the
	/// [`HttpResourceLimits`] set with [`Self::http_limits`].
	pub fn http_client_factory<F>(mut self, factory: F) -> Self
	where
		F: Fn() -> reqwest::ClientBuilder + Send + Sync + 'static,
	{
		self.http_client = Some(DEFAULT_HTTP_CLIENT.clone());
		self.http_client_factory = Some(Arc::new(factory));
		self
	}

	/// Disable HTTP client.
	pub fn no_http_client(mut self) -> Self {
		self.http_client = None;
		self.http_client_factory = None;
		self
	}

	/// Set the resource limits applied to HTTP host calls.
	///
	/// Defaults to [`HttpResourceLimits::default`]. The limits apply to both
	/// the default clients and clients from [`Self::http_client_factory`].
	pub fn http_limits(mut self, limits: HttpResourceLimits) -> Self {
		self.http_limits = limits;
		self
	}

	/// Reuse an existing in-flight HTTP permit pool.
	///
	/// Used by `HostState::clone` so that clones share one concurrency budget.
	fn shared_http_permits(mut self, permits: Arc<Semaphore>) -> Self {
		self.http_permits = Some(permits);
		self
	}

	/// Set a database connection.
	///
	/// Query host calls stream rows from it, so its backend must support row
	/// streaming; see [`DatabaseResourceLimits`].
	#[cfg(feature = "wasm")]
	pub fn db_connection(mut self, connection: Arc<DatabaseConnection>) -> Self {
		self.db_connection = Some(connection);
		self
	}

	/// Set the resource limits applied to database query host calls.
	///
	/// Defaults to [`DatabaseResourceLimits::default`].
	pub fn db_limits(mut self, limits: DatabaseResourceLimits) -> Self {
		self.db_limits = limits;
		self
	}

	/// Reuse an existing in-flight database query permit pool.
	///
	/// Used by `HostState::clone` so that clones share one concurrency budget.
	fn shared_db_permits(mut self, permits: Arc<Semaphore>) -> Self {
		self.db_permits = Some(permits);
		self
	}

	/// Set a shared event bus.
	///
	/// If not set, a new event bus will be created for this host state.
	/// To enable inter-plugin communication, share the same event bus
	/// across multiple host states.
	pub fn event_bus(mut self, event_bus: SharedEventBus) -> Self {
		self.event_bus = Some(event_bus);
		self
	}

	/// Set a shared model registry.
	///
	/// If not set, a new model registry will be created for this host state.
	/// To share model schemas across plugins, use the same model registry
	/// for multiple host states.
	pub fn model_registry(mut self, model_registry: SharedModelRegistry) -> Self {
		self.model_registry = Some(model_registry);
		self
	}

	/// Set a shared SSR proxy.
	///
	/// If not set, a new SSR proxy will be created for this host state.
	/// By default, the SSR proxy is unavailable (TypeScript runtime not enabled).
	/// Use [`SsrProxy::with_availability`] with `true` to enable SSR.
	pub fn ssr_proxy(mut self, ssr_proxy: SharedSsrProxy) -> Self {
		self.ssr_proxy = Some(ssr_proxy);
		self
	}

	/// Set a TypeScript runtime for SSR.
	///
	/// This enables JavaScript/TypeScript SSR with full TypeScript support.
	/// The TypeScript runtime uses rustyscript (deno_core based) with
	/// Preact loaded from esm.sh CDN for React-compatible SSR.
	///
	/// # Example
	///
	/// ```ignore
	/// use reinhardt_dentdelion::wasm::{TsRuntime, HostStateBuilder};
	/// use std::sync::Arc;
	///
	/// let ts_runtime = Arc::new(TsRuntime::new()?);
	/// let host_state = HostStateBuilder::new("my-plugin")
	///     .ts_runtime(ts_runtime)
	///     .build();
	/// ```
	#[cfg(feature = "ts")]
	pub fn ts_runtime(mut self, runtime: SharedTsRuntime) -> Self {
		self.ssr_proxy = Some(Arc::new(SsrProxy::with_ts_runtime(runtime)));
		self
	}

	/// Build the host state.
	pub fn build(self) -> HostState {
		let http_permits = self
			.http_permits
			.unwrap_or_else(|| new_http_permits(&self.http_limits));
		let db_permits = self
			.db_permits
			.unwrap_or_else(|| new_db_permits(&self.db_limits));
		HostState {
			plugin_name: self.plugin_name,
			trust_level: self.trust_level,
			config: RwLock::new(self.config),
			services: RwLock::new(HashMap::new()),
			typed_services: RwLock::new(HashMap::new()),
			capabilities: self.capabilities,
			http_client: self.http_client,
			http_client_factory: self.http_client_factory,
			http_limits: self.http_limits,
			http_permits,
			db_connection: self.db_connection,
			db_limits: self.db_limits,
			db_permits,
			event_bus: self.event_bus.unwrap_or_else(|| Arc::new(EventBus::new())),
			model_registry: self
				.model_registry
				.unwrap_or_else(|| Arc::new(ModelRegistry::new())),
			ssr_proxy: self.ssr_proxy.unwrap_or_else(|| Arc::new(SsrProxy::new())),
			#[cfg(feature = "wasm")]
			wasi_ctx: parking_lot::Mutex::new(WasiCtxBuilder::new().build()),
			#[cfg(feature = "wasm")]
			resource_table: parking_lot::Mutex::new(ResourceTable::new()),
			#[cfg(feature = "wasm")]
			store_limits: None,
			#[cfg(feature = "wasm")]
			execution_deadline: None,
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
		// Pass the owned body through so it is not copied a second time
		let result = Self::http_post_owned(self, &url, body, &headers)
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

// ===== Events Host Trait Implementation =====
//
// Implement the events Host trait for inter-plugin communication.
// The trait is generated at crate::wasm::runtime::reinhardt::dentdelion::events::Host

#[cfg(feature = "wasm")]
type GeneratedEvent = crate::wasm::runtime::reinhardt::dentdelion::events::Event;

#[cfg(feature = "wasm")]
fn to_generated_event(event: Event) -> GeneratedEvent {
	GeneratedEvent {
		name: event.name,
		payload: event.payload,
		source: event.source,
		timestamp: event.timestamp,
	}
}

#[cfg(feature = "wasm")]
impl crate::wasm::runtime::reinhardt::dentdelion::events::Host for HostState {
	async fn emit(
		&mut self,
		name: String,
		payload: Vec<u8>,
	) -> Result<Result<(), GeneratedPluginError>, anyhow::Error> {
		if let Err(e) = self.authorize_event_call() {
			return Ok(Err(to_generated_error(e)));
		}
		match self.try_emit_event(&name, payload) {
			Ok(_delivered) => Ok(Ok(())),
			Err(e) => Ok(Err(to_generated_error(WitPluginError::new(
				413,
				e.to_string(),
			)))),
		}
	}

	async fn subscribe(
		&mut self,
		pattern: String,
	) -> Result<Result<u64, GeneratedPluginError>, anyhow::Error> {
		if let Err(e) = self.authorize_event_call() {
			return Ok(Err(to_generated_error(e)));
		}
		match self.subscribe_events(&pattern) {
			Ok(subscription_id) => Ok(Ok(subscription_id)),
			Err(e) => Ok(Err(to_generated_error(WitPluginError::new(
				429,
				e.to_string(),
			)))),
		}
	}

	async fn unsubscribe(
		&mut self,
		id: u64,
	) -> Result<Result<(), GeneratedPluginError>, anyhow::Error> {
		if let Err(e) = self.authorize_event_call() {
			return Ok(Err(to_generated_error(e)));
		}
		let removed = self.unsubscribe_events(id);
		if removed {
			Ok(Ok(()))
		} else {
			Ok(Err(to_generated_error(WitPluginError::new(
				404,
				"Subscription not found",
			))))
		}
	}

	async fn poll_events(
		&mut self,
		id: u64,
		limit: u32,
	) -> Result<Vec<GeneratedEvent>, anyhow::Error> {
		// `poll-events` has no error channel in the WIT ABI, so a denied call
		// returns an empty list and leaves the queue untouched.
		if self.authorize_event_call().is_err() {
			return Ok(Vec::new());
		}
		let events = self.poll_pending_events(id, limit as usize);
		Ok(events.into_iter().map(to_generated_event).collect())
	}
}

// ===== Models Host Trait Implementation =====
//
// Implement the models Host trait for schema and migration registration.
// The trait is generated at crate::wasm::runtime::reinhardt::dentdelion::models::Host

#[cfg(feature = "wasm")]
type GeneratedModelSchema = crate::wasm::runtime::reinhardt::dentdelion::models::ModelSchema;
#[cfg(feature = "wasm")]
type GeneratedColumnDef = crate::wasm::runtime::reinhardt::dentdelion::models::ColumnDef;
#[cfg(feature = "wasm")]
type GeneratedColumnType = crate::wasm::runtime::reinhardt::dentdelion::models::ColumnType;
#[cfg(feature = "wasm")]
type GeneratedIndexDef = crate::wasm::runtime::reinhardt::dentdelion::models::IndexDef;
#[cfg(feature = "wasm")]
type GeneratedSqlMigration = crate::wasm::runtime::reinhardt::dentdelion::models::SqlMigration;

use super::models::{ColumnDef, ColumnType, IndexDef};

/// Convert generated ColumnType to internal ColumnType
#[cfg(feature = "wasm")]
fn from_generated_column_type(col_type: GeneratedColumnType) -> ColumnType {
	use crate::wasm::runtime::reinhardt::dentdelion::models::ColumnType as Gen;
	match col_type {
		Gen::Integer => ColumnType::Integer,
		Gen::BigInteger => ColumnType::BigInteger,
		Gen::Text => ColumnType::Text,
		Gen::Varchar(len) => ColumnType::Varchar(len),
		Gen::Boolean => ColumnType::Boolean,
		Gen::Timestamp => ColumnType::Timestamp,
		Gen::Uuid => ColumnType::Uuid,
		Gen::Json => ColumnType::Json,
		Gen::Decimal((precision, scale)) => ColumnType::Decimal { precision, scale },
		Gen::ForeignKey(table) => ColumnType::ForeignKey(table),
	}
}

/// Convert generated ColumnDef to internal ColumnDef
#[cfg(feature = "wasm")]
fn from_generated_column_def(col: GeneratedColumnDef) -> ColumnDef {
	ColumnDef {
		name: col.name,
		column_type: from_generated_column_type(col.column_type),
		nullable: col.nullable,
		primary_key: col.primary_key,
		unique_value: col.unique_value,
		default_value: col.default_value,
	}
}

/// Convert generated IndexDef to internal IndexDef
#[cfg(feature = "wasm")]
fn from_generated_index_def(idx: GeneratedIndexDef) -> IndexDef {
	IndexDef {
		name: idx.name,
		columns: idx.columns,
		unique_value: idx.unique_value,
	}
}

/// Convert generated ModelSchema to internal ModelSchema
#[cfg(feature = "wasm")]
fn from_generated_model_schema(schema: GeneratedModelSchema) -> ModelSchema {
	ModelSchema {
		table_name: schema.table_name,
		columns: schema
			.columns
			.into_iter()
			.map(from_generated_column_def)
			.collect(),
		indexes: schema
			.indexes
			.into_iter()
			.map(from_generated_index_def)
			.collect(),
	}
}

/// Convert generated SqlMigration to internal SqlMigration
#[cfg(feature = "wasm")]
fn from_generated_sql_migration(migration: GeneratedSqlMigration) -> SqlMigration {
	SqlMigration {
		version: migration.version,
		description: migration.description,
		up_sql: migration.up_sql,
		down_sql: migration.down_sql,
	}
}

#[cfg(feature = "wasm")]
impl crate::wasm::runtime::reinhardt::dentdelion::models::Host for HostState {
	async fn register_model(
		&mut self,
		schema: GeneratedModelSchema,
	) -> Result<Result<(), GeneratedPluginError>, anyhow::Error> {
		let internal_schema = from_generated_model_schema(schema);
		let result = self
			.register_model_schema(internal_schema)
			.map_err(|e| to_generated_error(WitPluginError::new(400, &e)));
		Ok(result)
	}

	async fn register_migration(
		&mut self,
		migration: GeneratedSqlMigration,
	) -> Result<Result<(), GeneratedPluginError>, anyhow::Error> {
		let internal_migration = from_generated_sql_migration(migration);
		let result = self
			.register_sql_migration(internal_migration)
			.map_err(|e| to_generated_error(WitPluginError::new(400, &e)));
		Ok(result)
	}

	async fn list_models(&mut self) -> Result<Vec<String>, anyhow::Error> {
		Ok(self.list_registered_models())
	}
}

// ===== SSR Host Trait Implementation =====
//
// Implement the ssr Host trait for server-side rendering support.
// The trait is generated at crate::wasm::runtime::reinhardt::dentdelion::ssr::Host

#[cfg(feature = "wasm")]
type GeneratedRenderOptions = crate::wasm::runtime::reinhardt::dentdelion::ssr::RenderOptions;
#[cfg(feature = "wasm")]
type GeneratedRenderResult = crate::wasm::runtime::reinhardt::dentdelion::ssr::RenderResult;

/// Convert generated RenderOptions to internal RenderOptions
#[cfg(feature = "wasm")]
fn from_generated_render_options(options: GeneratedRenderOptions) -> RenderOptions {
	RenderOptions {
		include_hydration: options.include_hydration,
		extract_css: options.extract_css,
		extract_meta: options.extract_meta,
	}
}

/// Convert internal RenderResult to generated RenderResult
#[cfg(feature = "wasm")]
fn to_generated_render_result(result: RenderResult) -> GeneratedRenderResult {
	GeneratedRenderResult {
		html: result.html,
		css: result.css,
		meta: result.meta,
		hydration_script: result.hydration_script,
	}
}

/// Convert internal SsrError to generated PluginError
#[cfg(feature = "wasm")]
fn ssr_error_to_plugin_error(err: SsrError) -> GeneratedPluginError {
	let code = match &err {
		SsrError::NotAvailable => 503,
		SsrError::ComponentNotFound(_) => 404,
		SsrError::PropsSerialization(_) => 400,
		SsrError::RenderFailed(_) => 500,
		SsrError::EvalFailed(_) => 500,
		SsrError::PermissionDenied(_) => 403,
		SsrError::DangerousPattern(_) => 403,
	};
	GeneratedPluginError {
		code,
		message: err.to_string(),
		details: None,
	}
}

#[cfg(feature = "wasm")]
impl crate::wasm::runtime::reinhardt::dentdelion::ssr::Host for HostState {
	async fn render_react(
		&mut self,
		component_path: String,
		props: Vec<u8>,
		options: GeneratedRenderOptions,
	) -> Result<Result<GeneratedRenderResult, GeneratedPluginError>, anyhow::Error> {
		let internal_options = from_generated_render_options(options);
		let result = Self::render_react(self, &component_path, &props, internal_options)
			.await
			.map(to_generated_render_result)
			.map_err(ssr_error_to_plugin_error);
		Ok(result)
	}

	async fn eval_js(
		&mut self,
		code: String,
	) -> Result<Result<Vec<u8>, GeneratedPluginError>, anyhow::Error> {
		let result = match self.require_js_execution_trust() {
			Ok(()) => self
				.ssr_proxy
				.eval_js_until(code, self.execution_deadline)
				.await
				.map(|value| value.into_bytes()),
			Err(error) => Err(error),
		}
		.map_err(ssr_error_to_plugin_error);
		Ok(result)
	}

	async fn is_available(&mut self) -> Result<bool, anyhow::Error> {
		Ok(self.is_ssr_available())
	}
}

#[cfg(test)]
mod tests {
	#[cfg(feature = "ts")]
	#[rstest::rstest]
	#[case(false)]
	#[case(true)]
	#[tokio::test(flavor = "current_thread")]
	async fn javascript_imports_yield_and_use_the_remaining_lifecycle_deadline(
		#[case] render: bool,
	) {
		use super::super::runtime::{WasmRuntime, WasmRuntimeConfigBuilder};
		use super::super::ts_runtime::{TsResourceLimits, TsRuntime};
		use crate::wasm::runtime::reinhardt::dentdelion::ssr::Host;
		use std::time::{Duration, Instant};
		// Arrange: JavaScript's own budget is much longer than the WASM invocation.
		let runtime = WasmRuntime::new(
			WasmRuntimeConfigBuilder::new()
				.timeout(Duration::from_millis(100))
				.build(),
		)
		.unwrap();
		let javascript = Arc::new(
			TsRuntime::with_limits(TsResourceLimits {
				execution_timeout: Duration::from_secs(2),
				loop_iteration_limit: u64::MAX,
				..TsResourceLimits::default()
			})
			.unwrap(),
		);
		let assets = tempfile::tempdir().unwrap();
		std::fs::write(
			assets.path().join("Component.js"),
			"function Component() { while (true) {} }",
		)
		.unwrap();
		let proxy = Arc::new(
			SsrProxy::with_ts_runtime(javascript).with_plugin_base_dir(assets.path().to_path_buf()),
		);
		let state = HostState::builder("short-js-import")
			.trust_level(TrustLevel::Trusted)
			.ssr_proxy(proxy)
			.build();
		let started = Instant::now();

		// Act: consume part of the invocation, then call the actual generated import.
		let evaluation = runtime.with_execution_timeout(async {
			let mut store = runtime.create_store(state);
			tokio::time::sleep(Duration::from_millis(40)).await;
			let result = if render {
				Host::render_react(
					store.data_mut(),
					"Component.js".into(),
					rmp_serde::to_vec(&serde_json::json!({})).unwrap(),
					GeneratedRenderOptions {
						include_hydration: false,
						extract_css: false,
						extract_meta: false,
					},
				)
				.await
				.unwrap()
				.map(|_| ())
			} else {
				Host::eval_js(store.data_mut(), "while (true) {}".into())
					.await
					.unwrap()
					.map(|_| ())
			};
			result.map_err(|error| crate::error::PluginError::WasmExecutionError(error.message))
		});
		let heartbeat = async {
			tokio::time::sleep(Duration::from_millis(20)).await;
			started.elapsed()
		};
		let (result, heartbeat_elapsed) = tokio::join!(evaluation, heartbeat);

		// Assert: a single-thread executor stays responsive and the shorter deadline wins.
		assert!(
			matches!(result, Err(crate::error::PluginError::WasmExecutionError(message))
			if message.contains("timed out"))
		);
		assert!(started.elapsed() < Duration::from_millis(500));
		assert!(heartbeat_elapsed < Duration::from_millis(100));
	}

	#[rstest::rstest]
	#[case("2001:1::1", true)]
	#[case("2001:1::2", true)]
	#[case("2001:1::3", true)]
	#[case("2001:1::4", false)]
	#[case("2001:2::1", false)]
	#[case("2001:2:ffff::1", false)]
	#[case("2001:3::1", true)]
	#[case("2001:3:ffff:ffff:ffff:ffff:ffff:ffff", true)]
	#[case("2001:4:112::1", true)]
	#[case("2001:4:113::1", false)]
	#[case("2001:10::1", false)]
	#[case("2001:20::1", false)]
	#[case("2001:2f:ffff::1", false)]
	#[case("2001:30::1", false)]
	#[case("2001:3f:ffff::1", false)]
	#[case("2001:1ff:ffff::1", false)]
	#[case("2001:200::1", true)]
	#[case("3fff::1", false)]
	#[case("3fff:fff:ffff:ffff:ffff:ffff:ffff:ffff", false)]
	#[case("3fff:1000::1", true)]
	#[case("2620:4f:8000::1", true)]
	#[case("64:ff9b::808:808", true)]
	#[case("64:ff9b::a00:1", false)]
	#[case("64:ff9b::7f00:1", false)]
	#[case("64:ff9b::a9fe:a9fe", false)]
	#[case("64:ff9b:1::808:808", false)]
	fn ipv6_special_assignments_preserve_global_locator_exceptions(
		#[case] address: &str,
		#[case] expected_public: bool,
	) {
		assert_eq!(!is_private_ip(&address.parse().unwrap()), expected_public);
	}

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

	#[test]
	fn test_host_state_event_bus() {
		// Create two host states sharing the same event bus
		let event_bus = Arc::new(EventBus::new());

		let producer = HostStateBuilder::new("producer-plugin")
			.event_bus(event_bus.clone())
			.build();

		let consumer = HostStateBuilder::new("consumer-plugin")
			.event_bus(event_bus.clone())
			.build();

		// Consumer subscribes to user events
		let sub_id = consumer.subscribe_events("user.*").unwrap();

		// Producer emits an event
		let delivered = producer.emit_event("user.created", vec![1, 2, 3]);
		assert_eq!(delivered, 1);

		// Consumer polls events
		let events = consumer.poll_pending_events(sub_id, 10);
		assert_eq!(events.len(), 1);
		assert_eq!(events[0].name, "user.created");
		assert_eq!(events[0].source, "producer-plugin");
		assert_eq!(events[0].payload, vec![1, 2, 3]);
	}

	#[test]
	fn test_host_state_event_bus_isolation() {
		// Without sharing an event bus, plugins are isolated
		let plugin_a = HostState::new("plugin-a");
		let plugin_b = HostState::new("plugin-b");

		let sub_id = plugin_b.subscribe_events("*").unwrap();
		plugin_a.emit_event("test.event", vec![]);

		// plugin_b won't see the event because they have different event buses
		let events = plugin_b.poll_pending_events(sub_id, 10);
		assert!(events.is_empty());
	}

	#[test]
	fn test_host_state_event_unsubscribe() {
		let state = HostState::new("test-plugin");

		let sub_id = state.subscribe_events("*").unwrap();
		assert!(state.unsubscribe_events(sub_id));
		assert!(!state.unsubscribe_events(sub_id)); // Already unsubscribed
	}

	/// Shared event bus whose payload limit is `max_payload_bytes`.
	fn event_bus_with_payload_limit(max_payload_bytes: usize) -> SharedEventBus {
		let limits = crate::wasm::EventByteLimits {
			max_payload_bytes,
			..crate::wasm::EventByteLimits::default()
		};
		Arc::new(EventBus::new().with_byte_limits(limits))
	}

	/// Capability that authorizes the WIT `events` interface.
	fn signals_grant() -> Capability {
		Capability::Core(crate::capability::PluginCapability::Signals)
	}

	/// Host state for `name` on `event_bus` holding exactly `capabilities`.
	fn event_plugin(
		name: &str,
		event_bus: &SharedEventBus,
		capabilities: impl IntoIterator<Item = Capability>,
	) -> HostState {
		HostStateBuilder::new(name)
			.event_bus(event_bus.clone())
			.capabilities(capabilities)
			.build()
	}

	#[cfg(feature = "wasm")]
	#[tokio::test]
	async fn wit_emit_rejects_payload_over_event_bus_limit() {
		use crate::wasm::runtime::reinhardt::dentdelion::events::Host;

		// Arrange
		let event_bus = event_bus_with_payload_limit(16);
		let consumer = HostStateBuilder::new("consumer")
			.event_bus(event_bus.clone())
			.build();
		let mut producer = event_plugin("producer", &event_bus, [signals_grant()]);
		let sub_ids: Vec<u64> = (0..3)
			.map(|_| consumer.subscribe_events("*").unwrap())
			.collect();

		// Act
		let result = Host::emit(&mut producer, "user.created".to_string(), vec![0; 17])
			.await
			.expect("host adapter should return a plugin result");

		// Assert
		assert_eq!(
			result
				.expect_err("oversized payload should be rejected")
				.code,
			413
		);
		for sub_id in sub_ids {
			assert_eq!(event_bus.pending_count(sub_id), 0);
		}
		assert_eq!(event_bus.total_pending_bytes(), 0);
	}

	#[cfg(feature = "wasm")]
	#[tokio::test]
	async fn wit_emit_rejects_name_over_default_limit() {
		use crate::wasm::runtime::reinhardt::dentdelion::events::Host;

		// Arrange
		let event_bus: SharedEventBus = Arc::new(EventBus::new());
		let consumer = HostStateBuilder::new("consumer")
			.event_bus(event_bus.clone())
			.build();
		let mut producer = event_plugin("producer", &event_bus, [signals_grant()]);
		let sub_id = consumer.subscribe_events("*").unwrap();
		let name = "e".repeat(event_bus.byte_limits().max_name_bytes + 1);

		// Act
		let result = Host::emit(&mut producer, name, Vec::new())
			.await
			.expect("host adapter should return a plugin result");

		// Assert
		assert_eq!(
			result.expect_err("oversized name should be rejected").code,
			413
		);
		assert_eq!(event_bus.pending_count(sub_id), 0);
	}

	#[cfg(feature = "wasm")]
	#[tokio::test]
	async fn wit_emit_delivers_payload_within_limit_to_every_subscriber() {
		use crate::wasm::runtime::reinhardt::dentdelion::events::Host;

		// Arrange
		let event_bus = event_bus_with_payload_limit(16);
		let mut consumer = event_plugin("consumer", &event_bus, [signals_grant()]);
		let mut producer = event_plugin("producer", &event_bus, [signals_grant()]);
		let sub_ids: Vec<u64> = (0..3)
			.map(|_| consumer.subscribe_events("user.*").unwrap())
			.collect();

		// Act
		let result = Host::emit(&mut producer, "user.created".to_string(), vec![7; 16])
			.await
			.expect("host adapter should return a plugin result");

		// Assert
		assert!(result.is_ok());
		for sub_id in sub_ids {
			let events = Host::poll_events(&mut consumer, sub_id, 10)
				.await
				.expect("host adapter should return polled events");
			assert_eq!(events.len(), 1);
			assert_eq!(events[0].name, "user.created");
			assert_eq!(events[0].payload, vec![7; 16]);
			assert_eq!(events[0].source, "producer");
		}
		assert_eq!(event_bus.total_pending_bytes(), 0);
	}

	/// WIT `events` calls require the `Signals` grant (GHSA-gcxp-frpr-r8qm).
	///
	/// Each denial case uses a plugin that holds no grant, an unrelated grant,
	/// or `SignalReceivers`, which is a separate capability and does not stand
	/// in for `Signals`.
	#[cfg(feature = "wasm")]
	mod event_grants {
		use super::*;
		use crate::capability::PluginCapability;
		use crate::wasm::runtime::reinhardt::dentdelion::events::Host;
		use rstest::rstest;

		#[rstest]
		#[case::no_grant(None)]
		#[case::network_access(Some(PluginCapability::NetworkAccess))]
		#[case::signal_receivers(Some(PluginCapability::SignalReceivers))]
		#[tokio::test]
		async fn emit_without_signals_is_denied(#[case] grant: Option<PluginCapability>) {
			// Arrange
			let event_bus: SharedEventBus = Arc::new(EventBus::new());
			let consumer = event_plugin("consumer", &event_bus, [signals_grant()]);
			let sub_id = consumer.subscribe_events("*").unwrap();
			let mut intruder = event_plugin("intruder", &event_bus, grant.map(Capability::Core));

			// Act
			let result = Host::emit(&mut intruder, "user.created".to_string(), vec![1, 2, 3])
				.await
				.expect("host adapter should return a plugin result");

			// Assert
			assert_eq!(
				result
					.expect_err("emit without Signals should be denied")
					.code,
				403
			);
			assert_eq!(event_bus.pending_count(sub_id), 0);
			assert_eq!(event_bus.total_pending_bytes(), 0);
		}

		#[rstest]
		#[case::no_grant(None)]
		#[case::network_access(Some(PluginCapability::NetworkAccess))]
		#[case::signal_receivers(Some(PluginCapability::SignalReceivers))]
		#[tokio::test]
		async fn subscribe_without_signals_is_denied(#[case] grant: Option<PluginCapability>) {
			// Arrange
			let event_bus: SharedEventBus = Arc::new(EventBus::new());
			let mut intruder = event_plugin("intruder", &event_bus, grant.map(Capability::Core));

			// Act
			let result = Host::subscribe(&mut intruder, "*".to_string())
				.await
				.expect("host adapter should return a plugin result");

			// Assert
			assert_eq!(
				result
					.expect_err("subscribe without Signals should be denied")
					.code,
				403
			);
			assert_eq!(event_bus.subscription_count(), 0);
		}

		// The subscription is created through the host-side API and owned by
		// the intruder, so only the missing grant can explain the denial.
		#[rstest]
		#[case::no_grant(None)]
		#[case::network_access(Some(PluginCapability::NetworkAccess))]
		#[case::signal_receivers(Some(PluginCapability::SignalReceivers))]
		#[tokio::test]
		async fn poll_without_signals_returns_nothing_and_keeps_queue(
			#[case] grant: Option<PluginCapability>,
		) {
			// Arrange
			let event_bus: SharedEventBus = Arc::new(EventBus::new());
			let mut intruder = event_plugin("intruder", &event_bus, grant.map(Capability::Core));
			let sub_id = intruder.subscribe_events("user.*").unwrap();
			let mut producer = event_plugin("producer", &event_bus, [signals_grant()]);
			Host::emit(&mut producer, "user.created".to_string(), vec![1, 2, 3])
				.await
				.expect("host adapter should return a plugin result")
				.expect("granted producer should emit");

			// Act
			let events = Host::poll_events(&mut intruder, sub_id, 10)
				.await
				.expect("host adapter should return polled events");

			// Assert
			assert!(events.is_empty());
			assert_eq!(event_bus.pending_count(sub_id), 1);
		}

		// As above, the intruder owns the subscription it tries to remove.
		#[rstest]
		#[case::no_grant(None)]
		#[case::network_access(Some(PluginCapability::NetworkAccess))]
		#[case::signal_receivers(Some(PluginCapability::SignalReceivers))]
		#[tokio::test]
		async fn unsubscribe_without_signals_is_denied(#[case] grant: Option<PluginCapability>) {
			// Arrange
			let event_bus: SharedEventBus = Arc::new(EventBus::new());
			let mut intruder = event_plugin("intruder", &event_bus, grant.map(Capability::Core));
			let sub_id = intruder.subscribe_events("*").unwrap();

			// Act
			let result = Host::unsubscribe(&mut intruder, sub_id)
				.await
				.expect("host adapter should return a plugin result");

			// Assert
			assert_eq!(
				result
					.expect_err("unsubscribe without Signals should be denied")
					.code,
				403
			);
			assert!(event_bus.has_subscription(sub_id));
		}

		#[rstest]
		#[tokio::test]
		async fn signals_grant_allows_subscribe_and_unsubscribe() {
			// Arrange
			let event_bus: SharedEventBus = Arc::new(EventBus::new());
			let mut plugin = event_plugin("plugin", &event_bus, [signals_grant()]);

			// Act
			let sub_id = Host::subscribe(&mut plugin, "*".to_string())
				.await
				.expect("host adapter should return a plugin result")
				.expect("Signals grant should allow subscribe");
			let subscribed = event_bus.has_subscription(sub_id);
			let result = Host::unsubscribe(&mut plugin, sub_id)
				.await
				.expect("host adapter should return a plugin result");

			// Assert
			assert!(subscribed);
			assert!(result.is_ok());
			assert!(!event_bus.has_subscription(sub_id));
		}
	}

	#[test]
	fn test_host_state_model_registry() {
		let state = HostState::new("test-plugin");

		// Register a model schema
		let schema = ModelSchema::new("users")
			.column(ColumnDef {
				name: "id".to_string(),
				column_type: ColumnType::Integer,
				nullable: false,
				primary_key: true,
				unique_value: true,
				default_value: None,
			})
			.column(ColumnDef {
				name: "email".to_string(),
				column_type: ColumnType::Varchar(255),
				nullable: false,
				primary_key: false,
				unique_value: true,
				default_value: None,
			});

		assert!(state.register_model_schema(schema).is_ok());

		// List models
		let models = state.list_registered_models();
		assert_eq!(models.len(), 1);
		assert!(models.contains(&"users".to_string()));

		// Get model
		let retrieved = state.get_registered_model("users");
		assert!(retrieved.is_some());
		assert_eq!(retrieved.unwrap().table_name, "users");

		// Non-existent model
		assert!(state.get_registered_model("nonexistent").is_none());
	}

	#[test]
	fn test_host_state_model_registry_duplicate() {
		let state = HostState::new("test-plugin");

		let schema = ModelSchema::new("products");
		assert!(state.register_model_schema(schema.clone()).is_ok());

		// Duplicate registration fails
		assert!(state.register_model_schema(schema).is_err());
	}

	#[test]
	fn test_host_state_migration_registry() {
		let state = HostState::new("test-plugin");

		let migration = SqlMigration::new(
			"0001_initial",
			"Create initial tables",
			"CREATE TABLE users (id INT PRIMARY KEY);",
			"DROP TABLE users;",
		);

		assert!(state.register_sql_migration(migration).is_ok());

		// List migrations
		let migrations = state.list_registered_migrations();
		assert_eq!(migrations.len(), 1);
		assert_eq!(migrations[0].version, "0001_initial");
	}

	#[test]
	fn test_host_state_shared_model_registry() {
		// Create two host states sharing the same model registry
		let model_registry = Arc::new(ModelRegistry::new());

		let plugin_a = HostStateBuilder::new("plugin-a")
			.model_registry(model_registry.clone())
			.build();

		let plugin_b = HostStateBuilder::new("plugin-b")
			.model_registry(model_registry.clone())
			.build();

		// Register models from both plugins
		plugin_a
			.register_model_schema(ModelSchema::new("users"))
			.unwrap();
		plugin_b
			.register_model_schema(ModelSchema::new("products"))
			.unwrap();

		// Each plugin can only see its own models
		assert_eq!(plugin_a.list_registered_models().len(), 1);
		assert!(
			plugin_a
				.list_registered_models()
				.contains(&"users".to_string())
		);

		assert_eq!(plugin_b.list_registered_models().len(), 1);
		assert!(
			plugin_b
				.list_registered_models()
				.contains(&"products".to_string())
		);

		// But the shared registry contains both
		assert_eq!(model_registry.schema_count(), 2);
	}

	#[test]
	fn test_host_state_builder_with_model_registry() {
		let model_registry = Arc::new(ModelRegistry::new());

		let state = HostStateBuilder::new("test-plugin")
			.model_registry(model_registry.clone())
			.build();

		// Verify the registry is shared
		assert!(Arc::ptr_eq(state.model_registry(), &model_registry));
	}

	#[test]
	fn test_host_state_ssr_proxy_default() {
		let state = HostState::new("test-plugin");

		// SSR is not available by default
		assert!(!state.is_ssr_available());
	}

	#[test]
	fn test_host_state_ssr_proxy_with_availability() {
		let ssr_proxy = Arc::new(SsrProxy::with_availability(true));

		let state = HostStateBuilder::new("test-plugin")
			.ssr_proxy(ssr_proxy.clone())
			.build();

		// SSR is available when explicitly enabled
		assert!(state.is_ssr_available());

		// Verify the proxy is shared
		assert!(Arc::ptr_eq(state.ssr_proxy(), &ssr_proxy));
	}

	#[tokio::test]
	async fn test_host_state_render_react_not_available() {
		// Verified plugin without SSR runtime should return NotAvailable
		let state = HostStateBuilder::new("test-plugin")
			.trust_level(TrustLevel::Verified)
			.build();

		let result = state
			.render_react("test.jsx", &[], RenderOptions::default())
			.await;

		// Should return NotAvailable error since SSR is disabled
		assert!(matches!(result, Err(SsrError::NotAvailable)));
	}

	#[test]
	fn test_host_state_eval_js_not_available() {
		// Trusted plugin without SSR runtime should return NotAvailable
		let state = HostStateBuilder::new("test-plugin")
			.trust_level(TrustLevel::Trusted)
			.build();

		let result = state.eval_js("console.log('test')");

		// Should return NotAvailable error since SSR is disabled
		assert!(matches!(result, Err(SsrError::NotAvailable)));
	}

	#[test]
	fn test_host_state_builder_with_ssr_proxy() {
		let ssr_proxy = Arc::new(SsrProxy::with_availability(true));

		let state = HostStateBuilder::new("test-plugin")
			.ssr_proxy(ssr_proxy.clone())
			.build();

		// Verify the proxy is shared
		assert!(Arc::ptr_eq(state.ssr_proxy(), &ssr_proxy));
		assert!(state.is_ssr_available());
	}

	// =========================================================================
	// Trust Level Enforcement Tests (Issue #675 + #677)
	// =========================================================================

	#[test]
	fn test_host_state_default_trust_level() {
		// Arrange
		let state = HostState::new("test-plugin");

		// Act & Assert
		assert_eq!(state.trust_level(), TrustLevel::Untrusted);
	}

	#[test]
	fn test_host_state_builder_trust_level() {
		// Arrange & Act
		let state = HostStateBuilder::new("test-plugin")
			.trust_level(TrustLevel::Trusted)
			.build();

		// Assert
		assert_eq!(state.trust_level(), TrustLevel::Trusted);
	}

	#[test]
	fn test_eval_js_denied_for_untrusted() {
		// Arrange
		let state = HostStateBuilder::new("untrusted-plugin")
			.trust_level(TrustLevel::Untrusted)
			.build();

		// Act
		let result = state.eval_js("1 + 1");

		// Assert
		assert!(matches!(result, Err(SsrError::PermissionDenied(_))));
	}

	#[test]
	fn test_eval_js_denied_for_verified() {
		// Arrange
		let state = HostStateBuilder::new("verified-plugin")
			.trust_level(TrustLevel::Verified)
			.build();

		// Act
		let result = state.eval_js("1 + 1");

		// Assert
		assert!(matches!(result, Err(SsrError::PermissionDenied(_))));
	}

	#[test]
	fn test_eval_js_allowed_for_trusted_returns_not_available() {
		// Arrange
		let state = HostStateBuilder::new("trusted-plugin")
			.trust_level(TrustLevel::Trusted)
			.build();

		// Act
		let result = state.eval_js("1 + 1");

		// Assert - passes trust check, fails with NotAvailable (no runtime)
		assert!(matches!(result, Err(SsrError::NotAvailable)));
	}

	#[test]
	fn test_render_component_denied_for_untrusted() {
		// Arrange
		let state = HostStateBuilder::new("untrusted-plugin")
			.trust_level(TrustLevel::Untrusted)
			.build();

		// Act
		let result = state.render_component(
			"function Component() { return h('div', null, 'test'); }",
			"{}",
			RenderOptions::default(),
		);

		// Assert
		assert!(matches!(result, Err(SsrError::PermissionDenied(_))));
	}

	#[test]
	fn test_render_component_allowed_for_verified() {
		// Arrange
		let state = HostStateBuilder::new("verified-plugin")
			.trust_level(TrustLevel::Verified)
			.build();

		// Act
		let result = state.render_component(
			"function Component() { return h('div', null, 'test'); }",
			"{}",
			RenderOptions::default(),
		);

		// Assert - passes trust check, fails with NotAvailable (no runtime)
		assert!(matches!(result, Err(SsrError::NotAvailable)));
	}

	#[test]
	fn test_render_component_allowed_for_trusted() {
		// Arrange
		let state = HostStateBuilder::new("trusted-plugin")
			.trust_level(TrustLevel::Trusted)
			.build();

		// Act
		let result = state.render_component(
			"function Component() { return h('div', null, 'test'); }",
			"{}",
			RenderOptions::default(),
		);

		// Assert - passes trust check, fails with NotAvailable (no runtime)
		assert!(matches!(result, Err(SsrError::NotAvailable)));
	}

	#[tokio::test]
	async fn test_render_react_denied_for_untrusted() {
		// Arrange
		let state = HostStateBuilder::new("untrusted-plugin")
			.trust_level(TrustLevel::Untrusted)
			.build();

		// Act
		let result = state
			.render_react("test.jsx", &[], RenderOptions::default())
			.await;

		// Assert
		assert!(matches!(result, Err(SsrError::PermissionDenied(_))));
	}

	#[cfg(feature = "wasm")]
	#[tokio::test]
	async fn wit_render_react_denies_untrusted_plugin() {
		use crate::wasm::runtime::reinhardt::dentdelion::ssr::Host;

		let mut state = HostStateBuilder::new("untrusted-plugin")
			.trust_level(TrustLevel::Untrusted)
			.ssr_proxy(Arc::new(SsrProxy::with_availability(true)))
			.build();

		let result = Host::render_react(
			&mut state,
			"test.jsx".to_string(),
			Vec::new(),
			GeneratedRenderOptions {
				include_hydration: false,
				extract_css: false,
				extract_meta: false,
			},
		)
		.await
		.expect("host adapter should return a plugin result");

		assert_eq!(
			result.expect_err("untrusted SSR should be denied").code,
			403
		);
	}

	#[test]
	fn test_render_component_rejects_dangerous_code_at_host_level() {
		// Arrange - Verified plugin with SSR available
		let ssr_proxy = Arc::new(SsrProxy::with_availability(true));
		let state = HostStateBuilder::new("verified-plugin")
			.trust_level(TrustLevel::Verified)
			.ssr_proxy(ssr_proxy)
			.build();

		// Act - attempt to render with dangerous code
		let result = state.render_component(
			"var fs = require('fs'); function Component() { return h('div'); }",
			"{}",
			RenderOptions::default(),
		);

		// Assert - dangerous pattern detected by SsrProxy
		assert!(matches!(result, Err(SsrError::DangerousPattern(_))));
	}

	#[cfg(feature = "wasm")]
	#[rstest::rstest]
	#[tokio::test]
	async fn lifecycle_deadline_cancels_a_pending_dns_lookup() {
		use super::super::runtime::{WasmRuntime, WasmRuntimeConfigBuilder};
		use std::sync::atomic::{AtomicBool, Ordering};
		// Arrange: a resolver future delayed longer than the invocation budget.
		let runtime = WasmRuntime::new(
			WasmRuntimeConfigBuilder::new()
				.timeout(std::time::Duration::from_millis(30))
				.build(),
		)
		.unwrap();
		let cancelled = Arc::new(AtomicBool::new(false));
		struct LookupGuard(Arc<AtomicBool>);
		impl Drop for LookupGuard {
			fn drop(&mut self) {
				self.0.store(true, Ordering::Release);
			}
		}
		let started = std::time::Instant::now();
		let flag = Arc::clone(&cancelled);

		// Act: exercise the production URL validation path with an asynchronous resolver.
		let result = runtime
			.with_execution_timeout(async {
				validate_url_for_ssrf_with_resolver(
					"https://slow.example/",
					|domain, port| async move {
						assert_eq!(domain, "slow.example");
						assert_eq!(port, 443);
						let _guard = LookupGuard(flag);
						tokio::time::sleep(std::time::Duration::from_secs(1)).await;
						Ok(vec!["8.8.8.8:443".parse().unwrap()])
					},
				)
				.await
				.map_err(|error| crate::error::PluginError::WasmExecutionError(error.message))
			})
			.await;

		// Assert: the executor regains control and drops the pending DNS wait.
		assert!(
			matches!(result, Err(crate::error::PluginError::WasmExecutionError(message)) if message.contains("timed out"))
		);
		assert!(started.elapsed() < std::time::Duration::from_millis(500));
		assert!(cancelled.load(Ordering::Acquire));
	}

	#[rstest::rstest]
	#[tokio::test]
	async fn resolved_addresses_are_all_validated_before_pinning() {
		// Arrange: DNS returns both a public and internal destination.
		let result = validate_url_for_ssrf_with_resolver("https://mixed.example/", |_, _| async {
			Ok(vec![
				"8.8.8.8:443".parse().unwrap(),
				"127.0.0.1:443".parse().unwrap(),
			])
		})
		.await;

		// Assert: no address in a mixed result bypasses validation.
		assert_eq!(result.unwrap_err().code, 403);
	}

	// ===== SSRF Validation Tests =====

	#[rstest::rstest]
	#[tokio::test]
	async fn test_ssrf_rejects_file_scheme() {
		// Arrange
		let url = "file:///etc/passwd";

		// Act
		let result = validate_url_for_ssrf(url).await;

		// Assert
		assert!(result.is_err());
		let err = result.unwrap_err();
		assert_eq!(err.code, 403);
	}

	#[rstest::rstest]
	#[tokio::test]
	async fn test_ssrf_rejects_gopher_scheme() {
		// Arrange
		let url = "gopher://evil.com/";

		// Act
		let result = validate_url_for_ssrf(url).await;

		// Assert
		assert!(result.is_err());
		let err = result.unwrap_err();
		assert_eq!(err.code, 403);
	}

	#[rstest::rstest]
	#[tokio::test]
	async fn test_ssrf_rejects_ftp_scheme() {
		// Arrange
		let url = "ftp://internal-server/data";

		// Act
		let result = validate_url_for_ssrf(url).await;

		// Assert
		assert!(result.is_err());
		let err = result.unwrap_err();
		assert_eq!(err.code, 403);
	}

	#[rstest::rstest]
	#[tokio::test]
	async fn test_ssrf_rejects_loopback_ipv4() {
		// Arrange
		let url = "http://127.0.0.1/admin";

		// Act
		let result = validate_url_for_ssrf(url).await;

		// Assert
		assert!(result.is_err());
		let err = result.unwrap_err();
		assert_eq!(err.code, 403);
	}

	#[rstest::rstest]
	#[tokio::test]
	async fn test_ssrf_rejects_loopback_ipv4_other() {
		// Arrange
		let url = "http://127.0.0.2:8080/internal";

		// Act
		let result = validate_url_for_ssrf(url).await;

		// Assert
		assert!(result.is_err());
		let err = result.unwrap_err();
		assert_eq!(err.code, 403);
	}

	#[rstest::rstest]
	#[tokio::test]
	async fn test_ssrf_rejects_private_10_network() {
		// Arrange
		let url = "http://10.0.0.1/";

		// Act
		let result = validate_url_for_ssrf(url).await;

		// Assert
		assert!(result.is_err());
		let err = result.unwrap_err();
		assert_eq!(err.code, 403);
	}

	#[rstest::rstest]
	#[tokio::test]
	async fn test_ssrf_rejects_private_172_network() {
		// Arrange
		let url = "http://172.16.0.1/";

		// Act
		let result = validate_url_for_ssrf(url).await;

		// Assert
		assert!(result.is_err());
		let err = result.unwrap_err();
		assert_eq!(err.code, 403);
	}

	#[rstest::rstest]
	#[tokio::test]
	async fn test_ssrf_rejects_private_192_168_network() {
		// Arrange
		let url = "http://192.168.1.1/";

		// Act
		let result = validate_url_for_ssrf(url).await;

		// Assert
		assert!(result.is_err());
		let err = result.unwrap_err();
		assert_eq!(err.code, 403);
	}

	#[rstest::rstest]
	#[tokio::test]
	async fn test_ssrf_rejects_link_local() {
		// Arrange
		let url = "http://169.254.169.254/latest/meta-data/";

		// Act
		let result = validate_url_for_ssrf(url).await;

		// Assert
		assert!(result.is_err());
		let err = result.unwrap_err();
		assert_eq!(err.code, 403);
	}

	#[rstest::rstest]
	#[tokio::test]
	async fn test_ssrf_rejects_ipv6_loopback() {
		// Arrange
		let url = "http://[::1]/admin";

		// Act
		let result = validate_url_for_ssrf(url).await;

		// Assert
		assert!(result.is_err());
		let err = result.unwrap_err();
		assert_eq!(err.code, 403);
	}

	#[rstest::rstest]
	#[tokio::test]
	async fn test_ssrf_rejects_localhost() {
		// Arrange
		let url = "http://localhost/admin";

		// Act
		let result = validate_url_for_ssrf(url).await;

		// Assert
		assert!(result.is_err());
		let err = result.unwrap_err();
		assert_eq!(err.code, 403);
	}

	#[rstest::rstest]
	#[tokio::test]
	async fn test_ssrf_rejects_zero_ip() {
		// Arrange
		let url = "http://0.0.0.0/";

		// Act
		let result = validate_url_for_ssrf(url).await;

		// Assert
		assert!(result.is_err());
		let err = result.unwrap_err();
		assert_eq!(err.code, 403);
	}

	#[rstest::rstest]
	#[tokio::test]
	async fn test_ssrf_rejects_invalid_url() {
		// Arrange
		let url = "not-a-valid-url";

		// Act
		let result = validate_url_for_ssrf(url).await;

		// Assert
		assert!(result.is_err());
		let err = result.unwrap_err();
		assert_eq!(err.code, 400);
	}

	#[rstest::rstest]
	#[tokio::test]
	async fn test_ssrf_allows_public_http() {
		// Arrange
		let url = "http://8.8.8.8/api/data";

		// Act
		let result = validate_url_for_ssrf(url).await;

		// Assert
		assert!(result.is_ok());
	}

	#[rstest::rstest]
	#[tokio::test]
	async fn test_ssrf_allows_public_https() {
		// Arrange
		let url = "https://1.1.1.1/repos";

		// Act
		let result = validate_url_for_ssrf(url).await;

		// Assert
		assert!(result.is_ok());
	}

	#[rstest::rstest]
	#[tokio::test]
	async fn pinned_http_factory_preserves_headers_and_overrides_unsafe_transport() {
		use tokio::io::{AsyncReadExt, AsyncWriteExt};
		let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
		let address = listener.local_addr().unwrap();
		let factory: HttpClientFactory = Arc::new(|| {
			let mut headers = reqwest::header::HeaderMap::new();
			headers.insert(
				"x-custom-client",
				reqwest::header::HeaderValue::from_static("retained"),
			);
			reqwest::Client::builder()
				.default_headers(headers)
				.proxy(reqwest::Proxy::http("http://127.0.0.1:9").unwrap())
				.redirect(reqwest::redirect::Policy::limited(10))
				.resolve("plugin.example", "127.0.0.1:9".parse().unwrap())
		});
		// Exercise the transport seam with a local socket after address validation.
		let url = format!("http://plugin.example:{}/", address.port());
		let client = build_pinned_http_client(&url, &factory, &[address]).unwrap();
		let server = async {
			let (mut socket, _) = listener.accept().await.unwrap();
			let mut request = Vec::new();
			while !request.windows(4).any(|window| window == b"\r\n\r\n") {
				let mut buffer = [0u8; 1024];
				let count = socket.read(&mut buffer).await.unwrap();
				assert_ne!(count, 0);
				request.extend_from_slice(&buffer[..count]);
			}
			assert!(
				String::from_utf8(request)
					.unwrap()
					.to_lowercase()
					.contains("x-custom-client: retained")
			);
			socket.write_all(b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:9/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await.unwrap();
		};
		let (response, ()) = tokio::time::timeout(std::time::Duration::from_secs(2), async {
			tokio::join!(client.get(&url).send(), server)
		})
		.await
		.unwrap();
		assert_eq!(response.unwrap().status(), reqwest::StatusCode::FOUND);
	}

	#[rstest::rstest]
	#[tokio::test]
	async fn http_client_factory_survives_cloning_and_disabled_clients_remain_disabled() {
		let host = HostStateBuilder::new("factory")
			.http_client_factory(reqwest::Client::builder)
			.build();
		assert!(host.clone().http_client_factory.is_some());
		let disabled = HostStateBuilder::new("disabled")
			.http_client_factory(reqwest::Client::builder)
			.no_http_client()
			.build();
		assert!(
			disabled
				.clone()
				.pinned_http_client("https://192.0.1.1/")
				.await
				.unwrap_err()
				.message
				.contains("not available")
		);
		let legacy = HostStateBuilder::new("legacy")
			.http_client(reqwest::Client::new())
			.build();
		assert!(
			legacy
				.pinned_http_client("https://192.0.1.1/")
				.await
				.unwrap_err()
				.message
				.contains("http_client_factory")
		);
	}

	#[rstest::rstest]
	#[case("192.0.0.1", true)]
	#[case("192.0.0.8", true)]
	#[case("192.0.0.9", false)]
	#[case("192.0.0.10", false)]
	#[case("192.0.0.11", true)]
	#[case("192.0.0.170", true)]
	#[case("192.0.0.171", true)]
	#[case("192.0.1.1", false)]
	#[case("192.0.2.1", true)]
	fn private_ipv4_filter_preserves_anycast_exceptions_and_adjacent_prefix(
		#[case] address: &str,
		#[case] expected_private: bool,
	) {
		assert_eq!(is_private_ip(&address.parse().unwrap()), expected_private);
	}

	#[rstest::rstest]
	#[case("192.0.0.8", false)]
	#[case("192.0.0.9", true)]
	#[case("192.0.0.10", true)]
	#[case("192.0.0.11", false)]
	#[case("192.0.0.170", false)]
	#[case("192.0.0.171", false)]
	#[case("192.88.99.1", true)]
	#[case("192.88.99.2", false)]
	#[case("192.88.99.3", true)]
	#[tokio::test]
	async fn plugin_urls_enforce_ipv4_special_assignments_for_literals_and_dns(
		#[case] address: &str,
		#[case] allowed: bool,
	) {
		// Arrange: one literal and one DNS response represent the same destination.
		let socket: std::net::SocketAddr = format!("{address}:443").parse().unwrap();
		let literal = format!("https://{address}/component");

		// Act: both paths run the production URL and resolved-address validation.
		let direct = validate_url_for_ssrf(&literal).await;
		let resolved = validate_url_for_ssrf_with_resolver(
			"https://anycast.example/component",
			|domain, port| async move {
				assert_eq!(domain, "anycast.example");
				assert_eq!(port, 443);
				Ok(vec![socket])
			},
		)
		.await;

		// Assert: literal and DNS destinations enforce the same narrow /32 assignments.
		if allowed {
			assert_eq!(direct.unwrap(), vec![socket]);
			assert_eq!(resolved.unwrap(), vec![socket]);
		} else {
			assert_eq!(direct.unwrap_err().code, 403);
			assert_eq!(resolved.unwrap_err().code, 403);
		}
	}

	#[rstest::rstest]
	#[tokio::test]
	async fn test_ssrf_allows_public_ipv6_literal() {
		assert!(
			validate_url_for_ssrf("https://[2606:4700:4700::1111]/")
				.await
				.is_ok()
		);
	}

	#[rstest::rstest]
	#[tokio::test]
	async fn test_ssrf_rejects_userinfo_and_reserved_addresses() {
		assert!(
			validate_url_for_ssrf("https://user@8.8.8.8/path")
				.await
				.is_err()
		);
		assert!(
			validate_url_for_ssrf("http://100.64.0.1/path")
				.await
				.is_err()
		);
		assert!(
			validate_url_for_ssrf("http://198.18.0.1/path")
				.await
				.is_err()
		);
		assert!(
			validate_url_for_ssrf("http://[2001:db8::1]/path")
				.await
				.is_err()
		);
		assert!(
			validate_url_for_ssrf("http://[2002:a00:1::]/path")
				.await
				.is_err()
		);
	}

	#[test]
	fn test_is_private_ip_loopback_v4() {
		// Arrange
		let ip = std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1));

		// Act & Assert
		assert!(is_private_ip(&ip));
	}

	#[test]
	fn test_is_private_ip_private_10() {
		// Arrange
		let ip = std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, 1, 2, 3));

		// Act & Assert
		assert!(is_private_ip(&ip));
	}

	#[test]
	fn test_is_private_ip_private_172() {
		// Arrange
		let ip = std::net::IpAddr::V4(std::net::Ipv4Addr::new(172, 16, 0, 1));

		// Act & Assert
		assert!(is_private_ip(&ip));
	}

	#[test]
	fn test_is_private_ip_private_192_168() {
		// Arrange
		let ip = std::net::IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 0, 1));

		// Act & Assert
		assert!(is_private_ip(&ip));
	}

	#[test]
	fn test_is_private_ip_link_local() {
		// Arrange
		let ip = std::net::IpAddr::V4(std::net::Ipv4Addr::new(169, 254, 1, 1));

		// Act & Assert
		assert!(is_private_ip(&ip));
	}

	#[test]
	fn test_is_private_ip_public() {
		// Arrange
		let ip = std::net::IpAddr::V4(std::net::Ipv4Addr::new(8, 8, 8, 8));

		// Act & Assert
		assert!(!is_private_ip(&ip));
	}

	#[test]
	fn test_is_private_ip_ipv6_loopback() {
		// Arrange
		let ip = std::net::IpAddr::V6(std::net::Ipv6Addr::LOCALHOST);

		// Act & Assert
		assert!(is_private_ip(&ip));
	}

	#[test]
	fn test_is_private_ip_ipv6_unique_local() {
		// Arrange - fc00::/7
		let ip = std::net::IpAddr::V6(std::net::Ipv6Addr::new(0xfc00, 0, 0, 0, 0, 0, 0, 1));

		// Act & Assert
		assert!(is_private_ip(&ip));
	}

	#[test]
	fn test_is_private_ip_ipv6_link_local() {
		// Arrange - fe80::/10
		let ip = std::net::IpAddr::V6(std::net::Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1));

		// Act & Assert
		assert!(is_private_ip(&ip));
	}

	#[test]
	fn test_is_private_ip_172_not_in_range() {
		// Arrange - 172.32.0.1 is NOT in 172.16.0.0/12 range
		let ip = std::net::IpAddr::V4(std::net::Ipv4Addr::new(172, 32, 0, 1));

		// Act & Assert
		assert!(!is_private_ip(&ip));
	}

	// ==========================================================================
	// Clone Preservation Tests (#682)
	// ==========================================================================

	#[rstest::rstest]
	fn test_clone_preserves_config() {
		// Arrange
		let state = HostState::new("test-plugin");
		state
			.set_config(
				"db_url",
				ConfigValue::StringVal("postgres://localhost".to_string()),
			)
			.unwrap();
		state
			.set_config("max_retries", ConfigValue::IntVal(3))
			.unwrap();

		// Act
		let cloned = state.clone();

		// Assert
		assert_eq!(cloned.plugin_name, "test-plugin");
		assert_eq!(
			cloned.get_config("db_url"),
			Some(ConfigValue::StringVal("postgres://localhost".to_string()))
		);
		assert_eq!(
			cloned.get_config("max_retries"),
			Some(ConfigValue::IntVal(3))
		);
	}

	#[rstest::rstest]
	fn test_clone_preserves_capabilities() {
		// Arrange
		use crate::capability::PluginCapability;

		let mut state = HostState::new("test-plugin");
		let network_cap = Capability::Core(PluginCapability::NetworkAccess);
		let db_cap = Capability::Core(PluginCapability::DatabaseAccess);
		state.add_capability(network_cap.clone());
		state.add_capability(db_cap.clone());

		// Act
		let cloned = state.clone();

		// Assert
		assert!(cloned.has_capability(&network_cap));
		assert!(cloned.has_capability(&db_cap));
		assert_eq!(cloned.capabilities().len(), 2);
	}

	#[rstest::rstest]
	fn test_clone_preserves_services() {
		// Arrange
		let state = HostState::new("test-plugin");
		let service_data = vec![10, 20, 30, 40];
		state
			.register_service("cache-service", service_data.clone())
			.unwrap();

		// Act
		let cloned = state.clone();

		// Assert
		assert_eq!(cloned.get_service("cache-service"), Some(service_data));
	}

	#[rstest::rstest]
	fn test_clone_shares_event_bus() {
		// Arrange
		let event_bus = Arc::new(EventBus::new());
		let state = HostStateBuilder::new("test-plugin")
			.event_bus(event_bus.clone())
			.build();

		// Act
		let cloned = state.clone();

		// Assert - shared event bus should be the same Arc
		assert!(Arc::ptr_eq(state.event_bus(), cloned.event_bus()));
	}

	#[rstest::rstest]
	fn test_clone_shares_model_registry() {
		// Arrange
		let model_registry = Arc::new(ModelRegistry::new());
		let state = HostStateBuilder::new("test-plugin")
			.model_registry(model_registry.clone())
			.build();

		// Act
		let cloned = state.clone();

		// Assert - shared model registry should be the same Arc
		assert!(Arc::ptr_eq(state.model_registry(), cloned.model_registry()));
	}

	#[rstest::rstest]
	fn test_clone_shares_ssr_proxy() {
		// Arrange
		let ssr_proxy = Arc::new(SsrProxy::with_availability(true));
		let state = HostStateBuilder::new("test-plugin")
			.ssr_proxy(ssr_proxy.clone())
			.build();

		// Act
		let cloned = state.clone();

		// Assert
		assert!(Arc::ptr_eq(state.ssr_proxy(), cloned.ssr_proxy()));
		assert!(cloned.is_ssr_available());
	}

	#[rstest::rstest]
	fn test_clone_preserves_no_http_client() {
		// Arrange
		let state = HostStateBuilder::new("test-plugin")
			.no_http_client()
			.build();

		// Act
		let cloned = state.clone();

		// Assert - http_client should remain None
		assert!(!cloned.has_http_capability());
	}

	mod http_limits {
		use super::*;
		use crate::capability::PluginCapability;
		use rstest::rstest;
		use std::io;
		use std::net::SocketAddr;
		use tokio::io::{AsyncReadExt, AsyncWriteExt};
		use tokio::net::{TcpListener, TcpStream};
		use tokio::sync::oneshot;
		use tokio::task::JoinHandle;

		/// Public address literal that passes SSRF validation without DNS.
		///
		/// Only used by tests whose request must be rejected before it is sent.
		const PUBLIC_URL: &str = "http://8.8.8.8/resource";

		/// Aborts the wrapped task on drop so stalled servers and in-flight
		/// requests never outlive the test.
		struct AbortOnDrop<T>(JoinHandle<T>);

		impl<T> Drop for AbortOnDrop<T> {
			fn drop(&mut self) {
				self.0.abort();
			}
		}

		/// What the mock upstream sends after reading one request.
		struct MockReply {
			head: String,
			body_len: usize,
			stall: bool,
		}

		/// Loopback HTTP server standing in for a validated destination.
		struct MockUpstream {
			/// Request URL on a reserved domain that the test client pins to
			/// `addr`, the same way production pins validated DNS results.
			url: String,
			addr: SocketAddr,
			request_seen: oneshot::Receiver<()>,
			server: AbortOnDrop<io::Result<()>>,
		}

		impl MockUpstream {
			/// Client from the production pinning path, connected to this
			/// upstream. Its factory sets no timeouts, so only the host
			/// deadline can end a stalled call.
			fn pinned_client(&self) -> reqwest::Client {
				let factory: HttpClientFactory = Arc::new(reqwest::Client::builder);
				build_pinned_http_client(&self.url, &factory, &[self.addr]).unwrap()
			}
		}

		async fn spawn_upstream(reply: MockReply) -> MockUpstream {
			let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
			let addr = listener.local_addr().unwrap();
			let (seen_tx, request_seen) = oneshot::channel();
			let server = tokio::spawn(async move {
				let (mut socket, _) = listener.accept().await?;
				read_request(&mut socket).await?;
				let _ = seen_tx.send(());
				socket.write_all(reply.head.as_bytes()).await?;
				let chunk = [b'x'; 8192];
				let mut remaining = reply.body_len;
				while remaining > 0 {
					let len = remaining.min(chunk.len());
					socket.write_all(&chunk[..len]).await?;
					remaining -= len;
				}
				if reply.stall {
					std::future::pending::<()>().await;
				}
				Ok(())
			});
			MockUpstream {
				url: format!("http://upstream.test:{}/resource", addr.port()),
				addr,
				request_seen,
				server: AbortOnDrop(server),
			}
		}

		/// Read one request head and its `Content-Length` body, so closing
		/// the socket afterwards never discards unread client data.
		async fn read_request(socket: &mut TcpStream) -> io::Result<()> {
			let mut received = Vec::new();
			let mut buf = [0u8; 1024];
			let head_end = loop {
				let read = socket.read(&mut buf).await?;
				if read == 0 {
					return Err(io::ErrorKind::UnexpectedEof.into());
				}
				received.extend_from_slice(&buf[..read]);
				if let Some(pos) = received.windows(4).position(|w| w == b"\r\n\r\n") {
					break pos + 4;
				}
			};
			let head = String::from_utf8_lossy(&received[..head_end]).to_ascii_lowercase();
			let body_len = head
				.lines()
				.find_map(|line| line.strip_prefix("content-length:"))
				.map_or(0, |value| value.trim().parse::<usize>().unwrap());
			let mut remaining = body_len.saturating_sub(received.len() - head_end);
			while remaining > 0 {
				let read = socket.read(&mut buf).await?;
				if read == 0 {
					return Err(io::ErrorKind::UnexpectedEof.into());
				}
				remaining = remaining.saturating_sub(read);
			}
			Ok(())
		}

		/// Response head framing the body by `Content-Length` when given,
		/// or by closing the connection otherwise.
		fn response_head(content_length: Option<u64>) -> String {
			match content_length {
				Some(len) => format!("HTTP/1.1 200 OK\r\nContent-Length: {len}\r\n\r\n"),
				None => "HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n".to_string(),
			}
		}

		fn limits(max_body_bytes: usize, request_timeout: Duration) -> HttpResourceLimits {
			HttpResourceLimits {
				max_response_body_bytes: max_body_bytes,
				max_request_body_bytes: max_body_bytes,
				request_timeout,
				..HttpResourceLimits::default()
			}
		}

		/// Host state holding `NetworkAccess` with the given HTTP limits.
		fn network_state(limits: HttpResourceLimits) -> HostState {
			HostStateBuilder::new("network-plugin")
				.capability(Capability::Core(PluginCapability::NetworkAccess))
				.http_limits(limits)
				.build()
		}

		#[rstest]
		fn test_host_state_new_applies_default_http_limits() {
			// Arrange
			let state = HostState::new("test-plugin");

			// Act
			let limits = state.http_limits();

			// Assert
			assert_eq!(limits.max_response_body_bytes, 10 * 1024 * 1024);
			assert_eq!(limits.max_request_body_bytes, 10 * 1024 * 1024);
			assert_eq!(limits.request_timeout, Duration::from_secs(30));
			assert_eq!(limits.max_concurrent_requests, 4);
		}

		#[rstest]
		fn test_clone_shares_http_limits_and_permits() {
			// Arrange
			let limits = limits(42, Duration::from_secs(1));
			let state = HostStateBuilder::new("test-plugin")
				.http_limits(limits.clone())
				.build();

			// Act
			let cloned = state.clone();

			// Assert
			assert_eq!(cloned.http_limits(), &limits);
			assert!(Arc::ptr_eq(&state.http_permits, &cloned.http_permits));
		}

		#[rstest]
		#[case::below_limit(1023)]
		#[case::at_limit(1024)]
		#[tokio::test]
		async fn test_http_get_returns_body_within_limit(
			#[case] body_len: usize,
			#[values(true, false)] declare_length: bool,
		) {
			// Arrange
			let upstream = spawn_upstream(MockReply {
				head: response_head(declare_length.then_some(body_len as u64)),
				body_len,
				stall: false,
			})
			.await;
			let state = network_state(limits(1024, Duration::from_secs(10)));
			let client = upstream.pinned_client();

			// Act
			let response = state
				.execute_http_request(client.get(&upstream.url))
				.await
				.unwrap();

			// Assert
			assert_eq!(response.status, 200);
			assert_eq!(response.body, vec![b'x'; body_len]);
		}

		#[rstest]
		#[tokio::test]
		async fn test_http_get_rejects_body_one_byte_over_limit(
			#[values(true, false)] declare_length: bool,
		) {
			// Arrange
			let upstream = spawn_upstream(MockReply {
				head: response_head(declare_length.then_some(1025)),
				body_len: 1025,
				stall: false,
			})
			.await;
			let state = network_state(limits(1024, Duration::from_secs(10)));
			let client = upstream.pinned_client();

			// Act
			let error = state
				.execute_http_request(client.get(&upstream.url))
				.await
				.unwrap_err();

			// Assert
			assert_eq!(error.code, 502);
			assert_eq!(error.message, "HTTP response body exceeds the host limit");
			assert_eq!(error.details.as_deref(), Some("limit is 1024 bytes"));
		}

		#[rstest]
		#[tokio::test]
		async fn test_http_get_rejects_declared_oversized_body_before_reading() {
			// Arrange - the upstream declares 10 GiB and then sends nothing, so
			// waiting for the body would end in the 504 deadline instead
			let upstream = spawn_upstream(MockReply {
				head: response_head(Some(10 * 1024 * 1024 * 1024)),
				body_len: 0,
				stall: true,
			})
			.await;
			let state = network_state(limits(1024, Duration::from_secs(10)));
			let client = upstream.pinned_client();

			// Act
			let error = state
				.execute_http_request(client.get(&upstream.url))
				.await
				.unwrap_err();

			// Assert
			assert_eq!(error.code, 502);
			assert_eq!(error.message, "HTTP response body exceeds the host limit");
		}

		#[rstest]
		#[tokio::test]
		async fn test_http_get_drops_connection_once_stream_exceeds_limit() {
			// Arrange - 64 MiB without Content-Length against a 1 KiB limit
			let mut upstream = spawn_upstream(MockReply {
				head: response_head(None),
				body_len: 64 * 1024 * 1024,
				stall: false,
			})
			.await;
			let state = network_state(limits(1024, Duration::from_secs(10)));
			let client = upstream.pinned_client();

			// Act
			let error = state
				.execute_http_request(client.get(&upstream.url))
				.await
				.unwrap_err();
			let write_error = tokio::time::timeout(Duration::from_secs(10), &mut upstream.server.0)
				.await
				.expect("upstream kept writing after the limit was exceeded")
				.unwrap()
				.unwrap_err();

			// Assert - the upstream could not finish writing because the host
			// closed the connection instead of draining the body
			assert_eq!(error.code, 502);
			assert!(matches!(
				write_error.kind(),
				io::ErrorKind::BrokenPipe | io::ErrorKind::ConnectionReset
			));
		}

		#[rstest]
		#[tokio::test]
		async fn test_http_get_times_out_when_body_stalls() {
			// Arrange - the test client has no timeouts, so only the host
			// deadline can end the call
			let upstream = spawn_upstream(MockReply {
				head: response_head(Some(10)),
				body_len: 5,
				stall: true,
			})
			.await;
			let state = network_state(limits(1024, Duration::from_millis(200)));
			let client = upstream.pinned_client();

			// Act
			let error = state
				.execute_http_request(client.get(&upstream.url))
				.await
				.unwrap_err();

			// Assert
			assert_eq!(error.code, 504);
			assert_eq!(error.message, "HTTP request timed out");
		}

		#[rstest]
		#[tokio::test]
		async fn test_http_post_rejects_oversized_body_before_sending(
			#[values(false, true)] owned_body: bool,
		) {
			// Arrange - the real entry points validate the literal destination
			// and pin a client without connecting, so only a check that runs
			// before sending can produce 413; the short deadline bounds a
			// request that is sent anyway
			let state = network_state(limits(16, Duration::from_secs(1)));

			// Act
			let error = if owned_body {
				state.http_post_owned(PUBLIC_URL, vec![0; 17], &[]).await
			} else {
				state.http_post(PUBLIC_URL, &[0; 17], &[]).await
			}
			.unwrap_err();

			// Assert
			assert_eq!(error.code, 413);
			assert_eq!(error.message, "HTTP request body exceeds the host limit");
			assert_eq!(
				error.details.as_deref(),
				Some("17 bytes exceeds the limit of 16 bytes")
			);
		}

		#[rstest]
		#[tokio::test]
		async fn test_http_post_sends_body_at_limit() {
			// Arrange
			let upstream = spawn_upstream(MockReply {
				head: response_head(Some(2)),
				body_len: 2,
				stall: false,
			})
			.await;
			let state = network_state(limits(16, Duration::from_secs(10)));
			let client = upstream.pinned_client();
			let body = vec![0; 16];

			// Act - the size check both POST entry points run, then the send
			state.check_http_request_body(body.len()).unwrap();
			let response = state
				.send_http_post(&client, &upstream.url, body, &[])
				.await
				.unwrap();

			// Assert
			assert_eq!(response.status, 200);
			assert_eq!(response.body, b"xx".to_vec());
		}

		#[rstest]
		#[tokio::test]
		async fn test_http_concurrency_budget_is_shared_with_clones() {
			// Arrange - a clone holds the only permit while the upstream stalls
			let mut upstream = spawn_upstream(MockReply {
				head: response_head(Some(10)),
				body_len: 0,
				stall: true,
			})
			.await;
			let mut limits = limits(1024, Duration::from_secs(10));
			limits.max_concurrent_requests = 1;
			let state = network_state(limits);
			let cloned = state.clone();
			let client = upstream.pinned_client();
			let in_flight_request = client.get(&upstream.url);
			let _in_flight = AbortOnDrop(tokio::spawn(async move {
				cloned.execute_http_request(in_flight_request).await
			}));
			(&mut upstream.request_seen).await.unwrap();

			// Act
			let error = state
				.execute_http_request(client.get(&upstream.url))
				.await
				.unwrap_err();

			// Assert
			assert_eq!(error.code, 429);
			assert_eq!(error.message, "Too many concurrent HTTP requests");
			assert_eq!(error.details.as_deref(), Some("limit is 1 per plugin"));
		}
	}

	#[cfg(feature = "wasm")]
	mod db_limits {
		use super::*;
		use crate::capability::PluginCapability;
		use futures_util::stream::BoxStream;
		use reinhardt_db::backends::types::{
			DatabaseType, QueryResult, QueryValue, Row, TransactionExecutor,
		};
		use reinhardt_db::backends::{DatabaseBackend, Result as BackendResult};
		use rstest::rstest;
		use tokio::sync::Notify;
		use tokio::task::JoinHandle;

		/// Recursive CTE whose result never ends: 1, 2, 3, ...
		const UNBOUNDED_SQL: &str =
			"WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM n) SELECT x FROM n";

		/// Query yielding exactly `rows` rows with one integer column `x`.
		fn counting_sql(rows: usize) -> String {
			format!(
				"WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM n) \
				 SELECT x FROM n LIMIT {rows}"
			)
		}

		/// The rows `counting_sql(rows)` returns, in result order.
		fn counting_rows(rows: usize) -> Vec<HashMap<String, QueryValue>> {
			(1..=rows as i64)
				.map(|x| HashMap::from([("x".to_string(), QueryValue::Int(x))]))
				.collect()
		}

		fn no_params() -> Vec<u8> {
			rmp_serde::to_vec(&Vec::<QueryValue>::new()).unwrap()
		}

		fn limits(max_result_rows: usize, max_result_bytes: usize) -> DatabaseResourceLimits {
			DatabaseResourceLimits {
				max_result_rows,
				max_result_bytes,
				..DatabaseResourceLimits::default()
			}
		}

		fn db_state(connection: DatabaseConnection, limits: DatabaseResourceLimits) -> HostState {
			HostStateBuilder::new("db-plugin")
				.capability(Capability::Core(PluginCapability::DatabaseAccess))
				.db_connection(Arc::new(connection))
				.db_limits(limits)
				.build()
		}

		async fn sqlite_state(limits: DatabaseResourceLimits) -> HostState {
			let connection = DatabaseConnection::connect_sqlite("sqlite::memory:")
				.await
				.unwrap();
			db_state(connection, limits)
		}

		/// Aborts the wrapped task on drop so a pending query never outlives
		/// the test.
		struct AbortOnDrop<T>(JoinHandle<T>);

		impl<T> Drop for AbortOnDrop<T> {
			fn drop(&mut self) {
				self.0.abort();
			}
		}

		/// Backend whose row stream signals `started` and then never yields.
		struct PendingStreamBackend {
			started: Arc<Notify>,
		}

		#[async_trait::async_trait]
		impl DatabaseBackend for PendingStreamBackend {
			fn database_type(&self) -> DatabaseType {
				DatabaseType::Sqlite
			}

			fn placeholder(&self, _index: usize) -> String {
				"?".to_string()
			}

			fn supports_returning(&self) -> bool {
				false
			}

			fn supports_on_conflict(&self) -> bool {
				false
			}

			async fn execute(
				&self,
				_sql: &str,
				_params: Vec<QueryValue>,
			) -> BackendResult<QueryResult> {
				unreachable!("query host calls only stream rows")
			}

			async fn fetch_one(&self, _sql: &str, _params: Vec<QueryValue>) -> BackendResult<Row> {
				unreachable!("query host calls only stream rows")
			}

			async fn fetch_all(
				&self,
				_sql: &str,
				_params: Vec<QueryValue>,
			) -> BackendResult<Vec<Row>> {
				unreachable!("query host calls only stream rows")
			}

			fn __fetch_stream<'a>(
				&'a self,
				_sql: String,
				_params: Vec<QueryValue>,
			) -> BackendResult<BoxStream<'a, BackendResult<Row>>> {
				self.started.notify_one();
				Ok(Box::pin(futures_util::stream::pending()))
			}

			async fn fetch_optional(
				&self,
				_sql: &str,
				_params: Vec<QueryValue>,
			) -> BackendResult<Option<Row>> {
				unreachable!("query host calls only stream rows")
			}

			async fn begin(&self) -> BackendResult<Box<dyn TransactionExecutor>> {
				unreachable!("query host calls only stream rows")
			}

			fn as_any(&self) -> &dyn Any {
				self
			}
		}

		/// Backend that keeps the default (unsupported) row streaming and
		/// panics if the host falls back to materializing a result.
		struct NonStreamingBackend;

		#[async_trait::async_trait]
		impl DatabaseBackend for NonStreamingBackend {
			fn database_type(&self) -> DatabaseType {
				DatabaseType::Sqlite
			}

			fn placeholder(&self, _index: usize) -> String {
				"?".to_string()
			}

			fn supports_returning(&self) -> bool {
				false
			}

			fn supports_on_conflict(&self) -> bool {
				false
			}

			async fn execute(
				&self,
				_sql: &str,
				_params: Vec<QueryValue>,
			) -> BackendResult<QueryResult> {
				unreachable!("query host calls must not run statements")
			}

			async fn fetch_one(&self, _sql: &str, _params: Vec<QueryValue>) -> BackendResult<Row> {
				unreachable!("query host calls must not materialize rows")
			}

			async fn fetch_all(
				&self,
				_sql: &str,
				_params: Vec<QueryValue>,
			) -> BackendResult<Vec<Row>> {
				unreachable!("query host calls must not materialize rows")
			}

			async fn fetch_optional(
				&self,
				_sql: &str,
				_params: Vec<QueryValue>,
			) -> BackendResult<Option<Row>> {
				unreachable!("query host calls must not materialize rows")
			}

			async fn begin(&self) -> BackendResult<Box<dyn TransactionExecutor>> {
				unreachable!("query host calls must not open transactions")
			}

			fn as_any(&self) -> &dyn Any {
				self
			}
		}

		#[rstest]
		fn test_host_state_new_applies_default_db_limits() {
			// Arrange
			let state = HostState::new("test-plugin");

			// Act
			let limits = state.db_limits();

			// Assert
			assert_eq!(limits.max_result_rows, 10_000);
			assert_eq!(limits.max_result_bytes, 10 * 1024 * 1024);
			assert_eq!(limits.query_timeout, Duration::from_secs(30));
			assert_eq!(limits.max_concurrent_queries, 4);
		}

		#[rstest]
		fn test_clone_shares_db_limits_and_permits() {
			// Arrange
			let limits = limits(42, 4096);
			let state = HostStateBuilder::new("test-plugin")
				.db_limits(limits.clone())
				.build();

			// Act
			let cloned = state.clone();

			// Assert
			assert_eq!(cloned.db_limits(), &limits);
			assert!(Arc::ptr_eq(&state.db_permits, &cloned.db_permits));
		}

		#[rstest]
		#[case::empty(0)]
		#[case::fixarray(15)]
		#[case::array16(16)]
		#[case::array32(70_000)]
		#[tokio::test]
		async fn test_db_query_encodes_rows_like_materialized_result(#[case] rows: usize) {
			// Arrange
			let state = sqlite_state(limits(rows, 10 * 1024 * 1024)).await;

			// Act
			let result = state
				.db_query(&counting_sql(rows), &no_params())
				.await
				.unwrap();

			// Assert
			assert_eq!(result, rmp_serde::to_vec(&counting_rows(rows)).unwrap());
		}

		#[rstest]
		#[tokio::test]
		async fn test_db_query_rejects_row_over_row_limit() {
			// Arrange
			let state = sqlite_state(limits(100, 10 * 1024 * 1024)).await;

			// Act
			let error = state
				.db_query(&counting_sql(101), &no_params())
				.await
				.unwrap_err();

			// Assert
			assert_eq!(error.code, 413);
			assert_eq!(
				error.message,
				"Database query result exceeds the host limit"
			);
			assert_eq!(error.details.as_deref(), Some("limit is 100 rows"));
		}

		#[rstest]
		#[tokio::test]
		async fn test_db_query_stops_unbounded_result_at_default_row_limit() {
			// Arrange
			let state = sqlite_state(DatabaseResourceLimits::default()).await;

			// Act
			let error = state
				.db_query(UNBOUNDED_SQL, &no_params())
				.await
				.unwrap_err();

			// Assert
			assert_eq!(error.code, 413);
			assert_eq!(error.details.as_deref(), Some("limit is 10000 rows"));
		}

		#[rstest]
		#[case::at_limit(0, true)]
		#[case::one_byte_over(1, false)]
		#[tokio::test]
		async fn test_db_query_enforces_encoded_byte_limit(
			#[case] bytes_short: usize,
			#[case] succeeds: bool,
		) {
			// Arrange - the limit counts encoded rows, excluding the array header
			let expected = counting_rows(50);
			let row_bytes: usize = expected
				.iter()
				.map(|row| rmp_serde::to_vec(row).unwrap().len())
				.sum();
			let max_result_bytes = row_bytes - bytes_short;
			let state = sqlite_state(limits(50, max_result_bytes)).await;

			// Act
			let result = state.db_query(&counting_sql(50), &no_params()).await;

			// Assert
			if succeeds {
				assert_eq!(result.unwrap(), rmp_serde::to_vec(&expected).unwrap());
			} else {
				let error = result.unwrap_err();
				assert_eq!(error.code, 413);
				assert_eq!(
					error.details,
					Some(format!("limit is {max_result_bytes} bytes"))
				);
			}
		}

		#[rstest]
		#[tokio::test]
		async fn test_db_query_rejects_single_row_over_byte_limit() {
			// Arrange - one 4 KiB value against a 1 KiB budget
			let state = sqlite_state(limits(10, 1024)).await;

			// Act
			let error = state
				.db_query("SELECT zeroblob(4096) AS payload", &no_params())
				.await
				.unwrap_err();

			// Assert
			assert_eq!(error.code, 413);
			assert_eq!(error.details.as_deref(), Some("limit is 1024 bytes"));
		}

		#[rstest]
		fn test_limited_writer_stops_before_exceeding_limit() {
			use std::io::Write;

			// Arrange
			let mut buf = Vec::new();
			let mut writer = LimitedWriter {
				buf: &mut buf,
				limit: 8,
				exceeded: false,
			};

			// Act
			let within = writer.write_all(&[1; 6]);
			let over = writer.write_all(&[2; 3]);

			// Assert
			assert!(within.is_ok());
			assert!(over.is_err());
			assert!(writer.exceeded);
			assert_eq!(buf, vec![1; 6]);
			assert!(
				buf.capacity() <= 8,
				"capacity {} exceeds the limit",
				buf.capacity()
			);
		}

		#[rstest]
		#[tokio::test]
		async fn test_db_query_releases_permit_and_connection_after_limit_error() {
			// Arrange - one permit and a single-connection pool
			let mut limits = limits(10, 10 * 1024 * 1024);
			limits.max_concurrent_queries = 1;
			let state = sqlite_state(limits).await;
			let rejected = state.db_query(UNBOUNDED_SQL, &no_params()).await;

			// Act
			let result = state.db_query(&counting_sql(10), &no_params()).await;

			// Assert
			assert_eq!(rejected.unwrap_err().code, 413);
			assert_eq!(
				result.unwrap(),
				rmp_serde::to_vec(&counting_rows(10)).unwrap()
			);
		}

		#[rstest]
		#[tokio::test]
		async fn test_db_query_times_out_while_waiting_for_rows() {
			// Arrange
			let backend = PendingStreamBackend {
				started: Arc::new(Notify::new()),
			};
			let limits = DatabaseResourceLimits {
				query_timeout: Duration::from_millis(50),
				..DatabaseResourceLimits::default()
			};
			let state = db_state(DatabaseConnection::new(Arc::new(backend)), limits);

			// Act
			let error = state.db_query("SELECT 1", &no_params()).await.unwrap_err();

			// Assert
			assert_eq!(error.code, 504);
			assert_eq!(error.message, "Database query timed out");
			assert_eq!(error.details.as_deref(), Some("deadline is 50ms"));
		}

		#[rstest]
		#[tokio::test]
		async fn test_db_query_fails_closed_without_row_streaming() {
			// Arrange
			let connection = DatabaseConnection::new(Arc::new(NonStreamingBackend));
			let state = db_state(connection, DatabaseResourceLimits::default());

			// Act
			let error = state.db_query("SELECT 1", &no_params()).await.unwrap_err();

			// Assert
			assert_eq!(error.code, 500);
			assert_eq!(error.message, "Database query failed");
			assert!(
				error
					.details
					.as_deref()
					.is_some_and(|details| details.contains("Row streaming is not supported")),
				"unexpected details: {:?}",
				error.details
			);
		}

		#[rstest]
		#[tokio::test]
		async fn test_db_concurrency_budget_is_shared_with_clones() {
			// Arrange - a clone holds the only permit while its stream pends
			let started = Arc::new(Notify::new());
			let backend = PendingStreamBackend {
				started: started.clone(),
			};
			let limits = DatabaseResourceLimits {
				max_concurrent_queries: 1,
				..DatabaseResourceLimits::default()
			};
			let state = db_state(DatabaseConnection::new(Arc::new(backend)), limits);
			let cloned = state.clone();
			let _in_flight = AbortOnDrop(tokio::spawn(async move {
				cloned.db_query("SELECT 1", &no_params()).await
			}));
			started.notified().await;

			// Act
			let error = state.db_query("SELECT 1", &no_params()).await.unwrap_err();

			// Assert
			assert_eq!(error.code, 429);
			assert_eq!(error.message, "Too many concurrent database queries");
			assert_eq!(error.details.as_deref(), Some("limit is 1 per plugin"));
		}
	}
}
