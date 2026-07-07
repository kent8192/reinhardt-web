use reinhardt_pages::{ClientFormChoiceSource, ClientFormChoices};

#[derive(Clone, Default, PartialEq, ClientFormChoices)]
#[serde(rename_all = "snake_case")]
enum ProviderMode {
	#[default]
	Fake,
	#[serde(rename = "live_api")]
	LiveApi,
	#[serde(alias = "legacy_http_status")]
	HTTPStatus,
	#[serde(skip)]
	Archived,
}

#[derive(Clone, Default, PartialEq, ClientFormChoices)]
#[serde(rename_all = "snake_case")]
enum RawVariantMode {
	#[default]
	Fake,
	// Raw keyword variants are intentionally lower-case to verify serialized values omit `r#`.
	#[allow(non_camel_case_types)]
	r#type,
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
	assert!(!choices
		.iter()
		.any(|choice| choice.serialized_value == "archived"));
	assert!(matches!(ProviderMode::Archived, ProviderMode::Archived));
	assert!(matches!(
		ProviderMode::client_form_default(),
		ProviderMode::Fake
	));

	let choices = RawVariantMode::client_form_choices();
	assert_eq!(choices.len(), 2);
	assert_eq!(choices[0].serialized_value, "fake");
	assert_eq!(choices[1].serialized_value, "type");
}
