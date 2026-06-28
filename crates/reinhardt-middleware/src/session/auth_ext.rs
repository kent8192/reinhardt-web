//! [`SessionAuthExt`]: ergonomic login/logout helpers on [`SessionData`].
//!
//! Collapses the four-step "rotate id → write user id → delete old store
//! entry → save new store entry" sequence that every server-function login
//! handler has had to spell out by hand. Symmetrically wraps the logout
//! sequence so callers cannot forget to rotate the id before clearing the
//! user reference. See issue #4446.

use reinhardt_http::Result;
use serde::Serialize;

use super::data::{SessionData, USER_ID_SESSION_KEY};
use super::store::SessionStore;

/// Login/logout helpers for [`SessionData`].
///
/// Both methods perform the session-fixation prevention rotation that is a
/// required step on authentication state transitions: each call regenerates
/// the session id, removes the old store entry referenced by the previous
/// id, and persists the updated [`SessionData`] under the new id.
///
/// The trait is provided as an extension so existing call sites can opt
/// in by adding a single `use` and replacing their inline blocks; the
/// implementation lives in `reinhardt-middleware` because that is the
/// crate that owns [`SessionData`] and [`SessionStore`]. `BaseUser` is
/// deliberately *not* a bound on `login` — taking `impl Serialize` keeps
/// the helper usable with any primary-key shape (`i64`, `Uuid`, a tenant
/// composite key, …) and avoids the otherwise-circular auth ↔ middleware
/// coupling.
///
/// The `store` parameter is a `&SessionStore`. Callers that have
/// `#[inject] store: Depends<SessionStoreKey, Arc<SessionStore>>` can pass
/// `&**store`, dereferencing first through [`Depends`][reinhardt_di::Depends]
/// to the shared `Arc<SessionStore>` and then through `Arc` to `SessionStore`.
///
/// # Usage
///
/// ```rust,ignore
/// use reinhardt::di::Depends;
/// use reinhardt::middleware::session::{
///     SessionAuthExt, SessionData, SessionStore, SessionStoreKey,
/// };
/// use std::sync::Arc;
///
/// #[server_fn]
/// pub async fn login(
///     username: String,
///     password: String,
///     #[inject] mut session: SessionData,
///     #[inject] store: Depends<SessionStoreKey, Arc<SessionStore>>,
/// ) -> Result<(), ServerFnError> {
///     // … authenticate `user` …
///     session.login(&**store, user.id())
///         .map_err(|e| ServerFnError::application(e.to_string()))?;
///     Ok(())
/// }
/// ```
pub trait SessionAuthExt {
	/// Mark the current session as authenticated for `user_id`.
	///
	/// Equivalent to the inline sequence:
	///
	/// ```text
	/// let old_id = self.regenerate_id();
	/// self.set(USER_ID_SESSION_KEY.to_string(), user_id)?;
	/// store.delete(&old_id);
	/// store.save(self.clone());
	/// ```
	///
	/// Returns a [`reinhardt_http::Result`] so the serialisation failure
	/// inside [`SessionData::set`] propagates with the same error type as
	/// the rest of the session API.
	fn login<V: Serialize + Send + Sync>(&mut self, store: &SessionStore, user_id: V)
	-> Result<()>;

	/// Clear the authenticated-user reference from the current session.
	///
	/// Rotates the session id, removes the old store entry, drops the
	/// user-id key from the session map (without clearing any other
	/// keys callers may have written), and persists the rotated session.
	/// Callers who want to drop *all* session state should call
	/// [`SessionData::clear`] before invoking this helper.
	fn logout(&mut self, store: &SessionStore);
}

impl SessionAuthExt for SessionData {
	fn login<V: Serialize + Send + Sync>(
		&mut self,
		store: &SessionStore,
		user_id: V,
	) -> Result<()> {
		let old_id = self.regenerate_id();
		self.set(USER_ID_SESSION_KEY.to_string(), user_id)?;
		store.delete(&old_id);
		store.save(self.clone());
		Ok(())
	}

	fn logout(&mut self, store: &SessionStore) {
		let old_id = self.regenerate_id();
		self.delete(USER_ID_SESSION_KEY);
		store.delete(&old_id);
		store.save(self.clone());
	}
}
