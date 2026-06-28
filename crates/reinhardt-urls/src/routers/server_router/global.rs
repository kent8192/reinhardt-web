//! Global router registry for URL inspection (showurls command)

use crate::routers::reverse::{clear_global_reverser, set_global_reverser};
use crate::routers::server_router::ServerRouter;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use std::sync::PoisonError;
use std::sync::RwLock as StdRwLock;

/// Global router registry
static GLOBAL_ROUTER: OnceCell<StdRwLock<Option<Arc<ServerRouter>>>> = OnceCell::new();

/// Global deferred DI registrations from route configuration
static GLOBAL_DI_REGISTRATIONS: OnceCell<StdRwLock<Option<reinhardt_di::DiRegistrationList>>> =
	OnceCell::new();

/// Register the application's main router globally
///
/// This allows commands like `showurls` to inspect registered routes.
/// The router is automatically wrapped in `Arc` internally.
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_urls::routers::{ServerRouter, register_router};
/// # use hyper::Method;
/// # use reinhardt_core::endpoint::EndpointInfo;
/// # use reinhardt_http::{Handler, Request, Response, Result};
/// # struct Health;
/// # impl EndpointInfo for Health {
/// #     fn path() -> &'static str { "/health" }
/// #     fn method() -> Method { Method::GET }
/// #     fn name() -> &'static str { "health" }
/// # }
/// # #[async_trait::async_trait]
/// # impl Handler for Health {
/// #     async fn handle(&self, _req: Request) -> Result<Response> { Ok(Response::ok()) }
/// # }
///
/// let router = ServerRouter::new()
///     .with_prefix("/api/v1")
///     .endpoint(|| Health);
///
/// // No Arc::new() needed!
/// register_router(router);
/// ```
pub fn register_router(mut router: ServerRouter) {
	let errors = router.register_all_routes();
	for error in &errors {
		tracing::warn!("{}", error);
	}
	// register_router_arc handles global reverser population internally
	register_router_arc(Arc::new(router));
}

/// Register a router that is already wrapped in Arc.
///
/// This is provided for cases where you already have an `Arc<ServerRouter>`.
/// In most cases, you should use [`register_router()`] instead.
///
/// **Important:** Unlike [`register_router()`], this function does **not** call
/// `register_all_routes()` because `Arc<ServerRouter>` cannot be mutated.
/// Callers must ensure routes have been registered before wrapping in `Arc`.
///
/// The global [`UrlReverser`](crate::routers::UrlReverser) is also populated
/// from the router's internal reverser. Callers must ensure
/// `register_all_routes()` was called before wrapping in `Arc` for the
/// reverser to contain routes.
pub fn register_router_arc(router: Arc<ServerRouter>) {
	set_global_reverser(router.reverser.clone());
	let cell = GLOBAL_ROUTER.get_or_init(|| StdRwLock::new(None));
	let mut guard = cell.write().unwrap_or_else(PoisonError::into_inner);
	*guard = Some(router);
}

/// Get a reference to the globally registered router
///
/// Returns `None` if no router has been registered.
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_urls::routers::get_router;
///
/// if let Some(router) = get_router() {
///     let routes = router.get_all_routes();
///     println!("Registered routes: {}", routes.len());
/// }
/// ```
pub fn get_router() -> Option<Arc<ServerRouter>> {
	GLOBAL_ROUTER
		.get()
		.and_then(|cell| cell.read().unwrap_or_else(PoisonError::into_inner).clone())
}

/// Check if a router has been registered
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_urls::routers::is_router_registered;
///
/// if !is_router_registered() {
///     println!("Warning: No router registered");
/// }
/// ```
pub fn is_router_registered() -> bool {
	GLOBAL_ROUTER
		.get()
		.map(|cell| {
			cell.read()
				.unwrap_or_else(PoisonError::into_inner)
				.is_some()
		})
		.unwrap_or(false)
}

/// Register deferred DI registrations globally
///
/// These registrations are captured during route configuration (e.g., in
/// `routes()` functions) and applied to the server's [`SingletonScope`]
/// during startup. This bridges the lifecycle gap between synchronous
/// route setup and the server's DI context creation.
///
/// [`SingletonScope`]: reinhardt_di::SingletonScope
pub fn register_di_registrations(list: reinhardt_di::DiRegistrationList) {
	let cell = GLOBAL_DI_REGISTRATIONS.get_or_init(|| StdRwLock::new(None));
	let mut guard = cell.write().unwrap_or_else(PoisonError::into_inner);
	match guard.as_mut() {
		Some(existing) => existing.merge(list),
		None => *guard = Some(list),
	}
}

/// Take the globally registered DI registrations
///
/// Returns `None` if no registrations have been stored.
/// After this call, the global cell is emptied (subsequent calls return `None`).
pub fn take_di_registrations() -> Option<reinhardt_di::DiRegistrationList> {
	GLOBAL_DI_REGISTRATIONS
		.get()
		.and_then(|cell| cell.write().unwrap_or_else(PoisonError::into_inner).take())
}

/// Get a clone of the DI context from the globally registered router, if one was set.
///
/// This allows server startup code to detect when the user has already
/// configured a DI context on their router (via [`ServerRouter::with_di_context`]
/// or [`UnifiedRouter::with_di_context`]) and reuse its singleton scope
/// instead of creating a new one.
///
/// Returns `None` if no router is registered or the router has no DI context.
///
/// [`UnifiedRouter::with_di_context`]: crate::routers::UnifiedRouter::with_di_context
pub fn get_router_di_context() -> Option<Arc<reinhardt_di::InjectionContext>> {
	get_router().and_then(|router| router.di_context().cloned())
}

/// Clear the registered router (useful for tests)
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_urls::routers::{clear_router, is_router_registered};
///
/// clear_router();
/// assert!(!is_router_registered());
/// ```
pub fn clear_router() {
	if let Some(cell) = GLOBAL_ROUTER.get() {
		let mut guard = cell.write().unwrap_or_else(PoisonError::into_inner);
		*guard = None;
	}
	// Also clear deferred DI registrations
	if let Some(cell) = GLOBAL_DI_REGISTRATIONS.get() {
		let mut guard = cell.write().unwrap_or_else(PoisonError::into_inner);
		*guard = None;
	}
	clear_global_reverser();
}

#[cfg(test)]
mod tests {
	use super::*;
	use hyper::Method;
	use reinhardt_core::endpoint::EndpointInfo;
	use reinhardt_http::{Handler, Request, Response};
	use reinhardt_testkit::resource::{TeardownGuard, TestResource};
	use rstest::{fixture, rstest};

	/// Fixture that clears global router state before each test
	/// and restores it after test completion via RAII.
	struct CleanGlobalRouter;

	impl TestResource for CleanGlobalRouter {
		fn setup() -> Self {
			clear_router();
			Self
		}

		fn teardown(&mut self) {
			clear_router();
		}
	}

	#[fixture]
	fn env() -> TeardownGuard<CleanGlobalRouter> {
		TeardownGuard::new()
	}

	struct UserDetail;
	struct HealthCheck;

	impl EndpointInfo for UserDetail {
		fn path() -> &'static str {
			"/users/{id}/"
		}

		fn method() -> Method {
			Method::GET
		}

		fn name() -> &'static str {
			"user-detail"
		}
	}

	impl EndpointInfo for HealthCheck {
		fn path() -> &'static str {
			"/health/"
		}

		fn method() -> Method {
			Method::GET
		}

		fn name() -> &'static str {
			"health-check"
		}
	}

	#[async_trait::async_trait]
	impl Handler for UserDetail {
		async fn handle(&self, _req: Request) -> reinhardt_core::exception::Result<Response> {
			Ok(Response::ok())
		}
	}

	#[async_trait::async_trait]
	impl Handler for HealthCheck {
		async fn handle(&self, _req: Request) -> reinhardt_core::exception::Result<Response> {
			Ok(Response::ok())
		}
	}

	#[rstest]
	#[serial_test::serial(global_router)]
	fn returns_none_when_no_router_registered(_env: TeardownGuard<CleanGlobalRouter>) {
		// Act
		let result = get_router_di_context();

		// Assert
		assert!(result.is_none());
	}

	#[rstest]
	#[serial_test::serial(global_router)]
	fn returns_none_when_router_has_no_di_context(_env: TeardownGuard<CleanGlobalRouter>) {
		// Arrange
		let router = crate::routers::ServerRouter::new();
		register_router(router);

		// Act
		let result = get_router_di_context();

		// Assert
		assert!(result.is_none());
	}

	#[rstest]
	#[serial_test::serial(global_router)]
	fn returns_context_when_router_has_di_context(_env: TeardownGuard<CleanGlobalRouter>) {
		// Arrange
		let singleton_scope = Arc::new(reinhardt_di::SingletonScope::new());
		singleton_scope.set(42u32);
		let di_ctx = Arc::new(reinhardt_di::InjectionContext::builder(singleton_scope).build());

		let router = crate::routers::ServerRouter::new().with_di_context(di_ctx);
		register_router(router);

		// Act
		let result = get_router_di_context();

		// Assert
		assert!(result.is_some());
		let ctx = result.unwrap();
		let value = ctx.singleton_scope().get::<u32>();
		assert!(value.is_some());
		assert_eq!(*value.unwrap(), 42);
	}

	#[rstest]
	#[serial_test::serial(global_router)]
	fn global_reverser_populated_after_register_router(_env: TeardownGuard<CleanGlobalRouter>) {
		// Arrange
		let router = crate::routers::ServerRouter::new().endpoint(|| UserDetail);

		// Act
		register_router(router);

		// Assert
		let reverser = crate::routers::UrlReverser::try_from_global();
		assert!(reverser.is_some());
		let reverser = reverser.unwrap();
		assert!(reverser.has_route("user-detail"));
	}

	#[rstest]
	#[serial_test::serial(global_router)]
	fn global_reverser_cleared_on_clear_router(_env: TeardownGuard<CleanGlobalRouter>) {
		// Arrange
		let router = crate::routers::ServerRouter::new().endpoint(|| HealthCheck);
		register_router(router);
		assert!(crate::routers::UrlReverser::try_from_global().is_some());

		// Act
		clear_router();

		// Assert
		assert!(crate::routers::UrlReverser::try_from_global().is_none());
	}

	#[rstest]
	#[serial_test::serial(global_router)]
	fn server_router_reverse_works_after_global_population(_env: TeardownGuard<CleanGlobalRouter>) {
		// Arrange
		let router = crate::routers::ServerRouter::new().endpoint(|| UserDetail);
		register_router(router);

		// Act
		let router = get_router().unwrap();
		let url = router.reverse("user-detail", &[("id", "42")]);

		// Assert
		assert_eq!(url, Some("/users/42/".to_string()));
	}
}
