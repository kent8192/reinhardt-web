use reinhardt_pages::{
	ClientForm, ClientFormChoiceSource, ClientFormChoices, FormRuntimeSource, use_form,
};

#[derive(Clone, Default, PartialEq, ClientFormChoices)]
#[serde(rename_all = "snake_case")]
enum ProviderMode {
	#[default]
	Fake,
	LiveApi,
}

#[derive(Clone, PartialEq, ClientForm)]
struct ProfileRequest {
	name: String,
	title: Option<String>,
	count: i32,
	optional_count: Option<i32>,
	active: bool,
	optional_active: Option<bool>,
	provider_mode: ProviderMode,
	optional_mode: Option<ProviderMode>,
}

fn main() {
	let form = ProfileRequestClientForm::new();
	let runtime = use_form(&form).build();
	runtime.set_value(ProfileRequestClientFormField::Title, "  ".to_string());
	let request = ProfileRequestClientForm::to_request(&runtime);
	assert_eq!(request.title, None);
	assert_eq!(
		form.provider_mode_choices().len(),
		ProviderMode::client_form_choices().len()
	);
	assert_eq!(
		FormRuntimeSource::runtime_fields(&form),
		&[
			ProfileRequestClientFormField::Name,
			ProfileRequestClientFormField::Title,
			ProfileRequestClientFormField::Count,
			ProfileRequestClientFormField::OptionalCount,
			ProfileRequestClientFormField::Active,
			ProfileRequestClientFormField::OptionalActive,
			ProfileRequestClientFormField::ProviderMode,
			ProfileRequestClientFormField::OptionalMode,
		]
	);
}
