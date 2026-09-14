//! DTOs shared by native and browser named-view regression tests.

use reinhardt_pages::server_fn::ServerFnError;
use reinhardt_pages::{client_form, server_fn::server_fn};
use serde::{Deserialize, Serialize};

#[client_form(server_fn = login, validate)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, reinhardt_core::Validate)]
pub struct LoginRequest {
	#[validate(length(min = 1, max = 150))]
	pub username: String,
	#[validate(length(min = 1, max = 128))]
	pub password: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AuthResponse {
	pub token: String,
}

#[client_form(server_fn = register, validate)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, reinhardt_core::Validate)]
pub struct RegisterRequest {
	#[validate(length(min = 3, max = 32))]
	pub username: String,
	#[validate(email, length(max = 254))]
	pub email: String,
	#[validate(length(min = 8, max = 128))]
	pub password: String,
}

#[server_fn]
pub async fn register(
	request: crate::support::RegisterRequest,
) -> Result<AuthResponse, ServerFnError> {
	Ok(AuthResponse {
		token: request.username,
	})
}

#[server_fn]
pub async fn login(request: crate::support::LoginRequest) -> Result<AuthResponse, ServerFnError> {
	Ok(AuthResponse {
		token: request.username,
	})
}

#[derive(
	Clone, Debug, Default, PartialEq, Serialize, Deserialize, reinhardt_pages::ClientFormChoices,
)]
pub enum Choice {
	#[default]
	#[serde(rename = "")]
	Empty,
	Active,
}

#[client_form(server_fn = save_values)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ValuesRequest {
	pub text: String,
	pub notes: String,
	pub optional_text: Option<String>,
	pub number: i32,
	pub optional_number: Option<i32>,
	pub checked: bool,
	pub optional_checked: Option<bool>,
	pub choice: Choice,
	pub optional_choice: Option<Choice>,
}

#[server_fn]
pub async fn save_values(request: crate::support::ValuesRequest) -> Result<String, ServerFnError> {
	Ok(request.text)
}
