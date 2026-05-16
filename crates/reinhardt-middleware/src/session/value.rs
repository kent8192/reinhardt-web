//! Typed session-value extractors for dependency injection.
//!
//! [`SessionValue<T>`] and [`OptionalSessionValue<T>`] collapse the
//! `#[inject] session: SessionData` + `session.get::<T>(USER_ID_SESSION_KEY)`
//! pattern that recurs in every authenticated server function into a single
//! injectable extractor parameterised over the value type. See issue #4446.
//!
//! Version 1 always reads the [`USER_ID_SESSION_KEY`] key; future versions
//! may extend the extractor with an attribute argument to choose another
//! key (see the issue for follow-up scope).

use async_trait::async_trait;
use reinhardt_di::{DiError, DiResult, Injectable, InjectionContext};
use serde::de::DeserializeOwned;

use super::data::{SessionData, USER_ID_SESSION_KEY};

/// Required typed session-value extractor.
///
/// Resolves the [`USER_ID_SESSION_KEY`] entry from the active
/// [`SessionData`], deserialises it as `T`, and fails injection when the key
/// is missing or the value cannot be deserialised. Use this extractor on
/// server functions that require an authenticated session — the absent
/// case is reported as `DiError::Authentication`, which maps to HTTP 401
/// upstream.
///
/// # Usage
///
/// ```rust,ignore
/// use reinhardt::middleware::session::SessionValue;
///
/// #[server_fn]
/// pub async fn current_user(
///     #[inject] SessionValue(user_id): SessionValue<i64>,
/// ) -> Result<UserInfo, ServerFnError> {
///     // user_id is the authenticated user's primary key
///     // ...
/// }
/// ```
#[derive(Debug, Clone)]
pub struct SessionValue<T>(pub T);

/// Optional typed session-value extractor.
///
/// Identical to [`SessionValue<T>`] except injection never fails: when the
/// session is missing, expired, or carries no value at [`USER_ID_SESSION_KEY`],
/// the extractor yields `OptionalSessionValue(None)`. Use this on handlers
/// that may serve both anonymous and authenticated callers (a public
/// "/current_user" endpoint, for instance).
#[derive(Debug, Clone)]
pub struct OptionalSessionValue<T>(pub Option<T>);

#[async_trait]
impl<T> Injectable for SessionValue<T>
where
	T: DeserializeOwned + Send + Sync + 'static,
{
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let session = SessionData::inject(ctx).await?;
		let value = session.get::<T>(USER_ID_SESSION_KEY).ok_or_else(|| {
			DiError::Authentication(format!(
				"SessionValue<{}>: no value stored under session key '{}'",
				std::any::type_name::<T>(),
				USER_ID_SESSION_KEY,
			))
		})?;
		Ok(SessionValue(value))
	}
}

#[async_trait]
impl<T> Injectable for OptionalSessionValue<T>
where
	T: DeserializeOwned + Send + Sync + 'static,
{
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		// Mirror SessionValue, but swallow the "no session"/"no value" cases
		// into `None` rather than propagating an injection error. Any other
		// error (such as a corrupted singleton scope) still bubbles up so
		// genuine misconfigurations remain visible.
		match SessionData::inject(ctx).await {
			Ok(session) => Ok(OptionalSessionValue(session.get::<T>(USER_ID_SESSION_KEY))),
			Err(DiError::NotFound(_)) => Ok(OptionalSessionValue(None)),
			Err(e) => Err(e),
		}
	}
}
