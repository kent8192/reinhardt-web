//! Shared site navigation bar.
//!
//! Renders a top-of-page bar that links back to the polls index and
//! conditionally shows Login or Logout based on the current session. The
//! session check is performed via the `current_user` server function and
//! the result is bound through `use_action`, so the bar reactively updates
//! once the WASM client finishes its first roundtrip.
//!
//! All `href` values are resolved through the typed `urls` modules emitted
//! by `#[url_patterns]` (issue #4656): `apps::polls::urls::client_router::urls`
//! and `apps::users::urls::client_router::urls`. The macro-generated
//! helpers wrap `ResolvedUrls::from_global()` internally; the reverser is
//! registered in `client::lib::main`.

// `nav_bar` crosses app boundaries: the home link belongs to the polls
// app, and the login/logout/signup links belong to the users app. Pulling
// each from its owning app's macro-emitted `urls` module makes that
// coupling explicit.
use crate::apps::polls::urls::client_router::urls as polls_links;
use crate::apps::users::urls::client_router::urls as users_links;
use crate::shared::types::UserInfo;
use reinhardt::pages::component::Page;
use reinhardt::pages::page;
use reinhardt::pages::reactive::hooks::{Action, use_action};

use crate::apps::users::server_fn::current_user;

/// Top navigation bar used by every page in the polls SPA.
///
/// Layout: left side is a "Polls" home link, right side switches between a
/// "Login" link (when no session is present) and a "username · Logout"
/// pair (when the session resolves to a user). While the lookup is
/// pending the auth area is left blank to avoid an
/// authenticated/unauthenticated flicker during hydration.
pub fn nav_bar() -> Page {
	let load_user =
		use_action(|_: ()| async move { current_user().await.map_err(|e| e.to_string()) });
	load_user.dispatch(());

	let auth_signal = load_user.clone();
	let polls_index_href = polls_links::index();
	let login_href = users_links::login();
	let logout_href = users_links::logout();
	let signup_href = users_links::signup();

	page!(|auth_signal: Action<Option<UserInfo>, String>, polls_index_href: String, login_href: String, logout_href: String, signup_href: String| {
		nav {
			class: "nav-bar",
			a {
				href: polls_index_href,
				class: "font-bold text-lg text-content-primary",
				"Polls"
			}
			watch {
				if auth_signal.is_pending() {
					div {
						class: "flex items-center gap-3",
						aria_busy: "true",
						span {
							class: "sr-only",
							"Checking sign-in status"
						}
						span {
							class: "h-4 w-32 rounded bg-surface-tertiary animate-pulse",
							aria_hidden: "true",
						}
						span {
							class: "h-9 w-20 rounded bg-surface-tertiary animate-pulse",
							aria_hidden: "true",
						}
					}
				} else if let Some(Some(user)) = auth_signal.result() {
					div {
						class: "flex items-center gap-3",
						span {
							class: "text-sm text-muted",
							{
								format!("Signed in as {}", user.username)
							}
						}
						a {
							href: logout_href.clone(),
							class: "btn-secondary",
							"Logout"
						}
					}
				} else {
					div {
						class: "flex items-center gap-2",
						a {
							href: signup_href.clone(),
							class: "btn-secondary",
							"Sign up"
						}
						a {
							href: login_href.clone(),
							class: "btn-primary",
							"Login"
						}
					}
				}
			}
		}
	})(
		auth_signal,
		polls_index_href,
		login_href,
		logout_href,
		signup_href,
	)
}

/// Compose the shared navigation bar above a page body.
///
/// Each page exposes a single `Page` and wraps its top-level expression
/// with this helper to inherit the same header without restructuring the
/// body itself. Early-return branches (loading / error states) can wrap
/// their return value the same way.
pub fn with_nav(body: Page) -> Page {
	Page::Fragment(vec![nav_bar(), body])
}
