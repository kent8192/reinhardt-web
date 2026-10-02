use crate::pages_api::{
	client_form,
	server_fn::{ServerFnError, server_fn},
};
use serde::{Deserialize, Serialize};

#[client_form(server_fn = login, validate)]
#[derive(Clone, Serialize, Deserialize, crate::pages_api::__private::client_form::DtoValidate)]
pub struct LoginRequest {
	#[validate(length(min = 1, max = 150))]
	pub username: String,
	#[validate(length(min = 1, max = 128))]
	pub password: String,
}
#[client_form(server_fn = register, validate)]
#[derive(Clone, Serialize, Deserialize, crate::pages_api::__private::client_form::DtoValidate)]
pub struct RegisterRequest {
	#[validate(length(min = 3, max = 32))]
	pub username: String,
	#[validate(email, length(max = 254))]
	pub email: String,
	#[validate(length(min = 8, max = 128))]
	pub password: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct AuthResponse {
	pub token: String,
}
#[server_fn]
pub async fn login(request: crate::dto::LoginRequest) -> Result<AuthResponse, ServerFnError> {
	Ok(AuthResponse {
		token: request.username,
	})
}
#[server_fn]
pub async fn register(request: crate::dto::RegisterRequest) -> Result<AuthResponse, ServerFnError> {
	Ok(AuthResponse {
		token: request.username,
	})
}
