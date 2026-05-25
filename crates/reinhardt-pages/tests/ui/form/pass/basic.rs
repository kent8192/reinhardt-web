//! Basic form! macro usage with simple fields

use reinhardt_pages::form;

fn main() {
	// Basic form with required fields
	let _login_form = form! {
		name: LoginForm,
		action: "/api/login",

		fields: {
			username: CharField {
				required,
			}
			password: CharField {
				required,
				widget: PasswordInput,
			}
		}

	};
}
