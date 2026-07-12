#![deny(private_interfaces)]

use reinhardt_pages::{ClientForm, use_form};

mod forms {
	use super::*;

	#[derive(Clone, Default)]
	struct HiddenToken {
		value: String,
	}

	#[derive(Clone, ClientForm)]
	pub struct SettingsRequest {
		pub name: String,
		#[client_form(skip)]
		token: HiddenToken,
	}

	pub fn exercise() {
		let form = SettingsRequestClientForm::new().with_defaults(SettingsRequest {
			name: "demo".to_string(),
			token: HiddenToken {
				value: "secret".to_string(),
			},
		});
		let runtime = use_form(&form).build();
		let request = SettingsRequestClientForm::to_request(&runtime);
		assert_eq!(request.token.value, "secret");
	}
}

fn main() {
	reinhardt_core::reactive::ReactiveScope::run(|| {
		forms::exercise();
	});
}
