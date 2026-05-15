//! Builder methods for [`ServerRouter`].
//!
//! Holds the constructor, prefix/namespace/DI configuration, middleware
//! registration, and child router composition (`mount`, `group`).

use super::ServerRouter;
use super::types::MiddlewareInfo;
use crate::routers::UrlReverser;
use matchit::Router as MatchitRouter;
use reinhardt_di::InjectionContext;
use reinhardt_http::ExcludeMiddleware;
use reinhardt_middleware::Middleware;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

impl ServerRouter {
	/// Validate that a prefix for `mount`/`include` follows Django URL conventions.
	///
	/// # Panics
	///
	/// Panics if the prefix doesn't end with "/".
	/// This matches Django's behavior where URL patterns must end with a trailing slash.
	/// Use "/" for root mounting instead of an empty string "".
	///
	/// # Examples
	///
	/// ```should_panic
	/// use reinhardt_urls::routers::ServerRouter;
	///
	/// // This will panic because "api" doesn't end with "/"
	/// let router = ServerRouter::new()
	///     .mount("api", ServerRouter::new());
	/// ```
	///
	/// ```should_panic
	/// use reinhardt_urls::routers::ServerRouter;
	///
	/// // This will panic because "" is not allowed, use "/" instead
	/// let router = ServerRouter::new()
	///     .mount("", ServerRouter::new());
	/// ```
	fn validate_prefix(prefix: &str) {
		// Prefix must not contain path parameter placeholders.
		// Mount prefixes are matched as literal strings, so a placeholder like
		// `{org}` would never match an actual path segment and all child routes
		// would silently 404. Fail early at construction time instead.
		if prefix.contains('{') || prefix.contains('}') {
			panic!(
				"`mount()` prefix `{prefix}` contains a path parameter placeholder (`{{...}}`); this is not supported.\nUse `route()` with the full path on the child router instead, or mount at a literal prefix."
			);
		}

		// Prefix must end with "/"
		if !prefix.ends_with('/') {
			if prefix.is_empty() {
				panic!(
					"URL route prefix cannot be an empty string. \
					 Use '/' instead of ''. \
					 This follows Django URL configuration conventions."
				);
			} else {
				panic!(
					"URL route '{}' must end with a trailing slash '/'. \
					 Use '{}/' instead of '{}'. \
					 This follows Django URL configuration conventions.",
					prefix, prefix, prefix,
				);
			}
		}
	}

	/// Create a new ServerRouter
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::ServerRouter;
	///
	/// let router = ServerRouter::new();
	/// ```
	pub fn new() -> Self {
		Self {
			prefix: String::new(),
			namespace: None,
			routes: Vec::new(),
			viewsets: HashMap::new(),
			functions: Vec::new(),
			views: Vec::new(),
			children: Vec::new(),
			di_context: None,
			middleware: Vec::new(),
			middleware_names: Vec::new(),
			middleware_exclusions: Vec::new(),
			reverser: UrlReverser::new(),
			get_router: RwLock::new(MatchitRouter::new()),
			post_router: RwLock::new(MatchitRouter::new()),
			put_router: RwLock::new(MatchitRouter::new()),
			delete_router: RwLock::new(MatchitRouter::new()),
			patch_router: RwLock::new(MatchitRouter::new()),
			head_router: RwLock::new(MatchitRouter::new()),
			options_router: RwLock::new(MatchitRouter::new()),
			routes_compiled: RwLock::new(false),
		}
	}

	/// Set the prefix for this router
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::ServerRouter;
	///
	/// let router = ServerRouter::new()
	///     .with_prefix("/api/v1");
	/// ```
	pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
		self.prefix = prefix.into();
		self
	}

	/// Set the namespace for this router
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::ServerRouter;
	///
	/// let router = ServerRouter::new()
	///     .with_namespace("v1");
	/// ```
	pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
		self.namespace = Some(namespace.into());
		self
	}

	/// Set the DI context for this router
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_urls::routers::ServerRouter;
	/// use reinhardt_di::{InjectionContext, SingletonScope};
	/// use std::sync::Arc;
	///
	/// let singleton_scope = Arc::new(SingletonScope::new());
	/// let di_ctx = Arc::new(InjectionContext::builder(singleton_scope).build());
	/// let router = ServerRouter::new()
	///     .with_di_context(di_ctx);
	/// ```
	pub fn with_di_context(mut self, ctx: Arc<InjectionContext>) -> Self {
		self.di_context = Some(ctx);
		self
	}

	/// Returns a reference to the DI context, if set.
	pub(crate) fn di_context(&self) -> Option<&Arc<InjectionContext>> {
		self.di_context.as_ref()
	}

	/// Add middleware to this router
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_urls::routers::ServerRouter;
	/// use reinhardt_middleware::LoggingMiddleware;
	///
	/// let router = ServerRouter::new()
	///     .with_middleware(LoggingMiddleware::new());
	/// ```
	pub fn with_middleware<M: Middleware + 'static>(mut self, mw: M) -> Self {
		let full_type_name = std::any::type_name::<M>().to_string();
		let short_name = full_type_name
			.rsplit("::")
			.next()
			.unwrap_or(&full_type_name)
			.to_string();
		// Harvest middleware-contributed DI singleton registrations and push
		// them onto the global deferred-registration list so server startup
		// can apply them to the `SingletonScope`. See #4426. Note that when
		// this router is wrapped by `UnifiedRouter::with_middleware`, the
		// outer wrapper has already harvested these into its local
		// `di_registrations` field; collecting again here is harmless because
		// both paths ultimately target the same `SingletonScope` and a
		// repeated `set_arc_any` simply overwrites with the identical `Arc`.
		let di_entries = mw.di_registrations();
		if !di_entries.is_empty() {
			let mut list = reinhardt_di::DiRegistrationList::new();
			for (type_id, value) in di_entries {
				list.register_arc_any(type_id, value);
			}
			crate::routers::register_di_registrations(list);
		}
		self.middleware_names.push(MiddlewareInfo {
			name: short_name,
			type_name: full_type_name,
		});
		self.middleware.push(Arc::new(mw));
		self.middleware_exclusions.push(Vec::new());
		self
	}

	/// Exclude a URL path from the most recently added middleware.
	///
	/// Paths ending with `/` are treated as prefix matches: any request
	/// path starting with the given prefix will skip the middleware.
	/// Paths without trailing `/` require an exact match.
	///
	/// This method operates on the **last middleware** added via
	/// [`with_middleware()`](Self::with_middleware). Multiple `.exclude()`
	/// calls accumulate exclusions on the same middleware.
	///
	/// # Panics
	///
	/// Panics if no middleware has been added yet.
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_urls::routers::ServerRouter;
	/// use reinhardt_middleware::LoggingMiddleware;
	///
	/// let router = ServerRouter::new()
	///     .with_middleware(LoggingMiddleware::new())
	///         .exclude("/api/auth/")    // prefix: skips /api/auth/*
	///         .exclude("/health");      // exact: skips only /health
	/// ```
	pub fn exclude(mut self, pattern: &str) -> Self {
		assert!(
			!self.middleware_exclusions.is_empty(),
			"exclude() called with no middleware. Call with_middleware() first."
		);
		self.middleware_exclusions
			.last_mut()
			.unwrap()
			.push(pattern.to_string());
		self
	}

	/// Build middleware list, wrapping any with exclusions in `ExcludeMiddleware`.
	pub(crate) fn build_middleware_with_exclusions(&self) -> Vec<Arc<dyn Middleware>> {
		let mut result: Vec<Arc<dyn Middleware>> = Vec::with_capacity(self.middleware.len());

		for (mw, exclusions) in self
			.middleware
			.iter()
			.zip(self.middleware_exclusions.iter())
		{
			if exclusions.is_empty() {
				result.push(mw.clone());
			} else {
				let mut exclude_mw = ExcludeMiddleware::new(mw.clone());
				for pattern in exclusions {
					exclude_mw.add_exclusion_mut(pattern);
				}
				result.push(Arc::new(exclude_mw) as Arc<dyn Middleware>);
			}
		}

		result
	}

	/// Mount a child router at the given prefix
	///
	/// # Panics
	///
	/// Panics if the prefix is non-empty, not "/" and doesn't end with "/".
	/// This follows Django's URL configuration conventions.
	///
	/// Also panics if the prefix contains a path parameter placeholder
	/// (e.g. `/orgs/{org}/`). Mount prefixes are matched as literal strings,
	/// so placeholders would silently cause all child routes to return 404.
	/// Use `route()` with the full path on the child router instead, or
	/// mount at a literal prefix.
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_urls::routers::ServerRouter;
	///
	/// let users_router = ServerRouter::new()
	///     .with_namespace("users");
	///
	/// let router = ServerRouter::new()
	///     .with_prefix("/api")
	///     .mount("/users/", users_router);  // Note: trailing slash required
	///
	/// // Verify the router was created successfully
	/// assert_eq!(router.prefix(), "/api");
	/// ```
	///
	/// Using "/" for root mounting is also valid:
	///
	/// ```rust
	/// use reinhardt_urls::routers::ServerRouter;
	///
	/// let app_router = ServerRouter::new();
	/// let router = ServerRouter::new().mount("/", app_router);
	/// ```
	pub fn mount(mut self, prefix: &str, mut child: ServerRouter) -> Self {
		// Validate prefix follows Django URL conventions
		Self::validate_prefix(prefix);

		// Set prefix if not already set
		if child.prefix.is_empty() {
			child.prefix = prefix.to_string();
		}

		// Inherit DI context if child doesn't have one
		if child.di_context.is_none() {
			child.di_context = self.di_context.clone();
		}

		self.children.push(child);
		self
	}

	/// Mount a child router (mutable version)
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_urls::routers::ServerRouter;
	///
	/// let mut router = ServerRouter::new();
	/// let users_router = ServerRouter::new();
	///
	/// router.mount_mut("/users/", users_router);
	/// ```
	pub fn mount_mut(&mut self, prefix: &str, mut child: ServerRouter) {
		// Validate prefix follows Django URL conventions
		Self::validate_prefix(prefix);

		if child.prefix.is_empty() {
			child.prefix = prefix.to_string();
		}
		if child.di_context.is_none() {
			child.di_context = self.di_context.clone();
		}
		self.children.push(child);
	}

	/// Add multiple child routers at once
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_urls::routers::ServerRouter;
	///
	/// let users = ServerRouter::new().with_prefix("/users");
	/// let posts = ServerRouter::new().with_prefix("/posts");
	///
	/// let router = ServerRouter::new()
	///     .group(vec![users, posts]);
	///
	/// // Verify the router was created successfully
	/// assert_eq!(router.prefix(), "");
	/// ```
	pub fn group(mut self, routers: Vec<ServerRouter>) -> Self {
		for router in routers {
			self.children.push(router);
		}
		self
	}
}
