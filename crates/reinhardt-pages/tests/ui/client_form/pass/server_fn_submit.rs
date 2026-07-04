use reinhardt_core::validators::{Validate, ValidationErrors};
use reinhardt_pages::server_fn::server_fn;
use reinhardt_pages::server_fn::ServerFnError;
use reinhardt_pages::{ClientForm, UseFormAsyncSubmitOutcome, use_form};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize, ClientForm)]
#[client_form(server_fn = submit_settings, validate)]
pub struct SettingsRequest {
	name: String,
}

impl Validate for SettingsRequest {
	fn validate(&self) -> Result<(), ValidationErrors> {
		Ok(())
	}
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SettingsResponse {
	name: String,
}

#[server_fn]
async fn submit_settings(
	request: crate::SettingsRequest,
) -> Result<SettingsResponse, ServerFnError> {
	Ok(SettingsResponse { name: request.name })
}

fn assert_submit_output(
	value: Result<UseFormAsyncSubmitOutcome<SettingsResponse>, ServerFnError>,
) -> Result<UseFormAsyncSubmitOutcome<SettingsResponse>, ServerFnError> {
	value
}

fn main() {
	let form = SettingsRequestClientForm::new();
	let runtime = use_form(&form).build();
	#[cfg(all(target_family = "wasm", target_os = "unknown"))]
	let _future = async { assert_submit_output(form.submit(&runtime).await) };
	let _ = (form, runtime);
}
