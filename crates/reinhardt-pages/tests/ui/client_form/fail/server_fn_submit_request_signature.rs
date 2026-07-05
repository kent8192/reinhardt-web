use reinhardt_pages::ClientForm;
use reinhardt_pages::server_fn::ServerFnError;
use reinhardt_pages::server_fn::server_fn;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize, ClientForm)]
#[client_form(server_fn = submit_settings)]
pub struct SettingsRequest {
	name: String,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct OtherRequest {
	name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SettingsResponse {
	name: String,
}

#[server_fn]
async fn submit_settings(request: crate::OtherRequest) -> Result<SettingsResponse, ServerFnError> {
	Ok(SettingsResponse { name: request.name })
}

fn main() {}
