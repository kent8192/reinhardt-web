//! Polls-app DI factories.
//!
//! ## Why a per-app `di` module?
//!
//! Per the project README, each application owns its own
//! `#[injectable_factory]` registrations. Keeping them in `polls/di.rs`
//! lets `server_fn.rs` stay focused on handler bodies and gives every
//! DI contribution a single, greppable home per app.
//!
//! ## Why a hand-rolled `SessionUser` instead of `reinhardt_auth::AuthUser<User>`?
//!
//! `AuthUser<U>`
//! (`crates/reinhardt-auth/src/auth_user.rs:43`) is the **canonical
//! authenticated-user extractor** — handler signatures spell
//! `#[inject] AuthUser(user): AuthUser<User>` and the framework loads
//! the row from `AuthState` in request extensions. It is the long-term
//! target for this tutorial.
//!
//! But `AuthUser<U>`'s `Injectable` impl
//! (`auth_user.rs:54-122`) reads `AuthState` from
//! `request.extensions`. `AuthState` is only populated when an auth
//! middleware writes it there
//! (`crates/reinhardt-middleware/src/cookie_session_auth.rs:209`,
//! `…/auth.rs`). This tutorial currently wires up
//! `SessionMiddleware` alone — it manages session cookies + store but
//! does not insert `AuthState`. Adopting `AuthUser<User>` therefore
//! requires either:
//!
//! 1. A bridge middleware (`CookieSessionAuthMiddleware`) — but its
//!    backend type (`AsyncSessionBackend`) is **not** implemented by
//!    `SessionStore`, so wiring it up needs a framework-level adapter
//!    we do not have today.
//! 2. Storing `user_id` as `String` in the session map (the bridge
//!    middleware deserialises it as `String`, but our `login` /
//!    `logout` / `current_user` server functions persist it as `i64`).
//!
//! Both gaps are tracked as a `rc-migration` proposal in
//! [#4652](https://github.com/kent8192/reinhardt-web/issues/4652).
//! Once that ships, this whole module collapses — handlers swap
//! `#[inject] session_user: Depends<SessionUser>` /
//! `session_user.require_active()?` for the upstream
//! `#[inject] AuthUser(user): AuthUser<User>` (plus an inline
//! `is_active` check) and `apps/polls/di.rs` is deleted entirely.
//!
//! The type is named `SessionUser` (not `CurrentUser`) for two reasons:
//!
//! - **Name-clash avoidance** — `reinhardt_auth::CurrentUser<U>` is
//!   `#[deprecated]` and scheduled to become `pub type CurrentUser<U> =
//!   AuthUser<U>` in 0.2.0
//!   (`crates/reinhardt-auth/src/current_user.rs:53-56`). Reusing the
//!   name would shadow that alias once #4652 lands.
//! - **Honest intent** — this factory is explicitly *session-derived*.
//!   The `SessionUser` name signals that it is the session-only fallback
//!   for the `AuthUser`-shaped story.
//!
//! ## Why not `Result<SessionUser, ServerFnError>` as the factory return?
//!
//! `#[injectable_factory]` registers its **literal return type** as the
//! DI key (`crates/reinhardt-di/macros/src/injectable_factory.rs:182`
//! `register_async::<#return_type, _, _>`). Returning
//! `Result<SessionUser, ServerFnError>` would force every handler's
//! `#[inject]` parameter to spell out
//! `Depends<Result<SessionUser, ServerFnError>>`, which is bulky and
//! moves error semantics into the type signature. Instead we use the
//! Django-style three-state enum below — handlers spell
//! `Depends<SessionUser>` and dispatch with a single
//! `.require_active()?` call.
//!
//! ## Limitation: dynamic request data
//!
//! `#[injectable_factory]` today rejects any parameter that is not
//! `#[inject]`-tagged
//! (`crates/reinhardt-di/macros/src/injectable_factory.rs:60-71`), so
//! authentication scoped to a *path-bound* resource — e.g.,
//! `require_question_author(question_id, &user)` — cannot be expressed
//! as a factory yet. That is tracked as a `rc-migration` follow-up in
//! [#4645](https://github.com/kent8192/reinhardt-web/issues/4645);
//! until it ships, the per-row authorization helper stays a plain
//! `async fn` in `server_fn.rs`.

#[cfg(native)]
use reinhardt::Model;
#[cfg(native)]
use reinhardt::di::injectable_factory;
#[cfg(native)]
use reinhardt::middleware::session::{SessionData, USER_ID_SESSION_KEY};
#[cfg(native)]
use reinhardt::pages::server_fn::ServerFnError;

#[cfg(native)]
use crate::apps::users::models::User;

/// Request-scoped wrapper around the user resolved from the session,
/// mirroring Django's `request.user`.
///
/// Three states are surfaced so handlers can distinguish the kind of
/// failure without re-running session/database lookups:
///
/// - [`SessionUser::Authenticated`] — a row exists for the session's
///   `user_id` and `is_active = true`.
/// - [`SessionUser::Inactive`] — a row exists but `is_active = false`.
///   Surfaced as **403** by [`SessionUser::require_active`].
/// - [`SessionUser::Anonymous`] — no `user_id` in the session, or the row
///   has been deleted between login and request. Surfaced as **401**.
///
/// `Clone` is derived so handlers can pull an owned `SessionUser` out of
/// `Depends<_>` with `(*session_user).clone()` when needed.
#[cfg(native)]
#[derive(Clone)]
pub enum SessionUser {
	Authenticated(User),
	Inactive(User),
	Anonymous,
}

#[cfg(native)]
impl SessionUser {
	/// Borrow the active authenticated user, or surface a 401/403
	/// `ServerFnError`.
	pub fn require_active(&self) -> std::result::Result<&User, ServerFnError> {
		match self {
			Self::Authenticated(u) => Ok(u),
			Self::Inactive(_) => Err(ServerFnError::server(403, "User account is inactive")),
			Self::Anonymous => Err(ServerFnError::server(401, "Authentication required")),
		}
	}
}

/// `#[injectable_factory(scope = "request")]` registers a factory that
/// runs **once per request** (see
/// `crates/reinhardt-di/macros/src/utils.rs:40` `KNOWN_ARGS = &["scope"]`
/// for the supported `scope` values). The `#[inject]` annotation is the
/// way factories compose over other injectables — `SessionData` is
/// provided by the session middleware
/// (`crates/reinhardt-middleware/src/session/` registers it through
/// `Middleware::di_registrations`), so the factory's only argument is
/// the session itself.
///
/// > Note: scope syntax is `(scope = "...")` as a macro argument, not a
/// > separate `#[scope(...)]` attribute. The README/docstring example
/// > using the separate-attribute form is incorrect and is tracked in
/// > [#4646](https://github.com/kent8192/reinhardt-web/issues/4646).
#[cfg(native)]
#[injectable_factory(scope = "request")]
async fn session_user_factory(#[inject] session: SessionData) -> SessionUser {
	let Some(user_id) = session.get::<i64>(USER_ID_SESSION_KEY) else {
		return SessionUser::Anonymous;
	};

	// A session can outlive the user row (manual delete, GDPR purge,
	// etc.). Treat "row not found" or transient lookup errors as
	// `Anonymous` so the handler returns a 401 instead of a 500 — the
	// session is not trustworthy and forcing re-auth is the right
	// answer.
	//
	// Ideal implementation (blocked on #4650): once `Manager::filter`
	// accepts the typed builder, this becomes:
	//   `.filter(User::field_id().eq(user_id))`.
	use reinhardt::db::orm::{FilterOperator, FilterValue};
	let user = match User::objects()
		.filter(
			User::field_id(),
			FilterOperator::Eq,
			FilterValue::Int(user_id),
		)
		.first()
		.await
	{
		Ok(Some(u)) => u,
		_ => return SessionUser::Anonymous,
	};

	if user.is_active {
		SessionUser::Authenticated(user)
	} else {
		SessionUser::Inactive(user)
	}
}
