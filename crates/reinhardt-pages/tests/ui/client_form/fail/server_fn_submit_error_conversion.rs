use reinhardt_pages::ClientForm;
use reinhardt_pages::server_fn::server_fn;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize, ClientForm)]
#[client_form(server_fn = submit_settings)]
pub struct SettingsRequest {
	name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SettingsResponse {
	name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitError {
	message: String,
}

impl std::fmt::Display for SubmitError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.message)
	}
}

impl std::error::Error for SubmitError {}

impl From<serde_json::Error> for SubmitError {
	fn from(err: serde_json::Error) -> Self {
		Self {
			message: err.to_string(),
		}
	}
}

#[server_fn]
async fn submit_settings(request: crate::SettingsRequest) -> Result<SettingsResponse, SubmitError> {
	Ok(SettingsResponse { name: request.name })
}

fn main() {}
