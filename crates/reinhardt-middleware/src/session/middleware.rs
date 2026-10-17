//! `SessionMiddleware`: cookie parsing, store wiring, and `Set-Cookie` writeback.

use async_trait::async_trait;
use reinhardt_http::{Handler, Middleware, MiddlewareDiRegistration, Request, Response, Result};
use std::any::TypeId;
use std::sync::Arc;
use std::time::SystemTime;

use super::config::SessionConfig;
use super::cookie::find_cookie_value;
use super::data::{PendingSession, SessionData};
use super::id::{ActiveSessionId, SessionCookieName, SessionId};
use super::store::{SessionStore, SessionStoreKey};

/// Session middleware
///
/// A request whose cookie names a valid stored session resumes that session,
/// and its expiry is extended. Any other request gets a fresh session that is
/// only stored once a handler saves it (for example through
/// `SessionAuthExt::login`, or `SessionStore::save` on an injected
/// `SessionData`), so requests that never save their session do not grow the
/// store.
///
/// The ID of such an unsaved session is still sent as the session cookie, so
/// values bound to it, such as CSRF tokens, stay valid on the client's next
/// request. The ID is signed with a random key held by the store and expires
/// after the session TTL, which lets the middleware accept it back without
/// storing it. Forged or expired IDs are replaced with a new one.
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
	/// Exposes the middleware-owned `Arc<SessionStore>` as DI singletons.
	///
	/// The raw `SessionStore` key is retained for `SessionData::inject`.
	/// The keyed `FactoryOutput<SessionStoreKey, Arc<SessionStore>>` key is
	/// used by handlers that request
	/// `#[inject] store: Depends<SessionStoreKey, Arc<SessionStore>>`.
	fn di_registrations(&self) -> Vec<MiddlewareDiRegistration> {
		let store = Arc::clone(&self.store);
		let keyed_store = Arc::new(reinhardt_di::FactoryOutput::<
			SessionStoreKey,
			Arc<SessionStore>,
		>::new(Arc::clone(&self.store)));
		vec![
			(
				TypeId::of::<SessionStore>(),
				store as Arc<dyn std::any::Any + Send + Sync>,
			),
			(
				TypeId::of::<reinhardt_di::FactoryOutput<SessionStoreKey, Arc<SessionStore>>>(),
				keyed_store as Arc<dyn std::any::Any + Send + Sync>,
			),
		]
	}

	async fn process(&self, request: Request, handler: Arc<dyn Handler>) -> Result<Response> {
		// Resume the valid stored session named by the cookie, extending its
		// expiry. Otherwise start a session that stays in the request
		// extensions until a handler saves it: storing it here would let every
		// request without a valid cookie allocate a store entry.
		let cookie_id = self.get_session_id(&request);
		let resumed = cookie_id
			.as_deref()
			.and_then(|id| self.store.get(id))
			.filter(|s| s.is_valid());
		let session_id = match resumed {
			Some(mut session) => {
				session.touch(self.config.ttl);
				let id = session.id.clone();
				self.store.save(session);
				id
			}
			None => {
				// Keep an unsaved session ID this store issued earlier so tokens
				// bound to it stay valid; never adopt an ID the client chose.
				let id = cookie_id
					.filter(|id| self.store.is_pending_id(id))
					.unwrap_or_else(|| {
						self.store
							.new_pending_id(SystemTime::now() + self.config.ttl)
					});
				let mut session = SessionData::new(self.config.ttl);
				session.id = id.clone();
				request.extensions.insert(PendingSession(session));
				id
			}
		};

		// Inject session ID and cookie name into request extensions
		// so downstream handlers and Injectable impls can access them
		request
			.extensions
			.insert(SessionId::new(session_id.clone()));
		request
			.extensions
			.insert(SessionCookieName::new(self.config.cookie_name.clone()));
		// Shared, mutable holder so handlers that rotate the session ID
		// (`SessionData::regenerate_id`) keep `Set-Cookie` in sync. See #3827.
		let active_id = ActiveSessionId::new(session_id);
		request.extensions.insert(active_id.clone());

		// Call the handler
		// Convert errors to responses so post-processing (e.g., security headers)
		// always runs, even when invoked outside MiddlewareChain. (#3244)
		let mut response = match handler.handle(request).await {
			Ok(resp) => resp,
			Err(e) => Response::from(e),
		};

		// Append Set-Cookie header (use append to preserve existing Set-Cookie headers).
		// Read the final session ID from the shared holder rather than a local
		// copy, since handlers may have rotated the ID via
		// `SessionData::regenerate_id`. See #3827.
		// Advertise a stored session or an unsaved one this store issued. A
		// rotated ID that no handler saved names nothing.
		let final_id = active_id.get();
		if !self.store.contains_valid(&final_id) && !self.store.is_pending_id(&final_id) {
			return Ok(response);
		}
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
	use crate::session::data::USER_ID_SESSION_KEY;
	use crate::session::test_support::{SessionSavingHandler, session_di_scope};
	use bytes::Bytes;
	use hyper::{HeaderMap, Method, Version};
	use reinhardt_http::{AuthState, Handler, IsActive, IsAdmin, IsAuthenticated, Request};
	use rstest::rstest;
	use std::sync::Mutex;
	use std::time::{Duration, SystemTime};

	#[derive(Default)]
	struct CapturedAuth {
		auth_state: Option<AuthState>,
		user_id: Option<String>,
		is_authenticated: Option<IsAuthenticated>,
		is_admin: Option<IsAdmin>,
		is_active: Option<IsActive>,
	}

	struct CaptureAuthHandler {
		captured: Arc<Mutex<CapturedAuth>>,
	}

	#[async_trait]
	impl Handler for CaptureAuthHandler {
		async fn handle(&self, request: Request) -> Result<Response> {
			*self.captured.lock().unwrap() = CapturedAuth {
				auth_state: request.extensions.get::<AuthState>(),
				user_id: request.extensions.get::<String>(),
				is_authenticated: request.extensions.get::<IsAuthenticated>(),
				is_admin: request.extensions.get::<IsAdmin>(),
				is_active: request.extensions.get::<IsActive>(),
			};
			Ok(Response::ok())
		}
	}

	/// Returns a `SessionMiddleware` with a fixed cookie name and TTL for
	/// deterministic tests.
	fn make_middleware() -> SessionMiddleware {
		SessionMiddleware::new(SessionConfig::new(
			"sessionid".to_string(),
			Duration::from_secs(3600),
		))
	}

	fn request_with_cookie(session_id: &str) -> Request {
		let mut headers = HeaderMap::new();
		headers.insert(
			hyper::header::COOKIE,
			hyper::header::HeaderValue::from_str(&format!("sessionid={}", session_id)).unwrap(),
		);
		Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(headers)
			.body(Bytes::new())
			.build()
			.unwrap()
	}

	fn request_without_cookie() -> Request {
		Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap()
	}

	fn capture_handler() -> (Arc<Mutex<CapturedAuth>>, Arc<dyn Handler>) {
		let captured = Arc::new(Mutex::new(CapturedAuth::default()));
		let handler = Arc::new(CaptureAuthHandler {
			captured: Arc::clone(&captured),
		});
		(captured, handler)
	}

	/// What `InjectingHandler` does with the session it injects.
	#[derive(Clone, Copy)]
	enum SessionUse {
		/// Read the session without saving it.
		Read,
		/// Log in through `SessionAuthExt::login`, which rotates the ID and
		/// saves the session.
		Login,
		/// Log out through `SessionAuthExt::logout`, which rotates the ID and
		/// stores nothing when no other key remains.
		Logout,
	}

	/// Handler that resolves `SessionData` through its `Injectable`
	/// implementation, as an `#[inject] session: SessionData` parameter does.
	struct InjectingHandler {
		scope: Arc<reinhardt_di::SingletonScope>,
		session_use: SessionUse,
	}

	#[async_trait]
	impl Handler for InjectingHandler {
		async fn handle(&self, request: Request) -> Result<Response> {
			use crate::session::SessionAuthExt;
			use reinhardt_di::{Injectable, InjectionContext};

			let ctx = InjectionContext::builder(Arc::clone(&self.scope)).build();
			ctx.set_request(request);
			let mut session = SessionData::inject(&ctx)
				.await
				.expect("every request behind SessionMiddleware must have a session");
			match self.session_use {
				SessionUse::Read => assert!(session.data.is_empty()),
				SessionUse::Login => {
					let store = ctx
						.get_singleton::<SessionStore>()
						.expect("SessionMiddleware must register its store for DI");
					session.login(&store, 42_i64).unwrap();
				}
				SessionUse::Logout => {
					let store = ctx
						.get_singleton::<SessionStore>()
						.expect("SessionMiddleware must register its store for DI");
					session.logout(&store);
				}
			}
			Ok(Response::ok())
		}
	}

	fn injecting_handler(
		middleware: &SessionMiddleware,
		session_use: SessionUse,
	) -> Arc<dyn Handler> {
		Arc::new(InjectingHandler {
			scope: session_di_scope(middleware),
			session_use,
		})
	}

	/// Value of the `name` cookie set by `response`, if any.
	fn set_cookie_value(response: &Response, name: &str) -> Option<String> {
		let prefix = format!("{name}=");
		response
			.headers
			.get_all(hyper::header::SET_COOKIE)
			.iter()
			.filter_map(|value| value.to_str().ok())
			.find_map(|cookie| cookie.split(';').next()?.strip_prefix(prefix.as_str()))
			.map(str::to_owned)
	}

	fn session_cookie_id(response: &Response) -> Option<String> {
		set_cookie_value(response, "sessionid")
	}

	/// Regression test for GHSA-wg7w-cm25-56qp: a client that omits its
	/// session cookie, or sends one naming no stored session, must not make
	/// the middleware store a session per request.
	#[rstest]
	#[case::without_cookie(None)]
	#[case::with_unknown_cookie(Some("not-a-stored-session"))]
	#[tokio::test]
	async fn unused_sessions_are_not_stored(#[case] cookie: Option<&str>) {
		// Arrange
		let middleware = make_middleware();
		let (_, handler) = capture_handler();

		for _ in 0..100 {
			let request = match cookie {
				Some(session_id) => request_with_cookie(session_id),
				None => request_without_cookie(),
			};

			// Act
			let response = middleware
				.process(request, Arc::clone(&handler))
				.await
				.unwrap();

			// Assert: the client gets an unsaved session ID issued by the store,
			// never the ID it sent.
			let session_id = session_cookie_id(&response).expect("a session ID must be issued");
			assert!(middleware.store().is_pending_id(&session_id));
			assert_ne!(Some(session_id.as_str()), cookie);
		}
		assert_eq!(middleware.store().len(), 0);
	}

	/// The unsaved session ID a client returns stays its session ID, so
	/// values bound to it remain valid, and still nothing is stored.
	#[rstest]
	#[tokio::test]
	async fn returned_unsaved_session_id_is_kept_without_storing() {
		// Arrange
		let middleware = make_middleware();
		let (_, handler) = capture_handler();
		let first = middleware
			.process(request_without_cookie(), Arc::clone(&handler))
			.await
			.unwrap();
		let issued = session_cookie_id(&first).unwrap();

		for _ in 0..10 {
			// Act
			let response = middleware
				.process(request_with_cookie(&issued), Arc::clone(&handler))
				.await
				.unwrap();

			// Assert
			assert_eq!(session_cookie_id(&response), Some(issued.clone()));
		}
		assert_eq!(middleware.store().len(), 0);
	}

	#[rstest]
	#[case::expired(true)]
	#[case::from_another_store(false)]
	#[tokio::test]
	async fn unusable_unsaved_session_id_is_replaced(#[case] expired: bool) {
		// Arrange
		let middleware = make_middleware();
		let unusable = if expired {
			middleware
				.store()
				.new_pending_id(SystemTime::now() - Duration::from_secs(1))
		} else {
			SessionStore::new().new_pending_id(SystemTime::now() + Duration::from_secs(60))
		};
		let (_, handler) = capture_handler();

		// Act
		let response = middleware
			.process(request_with_cookie(&unusable), handler)
			.await
			.unwrap();

		// Assert
		let session_id = session_cookie_id(&response).expect("a session ID must be issued");
		assert_ne!(session_id, unusable);
		assert!(middleware.store().is_pending_id(&session_id));
		assert_eq!(middleware.store().len(), 0);
	}

	#[rstest]
	#[tokio::test]
	async fn expired_session_cookie_does_not_store_a_replacement() {
		// Arrange
		let middleware = make_middleware();
		let mut expired = SessionData::new(Duration::from_secs(3600));
		expired.expires_at = SystemTime::now() - Duration::from_secs(1);
		let expired_id = expired.id.clone();
		middleware.store().save(expired);
		let (_, handler) = capture_handler();

		// Act
		let response = middleware
			.process(request_with_cookie(&expired_id), handler)
			.await
			.unwrap();

		// Assert: only the expired entry remains until cleanup removes it.
		let session_id = session_cookie_id(&response).expect("a session ID must be issued");
		assert_ne!(session_id, expired_id);
		assert_eq!(middleware.store().len(), 1);
		assert!(!middleware.store().contains_valid(&expired_id));
	}

	#[rstest]
	#[tokio::test]
	async fn injected_session_is_not_stored_unless_saved() {
		// Arrange
		let middleware = make_middleware();
		let handler = injecting_handler(&middleware, SessionUse::Read);

		// Act
		let response = middleware
			.process(request_without_cookie(), handler)
			.await
			.unwrap();

		// Assert
		let session_id = session_cookie_id(&response).expect("a session ID must be issued");
		assert!(middleware.store().is_pending_id(&session_id));
		assert_eq!(middleware.store().len(), 0);
	}

	/// A visitor without a stored session can submit a CSRF-protected form:
	/// the unsaved session ID the CSRF token is bound to comes back unchanged.
	#[rstest]
	#[tokio::test]
	async fn csrf_form_works_for_a_visitor_without_a_stored_session() {
		use crate::csrf::{CsrfMiddleware, CsrfMiddlewareConfig};
		use reinhardt_http::MiddlewareChain;

		// Arrange: SessionMiddleware wrapping CsrfMiddleware wrapping the handler.
		let middleware = make_middleware();
		let csrf_config = CsrfMiddlewareConfig {
			check_referer_header: false,
			..Default::default()
		};
		let csrf: Arc<dyn Middleware> = Arc::new(CsrfMiddleware::with_config(csrf_config));
		let (_, handler) = capture_handler();
		let chain: Arc<dyn Handler> = Arc::new(MiddlewareChain::new(handler).with_middleware(csrf));
		let first = middleware
			.process(request_without_cookie(), Arc::clone(&chain))
			.await
			.unwrap();
		let session_id = session_cookie_id(&first).unwrap();
		let form = middleware
			.process(request_with_cookie(&session_id), Arc::clone(&chain))
			.await
			.unwrap();
		let token = set_cookie_value(&form, "csrftoken").expect("CSRF token cookie");
		let mut headers = HeaderMap::new();
		headers.insert(
			hyper::header::COOKIE,
			format!("sessionid={session_id}; csrftoken={token}")
				.parse()
				.unwrap(),
		);
		headers.insert("X-CSRFToken", token.parse().unwrap());
		let submit = Request::builder()
			.method(Method::POST)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(headers)
			.body(Bytes::new())
			.build()
			.unwrap();

		// Act
		let response = middleware.process(submit, chain).await.unwrap();

		// Assert
		assert_eq!(response.status, hyper::StatusCode::OK);
		assert_eq!(session_cookie_id(&response), Some(session_id));
		assert_eq!(middleware.store().len(), 0);
	}

	#[rstest]
	#[tokio::test]
	async fn saved_session_is_stored_and_advertised() {
		// Arrange
		let middleware = make_middleware();
		let handler: Arc<dyn Handler> = Arc::new(SessionSavingHandler::new(&middleware));

		// Act
		let response = middleware
			.process(request_without_cookie(), handler)
			.await
			.unwrap();

		// Assert
		let session_id = session_cookie_id(&response).expect("a saved session must be advertised");
		assert_eq!(middleware.store().len(), 1);
		assert!(middleware.store().contains_valid(&session_id));
	}

	/// Login on a request without a cookie goes through the request-scoped
	/// session: the rotated session is the one stored and advertised.
	#[rstest]
	#[tokio::test]
	async fn login_without_cookie_advertises_the_rotated_session() {
		// Arrange
		let middleware = make_middleware();
		let handler = injecting_handler(&middleware, SessionUse::Login);

		// Act
		let response = middleware
			.process(request_without_cookie(), handler)
			.await
			.unwrap();

		// Assert
		let session_id = session_cookie_id(&response).expect("login must advertise its session");
		let stored = middleware
			.store()
			.get(&session_id)
			.expect("Set-Cookie must name the stored session");
		assert_eq!(stored.get::<i64>(USER_ID_SESSION_KEY), Some(42));
		assert_eq!(middleware.store().len(), 1);
	}

	#[rstest]
	#[tokio::test]
	async fn anonymous_logout_advertises_an_unsaved_session() {
		// Arrange
		let middleware = make_middleware();

		// Act: a client without credentials logs out repeatedly.
		let mut session_ids = Vec::new();
		for _ in 0..10 {
			let handler = injecting_handler(&middleware, SessionUse::Logout);
			let response = middleware
				.process(request_without_cookie(), handler)
				.await
				.unwrap();
			session_ids.push(session_cookie_id(&response));
		}

		// Assert: every response advertises a signed unsaved ID, so CSRF
		// tokens bound to it keep working, and the store stays empty.
		for session_id in session_ids {
			let session_id = session_id.expect("logout must advertise the rotated session");
			assert!(middleware.store().is_pending_id(&session_id));
		}
		assert!(middleware.store().is_empty());
	}

	#[rstest]
	#[tokio::test]
	async fn resumed_session_is_extended_and_advertised() {
		// Arrange
		let middleware = make_middleware();
		let session = SessionData::new(Duration::from_secs(60));
		let session_id = session.id.clone();
		let original_expiry = session.expires_at;
		middleware.store().save(session);
		let (_, handler) = capture_handler();

		// Act
		let response = middleware
			.process(request_with_cookie(&session_id), handler)
			.await
			.unwrap();

		// Assert
		assert_eq!(session_cookie_id(&response), Some(session_id.clone()));
		assert_eq!(middleware.store().len(), 1);
		let resumed = middleware.store().get(&session_id).unwrap();
		assert!(resumed.expires_at > original_expiry);
	}

	/// Sessions that handlers do save stay within the store's hard limit.
	#[rstest]
	#[tokio::test]
	async fn saved_sessions_stay_within_the_store_limit() {
		// Arrange
		let store = Arc::new(SessionStore::new());
		store.set_max_sessions(std::num::NonZeroUsize::new(5).unwrap());
		let middleware = SessionMiddleware::from_arc(
			SessionConfig::new("sessionid".to_string(), Duration::from_secs(3600)),
			Arc::clone(&store),
		);
		let handler: Arc<dyn Handler> = Arc::new(SessionSavingHandler::new(&middleware));

		for _ in 0..50 {
			// Act
			let response = middleware
				.process(request_without_cookie(), Arc::clone(&handler))
				.await
				.unwrap();

			// Assert: the newest session is kept and advertised.
			let session_id =
				session_cookie_id(&response).expect("a saved session must be advertised");
			assert!(store.contains_valid(&session_id));
			assert!(store.len() <= 5);
		}
	}

	#[rstest]
	fn test_session_middleware_di_registrations_returns_store() {
		// Arrange: build a middleware (which internally creates an Arc<SessionStore>).
		let middleware = make_middleware();
		let store_arc = middleware.store_arc();

		// Act: ask the middleware for its DI registrations.
		let registrations = middleware.di_registrations();

		assert_eq!(registrations.len(), 2);
		let (type_id, value) = &registrations[0];
		assert_eq!(*type_id, TypeId::of::<SessionStore>());
		let downcast = value
			.clone()
			.downcast::<SessionStore>()
			.expect("registered Arc must downcast to SessionStore");
		assert!(
			Arc::ptr_eq(&downcast, &store_arc),
			"middleware DI registration must expose the same Arc<SessionStore> the middleware writes to"
		);

		let (type_id, value) = &registrations[1];
		assert_eq!(
			*type_id,
			TypeId::of::<reinhardt_di::FactoryOutput<SessionStoreKey, Arc<SessionStore>>>()
		);
		let downcast = value
			.clone()
			.downcast::<reinhardt_di::FactoryOutput<SessionStoreKey, Arc<SessionStore>>>()
			.expect("registered Arc must downcast to keyed SessionStore factory output");
		assert!(
			Arc::ptr_eq(downcast.as_ref(), &store_arc),
			"keyed DI registration must expose the same Arc<SessionStore> the middleware writes to"
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

		// Assert: the scope now resolves `SessionStore` (keyed by its own
		// TypeId after the #4437 migration) to the same Arc the middleware
		// uses, mirroring what `SessionData::inject` does via
		// `get_singleton::<SessionStore>()`, so handlers see the same store.
		let resolved = scope
			.get::<SessionStore>()
			.expect("SingletonScope must resolve SessionStore after applying middleware DI");
		assert!(
			Arc::ptr_eq(&resolved, &store_arc),
			"resolved Arc<SessionStore> must point at the same allocation the middleware owns"
		);
	}

	#[tokio::test]
	async fn session_middleware_does_not_authenticate_session_user_id() {
		// Arrange: a valid session can contain an identity, but it cannot prove the
		// account remains active or authorized.
		let middleware = make_middleware();
		let store = middleware.store_arc();
		let mut session = SessionData::new(Duration::from_secs(3600));
		session
			.set(USER_ID_SESSION_KEY.to_string(), 42_i64)
			.unwrap();
		let session_id = session.id.clone();
		store.save(session);

		let (captured, handler) = capture_handler();
		middleware
			.process(request_with_cookie(&session_id), handler)
			.await
			.unwrap();

		// Assert: an authentication middleware must validate the current account.
		let captured = captured.lock().unwrap();
		assert!(captured.auth_state.is_none());
		assert!(captured.user_id.is_none());
		assert!(captured.is_authenticated.is_none());
		assert!(captured.is_admin.is_none());
		assert!(captured.is_active.is_none());
	}

	#[tokio::test]
	async fn session_middleware_preserves_existing_auth_state() {
		let middleware = make_middleware();
		let request = request_without_cookie();
		request
			.extensions
			.insert(AuthState::authenticated("jwt-user", true, true));
		let (captured, handler) = capture_handler();

		middleware.process(request, handler).await.unwrap();

		let captured = captured.lock().unwrap();
		let auth_state = captured
			.auth_state
			.as_ref()
			.expect("pre-existing AuthState must remain visible");
		assert_eq!(auth_state.user_id(), "jwt-user");
		assert!(auth_state.is_authenticated());
		assert!(auth_state.is_admin());
		assert!(auth_state.is_active());
		assert!(captured.is_authenticated.is_none());
		assert!(captured.is_admin.is_none());
		assert!(captured.is_active.is_none());
	}

	/// End-to-end injection test: drives the same path a handler with
	/// `#[inject] session: SessionData` would take. Catches `TypeId` /
	/// shape regressions in `di_registrations` that `SingletonScope::get`-only
	/// tests would miss, by going through `InjectionContext` and the real
	/// `Injectable for SessionData` implementation. See PR #4435 Copilot review.
	#[tokio::test]
	async fn test_session_data_inject_resolves_via_middleware_di_registrations() {
		use crate::session::data::SessionData;
		use bytes::Bytes;
		use hyper::{Method, Version};
		use reinhardt_di::{Injectable, InjectionContext, SingletonScope};
		use reinhardt_http::Request;

		// Arrange: middleware contributes its Arc<SessionStore> via DI; pre-seed
		// the store with a valid session so the inject path can load it.
		let middleware = make_middleware();
		let store_arc = middleware.store_arc();
		let mut seeded = SessionData::new(Duration::from_secs(3600));
		seeded
			.set("user_id".to_string(), "alice".to_string())
			.unwrap();
		let seeded_id = seeded.id.clone();
		store_arc.save(seeded.clone());

		let scope = SingletonScope::new();
		let mut list = reinhardt_di::DiRegistrationList::new();
		for (type_id, value) in middleware.di_registrations() {
			list.register_arc_any(type_id, value);
		}
		list.apply_to(&scope);

		// Build a request that carries the SessionId extension the middleware
		// would normally inject during `process`. This bypasses Cookie parsing
		// but exercises the exact branch `SessionData::inject` takes when
		// `SessionMiddleware` is upstream.
		let request = Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.body(Bytes::new())
			.build()
			.unwrap();
		request.extensions.insert(SessionId::new(seeded_id.clone()));

		// `SessionData::inject` reads the request from the per-request scope
		// via `ctx.get_request::<Request>()`, so the request must be stored
		// with `set_request` (request scope) rather than the builder's
		// `with_request` (which populates the HTTP-request slot accessed by
		// `get_http_request`).
		let ctx = InjectionContext::builder(Arc::new(scope)).build();
		ctx.set_request(request);

		// Act: resolve `SessionData` through the real `#[inject]` code path.
		let resolved = SessionData::inject(&ctx)
			.await
			.expect("SessionData::inject must succeed when middleware DI is registered");

		// Assert: the resolved session is the one the store holds.
		assert_eq!(resolved.id, seeded_id);
		assert_eq!(resolved.get::<String>("user_id").as_deref(), Some("alice"));
	}
}
