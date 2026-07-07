use reinhardt_pages::{ClientFormChoiceSource, ClientFormChoices};

#[derive(Clone, Default, PartialEq, ClientFormChoices)]
#[serde(
	rename_all = "snake_case",
	crate = "serde",
	bound = "",
	deny_unknown_fields
)]
enum ProviderMode {
	#[default]
	LiveApi,
	TestHarness,
}

fn main() {
	let _choices = ProviderMode::client_form_choices();
}
