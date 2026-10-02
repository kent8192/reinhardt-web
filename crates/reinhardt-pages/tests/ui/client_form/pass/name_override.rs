use reinhardt_core::validators::{Validate, ValidationErrors};
use reinhardt_pages::ClientForm;

#[derive(Clone, PartialEq, ClientForm)]
#[client_form(name = SettingsForm, validate)]
struct SettingsRequest {
	name: String,
}

impl Validate for SettingsRequest {
	fn validate(&self) -> Result<(), ValidationErrors> {
		Ok(())
	}
}

fn main() {
	reinhardt_core::reactive::ReactiveScope::run(|| {
		let _form = SettingsForm::new();
		let _field = SettingsFormField::Name;
	});
}
