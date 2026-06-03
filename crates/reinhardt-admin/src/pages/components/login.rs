//! Login component for Reinhardt Admin Panel
//!
//! Provides a login form that authenticates admin users via JWT.

use reinhardt_pages::component::Page;
use reinhardt_pages::form;
use reinhardt_pages::page;

/// Login form component
///
/// Renders a login form with username and password fields.
/// On successful authentication, the server sets a JWT HTTP-Only cookie
/// and the client updates the reactive auth state.
///
/// # Example
///
/// ```ignore
/// use reinhardt_admin::pages::components::login::login_form;
///
/// let page = login_form(None);
/// ```
pub fn login_form(error_message: Option<&str>) -> Page {
	let error_html = error_message.map(|msg| {
		let msg = msg.to_string();
		page!(|msg: String| {
			div {
				class: "admin-alert admin-alert-danger mt-4 text-center text-sm",
				role: "alert",
				{ msg }
			}
		})(msg)
	});

	let form_page = build_login_form();
	let error_page = error_html.unwrap_or_else(|| page!(|| { span {} })());

	page!(|form_page: Page, error_page: Page| {
		div {
			class: "flex justify-center items-center min-h-screen bg-slate-50 animate__animated animate__fadeIn",
			div {
				class: "admin-login-card",
				div {
					class: "p-8",
					h2 {
						class: "font-display text-2xl font-bold text-center mb-1 text-slate-900",
						"Admin Login"
					}
					p {
						class: "text-sm text-slate-500 text-center mb-6",
						"Sign in to manage your application"
					}
					{ form_page }
				}
			}
			{ error_page }
		}
	})(form_page, error_page)
}

/// Builds the login form HTML structure using the `form!` macro.
///
/// The struct name `AdminLoginForm` generates `id="admin-login-form"` on the
/// form element. The `server_fn: admin_login_with_header` directive auto-generates the
/// submit handler, replacing the manual `setup_login_handler()`.
///
/// JWT token storage is handled server-side via HTTP-Only cookie, so no
/// client-side token storage is needed.
fn build_login_form() -> Page {
	#[allow(unused_imports)]
	use crate::server::login::admin_login_with_header;

	let login_form = form! {
		name: AdminLoginForm,
		server_fn: admin_login_with_header,
		method: Post,
		redirect_on_success: "/admin/",
		fields: {
			username: CharField {
				required,
				label: "Username",
				label_class: "admin-label",
				wrapper_class: "mb-4",
				class: "admin-input",
				autocomplete: "username",
				autofocus,
				placeholder: "Enter your username",
			}
			password: CharField {
				required,
				widget: PasswordInput,
				label: "Password",
				label_class: "admin-label",
				wrapper_class: "mb-5",
				class: "admin-input",
				autocomplete: "current-password",
				placeholder: "Enter your password",
			}
		}
		watch: {
			login_error: |form| {
				let error = form.error().get();
				let has_error = error.is_some();
				let error_message = error.unwrap_or_default();
				page!(|has_error: bool, error_message: String| {
					div {
						id : "login-error",
						class : if has_error { "admin-alert admin-alert-danger mb-4" } else { "hidden" },
						role : "alert",
						{ error_message }
					}
				})(has_error, error_message)
			},
		}
		slots: {
			after_fields: | | page!(|| {
				button {
					type : "submit",
					class : "admin-btn admin-btn-primary w-full py-2.5 text-base",
					id : "login-submit-btn",
					"Sign in"
				}
			})(),
		}
	};

	login_form.into_page()
}

/// Login view component.
///
/// The `form!` macro with `server_fn: admin_login` auto-generates the submit
/// handler, so no separate `setup_login_handler()` is needed.
pub fn login_view() -> Page {
	login_form(None)
}

#[cfg(all(test, server))]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_login_form_renders() {
		// Arrange & Act
		let page = login_form(None);
		let html = page.render_to_string();

		// Assert
		assert!(html.contains("Admin Login"));
		assert!(html.contains("username"));
		assert!(html.contains("password"));
		assert!(html.contains("Sign in"));
	}

	#[rstest]
	fn test_login_form_with_error() {
		// Arrange & Act
		let page = login_form(Some("Invalid credentials"));
		let html = page.render_to_string();

		// Assert
		assert!(html.contains("Invalid credentials"));
		assert!(html.contains("admin-alert-danger"));
	}

	#[rstest]
	fn test_login_view_renders() {
		// Arrange & Act
		let page = login_view();
		let html = page.render_to_string();

		// Assert
		assert!(html.contains("admin-login-form"));
	}
}
