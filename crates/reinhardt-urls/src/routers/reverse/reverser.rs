//! Runtime [`UrlReverser`] for name-to-URL lookup, plus the top-level
//! [`reverse`] convenience function.
//!
//! The global reverser singleton is automatically populated when
//! [`register_router()`](crate::routers::register_router) is called,
//! making [`UrlReverser::from_global()`] available without manual wiring.

use super::super::Route;
use super::super::pattern::PathPattern;
use super::runtime::ReverseResult;
use once_cell::sync::OnceCell;
use reinhardt_core::exception::Error;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::PoisonError;
use std::sync::RwLock as StdRwLock;

static GLOBAL_REVERSER: OnceCell<StdRwLock<Option<Arc<UrlReverser>>>> = OnceCell::new();

/// URL reverser for resolving names back to URLs
/// Similar to Django's URLResolver reverse functionality
#[derive(Clone)]
pub struct UrlReverser {
	/// Map of route names (including namespace) to routes
	routes: HashMap<String, Route>,
	/// Alias map: alias name → canonical name.
	/// Used for backward compatibility when route names change format.
	aliases: HashMap<String, String>,
}

impl UrlReverser {
	/// Create a new empty URL reverser.
	pub fn new() -> Self {
		Self {
			routes: HashMap::new(),
			aliases: HashMap::new(),
		}
	}

	/// Register a route for reverse lookup.
	///
	/// Returns `Err` with a descriptive message if a route with the same
	/// fully-qualified name has already been registered.
	pub fn register(&mut self, mut route: Route) -> std::result::Result<(), String> {
		// Validate the route-name segment against the kebab-case convention.
		// A leading `!` opts out of the warning and is stripped before storage,
		// so reverse lookups still use the clean name.
		if let Some(name) = route.name.take() {
			route.name = Some(normalize_route_name(&name));
		}
		if let Some(full_name) = route.full_name() {
			use std::collections::hash_map::Entry;
			match self.routes.entry(full_name.clone()) {
				Entry::Occupied(existing) => Err(format!(
					"Duplicate route name '{}': path '{}' conflicts with existing path '{}'",
					full_name,
					route.path,
					existing.get().path
				)),
				Entry::Vacant(entry) => {
					entry.insert(route);
					Ok(())
				}
			}
		} else {
			Ok(())
		}
	}

	/// Register a route by name and path (without handler)
	///
	/// This is used for hierarchical routers where we only need the name-to-path mapping
	/// for URL reversal, not the actual handler.
	///
	/// # Arguments
	///
	/// * `name` - The fully qualified route name (e.g., "v1:users:detail")
	/// * `path` - The URL path pattern (e.g., "/users/{id}/")
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::UrlReverser;
	///
	/// let mut reverser = UrlReverser::new();
	/// reverser.register_path("v1:users:detail", "/api/v1/users/{id}/").unwrap();
	///
	/// let url = reverser.reverse_with("v1:users:detail", &[("id", "123")]).unwrap();
	/// assert_eq!(url, "/api/v1/users/123/");
	/// ```
	///
	/// # Naming convention
	///
	/// The route-name segment (the part after the last `:` namespace separator)
	/// should be kebab-case (e.g. `users-detail`), matching ViewSet
	/// auto-generated names. A segment containing `_` or uppercase letters emits
	/// a `tracing::warn!` at registration time suggesting the kebab-case form.
	/// The warning is advisory, not an error. Prefix the segment with `!` to opt
	/// out, or set the `REINHARDT_URL_NAME_WARNINGS=0` environment variable to
	/// silence the warning globally. The `!` sigil is stripped before storage, so
	/// reverse lookups use the clean name:
	///
	/// ```
	/// use reinhardt_urls::routers::UrlReverser;
	///
	/// let mut reverser = UrlReverser::new();
	/// // The leading `!` suppresses the kebab-case warning for this intentional
	/// // snake_case name; it is stored (and reversed) as "user_detail".
	/// reverser.register_path("!user_detail", "/users/{id}/").unwrap();
	///
	/// assert!(reverser.has_route("user_detail"));
	/// assert!(!reverser.has_route("!user_detail"));
	/// let url = reverser.reverse_with("user_detail", &[("id", "7")]).unwrap();
	/// assert_eq!(url, "/users/7/");
	/// ```
	pub fn register_path(&mut self, name: &str, path: &str) -> std::result::Result<(), String> {
		// Create a dummy handler for the route
		// The handler is never used for URL reversal
		use reinhardt_http::Handler;

		#[derive(Clone)]
		struct DummyHandler;

		#[async_trait::async_trait]
		impl Handler for DummyHandler {
			async fn handle(
				&self,
				_req: reinhardt_http::Request,
			) -> reinhardt_core::exception::Result<reinhardt_http::Response> {
				unreachable!("DummyHandler should never be called")
			}
		}

		// Parse the name to extract namespace (if any). Only the segment after
		// the last ':' namespace separator is validated against the kebab-case
		// convention; a leading `!` on that segment opts out of the warning and
		// is stripped before storage.
		let (namespace, raw_segment) = match name.rsplit_once(':') {
			Some((ns, seg)) => (Some(ns.to_string()), seg),
			None => (None, name),
		};
		let route_name = normalize_route_name(raw_segment);
		let qualified_name = match &namespace {
			Some(ns) => format!("{ns}:{route_name}"),
			None => route_name.clone(),
		};

		let mut route = Route::new(path, Arc::new(DummyHandler));
		route.name = Some(route_name);

		let route = if let Some(ns) = namespace {
			route.with_namespace(&ns)
		} else {
			route
		};

		use std::collections::hash_map::Entry;
		match self.routes.entry(qualified_name.clone()) {
			Entry::Occupied(existing) => Err(format!(
				"Duplicate route name '{}': path '{}' conflicts with existing path '{}'",
				qualified_name,
				path,
				existing.get().path
			)),
			Entry::Vacant(entry) => {
				entry.insert(route);
				Ok(())
			}
		}
	}

	/// Reverse a URL name to a path with parameters
	/// Similar to Django's reverse() function
	///
	/// # Arguments
	///
	/// * `name` - The route name, optionally with namespace (e.g., "users:detail")
	/// * `params` - Map of parameter names to values
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::{UrlReverser, Route};
	/// use reinhardt_http::Handler;
	/// use std::sync::Arc;
	/// use std::collections::HashMap;
	///
	/// # use async_trait::async_trait;
	/// # use reinhardt_http::{Request, Response, Result};
	/// # struct DummyHandler;
	/// # #[async_trait]
	/// # impl Handler for DummyHandler {
	/// #     async fn handle(&self, _req: Request) -> Result<Response> {
	/// #         Ok(Response::ok())
	/// #     }
	/// # }
	/// let handler = Arc::new(DummyHandler);
	/// let mut reverser = UrlReverser::new();
	/// let mut route = Route::new("/users/{id}/", handler)
	///     .with_namespace("users");
	/// route.name = Some("detail".to_string());
	/// reverser.register(route).unwrap();
	///
	/// let mut params = HashMap::new();
	/// params.insert("id".to_string(), "123".to_string());
	///
	/// let url = reverser.reverse("users:detail", &params).unwrap();
	/// assert_eq!(url, "/users/123/");
	/// ```
	pub fn reverse(&self, name: &str, params: &HashMap<String, String>) -> ReverseResult<String> {
		// Prefer canonical route name; fall back to alias resolution only
		// when the direct lookup misses. This prevents an alias entry from
		// shadowing a real route with the same key.
		let route = if let Some(r) = self.routes.get(name) {
			r
		} else {
			let resolved_name = self.aliases.get(name).map(|s| s.as_str()).unwrap_or(name);
			self.routes
				.get(resolved_name)
				.ok_or_else(|| Error::NotFound(name.to_string()))?
		};

		// Parse the path pattern and delegate substitution to PathPattern::reverse.
		// PathPattern normalizes typed placeholders (e.g., "{<path:filepath>}" -> "{filepath}")
		// and substitutes against the normalized form, so reversal works correctly even when
		// the raw route.path contains typed placeholder syntax.
		let pattern = PathPattern::new(&route.path)
			.map_err(|e| Error::Validation(format!("pattern: {}", e)))?;

		pattern.reverse(params).map_err(Error::Validation)
	}

	/// Reverse a URL name to a path with positional parameters
	/// Convenience method that takes a slice of key-value pairs
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::{UrlReverser, Route};
	/// use reinhardt_http::Handler;
	/// use std::sync::Arc;
	///
	/// # use async_trait::async_trait;
	/// # use reinhardt_http::{Request, Response, Result};
	/// # struct DummyHandler;
	/// # #[async_trait]
	/// # impl Handler for DummyHandler {
	/// #     async fn handle(&self, _req: Request) -> Result<Response> {
	/// #         Ok(Response::ok())
	/// #     }
	/// # }
	/// let handler = Arc::new(DummyHandler);
	/// let mut reverser = UrlReverser::new();
	/// let mut route = Route::new("/users/{id}/", handler);
	/// route.name = Some("detail".to_string());
	/// reverser.register(route).unwrap();
	///
	/// let url = reverser.reverse_with("detail", &[("id", "123")]).unwrap();
	/// assert_eq!(url, "/users/123/");
	/// ```
	pub fn reverse_with<S: AsRef<str>>(
		&self,
		name: &str,
		params: &[(S, S)],
	) -> ReverseResult<String> {
		let params_map: HashMap<String, String> = params
			.iter()
			.map(|(k, v)| (k.as_ref().to_string(), v.as_ref().to_string()))
			.collect();

		self.reverse(name, &params_map)
	}

	/// Check if a route name is registered
	pub fn has_route(&self, name: &str) -> bool {
		self.routes.contains_key(name)
	}

	/// Get all registered route names
	pub fn route_names(&self) -> Vec<String> {
		self.routes.keys().cloned().collect()
	}

	/// Register an alias for a route name.
	///
	/// `reverse(alias)` will resolve to the same URL as `reverse(canonical)`.
	/// If the alias already exists, it is overwritten (last-write-wins).
	///
	/// The canonical target is resolved lazily — if the canonical route does
	/// not exist at the time of `reverse()`, the lookup returns `NotFound`.
	pub fn add_name_alias(&mut self, alias: &str, canonical: &str) {
		self.aliases
			.insert(alias.to_string(), canonical.to_string());
	}

	/// Retrieve the global URL reverser.
	///
	/// The global reverser is automatically populated when
	/// [`register_router()`](crate::routers::register_router) is called.
	///
	/// # Panics
	///
	/// Panics if no router has been registered yet.
	pub fn from_global() -> Arc<UrlReverser> {
		Self::try_from_global().expect(
			"global URL reverser is not registered. \
			 Call register_router() before using from_global().",
		)
	}

	/// Retrieve the global URL reverser, returning `None` if not yet registered.
	///
	/// Non-panicking variant of [`from_global()`](Self::from_global).
	pub fn try_from_global() -> Option<Arc<UrlReverser>> {
		GLOBAL_REVERSER
			.get()
			.and_then(|cell| cell.read().unwrap_or_else(PoisonError::into_inner).clone())
	}

	/// Register this reverser as the global instance.
	///
	/// In most cases you do not need to call this manually —
	/// [`register_router()`](crate::routers::register_router) does it
	/// automatically. Use this only when building a reverser independently
	/// of the router system.
	///
	/// # Panics
	///
	/// Panics if a global reverser has already been registered.
	pub fn register_global(self) {
		let cell = GLOBAL_REVERSER.get_or_init(|| StdRwLock::new(None));
		let mut guard = cell.write().unwrap_or_else(PoisonError::into_inner);
		if guard.is_some() {
			panic!("global URL reverser already registered");
		}
		*guard = Some(Arc::new(self));
	}
}

/// Set the global reverser, overwriting any previous value.
pub(crate) fn set_global_reverser(reverser: UrlReverser) {
	let cell = GLOBAL_REVERSER.get_or_init(|| StdRwLock::new(None));
	let mut guard = cell.write().unwrap_or_else(PoisonError::into_inner);
	*guard = Some(Arc::new(reverser));
}

/// Clear the global reverser (for test cleanup).
pub(crate) fn clear_global_reverser() {
	if let Some(cell) = GLOBAL_REVERSER.get() {
		let mut guard = cell.write().unwrap_or_else(PoisonError::into_inner);
		*guard = None;
	}
}

impl Default for UrlReverser {
	fn default() -> Self {
		Self::new()
	}
}

/// Standalone reverse function for convenience
/// Similar to Django's reverse() function
///
/// This requires routes to be registered with a global reverser.
/// For more control, use UrlReverser directly.
pub fn reverse(
	name: &str,
	params: &HashMap<String, String>,
	reverser: &UrlReverser,
) -> ReverseResult<String> {
	reverser.reverse(name, params)
}

/// Returns `true` if `segment` follows the kebab-case convention.
///
/// A segment is kebab-case when it contains no underscores and no ASCII
/// uppercase letters. Hyphens and digits are allowed; leading/trailing hyphens
/// are intentionally not flagged, to avoid false positives.
fn is_kebab_case(segment: &str) -> bool {
	!segment.chars().any(|c| c == '_' || c.is_ascii_uppercase())
}

/// Convert a snake_case, camelCase, or PascalCase `segment` to kebab-case.
///
/// Used only to build the suggestion shown in the non-kebab-case warning.
fn to_kebab_case(segment: &str) -> String {
	let mut out = String::with_capacity(segment.len() + 4);
	// Treat the start as a boundary so we never emit a leading '-'.
	let mut prev_is_boundary = true;
	for c in segment.chars() {
		if c == '_' || c == '-' {
			if !prev_is_boundary {
				out.push('-');
				prev_is_boundary = true;
			}
		} else if c.is_ascii_uppercase() {
			if !prev_is_boundary {
				out.push('-');
			}
			out.push(c.to_ascii_lowercase());
			prev_is_boundary = false;
		} else {
			out.push(c);
			prev_is_boundary = false;
		}
	}
	out
}

/// Environment variable that globally toggles the kebab-case URL-name warning.
///
/// Set it to `0`, `false`, `off`, or `no` (case-insensitive) to silence the
/// warning for every route name. Any other value — or leaving it unset — keeps
/// the warning enabled. The per-route `!` opt-out sigil remains the robust,
/// build-cache-independent way to suppress a single name.
const URL_NAME_WARNINGS_ENV: &str = "REINHARDT_URL_NAME_WARNINGS";

/// Parse a `REINHARDT_URL_NAME_WARNINGS` value into an enabled/disabled flag.
///
/// Split out from [`url_name_warnings_enabled`] so the precedence rules can be
/// unit-tested without mutating process-global environment state.
fn warnings_enabled_from_env(value: Option<&str>) -> bool {
	match value {
		Some(v) => !matches!(
			v.trim().to_ascii_lowercase().as_str(),
			"0" | "false" | "off" | "no"
		),
		None => true,
	}
}

/// Returns `true` when the kebab-case URL-name warning is enabled for this
/// process, honoring the [`URL_NAME_WARNINGS_ENV`] global toggle.
fn url_name_warnings_enabled() -> bool {
	warnings_enabled_from_env(std::env::var(URL_NAME_WARNINGS_ENV).ok().as_deref())
}

/// Normalize a bare route-name segment against the kebab-case convention.
///
/// If the segment begins with the `!` opt-out sigil, the sigil is stripped and
/// no warning is emitted. Otherwise a non-kebab-case segment triggers a
/// `tracing::warn!` suggesting the kebab-case form, unless the global toggle
/// ([`URL_NAME_WARNINGS_ENV`]) disables it. Returns the cleaned segment to be
/// stored and reversed against.
fn normalize_route_name(segment: &str) -> String {
	if let Some(stripped) = segment.strip_prefix('!') {
		return stripped.to_string();
	}
	if !is_kebab_case(segment) && url_name_warnings_enabled() {
		let suggestion = to_kebab_case(segment);
		tracing::warn!(
			target: "reinhardt_urls::reverse",
			"URL name '{segment}' is not kebab-case; suggestion: use '{suggestion}'. \
			 To suppress this warning, prefix the name with '!' (e.g. name = \"!{segment}\") \
			 or set {URL_NAME_WARNINGS_ENV}=0."
		);
	}
	segment.to_string()
}

#[cfg(test)]
mod kebab_convention_tests {
	use super::{is_kebab_case, normalize_route_name, to_kebab_case, warnings_enabled_from_env};
	use rstest::rstest;

	#[rstest]
	#[case(None, true)]
	#[case(Some("1"), true)]
	#[case(Some("true"), true)]
	#[case(Some("anything"), true)]
	#[case(Some("0"), false)]
	#[case(Some("false"), false)]
	#[case(Some("OFF"), false)]
	#[case(Some(" no "), false)]
	fn warnings_enabled_from_env_honors_disable_values(
		#[case] value: Option<&str>,
		#[case] expected: bool,
	) {
		// Act
		let result = warnings_enabled_from_env(value);

		// Assert
		assert_eq!(result, expected);
	}

	#[rstest]
	#[case("users-list", true)]
	#[case("users-detail", true)]
	#[case("detail", true)]
	#[case("v2", true)]
	#[case("user_detail", false)]
	#[case("userDetail", false)]
	#[case("UserDetail", false)]
	fn is_kebab_case_classifies_segments(#[case] segment: &str, #[case] expected: bool) {
		// Act
		let result = is_kebab_case(segment);

		// Assert
		assert_eq!(result, expected);
	}

	#[rstest]
	#[case("user_detail", "user-detail")]
	#[case("userDetail", "user-detail")]
	#[case("UserDetail", "user-detail")]
	#[case("users-list", "users-list")]
	fn to_kebab_case_converts_segments(#[case] segment: &str, #[case] expected: &str) {
		// Act
		let result = to_kebab_case(segment);

		// Assert
		assert_eq!(result, expected);
	}

	#[rstest]
	#[case("!user_detail", "user_detail")]
	#[case("!users-list", "users-list")]
	#[case("user_detail", "user_detail")]
	#[case("users-list", "users-list")]
	fn normalize_route_name_strips_optout_sigil(#[case] segment: &str, #[case] expected: &str) {
		// Act
		let result = normalize_route_name(segment);

		// Assert
		assert_eq!(result, expected);
	}
}
