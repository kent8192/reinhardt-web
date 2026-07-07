use reinhardt_pages::{ClientForm, use_form};

#[derive(Clone, ClientForm)]
pub struct SettingsRequest {
	pub name: String,
	secret: String,
}

fn main() {
	let form = SettingsRequestClientForm::new();
	let runtime = use_form(&form).build();
	runtime.set_value(SettingsRequestClientFormField::Secret, "leak".to_string());
}
