//! Integration tests for the `FromRequest` (no-`#[inject]`) path of
//! [`SessionValue`], [`OptionalSessionValue`], and [`SessionValueNamed`].
//!
//! Companion to `tests/session_value_extractor.rs` (which exercises the
//! `Injectable` path). These tests confirm that the typed session
//! extractors compose with the handler macro's auto-extraction
//! whitelist — i.e. they work without the `#[inject]` attribute, just
//! like `Path(...)` / `Json(...)`. See issue #4446.

#[cfg(feature = "sessions")]
mod tests {
	use std::sync::Arc;
	use std::time::Duration;

	use bytes::Bytes;
	use hyper::{HeaderMap, Method, Version};
	use reinhardt_di::params::{ParamContext, ParamError, extract::FromRequest};
	use reinhardt_di::{InjectionContext, SingletonScope};
	use reinhardt_http::Request;
	use reinhardt_middleware::session::{
		OptionalSessionValue, OptionalSessionValueNamed, SessionData, SessionKey, SessionStore,
		SessionValue, SessionValueNamed, USER_ID_SESSION_KEY,
	};

	/// Build a request whose extensions carry the active `Arc<InjectionContext>`
	/// (so `req.get_di_context::<InjectionContext>()` returns it) and an
	/// optional session cookie that `SessionData::inject` will resolve from
	/// the singleton-scoped `Arc<SessionStore>`.
	fn build_request_with_session(
		store: Arc<SessionStore>,
		session_cookie: Option<&str>,
	) -> Request {
		// `reinhardt_http::Request` does not implement `Clone`, and
		// `SessionData::inject` reads the request via
		// `ctx.get_request::<Request>()` while `req.get_di_context::<…>()`
		// reads from the live request's extensions. Build two structurally
		// identical requests — one to seed the DI context's request scope,
		// one returned to the caller (and stamped with the DI context).
		let build = || -> Request {
			let mut headers = HeaderMap::new();
			if let Some(cookie) = session_cookie {
				headers.insert(
					hyper::header::COOKIE,
					hyper::header::HeaderValue::from_str(&format!("sessionid={cookie}")).unwrap(),
				);
			}
			Request::builder()
				.method(Method::GET)
				.uri("/test")
				.version(Version::HTTP_11)
				.headers(headers)
				.body(Bytes::new())
				.build()
				.unwrap()
		};

		let request_for_ctx = build();
		let request = build();

		let singleton: Arc<SingletonScope> = Arc::new(SingletonScope::new());
		singleton.set::<Arc<SessionStore>>(store);

		// Mirror what `RouterMiddleware` does in production: seed the
		// request scope so `SessionData::inject` can pull the live request.
		let ctx = InjectionContext::builder(singleton).build();
		ctx.set_request(request_for_ctx);

		// Stash the `Arc<InjectionContext>` on the request extensions where
		// `Request::get_di_context` looks for it. Reach extensions
		// directly to avoid the `&mut self` requirement of
		// `Request::set_di_context`.
		request.extensions.insert(Arc::new(ctx));
		request
	}

	struct TenantIdKey;
	impl SessionKey for TenantIdKey {
		const KEY: &'static str = "tenant_id";
	}

	#[tokio::test]
	async fn session_value_from_request_returns_stored_value() {
		// Arrange
		let store = Arc::new(SessionStore::new());
		let mut session = SessionData::new(Duration::from_secs(3600));
		session.set(USER_ID_SESSION_KEY.to_string(), 7i64).unwrap();
		let session_id = session.id.clone();
		store.save(session);

		let request = build_request_with_session(Arc::clone(&store), Some(&session_id));
		let ctx = ParamContext::new();

		// Act
		let SessionValue(user_id) =
			<SessionValue<i64> as FromRequest>::from_request(&request, &ctx)
				.await
				.expect("SessionValue should extract when the user-id key is present");

		// Assert
		assert_eq!(user_id, 7);
	}

	#[tokio::test]
	async fn session_value_from_request_fails_authentication_when_session_missing() {
		// Arrange — no session cookie => SessionData::inject -> DiError::NotFound,
		// which the FromRequest path must map to ParamError::Authentication so
		// the handler macro returns HTTP 401 rather than 400.
		let store = Arc::new(SessionStore::new());
		let request = build_request_with_session(store, None);
		let ctx = ParamContext::new();

		// Act
		let err = <SessionValue<i64> as FromRequest>::from_request(&request, &ctx)
			.await
			.expect_err("SessionValue must fail when no session is present");

		// Assert
		assert!(
			matches!(err, ParamError::Authentication(_)),
			"expected ParamError::Authentication, got {err:?}"
		);
	}

	#[tokio::test]
	async fn optional_session_value_from_request_yields_none_when_session_missing() {
		// Arrange
		let store = Arc::new(SessionStore::new());
		let request = build_request_with_session(store, None);
		let ctx = ParamContext::new();

		// Act
		let OptionalSessionValue(maybe_id) =
			<OptionalSessionValue<i64> as FromRequest>::from_request(&request, &ctx)
				.await
				.expect("OptionalSessionValue must never fail extraction");

		// Assert
		assert_eq!(maybe_id, None);
	}

	#[tokio::test]
	async fn optional_session_value_from_request_returns_some_when_key_present() {
		// Arrange
		let store = Arc::new(SessionStore::new());
		let mut session = SessionData::new(Duration::from_secs(3600));
		session.set(USER_ID_SESSION_KEY.to_string(), 42i64).unwrap();
		let session_id = session.id.clone();
		store.save(session);
		let request = build_request_with_session(Arc::clone(&store), Some(&session_id));
		let ctx = ParamContext::new();

		// Act
		let OptionalSessionValue(maybe_id) =
			<OptionalSessionValue<i64> as FromRequest>::from_request(&request, &ctx)
				.await
				.expect("OptionalSessionValue must succeed when the key is present");

		// Assert
		assert_eq!(maybe_id, Some(42));
	}

	#[tokio::test]
	async fn session_value_named_from_request_reads_custom_key() {
		// Arrange
		let store = Arc::new(SessionStore::new());
		let mut session = SessionData::new(Duration::from_secs(3600));
		session.set(TenantIdKey::KEY.to_string(), 9001i64).unwrap();
		let session_id = session.id.clone();
		store.save(session);

		let request = build_request_with_session(Arc::clone(&store), Some(&session_id));
		let ctx = ParamContext::new();

		// Act
		let extracted =
			<SessionValueNamed<TenantIdKey, i64> as FromRequest>::from_request(&request, &ctx)
				.await
				.expect("SessionValueNamed should extract the configured key");

		// Assert
		assert_eq!(*extracted, 9001);
	}

	#[tokio::test]
	async fn session_value_named_from_request_fails_when_key_missing() {
		// Arrange — session exists but does NOT carry the tenant_id key.
		let store = Arc::new(SessionStore::new());
		let session = SessionData::new(Duration::from_secs(3600));
		let session_id = session.id.clone();
		store.save(session);
		let request = build_request_with_session(Arc::clone(&store), Some(&session_id));
		let ctx = ParamContext::new();

		// Act
		let err =
			<SessionValueNamed<TenantIdKey, i64> as FromRequest>::from_request(&request, &ctx)
				.await
				.expect_err("SessionValueNamed must fail when the named key is absent");

		// Assert
		assert!(
			matches!(err, ParamError::Authentication(_)),
			"expected ParamError::Authentication, got {err:?}"
		);
	}

	#[tokio::test]
	async fn optional_session_value_named_from_request_returns_some_when_key_present() {
		// Arrange
		let store = Arc::new(SessionStore::new());
		let mut session = SessionData::new(Duration::from_secs(3600));
		session.set(TenantIdKey::KEY.to_string(), 4242i64).unwrap();
		let session_id = session.id.clone();
		store.save(session);

		let request = build_request_with_session(Arc::clone(&store), Some(&session_id));
		let ctx = ParamContext::new();

		// Act
		let extracted = <OptionalSessionValueNamed<TenantIdKey, i64> as FromRequest>::from_request(
			&request, &ctx,
		)
		.await
		.expect("OptionalSessionValueNamed must succeed when the key is present");

		// Assert
		assert_eq!(*extracted, Some(4242));
	}

	#[tokio::test]
	async fn optional_session_value_named_from_request_yields_none_when_key_missing() {
		// Arrange — session exists but does NOT carry the tenant_id key.
		// The optional variant must collapse this to `None` rather than 401.
		let store = Arc::new(SessionStore::new());
		let session = SessionData::new(Duration::from_secs(3600));
		let session_id = session.id.clone();
		store.save(session);

		let request = build_request_with_session(Arc::clone(&store), Some(&session_id));
		let ctx = ParamContext::new();

		// Act
		let extracted = <OptionalSessionValueNamed<TenantIdKey, i64> as FromRequest>::from_request(
			&request, &ctx,
		)
		.await
		.expect("OptionalSessionValueNamed must never fail extraction");

		// Assert
		assert_eq!(*extracted, None);
	}

	#[tokio::test]
	async fn optional_session_value_named_from_request_yields_none_when_session_missing() {
		// Arrange — no session cookie => `SessionData::inject` fails with
		// `DiError::NotFound`. The optional variant must swallow that into
		// `None` rather than propagating an authentication error.
		let store = Arc::new(SessionStore::new());
		let request = build_request_with_session(store, None);
		let ctx = ParamContext::new();

		// Act
		let extracted = <OptionalSessionValueNamed<TenantIdKey, i64> as FromRequest>::from_request(
			&request, &ctx,
		)
		.await
		.expect("OptionalSessionValueNamed must tolerate a missing session");

		// Assert
		assert_eq!(*extracted, None);
	}

	#[tokio::test]
	async fn optional_session_value_named_from_request_yields_none_on_deserialisation_mismatch() {
		// Arrange — store a string value under the tenant_id key, then ask
		// for an i64. The Option<T> semantics of `SessionData::get` collapse
		// a deserialisation failure to `None`, which `OptionalSessionValueNamed`
		// must surface as `None` rather than 401/500.
		let store = Arc::new(SessionStore::new());
		let mut session = SessionData::new(Duration::from_secs(3600));
		session
			.set(TenantIdKey::KEY.to_string(), "not-an-i64".to_string())
			.unwrap();
		let session_id = session.id.clone();
		store.save(session);

		let request = build_request_with_session(Arc::clone(&store), Some(&session_id));
		let ctx = ParamContext::new();

		// Act
		let extracted = <OptionalSessionValueNamed<TenantIdKey, i64> as FromRequest>::from_request(
			&request, &ctx,
		)
		.await
		.expect("OptionalSessionValueNamed must absorb deserialisation failures");

		// Assert
		assert_eq!(*extracted, None);
	}

	#[tokio::test]
	async fn optional_session_value_named_from_request_yields_none_without_di_context() {
		// Arrange — no DI context attached to the request. Mirror the
		// behaviour verified for `OptionalSessionValue` (must collapse to
		// `None` rather than 500).
		let request = Request::builder()
			.method(Method::GET)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();
		let ctx = ParamContext::new();

		// Act
		let extracted = <OptionalSessionValueNamed<TenantIdKey, i64> as FromRequest>::from_request(
			&request, &ctx,
		)
		.await
		.expect("OptionalSessionValueNamed must tolerate a missing DI context");

		// Assert
		assert_eq!(*extracted, None);
	}

	#[tokio::test]
	async fn optional_session_value_from_request_yields_none_without_di_context() {
		// Arrange — request has no DI context attached. OptionalSessionValue
		// must still succeed (collapsing to None) rather than 500.
		let request = Request::builder()
			.method(Method::GET)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();
		let ctx = ParamContext::new();

		// Act
		let OptionalSessionValue(maybe_id) =
			<OptionalSessionValue<i64> as FromRequest>::from_request(&request, &ctx)
				.await
				.expect("OptionalSessionValue must tolerate a missing DI context");

		// Assert
		assert_eq!(maybe_id, None);
	}

	#[tokio::test]
	async fn session_value_from_request_fails_internal_without_di_context() {
		// Arrange — required SessionValue with no DI context must surface
		// the configuration mistake as ParamError::Internal so that the
		// handler returns 500 (rather than masking the misconfiguration as
		// a 401 Authentication or a vague 400 Validation).
		let request = Request::builder()
			.method(Method::GET)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();
		let ctx = ParamContext::new();

		// Act
		let err = <SessionValue<i64> as FromRequest>::from_request(&request, &ctx)
			.await
			.expect_err("SessionValue must fail without a DI context");

		// Assert
		assert!(
			matches!(err, ParamError::Internal(_)),
			"expected ParamError::Internal, got {err:?}"
		);
	}
}
