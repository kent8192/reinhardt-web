use reinhardt_pages::use_form;

mod forms {
	use reinhardt_pages::ClientForm;

	#[derive(Clone, Default)]
	pub struct HiddenToken {
		pub value: String,
	}

	#[derive(Clone, ClientForm)]
	pub struct SettingsRequest {
		pub name: String,
		#[client_form(skip)]
		pub token: HiddenToken,
	}
}

fn main() {
	let form = forms::SettingsRequestClientForm::new().with_defaults(forms::SettingsRequest {
		name: "demo".to_string(),
		token: forms::HiddenToken {
			value: "secret".to_string(),
		},
	});
	let runtime = use_form(&form).build();
	let mut values = runtime.get_values();
	assert_eq!(values.name, "demo");
	assert_eq!(values.token.value, "secret");
	values.name = "updated".to_string();
	runtime.set_values(values);
}
