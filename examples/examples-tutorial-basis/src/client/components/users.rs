//! Authentication UI components for the tutorial-basis example.
//!
//! Provides minimal login and logout pages backed by the `users` server
//! functions. The login form uses the `form!` macro to bind fields and
//! attach the CSRF token automatically.

use reinhardt::pages::component::Page;
use reinhardt::pages::form;
use reinhardt::pages::page;
use reinhardt::pages::reactive::Signal;

use crate::client::links;
#[cfg(wasm)]
use crate::server_fn::users::{login, logout};

/// Login page: username + password form posting to the `login` server function.
///
/// On success, redirects to the polls index. Field bindings, loading state,
/// and CSRF token are managed by the `form!` macro.
pub fn login_form() -> Page {
	let login_form = form! {
		name: LoginForm,
		server_fn: login,
		state: { loading, error },
		redirect_on_success: "/",

		fields: {
			username: CharField {
				label: "Username",
				placeholder: "your-username",
				max_length: 150,
				class: "form-control",
			},
			password: PasswordField {
				label: "Password",
				placeholder: "Enter your password",
				class: "form-control",
			},
		},
	};

	let loading_signal = login_form.loading().clone();
	let error_signal = login_form.error().clone();
	let form_view = login_form.into_page();
	let polls_index_href = links::polls_index();

	page!(|loading_signal: Signal<bool>, error_signal: Signal<Option<String>>, form_view: Page, polls_index_href: String| {
		div {
			class: "container mt-5",
			div {
				class: "row justify-content-center",
				div {
					class: "col-md-6",
					div {
						class: "card",
						div {
							class: "card-body",
							h1 {
								class: "card-title mb-4",
								"Sign in"
							}
							watch {
								if error_signal.get().is_some() {
									div {
										class: "alert alert-danger",
										{ error_signal.get().unwrap_or_default() }
									}
								}
							}
							{ form_view }
							div {
								class: "mt-3",
								watch {
									if loading_signal.get() {
										button {
											type: "submit",
											class: "btn btn-primary w-100",
											disabled: loading_signal.get(),
											form: "login-form",
											"Signing in..."
										}
									} else {
										button {
											type: "submit",
											class: "btn btn-primary w-100",
											form: "login-form",
											"Sign in"
										}
									}
								}
							}
						}
					}
					div {
						class: "text-center mt-3",
						a {
							href: polls_index_href,
							class: "text-muted",
							"Back to polls"
						}
					}
				}
			}
		}
	})(loading_signal, error_signal, form_view, polls_index_href)
}

/// Logout page: presents a single button that invokes the `logout` server fn
/// and redirects to the polls index on success.
pub fn logout_form() -> Page {
	let logout_form = form! {
		name: LogoutForm,
		server_fn: logout,
		state: { loading, error },
		redirect_on_success: "/",
		fields: {},
	};

	let error_signal = logout_form.error().clone();
	let form_view = logout_form.into_page();
	let polls_index_href = links::polls_index();

	page!(|error_signal: Signal<Option<String>>, form_view: Page, polls_index_href: String| {
		div {
			class: "container mt-5",
			div {
				class: "row justify-content-center",
				div {
					class: "col-md-6",
					div {
						class: "card",
						div {
							class: "card-body",
							h1 {
								class: "card-title mb-4",
								"Sign out"
							}
							p {
								class: "card-text",
								"Click the button below to end your session."
							}
							watch {
								if error_signal.get().is_some() {
									div {
										class: "alert alert-danger",
										{ error_signal.get().unwrap_or_default() }
									}
								}
							}
							{ form_view }
							button {
								type: "submit",
								class: "btn btn-secondary",
								form: "logout-form",
								"Sign out"
							}
						}
					}
					div {
						class: "text-center mt-3",
						a {
							href: polls_index_href,
							class: "text-muted",
							"Back to polls"
						}
					}
				}
			}
		}
	})(error_signal, form_view, polls_index_href)
}
