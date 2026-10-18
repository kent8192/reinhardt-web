//! Runserver lifecycle hooks for extending server startup behavior.
//!
//! Hooks are auto-discovered via `inventory` and invoked by
//! [`crate::RunServerCommand`] at three lifecycle points:
//!
//! 1. **Validation** ([`RunserverHook::validate`]) — before DI setup; return `Err` to abort.
//! 2. **Startup** ([`RunserverHook::on_server_start`]) — after DI is ready and
//!    native protocol preflight has completed, before listeners bind.
//! 3. **Request admission** ([`RunserverHook::request_admission`]) — after
//!    every hook's startup phase, before the HTTP server is built; at most one
//!    hook may return the admission policy that the server applies before it
//!    reads request bodies.
//!
//! With `--with-pages`, [`RunserverHook::static_response_headers`] also lets a
//! hook attach response headers to the SPA shell and static asset responses.
//!
//! Register hooks with the `#[hook(on = runserver)]` attribute macro.
//! `runserver` launches the aggregated HTTP/WebSocket server and, when
//! services are registered, the gRPC listener itself. Hooks remain the
//! extension point for additional services or custom transport settings.
//!
//! ## Autoreload semantics
//!
//! When the `autoreload` feature is enabled and `runserver` is invoked
//! without `--noreload`, only [`RunserverHook::validate`] runs in the parent
//! process; [`RunserverHook::on_server_start`] and
//! [`RunserverHook::request_admission`] run **only inside the spawned
//! `--noreload` child** that actually binds the listening port. Listeners
//! opened in `on_server_start` therefore live in the child and exit with the
//! child on each rebuild, which prevents parent-side listeners from blocking
//! the child's HTTP bind (#4244). Each rebuilt child also starts with a fresh
//! admission policy.
//!
//! # Hot-reload
//!
//! When the `autoreload` feature is enabled, `runserver` watches the workspace
//! source roots and rebuilds in place on every change:
//!
//! ```text
//! cargo run --bin manage -- runserver --with-pages
//! ```
//!
//! Edit any Rust source file (server-side or wasm-side) and the bundle plus
//! the server are rebuilt automatically. Pass `--noreload` to disable
//! auto-reload entirely, or `--no-wasm-rebuild` to keep server reload but
//! manage the wasm build yourself.
//!
//! ## Failure modes
//!
//! - **OL-1 (loop persistence)**: a build failure never terminates the watch
//!   loop. Termination requires SIGINT/SIGTERM or an explicit user quit.
//! - **OL-2 (greppable logs)**: every rebuild emits a single summary line
//!   prefixed `[hot-reload]`, e.g. `[hot-reload] WASM rebuild OK (took 1.2s)`
//!   on success or `[hot-reload] WASM rebuild FAILED (took 2.3s):` followed
//!   by the cargo exit code and the last lines of stderr on failure.
//! - **OL-3 (partial-failure preservation)**: if only one pipeline fails, the
//!   other side's last-good output is kept — failed wasm leaves the previous
//!   `dist/` in place; failed server keeps the previous server process. The
//!   next save retriggers both pipelines.

use std::error::Error;
use std::sync::Arc;

use async_trait::async_trait;
use reinhardt_di::InjectionContext;
use reinhardt_server::{RequestAdmission, ShutdownCoordinator};

/// Context passed to [`RunserverHook::on_server_start`] and
/// [`RunserverHook::request_admission`].
pub struct RunserverContext {
	/// Shutdown coordinator for subscribing to graceful shutdown signals.
	///
	/// Use `shutdown_coordinator.subscribe()` to receive shutdown notifications
	/// in spawned concurrent services.
	pub shutdown_coordinator: ShutdownCoordinator,

	/// DI context for resolving application services.
	pub di_context: Arc<InjectionContext>,
}

/// Hook for extending runserver behavior.
///
/// Implementations are auto-discovered via `inventory` when registered
/// with the `#[hook(on = runserver)]` attribute macro.
///
/// # Lifecycle
///
/// 1. `validate()` is called before DI context setup.
///    Return `Err` to abort server startup (fail-fast).
/// 2. `on_server_start()` is called after DI context is ready,
///    before the server starts listening.
///    Spawn concurrent services here and subscribe to shutdown via
///    `ctx.shutdown_coordinator.subscribe()`.
/// 3. `request_admission()` is called after every hook's `on_server_start()`
///    has completed, before the HTTP server is built.
///    Return the admission policy that the server applies before it reads
///    request bodies.
///
/// Every method has a default implementation. Implement only the phases
/// your hook needs.
///
/// Under the `autoreload` feature, `validate()` runs once in the parent
/// (and again in each child invocation) while `on_server_start()` and
/// `request_admission()` run **only inside the spawned `--noreload` child**
/// so concurrent-service listeners do not survive across rebuilds and do not
/// collide with the child's own server bind (#4244).
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt::commands::{RunserverHook, RunserverContext};
///
/// #[reinhardt::hook(on = runserver)]
/// struct MyHook;
///
/// #[async_trait]
/// impl RunserverHook for MyHook {
///     async fn validate(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
///         // Check required configuration
///         Ok(())
///     }
///
///     async fn on_server_start(
///         &self,
///         ctx: &RunserverContext,
///     ) -> Result<(), Box<dyn Error + Send + Sync>> {
///         // Spawn a concurrent service
///         let mut shutdown_rx = ctx.shutdown_coordinator.subscribe();
///         tokio::spawn(async move {
///             tokio::select! {
///                 _ = shutdown_rx.recv() => { /* cleanup */ }
///             }
///         });
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait RunserverHook: Send + Sync {
	/// Validation phase: return `Err` to prevent server startup.
	///
	/// Runs before `on_server_start` and before DI context setup.
	async fn validate(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
		Ok(())
	}

	/// Startup phase: spawn concurrent services, register shutdown handlers.
	///
	/// Receives [`RunserverContext`] with access to [`ShutdownCoordinator`]
	/// and [`InjectionContext`].
	async fn on_server_start(
		&self,
		ctx: &RunserverContext,
	) -> Result<(), Box<dyn Error + Send + Sync>> {
		let _ = ctx;
		Ok(())
	}

	/// Response headers attached to every static asset and SPA shell response.
	///
	/// `runserver --with-pages` serves the SPA shell (`index.html` and the SPA
	/// fallback) and static assets from server-level static middleware that
	/// answers before the application router, so router middleware never sees
	/// those responses. Return security headers such as
	/// `Content-Security-Policy`, `X-Content-Type-Options`, `X-Frame-Options`,
	/// `Referrer-Policy`, or `Strict-Transport-Security` here to attach them.
	///
	/// The returned map is merged over the `response_headers` table of the
	/// `[static_files]` (or `[static]`) settings section:
	///
	/// ```toml
	/// [static_files.response_headers]
	/// "X-Content-Type-Options" = "nosniff"
	/// ```
	///
	/// For every header name a hook returns, the hook's values replace the
	/// settings values. Hooks are merged in discovery order, which is not
	/// stable across builds, so define each header in at most one hook. The
	/// merged headers replace same-named headers set by the static middleware
	/// (for example `Cache-Control`) and leave its other headers intact; see
	/// [`StaticFilesConfig::response_headers`](reinhardt_utils::staticfiles::middleware::StaticFilesConfig#structfield.response_headers).
	///
	/// Called once while `runserver --with-pages` builds its static file
	/// middleware, after [`RunserverHook::on_server_start`]. The default
	/// implementation returns an empty map.
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use hyper::header::{self, HeaderMap, HeaderValue};
	/// use reinhardt::commands::RunserverHook;
	///
	/// #[reinhardt::hook(on = runserver)]
	/// struct SecurityHeadersHook;
	///
	/// #[async_trait]
	/// impl RunserverHook for SecurityHeadersHook {
	///     fn static_response_headers(&self) -> HeaderMap {
	///         let mut headers = HeaderMap::new();
	///         headers.insert(
	///             header::CONTENT_SECURITY_POLICY,
	///             HeaderValue::from_static("default-src 'self'; frame-ancestors 'none'"),
	///         );
	///         headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
	///         headers
	///     }
	/// }
	/// ```
	fn static_response_headers(&self) -> hyper::HeaderMap {
		hyper::HeaderMap::new()
	}

	/// Admission policy that `runserver` applies before it reads request bodies.
	///
	/// Router middleware runs only after the server has collected the request
	/// body, so it cannot cap the body size, bound body receipt, or shed load
	/// before the body arrives. Return the policy that the project passes to
	/// [`HttpServer::with_request_admission`](reinhardt_server::HttpServer::with_request_admission)
	/// in deployment, so `runserver` admits requests the same way. Without a
	/// policy, `runserver` collects bodies up to the server's default size limit
	/// with no receive timeout.
	///
	/// Called on every hook after every hook's [`RunserverHook::on_server_start`]
	/// has completed and before `runserver` builds its HTTP server, so a hook
	/// can return a policy that a startup phase registered in `ctx.di_context`.
	/// At most one hook may return a policy. Hook discovery order is not stable
	/// across builds, so when several hooks return one, `runserver` aborts
	/// startup and names every supplying hook instead of picking one. Returning
	/// `Err` also aborts startup.
	///
	/// The policy admits every request on the HTTP listener, including the
	/// `--with-pages` static and SPA responses, OpenAPI routes, and WebSocket
	/// handshakes. A policy that must treat development-only routes differently
	/// can branch on [`RequestHead::uri`](reinhardt_server::RequestHead#structfield.uri).
	/// The policy does not apply to the gRPC listener.
	///
	/// The default implementation returns `Ok(None)`.
	///
	/// # Examples
	///
	/// Return the policy that `on_server_start` registered, so request handlers
	/// that resolve it from DI share its concurrency permits:
	///
	/// ```rust,ignore
	/// use std::sync::Arc;
	/// use reinhardt::commands::{RunserverContext, RunserverHook};
	/// use reinhardt::server::RequestAdmission;
	///
	/// #[reinhardt::hook(on = runserver)]
	/// struct AdmissionHook;
	///
	/// #[async_trait]
	/// impl RunserverHook for AdmissionHook {
	///     async fn on_server_start(
	///         &self,
	///         ctx: &RunserverContext,
	///     ) -> Result<(), Box<dyn Error + Send + Sync>> {
	///         // `Protection` implements `RequestAdmission`.
	///         ctx.di_context.singleton_scope().set(Protection::new(64));
	///         Ok(())
	///     }
	///
	///     async fn request_admission(
	///         &self,
	///         ctx: &RunserverContext,
	///     ) -> Result<Option<Arc<dyn RequestAdmission>>, Box<dyn Error + Send + Sync>> {
	///         let protection = ctx
	///             .di_context
	///             .singleton_scope()
	///             .get::<Protection>()
	///             .ok_or("Protection is not registered")?;
	///         Ok(Some(protection))
	///     }
	/// }
	/// ```
	async fn request_admission(
		&self,
		ctx: &RunserverContext,
	) -> Result<Option<Arc<dyn RequestAdmission>>, Box<dyn Error + Send + Sync>> {
		let _ = ctx;
		Ok(None)
	}
}

/// Compile-time registration entry for auto-discovered runserver hooks.
///
/// Submitted via `inventory::submit!` by the `#[hook(on = runserver)]` macro.
/// You typically do not create this struct directly.
pub struct RunserverHookRegistration {
	/// Factory function that produces a boxed [`RunserverHook`].
	pub create: fn() -> Box<dyn RunserverHook>,

	/// Type name of the hook struct (for diagnostics).
	pub type_name: &'static str,
}

impl RunserverHookRegistration {
	/// Internal constructor used by the `#[hook]` macro.
	#[doc(hidden)]
	pub const fn __macro_new(
		create: fn() -> Box<dyn RunserverHook>,
		type_name: &'static str,
	) -> Self {
		Self { create, type_name }
	}
}

inventory::collect!(RunserverHookRegistration);

/// A collected runserver hook paired with its registration metadata.
pub(crate) struct CollectedRunserverHook {
	/// Instantiated hook implementation.
	pub hook: Box<dyn RunserverHook>,

	/// Type name of the hook struct (for diagnostics).
	pub type_name: &'static str,
}

/// Collect all registered runserver hooks from inventory, preserving
/// diagnostic identity for error reporting.
pub(crate) fn collect_hooks() -> Vec<CollectedRunserverHook> {
	inventory::iter::<RunserverHookRegistration>
		.into_iter()
		.map(|reg| CollectedRunserverHook {
			hook: (reg.create)(),
			type_name: reg.type_name,
		})
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::*;

	// -- Test hook implementations --

	struct PassingValidateHook;

	#[async_trait]
	impl RunserverHook for PassingValidateHook {
		async fn validate(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
			Ok(())
		}
	}

	struct FailingValidateHook {
		message: &'static str,
	}

	#[async_trait]
	impl RunserverHook for FailingValidateHook {
		async fn validate(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
			Err(self.message.into())
		}
	}

	struct StartHookWithSideEffect {
		flag: Arc<std::sync::atomic::AtomicBool>,
	}

	#[async_trait]
	impl RunserverHook for StartHookWithSideEffect {
		async fn on_server_start(
			&self,
			_ctx: &RunserverContext,
		) -> Result<(), Box<dyn Error + Send + Sync>> {
			self.flag.store(true, std::sync::atomic::Ordering::SeqCst);
			Ok(())
		}
	}

	struct FailingStartHook;

	#[async_trait]
	impl RunserverHook for FailingStartHook {
		async fn on_server_start(
			&self,
			_ctx: &RunserverContext,
		) -> Result<(), Box<dyn Error + Send + Sync>> {
			Err("startup failed".into())
		}
	}

	struct NoOpHook;

	#[async_trait]
	impl RunserverHook for NoOpHook {}

	struct ValidateAndStartHook {
		validate_flag: Arc<std::sync::atomic::AtomicBool>,
		start_flag: Arc<std::sync::atomic::AtomicBool>,
	}

	#[async_trait]
	impl RunserverHook for ValidateAndStartHook {
		async fn validate(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
			self.validate_flag
				.store(true, std::sync::atomic::Ordering::SeqCst);
			Ok(())
		}

		async fn on_server_start(
			&self,
			_ctx: &RunserverContext,
		) -> Result<(), Box<dyn Error + Send + Sync>> {
			self.start_flag
				.store(true, std::sync::atomic::Ordering::SeqCst);
			Ok(())
		}
	}

	// -- Helper --

	fn make_runserver_context() -> RunserverContext {
		RunserverContext {
			shutdown_coordinator: ShutdownCoordinator::new(std::time::Duration::from_secs(1)),
			di_context: Arc::new(
				reinhardt_di::InjectionContext::builder(Arc::new(
					reinhardt_di::SingletonScope::new(),
				))
				.build(),
			),
		}
	}

	// ========================================================================
	// Normal cases
	// ========================================================================

	#[rstest]
	#[tokio::test]
	async fn validate_succeeds_for_passing_hook() {
		let hook = PassingValidateHook;
		assert!(hook.validate().await.is_ok());
	}

	#[rstest]
	#[tokio::test]
	async fn on_server_start_sets_side_effect_flag() {
		// Arrange
		let flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
		let hook = StartHookWithSideEffect { flag: flag.clone() };
		let ctx = make_runserver_context();

		// Act
		let result = hook.on_server_start(&ctx).await;

		// Assert
		assert!(result.is_ok());
		assert!(flag.load(std::sync::atomic::Ordering::SeqCst));
	}

	#[rstest]
	#[tokio::test]
	async fn hook_implementing_both_phases_calls_both() {
		// Arrange
		let validate_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
		let start_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
		let hook = ValidateAndStartHook {
			validate_flag: validate_flag.clone(),
			start_flag: start_flag.clone(),
		};
		let ctx = make_runserver_context();

		// Act
		hook.validate().await.unwrap();
		hook.on_server_start(&ctx).await.unwrap();

		// Assert
		assert!(validate_flag.load(std::sync::atomic::Ordering::SeqCst));
		assert!(start_flag.load(std::sync::atomic::Ordering::SeqCst));
	}

	#[rstest]
	#[tokio::test]
	async fn context_provides_shutdown_coordinator() {
		// Arrange
		let ctx = make_runserver_context();

		// Act: subscribe to shutdown coordinator (should not panic)
		let _rx = ctx.shutdown_coordinator.subscribe();

		// Assert: subscription succeeded (no panic)
	}

	#[rstest]
	#[tokio::test]
	async fn context_provides_di_context() {
		// Arrange
		let ctx = make_runserver_context();

		// Assert: DI context is accessible
		let _ = ctx.di_context.clone();
	}

	// ========================================================================
	// Error / failure cases
	// ========================================================================

	#[rstest]
	#[tokio::test]
	async fn validate_returns_error_with_message() {
		// Arrange
		let hook = FailingValidateHook {
			message: "JWT_SECRET not configured",
		};

		// Act
		let result = hook.validate().await;

		// Assert
		assert!(result.is_err());
		assert_eq!(result.unwrap_err().to_string(), "JWT_SECRET not configured");
	}

	#[rstest]
	#[tokio::test]
	async fn on_server_start_returns_error() {
		// Arrange
		let hook = FailingStartHook;
		let ctx = make_runserver_context();

		// Act
		let result = hook.on_server_start(&ctx).await;

		// Assert
		assert!(result.is_err());
		assert_eq!(result.unwrap_err().to_string(), "startup failed");
	}

	// ========================================================================
	// Edge cases / default implementations
	// ========================================================================

	#[rstest]
	fn default_static_response_headers_is_empty() {
		// Arrange
		let hook = NoOpHook;

		// Act
		let headers = hook.static_response_headers();

		// Assert
		assert!(headers.is_empty());
	}

	#[rstest]
	#[tokio::test]
	async fn default_request_admission_returns_none() {
		// Arrange
		let hook = NoOpHook;
		let ctx = make_runserver_context();

		// Act
		let admission = hook.request_admission(&ctx).await.unwrap();

		// Assert
		assert!(admission.is_none());
	}

	#[rstest]
	#[tokio::test]
	async fn default_validate_returns_ok() {
		let hook = NoOpHook;
		assert!(hook.validate().await.is_ok());
	}

	#[rstest]
	#[tokio::test]
	async fn default_on_server_start_returns_ok() {
		// Arrange
		let hook = NoOpHook;
		let ctx = make_runserver_context();

		// Act & Assert
		assert!(hook.on_server_start(&ctx).await.is_ok());
	}

	#[rstest]
	fn collect_hooks_returns_vec_without_panic() {
		// Act: collect hooks from inventory
		let hooks = collect_hooks();

		// Assert: returns a Vec of CollectedRunserverHook
		for collected in &hooks {
			// type_name is preserved from registration
			assert!(!collected.type_name.is_empty());
		}
	}

	#[rstest]
	fn registration_macro_new_creates_valid_entry() {
		// Arrange & Act
		let reg = RunserverHookRegistration::__macro_new(|| Box::new(NoOpHook), "NoOpHook");

		// Assert
		assert_eq!(reg.type_name, "NoOpHook");
		let hook = (reg.create)();
		// Verify the created hook is usable (trait object is valid)
		let _ = hook;
	}

	#[rstest]
	#[tokio::test]
	async fn registration_factory_produces_working_hook() {
		// Arrange
		let reg = RunserverHookRegistration::__macro_new(
			|| Box::new(PassingValidateHook),
			"PassingValidateHook",
		);

		// Act
		let hook = (reg.create)();
		let result = hook.validate().await;

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	#[tokio::test]
	async fn multiple_hooks_sequential_validation() {
		// Arrange
		let hooks: Vec<Box<dyn RunserverHook>> = vec![
			Box::new(PassingValidateHook),
			Box::new(PassingValidateHook),
			Box::new(PassingValidateHook),
		];

		// Act & Assert: all validations pass
		for hook in &hooks {
			assert!(hook.validate().await.is_ok());
		}
	}

	#[rstest]
	#[tokio::test]
	async fn validation_stops_on_first_error() {
		// Arrange
		let hooks: Vec<Box<dyn RunserverHook>> = vec![
			Box::new(PassingValidateHook),
			Box::new(FailingValidateHook {
				message: "second hook fails",
			}),
			Box::new(PassingValidateHook),
		];

		// Act: simulate the run_server validation loop
		let mut first_error = None;
		for hook in &hooks {
			if let Err(e) = hook.validate().await {
				first_error = Some(e);
				break;
			}
		}

		// Assert: stopped at second hook
		assert!(first_error.is_some());
		assert_eq!(first_error.unwrap().to_string(), "second hook fails");
	}

	#[rstest]
	#[tokio::test]
	async fn on_server_start_not_called_when_validate_fails() {
		// Arrange
		let start_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
		let hook = FailingValidateHook {
			message: "config error",
		};
		let ctx = make_runserver_context();

		// Act: simulate run_server flow — validate first, skip on_server_start on error
		let validate_result = hook.validate().await;
		if validate_result.is_ok() {
			// This simulates the NoOpHook case for on_server_start
			let _ = NoOpHook.on_server_start(&ctx).await;
			start_flag.store(true, std::sync::atomic::Ordering::SeqCst);
		}

		// Assert: on_server_start was never called
		assert!(validate_result.is_err());
		assert!(!start_flag.load(std::sync::atomic::Ordering::SeqCst));
	}
}
