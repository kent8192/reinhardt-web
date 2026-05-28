//! Polls-app DI factories.
//!
//! ## Why a per-app `di` module?
//!
//! Per the project README, each application owns its own
//! `#[injectable_factory]` registrations. Keeping them in `polls/di.rs`
//! lets `server_fn.rs` stay focused on handler bodies and gives every
//! DI contribution a single, greppable home per app.
//!
//! ## Why a hand-rolled session-user factory instead of `reinhardt_auth::AuthUser<User>`?
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
//! `#[inject] user: Depends<Result<User, SessionError>>` /
//! `let user = user.as_ref().map_err(ServerFnError::from)?` for the
//! upstream `#[inject] AuthUser(user): AuthUser<User>` (plus an inline
//! `is_active` check) and `apps/polls/di.rs` is deleted entirely.
//!
//! ## Factory return type: `Result<User, SessionError>`
//!
//! `#[injectable_factory]` registers its **literal return type** as the
//! DI key (`crates/reinhardt-di/macros/src/injectable_factory.rs:182`
//! `register_async::<#return_type, _, _>`). The factory returns
//! `Result<User, SessionError>`, so handlers spell
//! `Depends<Result<User, SessionError>>` and convert to a
//! `ServerFnError` via `From<&SessionError>`. The three `SessionError`
//! variants (`Anonymous` / `Inactive` / `Unavailable`) keep the
//! "DB outage" branch separate from "user is anonymous" so an
//! availability problem cannot be silently rewritten into a fake 401.
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

/// Error variants for the session-based user lookup factory.
///
/// Three failure modes are distinguished so handlers can surface the
/// correct HTTP status *and* so that an operational outage (DB
/// connection drop, query timeout) does not get silently rewritten into
/// a fake 401:
///
/// - [`SessionError::Anonymous`] — no `user_id` in the session, or the
///   row has been deleted between login and request. Surfaced as **401**
///   via `From<&SessionError> for ServerFnError`.
/// - [`SessionError::Inactive`] — a row exists but `is_active = false`.
///   Surfaced as **403**.
/// - [`SessionError::Unavailable`] — the user-lookup query itself failed
///   (DB down, pool exhausted, schema mismatch, …). Surfaced as **500**
///   so the client sees an operational error instead of being pushed
///   into a misleading re-auth loop.
///
/// The split between `Anonymous` and `Unavailable` matters: collapsing a
/// DB error into `Anonymous` would hide the outage from monitoring, and
/// the recommended client behaviour (a redirect to the login page) would
/// punish callers for an availability problem on the server.
///
/// `Clone` and `Debug` are derived so handlers can inspect and clone the
/// error out of `Depends<Result<User, SessionError>>` when needed.
#[cfg(native)]
#[derive(Clone, Debug)]
pub enum SessionError {
	Anonymous,
	Inactive,
	/// User-lookup query failed at the database layer. The wrapped
	/// `String` is the underlying error message — the factory keeps it
	/// for logging / future propagation; the `From<&SessionError>` impl
	/// does not echo it to the client (only the 500 status + a generic
	/// message reaches the response body) to avoid leaking schema
	/// details.
	Unavailable(String),
}

/// Convert a `SessionError` reference to a `ServerFnError` with the
/// appropriate HTTP status code.
///
/// Handlers use this via `user.as_ref().map_err(ServerFnError::from)?`
/// to surface a 401, 403, or 500 depending on the failure mode.
#[cfg(native)]
impl From<&SessionError> for ServerFnError {
	fn from(err: &SessionError) -> Self {
		match err {
			SessionError::Anonymous => ServerFnError::server(401, "Authentication required"),
			SessionError::Inactive => ServerFnError::server(403, "User account is inactive"),
			SessionError::Unavailable(_) => {
				ServerFnError::server(500, "User lookup temporarily unavailable")
			}
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
async fn session_user_factory(
	#[inject] session: SessionData,
) -> Result<User, SessionError> {
	let Some(user_id) = session.get::<i64>(USER_ID_SESSION_KEY) else {
		return Err(SessionError::Anonymous);
	};

	// Distinguish the three outcomes explicitly:
	//
	// - `Ok(Some(u))` — a row exists. Authentication is meaningful;
	//   fall through to the is_active check below.
	// - `Ok(None)` — the session points at a user_id that no longer
	//   exists (deleted account, GDPR purge, …). The session itself
	//   has outlived the user, so the right behaviour is to force
	//   re-auth → **`Err(Anonymous)` (401)**.
	// - `Err(e)` — the lookup query itself failed (DB outage, pool
	//   exhaustion, schema drift, etc.). This is an *availability*
	//   problem, not an *authentication* problem, so collapsing it
	//   into `Anonymous` would (a) hide the outage from monitoring
	//   and (b) push callers into a misleading "log in again" loop.
	//   Surface it as **`Err(Unavailable)` (500)** instead.
	//
	// `tracing::warn!` logs the underlying error for observability
	// while the `From<&SessionError>` impl echoes only a generic 500
	// message to the client, so schema details do not leak via error
	// bodies.
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
		Ok(None) => return Err(SessionError::Anonymous),
		Err(e) => {
			::tracing::warn!(
				user_id = user_id,
				error = %e,
				"session_user_factory: user lookup failed"
			);
			return Err(SessionError::Unavailable(e.to_string()));
		}
	};

	if user.is_active {
		Ok(user)
	} else {
		Err(SessionError::Inactive)
	}
}
