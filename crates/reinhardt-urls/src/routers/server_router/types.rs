//! Type definitions and small helpers used by [`ServerRouter`].
//!
//! Splitting the data definitions out of the main module keeps the
//! `ServerRouter` definition file focused on the struct itself, while the
//! method bodies live in dedicated submodules (`builder`, `registration`,
//! `compile`, `introspection`, `dispatch`).

use hyper::Method;
use reinhardt_di::InjectionContext;
use reinhardt_http::Handler;
use reinhardt_middleware::Middleware;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Information about a registered middleware
///
/// Captures the short name and full type path of a middleware added via
/// [`ServerRouter::with_middleware()`](crate::routers::server_router::ServerRouter::with_middleware).
/// This enables runtime introspection of the middleware stack without
/// requiring `Middleware` to be `Debug`.
///
/// # Examples
///
/// ```
/// use reinhardt_urls::routers::server_router::MiddlewareInfo;
///
/// let info = MiddlewareInfo {
///     name: "LoggingMiddleware".to_string(),
///     type_name: "reinhardt_middleware::LoggingMiddleware".to_string(),
/// };
/// assert_eq!(info.name, "LoggingMiddleware");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MiddlewareInfo {
	/// Short name of the middleware (last segment of the type path)
	pub name: String,

	/// Full type path (e.g., `"reinhardt_middleware::logging::LoggingMiddleware"`)
	pub type_name: String,
}

/// Route information tuple: (path, name, namespace, methods)
pub type RouteInfo = Vec<(String, Option<String>, Option<String>, Vec<Method>)>;

/// Join two path segments, normalizing any double slashes.
///
/// Concatenates `prefix` and `suffix` and collapses consecutive `/` characters
/// into a single `/`. This helper does **not** insert a separator between
/// `prefix` and `suffix`; the caller is responsible for ensuring that either
/// `prefix` ends with `/` or `suffix` starts with `/` when a separator is
/// desired. The leading slash is always preserved.
///
/// # Contract
///
/// - If `prefix` ends with `/` or `suffix` starts with `/`, the result is a
///   well-formed joined path with a single `/` between segments.
/// - If neither holds, the segments are concatenated verbatim. This is
///   intentional: internal call sites always satisfy the contract above, and
///   collapsing only deduplicates existing slashes rather than inventing one.
///
/// # Examples
///
/// ```ignore
/// // crate-internal usage only
/// assert_eq!(join_path("/api/", "/users"), "/api/users");
/// assert_eq!(join_path("/api", "/users"), "/api/users");
/// // Caller-provided invariant violated: no separator is inserted.
/// assert_eq!(join_path("/api", "users"), "/apiusers");
/// assert_eq!(join_path("", "/users"), "/users");
/// ```
pub(crate) fn join_path(prefix: &str, suffix: &str) -> String {
	let raw = format!("{}{}", prefix, suffix);
	let mut result = String::with_capacity(raw.len());
	let mut prev_slash = false;
	for ch in raw.chars() {
		if ch == '/' {
			if !prev_slash {
				result.push(ch);
			}
			prev_slash = true;
		} else {
			result.push(ch);
			prev_slash = false;
		}
	}
	result
}

/// Handler information stored in matchit router
#[derive(Clone)]
pub(crate) struct RouteHandler {
	/// The actual handler
	pub(crate) handler: Arc<dyn Handler>,

	/// Route-level middleware
	pub(crate) middleware: Vec<Arc<dyn Middleware>>,
}

/// Route match result with metadata
#[derive(Clone)]
pub(crate) struct RouteMatch {
	/// Matched handler
	pub handler: Arc<dyn Handler>,

	/// Extracted path parameters in URL pattern declaration order.
	///
	/// Stored as an ordered `Vec<(name, value)>` so downstream extractors such
	/// as `Path<(T1, T2)>` can rely on URL declaration order. See issue #4013.
	pub params: Vec<(String, String)>,

	/// Middleware stack to apply (parent → child order)
	pub middleware_stack: Vec<Arc<dyn Middleware>>,

	/// DI context
	pub di_context: Option<Arc<InjectionContext>>,
}

impl RouteMatch {
	/// Look up a path parameter by name.
	///
	/// `params` is stored as an ordered `Vec` (see issue #4013) so this helper
	/// performs a linear scan. Path parameter sets are tiny in practice
	/// (typically 1–3 entries), so the cost is negligible compared to a
	/// `HashMap` lookup.
	#[cfg(test)]
	pub(crate) fn param(&self, name: &str) -> Option<&str> {
		self.params
			.iter()
			.find(|(k, _)| k == name)
			.map(|(_, v)| v.as_str())
	}
}

/// Function-based route
pub(crate) struct FunctionRoute {
	pub path: String,
	pub method: Method,
	pub handler: Arc<dyn Handler>,
	pub name: Option<String>,
	/// Middleware stack for this route
	pub middleware: Vec<Arc<dyn Middleware>>,
}

/// Class-based view route
pub(crate) struct ViewRoute {
	pub path: String,
	pub handler: Arc<dyn Handler>,
	pub name: Option<String>,
	/// Middleware stack for this route
	pub middleware: Vec<Arc<dyn Middleware>>,
}
