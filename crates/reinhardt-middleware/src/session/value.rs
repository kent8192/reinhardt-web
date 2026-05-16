//! Typed session-value extractors usable directly in handler signatures.
//!
//! Three flavours mirror the rest of the Reinhardt extractor surface:
//!
//! - [`SessionValue<T>`] reads `session["user_id"]` and deserialises it as
//!   `T`; 401 when the session or key is missing.
//! - [`OptionalSessionValue<T>`] is the optional variant: any failure
//!   collapses to `OptionalSessionValue(None)` rather than propagating.
//! - [`SessionValueNamed<K, T>`] reads a custom session key chosen at
//!   compile time via a marker type implementing [`SessionKey`].
//!
//! Each extractor is wired through both `Injectable` (for `#[inject]`
//! parameters) **and** `FromRequest` (for `Path(...)`-style auto-extraction
//! without the `#[inject]` attribute). Pick whichever ergonomics you
//! prefer:
//!
//! ```rust,ignore
//! use reinhardt::middleware::session::{OptionalSessionValue, SessionValue};
//!
//! // Auto-extraction (no `#[inject]`, matches `Path(...)` ergonomics).
//! #[server_fn]
//! pub async fn current_user(
//!     SessionValue(user_id): SessionValue<i64>,
//! ) -> Result<UserInfo, ServerFnError> { /* ... */ }
//!
//! // Equivalent legacy form with `#[inject]`.
//! #[server_fn]
//! pub async fn current_user(
//!     #[inject] SessionValue(user_id): SessionValue<i64>,
//! ) -> Result<UserInfo, ServerFnError> { /* ... */ }
//! ```
//!
//! See issue #4446 for the motivating discussion.

use async_trait::async_trait;
use reinhardt_di::params::{ParamContext, ParamError, ParamResult, extract::FromRequest};
use reinhardt_di::{DiError, DiResult, Injectable, InjectionContext};
use reinhardt_http::Request;
use serde::de::DeserializeOwned;
use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::ops::Deref;

use super::data::{SessionData, USER_ID_SESSION_KEY};

/// Marker trait identifying a session-storage key at the type level.
///
/// Implementors are zero-sized marker types similar to
/// `reinhardt_di::params::CookieName` — define one type per logical key
/// and reuse it across handlers:
///
/// ```rust,ignore
/// use reinhardt::middleware::session::{SessionKey, SessionValueNamed};
///
/// pub struct TenantIdKey;
/// impl SessionKey for TenantIdKey {
///     const KEY: &'static str = "tenant_id";
/// }
///
/// #[server_fn]
/// pub async fn current_tenant(
///     SessionValueNamed::<TenantIdKey, i64>(tenant_id): SessionValueNamed<TenantIdKey, i64>,
/// ) -> Result<TenantInfo, ServerFnError> { /* ... */ }
/// ```
pub trait SessionKey: Send + Sync + 'static {
	/// The session-store key whose value this marker maps to.
	const KEY: &'static str;
}

/// Default marker pointing at [`USER_ID_SESSION_KEY`] — the authenticated
/// user's primary key in every Reinhardt example app.
#[derive(Debug, Clone, Copy)]
pub struct UserIdKey;

impl SessionKey for UserIdKey {
	const KEY: &'static str = USER_ID_SESSION_KEY;
}

/// Required typed session-value extractor.
///
/// Resolves the [`USER_ID_SESSION_KEY`] entry from the active
/// [`SessionData`], deserialises it as `T`, and fails extraction when the
/// key is missing or the value cannot be deserialised. Use this extractor
/// on server functions that require an authenticated session — the
/// absent case surfaces as HTTP 401 via `CoreError::Authentication`.
///
/// # Usage
///
/// ```rust,ignore
/// use reinhardt::middleware::session::SessionValue;
///
/// #[server_fn]
/// pub async fn current_user(
///     SessionValue(user_id): SessionValue<i64>,
/// ) -> Result<UserInfo, ServerFnError> {
///     // user_id is the authenticated user's primary key
///     // ...
/// }
/// ```
///
/// Adding `#[inject]` continues to work for code that prefers explicit
/// dependency markers (see the module-level docs).
#[derive(Debug, Clone)]
pub struct SessionValue<T>(pub T);

/// Optional typed session-value extractor.
///
/// Identical to [`SessionValue<T>`] except extraction never fails: when
/// the session is missing, expired, or carries no value at
/// [`USER_ID_SESSION_KEY`], the extractor yields
/// `OptionalSessionValue(None)`. Use this on handlers that may serve
/// both anonymous and authenticated callers (a public "/current_user"
/// endpoint, for instance).
#[derive(Debug, Clone)]
pub struct OptionalSessionValue<T>(pub Option<T>);

/// Typed session-value extractor parameterised by a [`SessionKey`].
///
/// Generalises [`SessionValue<T>`] to keys other than
/// [`USER_ID_SESSION_KEY`]. Construct one marker per logical key (see
/// the [`SessionKey`] trait docs) and use the marker as the first type
/// parameter:
///
/// ```rust,ignore
/// use reinhardt::middleware::session::{SessionKey, SessionValueNamed};
///
/// pub struct TenantIdKey;
/// impl SessionKey for TenantIdKey {
///     const KEY: &'static str = "tenant_id";
/// }
///
/// #[server_fn]
/// pub async fn current_tenant(
///     SessionValueNamed::<TenantIdKey, i64>(tenant_id): SessionValueNamed<TenantIdKey, i64>,
/// ) -> Result<TenantInfo, ServerFnError> { /* ... */ }
/// ```
pub struct SessionValueNamed<K: SessionKey, T> {
	value: T,
	_phantom: PhantomData<fn() -> K>,
}

impl<K: SessionKey, T> SessionValueNamed<K, T> {
	/// Construct a `SessionValueNamed` directly from a value. Primarily
	/// useful in tests where extraction is bypassed.
	pub fn new(value: T) -> Self {
		Self {
			value,
			_phantom: PhantomData,
		}
	}

	/// Unwrap the extractor and return the inner value.
	pub fn into_inner(self) -> T {
		self.value
	}
}

impl<K: SessionKey, T> Deref for SessionValueNamed<K, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.value
	}
}

impl<K: SessionKey, T: Debug> Debug for SessionValueNamed<K, T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("SessionValueNamed")
			.field("key", &K::KEY)
			.field("value", &self.value)
			.finish()
	}
}

impl<K: SessionKey, T: Clone> Clone for SessionValueNamed<K, T> {
	fn clone(&self) -> Self {
		Self {
			value: self.value.clone(),
			_phantom: PhantomData,
		}
	}
}

// ---------------------------------------------------------------------------
// Internal helpers shared between `Injectable` and `FromRequest` impls.
// ---------------------------------------------------------------------------

/// Load the active `SessionData` via the standard `Injectable` path,
/// then extract the value at `key` and deserialise it as `T`.
async fn load_session_value_via_di<T>(ctx: &InjectionContext, key: &str) -> DiResult<T>
where
	T: DeserializeOwned + Send + Sync + 'static,
{
	let session = SessionData::inject(ctx).await?;
	session.get::<T>(key).ok_or_else(|| {
		DiError::Authentication(format!(
			"SessionValue<{}>: no value stored under session key '{}'",
			std::any::type_name::<T>(),
			key,
		))
	})
}

/// Reach the request-scoped `InjectionContext` and delegate to
/// [`load_session_value_via_di`]. Wraps the resulting `DiError` into a
/// `ParamError` so the handler macro can surface the right HTTP status.
async fn load_session_value_via_request<T>(req: &Request, key: &str) -> ParamResult<T>
where
	T: DeserializeOwned + Send + Sync + 'static,
{
	let di_ctx = req.get_di_context::<InjectionContext>().ok_or_else(|| {
		ParamError::Authentication(
			"SessionValue: DI context not available on the request. \
			 Ensure the router is configured with `.with_di_context()` and \
			 `SessionMiddleware` is installed in the middleware chain."
				.to_string(),
		)
	})?;
	load_session_value_via_di::<T>(&di_ctx, key)
		.await
		.map_err(di_error_to_param_error)
}

/// Project `DiError` into the matching `ParamError` variant. Authentication
/// and not-found cases collapse into `ParamError::Authentication` so they
/// reach the response as HTTP 401 (see #4446 + `ParamError::Authentication`
/// in `reinhardt-di`).
fn di_error_to_param_error(err: DiError) -> ParamError {
	match err {
		DiError::Authentication(msg) | DiError::NotFound(msg) => ParamError::Authentication(msg),
		other => ParamError::Authentication(other.to_string()),
	}
}

// ---------------------------------------------------------------------------
// Injectable impls (back-compat with `#[inject]` parameters).
// ---------------------------------------------------------------------------

#[async_trait]
impl<T> Injectable for SessionValue<T>
where
	T: DeserializeOwned + Send + Sync + 'static,
{
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		load_session_value_via_di::<T>(ctx, USER_ID_SESSION_KEY)
			.await
			.map(SessionValue)
	}
}

#[async_trait]
impl<T> Injectable for OptionalSessionValue<T>
where
	T: DeserializeOwned + Send + Sync + 'static,
{
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		// Mirror `SessionValue`, but collapse "no session"/"no value" into
		// `None` rather than propagating an injection error. Any other
		// error (such as a corrupted singleton scope) still bubbles up so
		// genuine misconfigurations remain visible.
		match SessionData::inject(ctx).await {
			Ok(session) => Ok(OptionalSessionValue(session.get::<T>(USER_ID_SESSION_KEY))),
			Err(DiError::NotFound(_)) => Ok(OptionalSessionValue(None)),
			Err(e) => Err(e),
		}
	}
}

#[async_trait]
impl<K, T> Injectable for SessionValueNamed<K, T>
where
	K: SessionKey,
	T: DeserializeOwned + Send + Sync + 'static,
{
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		load_session_value_via_di::<T>(ctx, K::KEY)
			.await
			.map(Self::new)
	}
}

// ---------------------------------------------------------------------------
// FromRequest impls (auto-extraction without `#[inject]`).
// ---------------------------------------------------------------------------

#[async_trait]
impl<T> FromRequest for SessionValue<T>
where
	T: DeserializeOwned + Send + Sync + 'static,
{
	async fn from_request(req: &Request, _ctx: &ParamContext) -> ParamResult<Self> {
		load_session_value_via_request::<T>(req, USER_ID_SESSION_KEY)
			.await
			.map(SessionValue)
	}
}

#[async_trait]
impl<T> FromRequest for OptionalSessionValue<T>
where
	T: DeserializeOwned + Send + Sync + 'static,
{
	async fn from_request(req: &Request, _ctx: &ParamContext) -> ParamResult<Self> {
		// Mirror the `Injectable` semantics: any failure to reach a live
		// session collapses to `None`. Successful session lookups still
		// honour the `session.get::<T>(...) -> Option<T>` semantics for
		// missing keys and deserialisation failures.
		let di_ctx = match req.get_di_context::<InjectionContext>() {
			Some(c) => c,
			None => return Ok(OptionalSessionValue(None)),
		};
		match SessionData::inject(&di_ctx).await {
			Ok(session) => Ok(OptionalSessionValue(session.get::<T>(USER_ID_SESSION_KEY))),
			Err(_) => Ok(OptionalSessionValue(None)),
		}
	}
}

#[async_trait]
impl<K, T> FromRequest for SessionValueNamed<K, T>
where
	K: SessionKey,
	T: DeserializeOwned + Send + Sync + 'static,
{
	async fn from_request(req: &Request, _ctx: &ParamContext) -> ParamResult<Self> {
		load_session_value_via_request::<T>(req, K::KEY)
			.await
			.map(Self::new)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	struct TenantIdKey;
	impl SessionKey for TenantIdKey {
		const KEY: &'static str = "tenant_id";
	}

	#[rstest]
	fn user_id_key_resolves_to_canonical_session_key() {
		// Arrange + Act
		let key = UserIdKey::KEY;

		// Assert
		assert_eq!(key, USER_ID_SESSION_KEY);
	}

	#[rstest]
	fn session_value_named_constructor_and_deref_roundtrip() {
		// Arrange
		let extractor = SessionValueNamed::<TenantIdKey, i64>::new(42);

		// Act
		let via_deref: i64 = *extractor;
		let via_into_inner = extractor.into_inner();

		// Assert
		assert_eq!(via_deref, 42);
		assert_eq!(via_into_inner, 42);
	}

	#[rstest]
	fn di_error_authentication_maps_to_param_authentication() {
		// Arrange
		let di_err = DiError::Authentication("nope".to_string());

		// Act
		let param_err = di_error_to_param_error(di_err);

		// Assert
		match param_err {
			ParamError::Authentication(msg) => assert_eq!(msg, "nope"),
			other => panic!("expected ParamError::Authentication, got {other:?}"),
		}
	}

	#[rstest]
	fn di_error_not_found_maps_to_param_authentication() {
		// Arrange
		let di_err = DiError::NotFound("missing session".to_string());

		// Act
		let param_err = di_error_to_param_error(di_err);

		// Assert: missing session collapses to 401 (Authentication) so the
		// handler macro returns the right status. See #4446.
		assert!(matches!(param_err, ParamError::Authentication(_)));
	}
}
