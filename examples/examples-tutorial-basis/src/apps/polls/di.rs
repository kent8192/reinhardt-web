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
//! moves error semantics into the type signature. Instead we use a
//! four-state enum (`Authenticated` / `Inactive` / `Anonymous` /
//! `Unavailable`) — handlers spell `Depends<SessionUser>` and dispatch
//! with a single `.require_active()?` call, and the enum keeps the
//! "DB outage" branch separate from "user is anonymous" so an
//! availability problem cannot be silently rewritten into a fake 401.
//!
//! ## Limitation: dynamic request data (status of #4645)
//!
//! `#[injectable_factory]` today still rejects any parameter that is
//! not `#[inject]`-tagged
//! (`crates/reinhardt-di/macros/src/injectable_factory.rs:60-71`), so a
//! plain `Path<i64>` cannot appear bare in a factory signature. The
//! first wave of [#4645](https://github.com/kent8192/reinhardt-web/issues/4645)
//! shipped `impl Injectable for Path<T> / Query<T> / Json<T>` (see
//! `crates/reinhardt-di/src/params/{path,query,json}.rs`), so factories
//! *can* now spell `#[inject] Path(id): Path<i64>` and the DI container
//! resolves it from the active request's `ParamContext`.
//!
//! Two gaps remain before per-row authorization (e.g.
//! `require_question_author(question_id, &user)`) collapses cleanly
//! into a factory:
//!
//! 1. **form! ABI**: `#[server_fn]` arguments today must be `String`
//!    (#4397 — relaxation in progress). The five Choice/Question
//!    mutation handlers below therefore still parse `String → i64` by
//!    hand even though `Path<i64>` itself is now injectable.
//! 2. **`{question_id}` URL slot**: `Path<i64>` reads from the active
//!    URL pattern's path params. The current `#[server_fn]` URLs are
//!    flat (e.g. `/api/polls/update_question/`), so a factory taking
//!    `Path<i64>` has nothing to bind to until either the form! relax
//!    lets handlers take `question_id: i64` natively and pass it on,
//!    or the server_fn URLs grow `{id}` slots.
//!
//! Until both gaps close, the per-row authorization helper
//! (`require_question_author` in `server_fn.rs`) stays a plain
//! `async fn`. The `AuthoredQuestion` / `AuthoredChoice` newtypes
//! defined below are the **forward-looking shape** the helper will
//! collapse into once #4397's relaxation lands; they are deliberately
//! left without an `#[injectable_factory]` annotation today because
//! firing them would always trip the `MissingParamContext` path in a
//! `#[server_fn]` request and surface as a hard 500.

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
/// Four states are surfaced so handlers can distinguish the kind of
/// failure without re-running session/database lookups *and* so that an
/// operational outage (DB connection drop, query timeout) does not get
/// silently rewritten into a fake 401:
///
/// - [`SessionUser::Authenticated`] — a row exists for the session's
///   `user_id` and `is_active = true`.
/// - [`SessionUser::Inactive`] — a row exists but `is_active = false`.
///   Surfaced as **403** by [`SessionUser::require_active`].
/// - [`SessionUser::Anonymous`] — no `user_id` in the session, or the
///   row has been deleted between login and request. Surfaced as **401**.
/// - [`SessionUser::Unavailable`] — the user-lookup query itself failed
///   (DB down, pool exhausted, schema mismatch, …). Surfaced as **500**
///   by [`SessionUser::require_active`] so the client sees an operational
///   error instead of being pushed into a misleading re-auth loop.
///
/// The split between `Anonymous` and `Unavailable` matters: collapsing a
/// DB error into `Anonymous` would hide the outage from monitoring, and
/// the recommended client behaviour (a redirect to the login page) would
/// punish callers for an availability problem on the server.
///
/// `Clone` is derived so handlers can pull an owned `SessionUser` out of
/// `Depends<_>` with `(*session_user).clone()` when needed.
#[cfg(native)]
#[derive(Clone)]
pub enum SessionUser {
	Authenticated(User),
	Inactive(User),
	Anonymous,
	/// User-lookup query failed at the database layer. The wrapped
	/// `String` is the underlying error message — the factory keeps it
	/// for logging / future propagation; `require_active()` does not
	/// echo it to the client (only the 500 status + a generic message
	/// reaches the response body) to avoid leaking schema details.
	Unavailable(String),
}

#[cfg(native)]
impl SessionUser {
	/// Borrow the active authenticated user, or surface a 401/403/500
	/// `ServerFnError`.
	pub fn require_active(&self) -> std::result::Result<&User, ServerFnError> {
		match self {
			Self::Authenticated(u) => Ok(u),
			Self::Inactive(_) => Err(ServerFnError::server(403, "User account is inactive")),
			Self::Anonymous => Err(ServerFnError::server(401, "Authentication required")),
			Self::Unavailable(_) => Err(ServerFnError::server(
				500,
				"User lookup temporarily unavailable",
			)),
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

	// Distinguish the three outcomes explicitly:
	//
	// - `Ok(Some(u))` — a row exists. Authentication is meaningful;
	//   fall through to the is_active check below.
	// - `Ok(None)` — the session points at a user_id that no longer
	//   exists (deleted account, GDPR purge, …). The session itself
	//   has outlived the user, so the right behaviour is to force
	//   re-auth → **`Anonymous` (401)**.
	// - `Err(e)` — the lookup query itself failed (DB outage, pool
	//   exhaustion, schema drift, etc.). This is an *availability*
	//   problem, not an *authentication* problem, so collapsing it
	//   into `Anonymous` would (a) hide the outage from monitoring
	//   and (b) push callers into a misleading "log in again" loop.
	//   Surface it as **`Unavailable` (500)** instead.
	//
	// `tracing::warn!` logs the underlying error for observability
	// while `require_active()` echoes only a generic 500 message to
	// the client, so schema details do not leak via error bodies.
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
		Ok(None) => return SessionUser::Anonymous,
		Err(e) => {
			::tracing::warn!(
				user_id = user_id,
				error = %e,
				"session_user_factory: user lookup failed"
			);
			return SessionUser::Unavailable(e.to_string());
		}
	};

	if user.is_active {
		SessionUser::Authenticated(user)
	} else {
		SessionUser::Inactive(user)
	}
}

/// Forward-looking newtype that collapses `require_question_author` into
/// a DI-resolvable type once #4645 fully lands (see module docs).
///
/// The shape is intentionally minimal: a verified `Question` whose
/// `author_id` has already been compared against the active session
/// user. Handlers that consume `Depends<AuthoredQuestion>` drop both
/// the `String → i64` parse and the inline 403 check.
///
/// Why no `#[injectable_factory]` here yet: see the "Limitation"
/// section in the module docs. Until form!'s String ABI relaxation
/// (#4397) ships, registering this factory would always fail at runtime
/// inside a `#[server_fn]` request.
#[cfg(native)]
#[allow(
	dead_code,
	reason = "forward-looking newtype — wired up once #4397 + #4645 ship; see module docs"
)]
#[derive(Clone)]
pub struct AuthoredQuestion(pub crate::apps::polls::models::Question);

/// Forward-looking newtype for the Choice mutation handlers.
///
/// Unlike `AuthoredQuestion`, an authored *Choice* needs a two-stage
/// lookup: the URL carries `choice_id`, the `Choice` row carries
/// `question_id`, and ownership lives on the parent `Question`. The
/// factory therefore loads the `Choice` first, then resolves the parent
/// `Question` and verifies authorship — the same flow currently
/// open-coded in `update_choice` / `delete_choice`.
///
/// The `choice` and `question` fields are kept side-by-side so handlers
/// can mutate the `Choice` without re-fetching it.
///
/// Same forward-looking status as [`AuthoredQuestion`]: definition is
/// in place, but no `#[injectable_factory]` is registered until the
/// upstream blockers (#4397, plus a URL `{choice_id}` slot for the
/// Choice mutation routes) clear.
#[cfg(native)]
#[allow(
	dead_code,
	reason = "forward-looking newtype — wired up once #4397 + #4645 ship; see module docs"
)]
#[derive(Clone)]
pub struct AuthoredChoice {
	pub choice: crate::apps::polls::models::Choice,
	pub question: crate::apps::polls::models::Question,
}
