//! `SessionMiddleware`: cookie parsing, store wiring, and `Set-Cookie` writeback.

use async_trait::async_trait;
#[allow(deprecated)]
use reinhardt_conf::Settings;
use reinhardt_http::{Handler, Middleware, MiddlewareDiRegistration, Request, Response, Result};
use std::any::TypeId;
use std::sync::Arc;

use super::config::SessionConfig;
use super::cookie::find_cookie_value;
use super::data::SessionData;
use super::id::{ActiveSessionId, SessionCookieName, SessionId};
use super::store::SessionStore;

/// Session middleware
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use std::time::Duration;
/// use reinhardt_middleware::session::{SessionMiddleware, SessionConfig};
/// use reinhardt_http::{Handler, Middleware, Request, Response};
/// use hyper::{StatusCode, Method, Version, HeaderMap};
/// use bytes::Bytes;
///
/// struct TestHandler;
///
/// #[async_trait::async_trait]
/// impl Handler for TestHandler {
///     async fn handle(&self, _request: Request) -> reinhardt_core::exception::Result<Response> {
///         Ok(Response::new(StatusCode::OK).with_body(Bytes::from("OK")))
///     }
/// }
///
/// # tokio_test::block_on(async {
/// let config = SessionConfig::new("sessionid".to_string(), Duration::from_secs(3600));
/// let middleware = SessionMiddleware::new(config);
/// let handler = Arc::new(TestHandler);
///
/// let request = Request::builder()
///     .method(Method::GET)
///     .uri("/api/data")
///     .version(Version::HTTP_11)
///     .headers(HeaderMap::new())
///     .body(Bytes::new())
///     .build()
///     .unwrap();
///
/// let response = middleware.process(request, handler).await.unwrap();
/// assert_eq!(response.status, StatusCode::OK);
/// # });
/// ```
pub struct SessionMiddleware {
	config: SessionConfig,
	store: Arc<SessionStore>,
}

impl SessionMiddleware {
	/// Create a new session middleware
	///
	/// # Examples
	///
	/// ```
	/// use std::time::Duration;
	/// use reinhardt_middleware::session::{SessionMiddleware, SessionConfig};
	///
	/// let config = SessionConfig::new("sessionid".to_string(), Duration::from_secs(3600));
	/// let middleware = SessionMiddleware::new(config);
	/// ```
	pub fn new(config: SessionConfig) -> Self {
		Self {
			config,
			store: Arc::new(SessionStore::new()),
		}
	}

	/// Create a `SessionMiddleware` from application `Settings`
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_conf::Settings;
	/// use reinhardt_middleware::session::SessionMiddleware;
	///
	/// #[allow(deprecated)]
	/// let settings = Settings::default();
	/// #[allow(deprecated)]
	/// let middleware = SessionMiddleware::from_settings(&settings);
	/// ```
	#[allow(deprecated)] // Settings is deprecated in favor of composable fragments
	pub fn from_settings(settings: &Settings) -> Self {
		Self::new(SessionConfig::from_settings(settings))
	}

	/// Create with default configuration
	pub fn with_defaults() -> Self {
		Self::new(SessionConfig::default())
	}

	/// Create from an existing Arc-wrapped session store
	///
	/// This is provided for cases where you already have an `Arc<SessionStore>`.
	/// In most cases, you should use `new()` instead, which creates the store internally.
	pub fn from_arc(config: SessionConfig, store: Arc<SessionStore>) -> Self {
		Self { config, store }
	}

	/// Get a reference to the session store
	///
	/// # Examples
	///
	/// ```
	/// use std::time::Duration;
	/// use reinhardt_middleware::session::{SessionMiddleware, SessionConfig};
	///
	/// let middleware = SessionMiddleware::new(
	///     SessionConfig::new("sessionid".to_string(), Duration::from_secs(3600))
	/// );
	///
	/// // Access the store
	/// let store = middleware.store();
	/// assert_eq!(store.len(), 0);
	/// ```
	pub fn store(&self) -> &SessionStore {
		&self.store
	}

	/// Get a cloned Arc of the store (for cases where you need ownership)
	///
	/// In most cases, you should use `store()` instead to get a reference.
	pub fn store_arc(&self) -> Arc<SessionStore> {
		Arc::clone(&self.store)
	}

	/// Get session ID from request.
	///
	/// Delegates to the shared `find_cookie_value` helper so this stays in
	/// lock-step with the `Injectable` DI path that also parses session cookies.
	fn get_session_id(&self, request: &Request) -> Option<String> {
		find_cookie_value(request, &self.config.cookie_name)
	}

	/// Build Set-Cookie header
	fn build_cookie_header(&self, session_id: &str) -> String {
		let mut parts = vec![format!("{}={}", self.config.cookie_name, session_id)];

		parts.push(format!("Path={}", self.config.path));

		if let Some(domain) = &self.config.domain {
			parts.push(format!("Domain={}", domain));
		}

		if self.config.http_only {
			parts.push("HttpOnly".to_string());
		}

		if self.config.secure {
			parts.push("Secure".to_string());
		}

		if let Some(same_site) = &self.config.same_site {
			parts.push(format!("SameSite={}", same_site));
		}

		parts.push(format!("Max-Age={}", self.config.ttl.as_secs()));

		parts.join("; ")
	}
}

impl Default for SessionMiddleware {
	fn default() -> Self {
		Self::with_defaults()
	}
}

#[async_trait]
impl Middleware for SessionMiddleware {
	/// Exposes the middleware-owned `Arc<SessionStore>` as a DI singleton.
	///
	/// Registered under `TypeId::of::<Arc<SessionStore>>()` to match the lookup
	/// performed by `SessionData::inject` (`get_singleton::<Arc<SessionStore>>()`),
	/// which downcasts to `Arc<Arc<SessionStore>>`. The outer `Arc` is the
	/// `dyn Any` envelope owned by `SingletonScope`; the inner `Arc<SessionStore>`
	/// is the value handlers actually receive. See #4426.
	fn di_registrations(&self) -> Vec<MiddlewareDiRegistration> {
		vec![(
			TypeId::of::<Arc<SessionStore>>(),
			Arc::new(Arc::clone(&self.store)) as Arc<dyn std::any::Any + Send + Sync>,
		)]
	}

	async fn process(&self, request: Request, handler: Arc<dyn Handler>) -> Result<Response> {
		// Get or generate session ID
		let session_id = self.get_session_id(&request);
		let mut session = if let Some(id) = session_id.clone() {
			self.store
				.get(&id)
				.filter(|s| s.is_valid())
				.unwrap_or_else(|| SessionData::new(self.config.ttl))
		} else {
			SessionData::new(self.config.ttl)
		};

		// Touch the session
		session.touch(self.config.ttl);

		// Save the session
		self.store.save(session.clone());

		// Inject session ID and cookie name into request extensions
		// so downstream handlers and Injectable impls can access them
		request
			.extensions
			.insert(SessionId::new(session.id.clone()));
		request
			.extensions
			.insert(SessionCookieName::new(self.config.cookie_name.clone()));
		// Shared, mutable holder so handlers that rotate the session ID
		// (`SessionData::regenerate_id`) keep `Set-Cookie` in sync. See #3827.
		let active_id = ActiveSessionId::new(session.id.clone());
		request.extensions.insert(active_id.clone());

		// Call the handler
		// Convert errors to responses so post-processing (e.g., security headers)
		// always runs, even when invoked outside MiddlewareChain. (#3244)
		let mut response = match handler.handle(request).await {
			Ok(resp) => resp,
			Err(e) => Response::from(e),
		};

		// Append Set-Cookie header (use append to preserve existing Set-Cookie headers).
		// Read the final session ID from the shared holder rather than the
		// local `session` clone, since handlers may have rotated the ID via
		// `SessionData::regenerate_id`. See #3827.
		let final_id = active_id.get();
		let cookie = self.build_cookie_header(&final_id);
		response.headers.append(
			hyper::header::SET_COOKIE,
			hyper::header::HeaderValue::from_str(&cookie).map_err(|e| {
				reinhardt_core::exception::Error::Internal(format!(
					"Failed to create cookie header: {}",
					e
				))
			})?,
		);

		Ok(response)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;
	use std::time::Duration;

	/// Returns a `SessionMiddleware` with a fixed cookie name and TTL for
	/// deterministic tests.
	fn make_middleware() -> SessionMiddleware {
		SessionMiddleware::new(SessionConfig::new(
			"sessionid".to_string(),
			Duration::from_secs(3600),
		))
	}

	#[rstest]
	fn test_session_middleware_di_registrations_returns_store() {
		// Arrange: build a middleware (which internally creates an Arc<SessionStore>).
		let middleware = make_middleware();
		let store_arc = middleware.store_arc();

		// Act: ask the middleware for its DI registrations.
		let registrations = middleware.di_registrations();

		// Assert: exactly one entry, keyed by Arc<SessionStore>'s TypeId to
		// match `SessionData::inject`'s `get_singleton::<Arc<SessionStore>>()`
		// lookup, pointing at the same underlying allocation as the middleware's
		// own store handle.
		assert_eq!(registrations.len(), 1);
		let (type_id, value) = &registrations[0];
		assert_eq!(*type_id, TypeId::of::<Arc<SessionStore>>());
		let downcast = value
			.clone()
			.downcast::<Arc<SessionStore>>()
			.expect("registered Arc must downcast to Arc<SessionStore>");
		assert!(
			Arc::ptr_eq(&*downcast, &store_arc),
			"middleware DI registration must expose the same Arc<SessionStore> the middleware writes to"
		);
	}

	#[rstest]
	fn test_session_middleware_di_registrations_apply_to_singleton_scope() {
		// Arrange: middleware + an empty SingletonScope.
		let middleware = make_middleware();
		let store_arc = middleware.store_arc();
		let scope = reinhardt_di::SingletonScope::new();

		// Act: feed the middleware's registrations into a DiRegistrationList and
		// apply it to the scope (mirroring what `with_middleware` does internally).
		let mut list = reinhardt_di::DiRegistrationList::new();
		for (type_id, value) in middleware.di_registrations() {
			list.register_arc_any(type_id, value);
		}
		list.apply_to(&scope);

		// Assert: the scope now resolves `Arc<SessionStore>` to the same Arc the
		// middleware uses, mirroring what `SessionData::inject` does via
		// `get_singleton::<Arc<SessionStore>>()`, so handlers see the same store.
		let resolved = scope
			.get::<Arc<SessionStore>>()
			.expect("SingletonScope must resolve Arc<SessionStore> after applying middleware DI");
		assert!(
			Arc::ptr_eq(&*resolved, &store_arc),
			"resolved Arc<SessionStore> must point at the same allocation the middleware owns"
		);
	}
}
