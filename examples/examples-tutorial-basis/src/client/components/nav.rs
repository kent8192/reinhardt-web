//! Shared site navigation bar.
//!
//! Renders a top-of-page bar that links back to the polls index and
//! conditionally shows Login or Logout based on the current session. The
//! session check is performed via the `current_user` server function and
//! the result is bound through `use_action`, so the bar reactively updates
//! once the WASM client finishes its first roundtrip.
//!
//! All `href` values are resolved through `client::links`, which delegates
//! to `ResolvedUrls::from_global()`. The reverser is registered in
//! `client::lib::main`.

use crate::client::links;
use crate::shared::types::UserInfo;
use reinhardt::pages::component::Page;
use reinhardt::pages::page;
use reinhardt::pages::reactive::hooks::{Action, use_action};

use crate::server_fn::users::current_user;

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
	let polls_index_href = links::polls_index();
	let login_href = links::login();
	let logout_href = links::logout();

	page!(|auth_signal: Action<Option<UserInfo>, String>, polls_index_href: String, login_href: String, logout_href: String| {
		nav {
			class: "max-w-4xl mx-auto px-4 pt-4 mb-4 flex justify-between items-center border-b pb-2",
			a {
				href: polls_index_href,
				class: "font-bold text-lg",
				"Polls"
			}
			watch {
				if auth_signal.is_pending() {
					span {
						class: "text-gray-400 text-sm",
						""
					}
				} else if let Some(Some(user)) = auth_signal.result() {
					div {
						class: "flex items-center gap-3",
						span {
							class: "text-sm text-gray-600",
							{ format!("Signed in as {}", user.username) }
						}
						a {
							href: logout_href.clone(),
							class: "btn-secondary",
							"Logout"
						}
					}
				} else {
					a {
						href: login_href.clone(),
						class: "btn-primary",
						"Login"
					}
				}
			}
		}
	})(auth_signal, polls_index_href, login_href, logout_href)
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
