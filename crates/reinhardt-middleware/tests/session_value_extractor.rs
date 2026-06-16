//! Integration tests for `SessionValue<T>` and `OptionalSessionValue<T>`.
//!
//! Drives `Injectable::inject` by hand against an `InjectionContext` whose
//! singleton scope contains a pre-populated `SessionStore` and whose
//! request carries a session cookie pointing at one of those entries.
//! Validates the three documented contract points from issue #4446:
//!
//! 1. `SessionValue::<T>::inject` returns the value stored under
//!    `USER_ID_SESSION_KEY`, deserialised as `T`.
//! 2. Missing-key / missing-session cases fail injection on
//!    `SessionValue` and yield `None` on `OptionalSessionValue`.
//! 3. Both extractors are wired through the standard `Injectable` path so
//!    they compose with `#[inject]` server-fn parameters.

#[cfg(feature = "sessions")]
mod tests {
	use std::sync::Arc;
	use std::time::Duration;

	use bytes::Bytes;
	use hyper::{HeaderMap, Method, Version};
	use reinhardt_di::{DiError, Injectable, InjectionContext, SingletonScope};
	use reinhardt_http::Request;
	use reinhardt_middleware::session::{
		OptionalSessionValue, SessionData, SessionStore, SessionValue, USER_ID_SESSION_KEY,
	};

	fn build_ctx_with_session(
		store: Arc<SessionStore>,
		session_cookie: Option<&str>,
	) -> InjectionContext {
		let mut headers = HeaderMap::new();
		if let Some(cookie) = session_cookie {
			headers.insert(
				hyper::header::COOKIE,
				hyper::header::HeaderValue::from_str(&format!("sessionid={cookie}")).unwrap(),
			);
		}
		let request = Request::builder()
			.method(Method::GET)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(headers)
			.body(Bytes::new())
			.build()
			.unwrap();

		let singleton: Arc<SingletonScope> = Arc::new(SingletonScope::new());
		// `SessionData::inject` reads the store from the singleton scope under
		// TypeId::of::<SessionStore>() (post-#4437 key). `set_arc` stores the
		// `Arc<SessionStore>` verbatim under that key, so handlers using
		// `#[inject] store: Depends<SessionStoreKey, Arc<SessionStore>>`
		// would resolve the same store through the keyed registration.
		singleton.set_arc(store);

		// `SessionData::inject` reads the `Request` from the request scope via
		// `ctx.get_request::<Request>()`, not the `with_request` field used by
		// `get_http_request()`. Mirror what `RouterMiddleware` does in production:
		// seed the request scope so the extractor finds it.
		let ctx = InjectionContext::builder(singleton).build();
		ctx.set_request(request);
		ctx
	}

	#[tokio::test]
	async fn session_value_returns_stored_user_id() {
		let store = Arc::new(SessionStore::new());
		let mut session = SessionData::new(Duration::from_secs(3600));
		session
			.set(USER_ID_SESSION_KEY.to_string(), 123i64)
			.unwrap();
		let session_id = session.id.clone();
		store.save(session);

		let ctx = build_ctx_with_session(Arc::clone(&store), Some(&session_id));
		let SessionValue(user_id) = SessionValue::<i64>::inject(&ctx)
			.await
			.expect("SessionValue should resolve when the key is present");

		assert_eq!(user_id, 123);
	}

	#[tokio::test]
	async fn session_value_fails_when_key_missing() {
		let store = Arc::new(SessionStore::new());
		let session = SessionData::new(Duration::from_secs(3600));
		let session_id = session.id.clone();
		store.save(session);

		let ctx = build_ctx_with_session(Arc::clone(&store), Some(&session_id));
		let err = SessionValue::<i64>::inject(&ctx)
			.await
			.expect_err("SessionValue must fail when the user-id key is absent");

		assert!(
			matches!(err, DiError::Authentication(_)),
			"expected DiError::Authentication, got {err:?}"
		);
	}

	#[tokio::test]
	async fn optional_session_value_yields_none_when_session_missing() {
		// No session cookie present — `SessionData::inject` will fail with
		// `DiError::NotFound`, and `OptionalSessionValue` must swallow that
		// into `None` rather than propagating the error.
		let store = Arc::new(SessionStore::new());
		let ctx = build_ctx_with_session(store, None);

		let OptionalSessionValue(maybe_id) = OptionalSessionValue::<i64>::inject(&ctx)
			.await
			.expect("OptionalSessionValue must never fail injection when session is absent");

		assert_eq!(maybe_id, None);
	}

	#[tokio::test]
	async fn optional_session_value_returns_some_when_key_present() {
		let store = Arc::new(SessionStore::new());
		let mut session = SessionData::new(Duration::from_secs(3600));
		session.set(USER_ID_SESSION_KEY.to_string(), 99i64).unwrap();
		let session_id = session.id.clone();
		store.save(session);

		let ctx = build_ctx_with_session(Arc::clone(&store), Some(&session_id));
		let OptionalSessionValue(maybe_id) = OptionalSessionValue::<i64>::inject(&ctx)
			.await
			.expect("OptionalSessionValue should resolve when the session carries the key");

		assert_eq!(maybe_id, Some(99));
	}

	#[tokio::test]
	async fn optional_session_value_yields_none_when_key_missing() {
		// Session exists in the store but does not carry USER_ID_SESSION_KEY.
		let store = Arc::new(SessionStore::new());
		let session = SessionData::new(Duration::from_secs(3600));
		let session_id = session.id.clone();
		store.save(session);

		let ctx = build_ctx_with_session(Arc::clone(&store), Some(&session_id));
		let OptionalSessionValue(maybe_id) = OptionalSessionValue::<i64>::inject(&ctx)
			.await
			.expect("OptionalSessionValue must succeed when the key is absent");

		assert_eq!(maybe_id, None);
	}
}
