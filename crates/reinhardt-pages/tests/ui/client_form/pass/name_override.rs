use reinhardt_core::validators::{Validate, ValidationErrors};
use reinhardt_pages::ClientForm;

#[derive(Clone, PartialEq, ClientForm)]
#[client_form(name = SettingsForm)]
struct SettingsRequest {
	name: String,
}

impl Validate for SettingsRequest {
	fn validate(&self) -> Result<(), ValidationErrors> {
		Ok(())
	}
}

fn main() {
	let _form = SettingsForm::new();
	let _field = SettingsFormField::Name;
}
