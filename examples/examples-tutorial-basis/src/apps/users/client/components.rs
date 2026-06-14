//! Authentication UI components for the tutorial-basis example.
//!
//! Provides minimal login / logout / sign-up pages backed by the `users`
//! server functions. Every form uses the `form!` macro to define static fields
//! while `#[server_fn]` client stubs attach the CSRF header automatically.
use crate::apps::polls::urls::client_router as polls_routes;
#[cfg(client)]
use crate::apps::users::server_fn::{login, logout, register};
use crate::apps::users::urls::client_router as users_routes;
use reinhardt::pages::component::Page;
use reinhardt::pages::form;
use reinhardt::pages::page;
use reinhardt::pages::reactive::Signal;

/// Login page: username + password form posting to the `login` server function.
///
/// On success, redirects to the polls index. Field bindings and CSRF header
/// plumbing are managed by the form/server_fn integration.
pub fn login_form() -> Page {
	let login_form = form! {
		name: LoginForm,
		server_fn: login,
		method: Post,
		redirect_on_success: "/",
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
	let polls_index_href = polls_routes::reverse("index", &[]);
	let signup_href = users_routes::reverse("signup", &[]);

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
					{
						error_signal.get().map(|message| page!(|message: String| {
							div {
								class: "alert-danger mb-3",
								{ message }
							}
						})(message)).unwrap_or(Page::Empty)
					}
					{ form_view }
					div {
						class: "mt-4",
						{
							let is_loading = loading_signal.get();
							page!(|is_loading: bool| {
								button {
									type: "submit",
									class: if is_loading { "btn-primary w-full opacity-50 cursor-not-allowed" } else { "btn-primary w-full" },
									disabled: is_loading,
									form: "login-form",
									{
										if is_loading { "Signing in..." } else { "Sign in" }
									}
								}
							})(is_loading)
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
		method: Post,
		redirect_on_success: "/",
		fields: {}
	};
	let error_signal = logout_form.error().clone();
	let form_view = logout_form.into_page();
	let polls_index_href = polls_routes::reverse("index", &[]);

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
					{
						error_signal.get().map(|message| page!(|message: String| {
							div {
								class: "alert-danger mb-3",
								{ message }
							}
						})(message)).unwrap_or(Page::Empty)
					}
					{ form_view }
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
/// hands the user to the polls index. Field bindings and CSRF header plumbing
/// are handled by the form/server_fn integration.
pub fn signup_form() -> Page {
	let signup_form = form! {
		name: SignupForm,
		server_fn: register,
		method: Post,
		redirect_on_success: "/",
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
	let polls_index_href = polls_routes::reverse("index", &[]);
	let login_href = users_routes::reverse("login", &[]);

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
					{
						error_signal.get().map(|message| page!(|message: String| {
							div {
								class: "alert-danger mb-3",
								{ message }
							}
						})(message)).unwrap_or(Page::Empty)
					}
					{ form_view }
					div {
						class: "mt-4",
						{
							let is_loading = loading_signal.get();
							page!(|is_loading: bool| {
								button {
									type: "submit",
									class: if is_loading { "btn-primary w-full opacity-50 cursor-not-allowed" } else { "btn-primary w-full" },
									disabled: is_loading,
									form: "signup-form",
									{
										if is_loading { "Creating account..." } else { "Create account" }
									}
								}
							})(is_loading)
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
