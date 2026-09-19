use reinhardt::model;
use reinhardt::pages::form;
use reinhardt::pages::server_fn::{ServerFnError, server_fn};

#[model(
	app_label = "profiles",
	table_name = "profiles",
	form(name = ProfileCreateForm, fields(name, enabled)),
	info = false
)]
#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct Profile {
	#[field(primary_key = true)]
	pub id: i64,
	pub name: String,
	#[field(default = false)]
	pub enabled: bool,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ProfileResponse {
	pub token: String,
}

#[server_fn(model_form = true)]
pub async fn save_profile(
	payload: ProfileCreateFormData,
) -> Result<ProfileResponse, ServerFnError> {
	let _ = payload;
	Ok(ProfileResponse {
		token: "saved".to_owned(),
	})
}

pub fn compile_facade_form() {
	let form = form! {
		name: ProfileForm,
		model_form: ProfileCreateForm,
		server_fn: save_profile,
	};
	let _payload: ProfileCreateFormData = form.data().expect("empty payload is valid");
	let _typed_submit = async {
		let response = form
			.submit_response()
			.await
			.expect("the generated response type is concrete");
		let _: ProfileResponse = response;
	};
}

pub fn compile_facade_mutation_page() {
	reinhardt::pages::reactive::ReactiveScope::run(|| {
		let form = form! {
			name: ProfileMutationForm,
			model_form: ProfileCreateForm,
			server_fn: save_profile,
		};
		let runtime = reinhardt::pages::use_form(&form).build();
		let action = form
			.server_mutation(&runtime)
			.reset_form_on_success()
			.build();
		let _: reinhardt::pages::Page = action.page();
		let _: Option<ProfileResponse> = action.result();
		let _pending: bool = action.is_pending();
		action.reset();
	});
}

/// The shared patch validator has no browser database dependency.
pub fn compile_patch_validation() {
	use reinhardt::pages::form::ModelFormPatchPayload;
	let raw: ProfileCreateFormData =
		serde_json::from_value(serde_json::json!({"enabled": false})).unwrap();
	let cleaned = raw.clean_and_validate_patch(None).unwrap();
	assert_eq!(
		serde_json::to_value(cleaned.into_raw()).unwrap(),
		serde_json::json!({"enabled": false})
	);
}

#[cfg(feature = "patch-persistence-must-not-compile")]
pub fn browser_cannot_create_persistence_capability() {
	let _ = ProfileCreateForm::validate_patch(ProfileCreateFormData::default());
}

#[model(app_label = "profiles", info = false, form(name = EditWindow, fields(start, end)))]
#[derive(Clone, serde::Deserialize, serde::Serialize)]
#[form(validate = window_order)]
pub struct Window {
	#[field(primary_key = true)]
	pub id: i64,
	pub start: i64,
	pub end: i64,
}

fn window_order<P: reinhardt::pages::form::ModelFormPolicy>(
	data: &CleanedWindowModelFormData<P>,
) -> Result<(), reinhardt::reinhardt_core::validators::ValidationErrors> {
	use reinhardt::reinhardt_core::validators::{ValidationError, ValidationErrors};
	let mut errors = ValidationErrors::new();
	if data
		.start()
		.zip(data.end())
		.is_none_or(|(start, end)| start > end)
	{
		errors.add(
			"start",
			ValidationError::Custom("invalid window".to_owned()),
		);
	}
	if errors.is_empty() {
		Ok(())
	} else {
		Err(errors)
	}
}

pub fn compile_contextual_patch() {
	use reinhardt::pages::form::ModelFormPatchPayload;
	let raw: EditWindowData = serde_json::from_value(serde_json::json!({"start":2})).unwrap();
	let existing: EditWindowData =
		serde_json::from_value(serde_json::json!({"start":1,"end":3})).unwrap();
	let cleaned = raw.clean_and_validate_patch(Some(&existing)).unwrap();
	assert_eq!(
		serde_json::to_value(cleaned.into_raw()).unwrap(),
		serde_json::json!({"start":2})
	);
}
