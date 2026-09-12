use crate::pages_api::{component::Page, form, page, use_form};
use auth_dto::dto::{LoginRequestClientForm, RegisterRequestClientForm};

pub fn handwritten_login(on_success: impl Fn() + 'static) -> Page {
	let definition = LoginRequestClientForm::new();
	let runtime = use_form(&definition)
		.on_submit_success(move |_| on_success())
		.build();
	let mutation = definition.server_mutation(&runtime).build();
	page!({
		form {
			novalidate: true,
			@submit: move |event| {
				event.prevent_default();
				mutation.dispatch();
			},
			label {
				r#for: "login-username",
				"Username"
			}
			input {
				id: "login-username",
				name: "username",
				autocomplete: "username",
				bind: runtime.field(definition.username_field())
			}
			label {
				r#for: "login-password",
				"Password"
			}
			input {
				id: "login-password",
				name: "password",
				type: "password",
				autocomplete: "current-password",
				bind: runtime.field(definition.password_field())
			}
			button {
				type: "submit",
				"Sign in"
			}
		}
	})
}

pub fn login(on_success: impl Fn() + 'static) -> Page {
	let definition = LoginRequestClientForm::new();
	let runtime = use_form(&definition)
		.on_submit_success(move |_| on_success())
		.build();
	let mutation = definition.server_mutation(&runtime).build();
	form! {
		client_form: LoginRequestClientForm,
		mutation: &mutation,
		id: "login",
		customize: {
			username: {
				label: "Username",
				autocomplete: "username"
			},
			password: {
				widget: PasswordInput,
				autocomplete: "current-password"
			},
		},
		submit: {
			label: "Sign in",
			pending_label: "Signing in..."
		},
	}
	.into_page()
}
pub fn register(on_success: impl Fn() + 'static) -> Page {
	let definition = RegisterRequestClientForm::new();
	let runtime = use_form(&definition)
		.on_submit_success(move |_| on_success())
		.build();
	let mutation = definition.server_mutation(&runtime).build();
	form! {
		client_form: RegisterRequestClientForm,
		mutation: &mutation,
		id: "register",
		customize: {
			username: {
				autocomplete: "username"
			},
			email: {
				widget: EmailInput,
				autocomplete: "email"
			},
			password: {
				widget: PasswordInput,
				autocomplete: "new-password"
			},
		},
		submit: {
			label: "Create account",
			pending_label: "Creating account..."
		},
	}
	.into_page()
}
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
	#[test]
	fn cloud_shaped_pages_render_without_wrapper_endpoints() {
		crate::pages_api::reactive::ReactiveScope::run(|| {
			let handwritten = super::handwritten_login(|| {}).render_to_string();
			let login = super::login(|| {}).render_to_string();
			let register = super::register(|| {}).render_to_string();
			assert_eq!(handwritten.matches("name=\"username\"").count(), 1);
			assert_eq!(handwritten.matches("type=\"password\"").count(), 1);
			assert_eq!(login.matches("name=\"username\"").count(), 1);
			assert_eq!(register.matches("type=\"email\"").count(), 1);
			assert_eq!(register.matches("name=\"password\"").count(), 1);
		});
	}
}
