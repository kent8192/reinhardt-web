use reinhardt_pages::{ClientFormChoiceSource, ClientFormChoices};

#[derive(Clone, Default, PartialEq, ClientFormChoices)]
#[serde(rename_all = "snake_case")]
enum ProviderMode {
	#[default]
	Fake,
	#[serde(rename = "live_api")]
	LiveApi,
	HTTPStatus,
}

fn main() {
	let choices = ProviderMode::client_form_choices();
	assert_eq!(choices.len(), 3);
	assert_eq!(choices[0].serialized_value, "fake");
	assert_eq!(choices[0].label, "fake");
	assert_eq!(choices[1].serialized_value, "live_api");
	assert_eq!(choices[1].label, "live_api");
	assert_eq!(choices[2].serialized_value, "h_t_t_p_status");
	assert_eq!(choices[2].label, "h_t_t_p_status");
	assert!(matches!(
		ProviderMode::client_form_default(),
		ProviderMode::Fake
	));
}
