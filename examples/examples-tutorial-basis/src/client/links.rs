//! Typed URL helpers backed by `ResolvedUrls`.
//!
//! Every page-to-page `href` in the SPA goes through this module rather
//! than calling `format!("/polls/{}/", ...)` inline. The helpers delegate
//! to `ResolvedUrls::from_global().resolve_client_url(name, params)`, so
//! if a route pattern ever changes in `apps::<app>::urls::client_router`
//! the components do not need to be updated — only the route definition
//! does.
//!
//! Name convention: route names are namespaced `<app>:<name>` (see
//! `#[url_patterns(InstalledApp::<app>, mode = client)]`); the bare names
//! used by `named_route` are auto-prefixed for the polls and users apps.

use reinhardt::ClientUrlResolver;

use crate::config::urls::ResolvedUrls;

fn urls() -> ResolvedUrls {
	ResolvedUrls::from_global()
}

fn resolve(name: &str, params: &[(&str, &str)]) -> String {
	urls().resolve_client_url(name, params)
}

// ---- polls ---------------------------------------------------------------

/// `/` — polls index.
pub fn polls_index() -> String {
	resolve("polls:index", &[])
}

/// `/polls/new/` — new-question form.
pub fn question_new() -> String {
	resolve("polls:question_new", &[])
}

/// `/polls/{question_id}/` — poll detail / voting page.
pub fn poll_detail(question_id: i64) -> String {
	resolve("polls:detail", &[("question_id", &question_id.to_string())])
}

/// `/polls/{question_id}/edit/` — edit-question form.
pub fn question_edit(question_id: i64) -> String {
	resolve(
		"polls:question_edit",
		&[("question_id", &question_id.to_string())],
	)
}

/// `/polls/{question_id}/delete/` — delete-question confirmation.
pub fn question_delete(question_id: i64) -> String {
	resolve(
		"polls:question_delete",
		&[("question_id", &question_id.to_string())],
	)
}

/// `/polls/{question_id}/results/` — voting results.
pub fn poll_results(question_id: i64) -> String {
	resolve(
		"polls:results",
		&[("question_id", &question_id.to_string())],
	)
}

/// `/polls/{question_id}/choices/new/` — add-choice form.
pub fn choice_new(question_id: i64) -> String {
	resolve(
		"polls:choice_new",
		&[("question_id", &question_id.to_string())],
	)
}

/// `/polls/{question_id}/choices/{choice_id}/edit/` — edit-choice form.
///
/// The parent `question_id` is part of the route so the page can show a
/// synchronous "Cancel" link back to the originating poll without an extra
/// server roundtrip.
pub fn choice_edit(question_id: i64, choice_id: i64) -> String {
	resolve(
		"polls:choice_edit",
		&[
			("question_id", &question_id.to_string()),
			("choice_id", &choice_id.to_string()),
		],
	)
}

/// `/polls/{question_id}/choices/{choice_id}/delete/` — delete-choice
/// confirmation. See [`choice_edit`] for why `question_id` is part of the
/// route.
pub fn choice_delete(question_id: i64, choice_id: i64) -> String {
	resolve(
		"polls:choice_delete",
		&[
			("question_id", &question_id.to_string()),
			("choice_id", &choice_id.to_string()),
		],
	)
}

// ---- users ---------------------------------------------------------------

/// `/users/login/` — sign-in form.
pub fn login() -> String {
	resolve("users:login", &[])
}

/// `/users/logout/` — sign-out form.
pub fn logout() -> String {
	resolve("users:logout", &[])
}

/// `/users/signup/` — account-creation form.
pub fn signup() -> String {
	resolve("users:signup", &[])
}
