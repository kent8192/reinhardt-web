use reinhardt_pages::server_fn::{ServerFnError, server_fn};
use reinhardt_pages::{ClientFormChoices, client_form, form, use_form};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, ClientFormChoices)]
pub enum Mode {
	#[default]
	#[serde(rename = "")]
	Empty,
	Active,
}

#[client_form(server_fn = endpoint)]
#[derive(Clone, Serialize, Deserialize)]
pub struct Request {
	pub text: String,
	pub count: Option<i32>,
	pub enabled: bool,
	pub optional_enabled: Option<bool>,
	pub mode: Mode,
	pub optional_mode: Option<Mode>,
	pub id: String,
	pub submit: String,
	pub mutation: String,
	#[client_form(skip)]
	pub internal: u32,
}

#[server_fn]
pub async fn endpoint(request: crate::Request) -> Result<String, ServerFnError> {
	let text = request.text;
	Ok(text)
}
