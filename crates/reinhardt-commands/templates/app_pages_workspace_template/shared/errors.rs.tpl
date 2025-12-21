//! {{ app_name }} - Shared error types

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {{ camel_case_app_name }}Error {
	pub message: String,
}

impl From<String> for {{ camel_case_app_name }}Error {
	fn from(message: String) -> Self {
		{{ camel_case_app_name }}Error { message }
	}
}

impl From<&str> for {{ camel_case_app_name }}Error {
	fn from(message: &str) -> Self {
		{{ camel_case_app_name }}Error {
			message: message.to_string(),
		}
	}
}
