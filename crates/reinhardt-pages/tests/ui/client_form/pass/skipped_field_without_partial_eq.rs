use reinhardt_pages::{ClientForm, use_form};

#[derive(Clone, Default)]
struct HiddenToken {
	value: String,
}

#[derive(Clone, ClientForm)]
struct TokenRequest {
	name: String,
	#[client_form(skip)]
	token: HiddenToken,
}

fn main() {
	let form = TokenRequestClientForm::new().with_defaults(TokenRequest {
		name: "demo".to_string(),
		token: HiddenToken {
			value: "secret".to_string(),
		},
	});
	let runtime = use_form(&form).build();
	let request = TokenRequestClientForm::to_request(&runtime);
	assert_eq!(request.token.value, "secret");
}
