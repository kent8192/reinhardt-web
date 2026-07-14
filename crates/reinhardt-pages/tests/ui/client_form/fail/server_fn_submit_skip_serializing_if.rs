use reinhardt_pages::ClientForm;
use reinhardt_pages::server_fn::ServerFnError;
use reinhardt_pages::server_fn::server_fn;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, ClientForm)]
#[client_form(server_fn = submit_settings)]
struct SettingsRequest {
	#[serde(skip_serializing_if = "String::is_empty")]
	name: String,
	enabled: bool,
}

#[server_fn]
async fn submit_settings(request: crate::SettingsRequest) -> Result<(), ServerFnError> {
	let _ = request;
	Ok(())
}

fn main() {}
