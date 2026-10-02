use reinhardt_pages_macros::server_fn;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct AppError(String);

impl std::fmt::Display for AppError {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(formatter, "{}", self.0)
	}
}

impl std::error::Error for AppError {}

impl From<reinhardt_pages::server_fn::ServerFnError> for AppError {
	fn from(error: reinhardt_pages::server_fn::ServerFnError) -> Self {
		Self(error.to_string())
	}
}

impl From<serde_json::Error> for AppError {
	fn from(error: serde_json::Error) -> Self {
		Self(error.to_string())
	}
}

type AppResult<T> = Result<T, AppError>;

#[derive(Serialize, Deserialize)]
struct User {
	id: u32,
}

#[server_fn]
async fn load_user(id: u32) -> AppResult<User> {
	Ok(User { id })
}

fn main() {
	let _ = load_user;
}
