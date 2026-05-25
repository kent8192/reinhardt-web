//! Authentication UI components for the tutorial-basis example.
//!
//! Provides minimal login / logout / sign-up pages backed by the `users`
//! server functions. Every form uses the `form!` macro to bind fields and
//! attach the CSRF token automatically.

use reinhardt::pages::component::Page;
use reinhardt::pages::form;
use reinhardt::pages::page;
use reinhardt::pages::reactive::Signal;

#[cfg(wasm)]
use crate::apps::users::server_fn::{login, logout, register};
// Typed URL helpers are now emitted by `#[url_patterns]` directly
// (issue #4656). We alias them locally as `polls_links` / `links` so the
// users-app's own login/logout/signup call sites stay concise, while
// the cross-app reference (now `polls_links::index()`, previously the
// hand-written `polls_index()` wrapper) remains explicit and greppable.
use crate::apps::polls::urls::client_router::urls as polls_links;
use crate::apps::users::urls::client_router::urls as links;

/// Login page: username + password form posting to the `login` server function.
///
/// On success, redirects to the polls index. Field bindings, loading state,
/// and CSRF token are managed by the `form!` macro.
pub fn login_form() -> Page {
	let login_form = form! {
		name: LoginForm,
		server_fn: login,
		redirect_on_success: "/",
		state: {
			loading,
			error,
		}
		fields: {
			username: CharField {
				label: "Username",
				placeholder: "your-username",
				max_length: 150,
				class: "form-control",
			}
			password: PasswordField {
				label: "Password",
				placeholder: "Enter your password",
				class: "form-control",
			}
		}
	};

	let loading_signal = login_form.loading().clone();
	let error_signal = login_form.error().clone();
	let form_view = login_form.into_page();
	let polls_index_href = polls_links::index();
	let signup_href = links::signup();

	page!(|loading_signal: Signal<bool>, error_signal: Signal<Option<String>>, form_view: Page, polls_index_href: String, signup_href: String| {
		div {
			class: "max-w-md mx-auto px-4 mt-12",
			div {
				class: "card",
				div {
					class: "card-body",
					h1 {
						class: "card-title",
						"Sign in"
					}
					watch {
						if error_signal.get().is_some() {
							div {
								class: "alert-danger mb-3",
								{
									error_signal.get().unwrap_or_default()
								}
							}
						}
					}
					{
						form_view
					}
					div {
						class: "mt-4",
						watch {
							if loading_signal.get() {
								button {
									type: "submit",
									class: "btn-primary w-full",
									disabled: loading_signal.get(),
									form: "login-form",
									"Signing in..."
								}
							}
							else {
								button {
									type: "submit",
									class: "btn-primary w-full",
									form: "login-form",
									"Sign in"
								}
							}
						}
					}
				}
			}
			div {
				class: "text-center mt-4 flex flex-col gap-1",
				a {
					href: signup_href,
					class: "text-brand",
					"Create an account"
				}
				a {
					href: polls_index_href,
					class: "text-muted",
					"Back to polls"
				}
			}
		}
	})(
		loading_signal,
		error_signal,
		form_view,
		polls_index_href,
		signup_href,
	)
}

/// Logout page: presents a single button that invokes the `logout` server fn
/// and redirects to the polls index on success.
pub fn logout_form() -> Page {
	let logout_form = form! {
		name: LogoutForm,
		server_fn: logout,
		redirect_on_success: "/",
		state: {
			loading,
			error,
		}
		fields: {
		}
	};

	let error_signal = logout_form.error().clone();
	let form_view = logout_form.into_page();
	let polls_index_href = polls_links::index();

	page!(|error_signal: Signal<Option<String>>, form_view: Page, polls_index_href: String| {
		div {
			class: "max-w-md mx-auto px-4 mt-12",
			div {
				class: "card",
				div {
					class: "card-body",
					h1 {
						class: "card-title",
						"Sign out"
					}
					p {
						class: "text-muted mb-4",
						"Click the button below to end your session."
					}
					watch {
						if error_signal.get().is_some() {
							div {
								class: "alert-danger mb-3",
								{
									error_signal.get().unwrap_or_default()
								}
							}
						}
					}
					{
						form_view
					}
					button {
						type: "submit",
						class: "btn-secondary w-full",
						form: "logout-form",
						"Sign out"
					}
				}
			}
			div {
				class: "text-center mt-4",
				a {
					href: polls_index_href,
					class: "text-muted",
					"Back to polls"
				}
			}
		}
	})(error_signal, form_view, polls_index_href)
}

/// Sign-up page: username + password (confirmed) form posting to the
/// `register` server function.
///
/// On success, the server rotates the session and persists `user_id`, so the
/// new account is logged in immediately; `redirect_on_success: "/"` then
/// hands the user to the polls index. Field bindings, loading state, and
/// CSRF token plumbing are handled by the `form!` macro.
pub fn signup_form() -> Page {
	let signup_form = form! {
		name: SignupForm,
		server_fn: register,
		redirect_on_success: "/",
		state: {
			loading,
			error,
		}
		fields: {
			username: CharField {
				label: "Username",
				placeholder: "choose-a-username",
				max_length: 150,
				class: "form-control",
			}
			password: PasswordField {
				label: "Password",
				placeholder: "At least 8 characters",
				class: "form-control",
			}
			password_confirmation: PasswordField {
				label: "Confirm password",
				placeholder: "Re-enter the password",
				class: "form-control",
			}
		}
	};

	let loading_signal = signup_form.loading().clone();
	let error_signal = signup_form.error().clone();
	let form_view = signup_form.into_page();
	let polls_index_href = polls_links::index();
	let login_href = links::login();

	page!(|loading_signal: Signal<bool>, error_signal: Signal<Option<String>>, form_view: Page, polls_index_href: String, login_href: String| {
		div {
			class: "max-w-md mx-auto px-4 mt-12",
			div {
				class: "card",
				div {
					class: "card-body",
					h1 {
						class: "card-title",
						"Create account"
					}
					watch {
						if error_signal.get().is_some() {
							div {
								class: "alert-danger mb-3",
								{
									error_signal.get().unwrap_or_default()
								}
							}
						}
					}
					{
						form_view
					}
					div {
						class: "mt-4",
						watch {
							if loading_signal.get() {
								button {
									type: "submit",
									class: "btn-primary w-full",
									disabled: loading_signal.get(),
									form: "signup-form",
									"Creating account..."
								}
							}
							else {
								button {
									type: "submit",
									class: "btn-primary w-full",
									form: "signup-form",
									"Create account"
								}
							}
						}
					}
				}
			}
			div {
				class: "text-center mt-4 flex flex-col gap-1",
				a {
					href: login_href,
					class: "text-brand",
					"Already have an account? Sign in"
				}
				a {
					href: polls_index_href,
					class: "text-muted",
					"Back to polls"
				}
			}
		}
	})(
		loading_signal,
		error_signal,
		form_view,
		polls_index_href,
		login_href,
	)
}
