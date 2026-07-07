#![cfg(not(target_arch = "wasm32"))]

use std::cell::Cell;

use reinhardt_core::validators::{Validate, ValidationError, ValidationErrors};
use reinhardt_pages::server_fn::ServerFnError;
use reinhardt_pages::server_fn::server_fn;
use reinhardt_pages::{
	ClientForm, ClientFormChoiceSource, ClientFormChoices, FieldError, ResetOnDeps,
	UseFormAsyncSubmitOutcome, use_form,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Debug, PartialEq, ClientFormChoices)]
#[serde(rename_all = "snake_case")]
enum ProviderMode {
	#[default]
	Fake,
	LiveApi,
	HTTPStatus,
	#[serde(skip)]
	Archived,
}

#[derive(Clone, Debug, PartialEq, ClientForm)]
#[client_form(validate)]
struct ProjectRequest {
	name: String,
	title: Option<String>,
	retry_count: i32,
	optional_retry_count: Option<i32>,
	active: bool,
	optional_active: Option<bool>,
	provider_mode: ProviderMode,
	optional_mode: Option<ProviderMode>,
	#[client_form(skip)]
	tenant_id: Option<String>,
	#[client_form(skip)]
	revision: u32,
	#[serde(skip)]
	server_token: String,
}

impl Validate for ProjectRequest {
	fn validate(&self) -> Result<(), ValidationErrors> {
		let mut errors = ValidationErrors::new();
		if self.name.trim().is_empty() {
			errors.add("name", ValidationError::TooShort { length: 0, min: 1 });
		}
		if errors.is_empty() {
			Ok(())
		} else {
			Err(errors)
		}
	}
}

#[test]
fn client_form_defaults_and_request_conversion() {
	let form = ProjectRequestClientForm::new().with_defaults(ProjectRequest {
		name: "demo".to_string(),
		title: Some("Seed".to_string()),
		retry_count: 2,
		optional_retry_count: Some(5),
		active: true,
		optional_active: Some(false),
		provider_mode: ProviderMode::LiveApi,
		optional_mode: Some(ProviderMode::Fake),
		tenant_id: Some("tenant-a".to_string()),
		revision: 7,
		server_token: "token-a".to_string(),
	});
	let runtime = use_form(&form).build();

	assert_eq!(
		runtime.watch_field::<String>(form.name_field()).get(),
		"demo"
	);
	assert_eq!(
		runtime
			.watch_field::<ProviderMode>(form.provider_mode_field())
			.get(),
		ProviderMode::LiveApi
	);

	runtime.set_value(ProjectRequestClientFormField::Title, "   ".to_string());
	let request = ProjectRequestClientForm::to_request(&runtime);

	assert_eq!(request.title, None);
	assert_eq!(request.retry_count, 2);
	assert_eq!(request.optional_retry_count, Some(5));
	assert!(request.active);
	assert_eq!(request.optional_active, Some(false));
	assert_eq!(request.optional_mode, Some(ProviderMode::Fake));
	assert_eq!(request.tenant_id.as_deref(), Some("tenant-a"));
	assert_eq!(request.revision, 7);
	assert_eq!(request.server_token, "token-a");
}

#[test]
fn client_form_enum_choice_metadata_uses_serialized_values() {
	let form = ProjectRequestClientForm::new();
	let choices = form.provider_mode_choices();

	assert_eq!(choices.len(), 3);
	assert_eq!(choices[0].serialized_value, "fake");
	assert_eq!(choices[0].label, "fake");
	assert_eq!(choices[1].serialized_value, "live_api");
	assert_eq!(choices[1].label, "live_api");
	assert_eq!(choices[2].serialized_value, "h_t_t_p_status");
	assert_eq!(choices[2].label, "h_t_t_p_status");
	assert_eq!(ProviderMode::client_form_default(), ProviderMode::Fake);
	assert!(matches!(ProviderMode::Archived, ProviderMode::Archived));
}

#[test]
fn client_form_reconcile_refreshes_skipped_defaults() {
	let form = ProjectRequestClientForm::new().with_defaults(ProjectRequest {
		name: "demo".to_string(),
		title: Some("Seed".to_string()),
		retry_count: 2,
		optional_retry_count: Some(5),
		active: true,
		optional_active: Some(false),
		provider_mode: ProviderMode::LiveApi,
		optional_mode: Some(ProviderMode::Fake),
		tenant_id: Some("tenant-a".to_string()),
		revision: 7,
		server_token: "token-a".to_string(),
	});
	let runtime = use_form(&form)
		.deps(0_u8)
		.reset_on_deps(ResetOnDeps::KeepDirtyValues)
		.build();
	runtime.set_value(ProjectRequestClientFormField::Name, "edited".to_string());

	let refreshed = ProjectRequestClientForm::new().with_defaults(ProjectRequest {
		name: "server".to_string(),
		title: Some("Server".to_string()),
		retry_count: 3,
		optional_retry_count: Some(8),
		active: false,
		optional_active: Some(true),
		provider_mode: ProviderMode::Fake,
		optional_mode: None,
		tenant_id: Some("tenant-b".to_string()),
		revision: 8,
		server_token: "token-b".to_string(),
	});
	runtime.reconcile_from(&refreshed, 1_u8);
	let request = ProjectRequestClientForm::to_request(&runtime);

	assert_eq!(request.name, "edited");
	assert_eq!(request.title.as_deref(), Some("Server"));
	assert_eq!(request.tenant_id.as_deref(), Some("tenant-b"));
	assert_eq!(request.revision, 8);
	assert_eq!(request.server_token, "token-b");
}

#[test]
fn client_form_validation_maps_dto_field_errors() {
	let form = ProjectRequestClientForm::new();
	let runtime = use_form(&form).build();

	let result = runtime.trigger();

	assert!(result.is_err());
	assert_eq!(
		runtime
			.get_field_state(ProjectRequestClientFormField::Name)
			.error
			.as_ref()
			.map(FieldError::message),
		Some("Length too short: 0 (minimum: 1)")
	);
}

#[derive(Clone, Default, Debug, PartialEq, ClientForm)]
#[client_form(validate)]
struct RawValidationRequest {
	r#type: String,
}

impl Validate for RawValidationRequest {
	fn validate(&self) -> Result<(), ValidationErrors> {
		let mut errors = ValidationErrors::new();
		errors.add(
			"r#type",
			ValidationError::PatternMismatch("expected raw field value".to_string()),
		);
		Err(errors)
	}
}

#[test]
fn client_form_validation_maps_raw_dto_field_errors() {
	let form = RawValidationRequestClientForm::new();
	let runtime = use_form(&form).build();

	let result = runtime.trigger();

	assert!(result.is_err());
	assert_eq!(
		runtime
			.get_field_state(RawValidationRequestClientFormField::Type)
			.error
			.as_ref()
			.map(FieldError::message),
		Some("Pattern mismatch: expected raw field value")
	);
}

thread_local! {
	static SUBMIT_CALL_COUNT: Cell<usize> = const { Cell::new(0) };
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ClientForm)]
#[client_form(server_fn = submit_project, validate)]
struct SubmitProjectRequest {
	name: String,
}

impl Validate for SubmitProjectRequest {
	fn validate(&self) -> Result<(), ValidationErrors> {
		let mut errors = ValidationErrors::new();
		if self.name.is_empty() {
			errors.add("name", ValidationError::TooShort { length: 0, min: 1 });
		}
		if errors.is_empty() {
			Ok(())
		} else {
			Err(errors)
		}
	}
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SubmitProjectResponse {
	name: String,
}

#[server_fn]
async fn submit_project(
	request: crate::SubmitProjectRequest,
) -> Result<SubmitProjectResponse, ServerFnError> {
	SUBMIT_CALL_COUNT.with(|count| count.set(count.get() + 1));
	Ok(SubmitProjectResponse { name: request.name })
}

#[tokio::test]
async fn client_form_server_submit_blocks_validation_failure() {
	SUBMIT_CALL_COUNT.with(|count| count.set(0));
	let form = SubmitProjectRequestClientForm::new();
	let runtime = use_form(&form).build();

	let outcome = runtime
		.submit_async(|| {
			let request = SubmitProjectRequestClientForm::to_request(&runtime);
			async move { submit_project(request).await }
		})
		.await
		.expect("validation outcome");

	assert_eq!(outcome, UseFormAsyncSubmitOutcome::ValidationFailed);
	assert_eq!(SUBMIT_CALL_COUNT.with(Cell::get), 0);
}

#[tokio::test]
async fn client_form_server_submit_calls_server_function_on_success() {
	SUBMIT_CALL_COUNT.with(|count| count.set(0));
	let form = SubmitProjectRequestClientForm::new().with_defaults(SubmitProjectRequest {
		name: "demo".to_string(),
	});
	let runtime = use_form(&form).build();

	let outcome = runtime
		.submit_async(|| {
			let request = SubmitProjectRequestClientForm::to_request(&runtime);
			async move { submit_project(request).await }
		})
		.await
		.expect("submit succeeds");

	assert_eq!(
		outcome,
		UseFormAsyncSubmitOutcome::Submitted(SubmitProjectResponse {
			name: "demo".to_string()
		})
	);
	assert_eq!(SUBMIT_CALL_COUNT.with(Cell::get), 1);
	assert!(runtime.form_state().is_submit_successful.get());
}
