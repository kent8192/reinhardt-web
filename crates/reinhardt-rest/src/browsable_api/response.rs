//! Browsable response types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Response for browsable API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowsableResponse {
	pub data: serde_json::Value,
	pub metadata: ResponseMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
	pub status: u16,
	pub method: String,
	pub path: String,
	pub headers: HashMap<String, String>,
}

impl BrowsableResponse {
	/// Create a new BrowsableResponse
	///
	/// # Examples
	///
	/// ```
	/// use crate::browsable_api::response::{BrowsableResponse, ResponseMetadata};
	/// use std::collections::HashMap;
	///
	/// let metadata = ResponseMetadata {
	///     status: 200,
	///     method: "GET".to_string(),
	///     path: "/api/test".to_string(),
	///     headers: HashMap::new(),
	/// };
	/// let response = BrowsableResponse::new(serde_json::json!({}), metadata);
	/// ```
	pub fn new(data: serde_json::Value, metadata: ResponseMetadata) -> Self {
		Self { data, metadata }
	}
	/// Create a successful response (200 OK)
	///
	/// # Examples
	///
	/// ```
	/// use crate::browsable_api::response::BrowsableResponse;
	///
	/// let response = BrowsableResponse::success(
	///     serde_json::json!({"message": "ok"}),
	///     "GET".to_string(),
	///     "/api/test".to_string()
	/// );
	/// ```
	pub fn success(data: serde_json::Value, method: String, path: String) -> Self {
		Self {
			data,
			metadata: ResponseMetadata {
				status: 200,
				method,
				path,
				headers: HashMap::new(),
			},
		}
	}
}
