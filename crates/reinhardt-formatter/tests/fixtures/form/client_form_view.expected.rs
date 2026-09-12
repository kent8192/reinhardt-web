fn login_page() {
	let _page = form! {
		client_form: auth::LoginClientForm,
		mutation: &mutation,
		id: "login",
		customize: {
			r#type: {
				label: "Type"
			},
			password: {
				widget: PasswordInput,
				autocomplete: "current-password"
			}
		},
		styling: {
			class: styles::auth().as_str()
		},
		submit: {
			label: "Sign in",
			pending_label: "Signing in..."
		},
		summary: {
			label: "Errors"
		},
	};
}
