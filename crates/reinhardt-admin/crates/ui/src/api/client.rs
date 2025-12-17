//! API Client for communicating with the backend

use futures_signals::signal::Mutable;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Headers, Request, RequestCredentials, RequestInit, RequestMode, Response};

/// API client for making HTTP requests to the backend
pub struct ApiClient {
	base_url: String,
	csrf_token: Mutable<Option<String>>,
}

/// API error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
	pub code: String,
	pub message: String,
	pub details: Option<serde_json::Value>,
}

impl ApiClient {
	/// Create a new API client with the given base URL
	pub fn new(base_url: String) -> Self {
		Self {
			base_url,
			csrf_token: Mutable::new(None),
		}
	}

	/// Set the CSRF token for subsequent mutation requests
	pub fn set_csrf_token(&self, token: String) {
		self.csrf_token.set(Some(token));
	}

	/// GET request
	pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
		let url = format!("{}{}", self.base_url, path);

		let opts = RequestInit::new();
		opts.set_method("GET");
		opts.set_mode(RequestMode::SameOrigin);
		opts.set_credentials(RequestCredentials::SameOrigin);

		let headers = Headers::new().map_err(Self::js_to_api_error)?;
		headers
			.append("Accept", "application/json")
			.map_err(Self::js_to_api_error)?;
		opts.set_headers(&headers);

		let request = Request::new_with_str_and_init(&url, &opts).map_err(Self::js_to_api_error)?;

		self.execute_request::<T>(request).await
	}

	/// POST request (with CSRF protection)
	pub async fn post<T: Serialize, R: DeserializeOwned>(
		&self,
		path: &str,
		body: &T,
	) -> Result<R, ApiError> {
		self.mutate_request("POST", path, Some(body)).await
	}

	/// PUT request (with CSRF protection)
	pub async fn put<T: Serialize, R: DeserializeOwned>(
		&self,
		path: &str,
		body: &T,
	) -> Result<R, ApiError> {
		self.mutate_request("PUT", path, Some(body)).await
	}

	/// DELETE request (with CSRF protection)
	pub async fn delete<R: DeserializeOwned>(&self, path: &str) -> Result<R, ApiError> {
		self.mutate_request::<(), R>("DELETE", path, None).await
	}

	/// Common mutation request logic (POST, PUT, DELETE)
	async fn mutate_request<T: Serialize, R: DeserializeOwned>(
		&self,
		method: &str,
		path: &str,
		body: Option<&T>,
	) -> Result<R, ApiError> {
		let url = format!("{}{}", self.base_url, path);

		let opts = RequestInit::new();
		opts.set_method(method);
		opts.set_mode(RequestMode::SameOrigin);
		opts.set_credentials(RequestCredentials::SameOrigin);

		let headers = Headers::new().map_err(Self::js_to_api_error)?;
		headers
			.append("Content-Type", "application/json")
			.map_err(Self::js_to_api_error)?;
		headers
			.append("Accept", "application/json")
			.map_err(Self::js_to_api_error)?;

		// Add CSRF token for state-changing requests
		if let Some(token) = self.csrf_token.get_cloned() {
			headers
				.append("X-CSRF-Token", &token)
				.map_err(Self::js_to_api_error)?;
		}

		opts.set_headers(&headers);

		// Add request body if provided
		if let Some(body_data) = body {
			let body_str = serde_json::to_string(body_data).map_err(|e| ApiError {
				code: "serialization_error".into(),
				message: e.to_string(),
				details: None,
			})?;
			opts.set_body(&JsValue::from_str(&body_str));
		}

		let request = Request::new_with_str_and_init(&url, &opts).map_err(Self::js_to_api_error)?;

		self.execute_request::<R>(request).await
	}

	/// Execute the HTTP request and parse the response
	async fn execute_request<T: DeserializeOwned>(&self, request: Request) -> Result<T, ApiError> {
		let window = web_sys::window().ok_or_else(|| ApiError {
			code: "no_window".into(),
			message: "No window object available".into(),
			details: None,
		})?;

		let resp_value = JsFuture::from(window.fetch_with_request(&request))
			.await
			.map_err(Self::js_to_api_error)?;

		let resp: Response = resp_value.dyn_into().map_err(Self::js_to_api_error)?;

		// Check HTTP status code
		if !resp.ok() {
			return self.parse_error_response(resp).await;
		}

		// Parse success response
		let json = JsFuture::from(resp.json().map_err(Self::js_to_api_error)?)
			.await
			.map_err(Self::js_to_api_error)?;

		serde_wasm_bindgen::from_value(json).map_err(|e| ApiError {
			code: "deserialization_error".into(),
			message: e.to_string(),
			details: None,
		})
	}

	/// Parse error response from the server
	async fn parse_error_response<T>(&self, resp: Response) -> Result<T, ApiError> {
		let status = resp.status();
		let json = JsFuture::from(resp.json().map_err(Self::js_to_api_error)?)
			.await
			.map_err(Self::js_to_api_error)?;

		match serde_wasm_bindgen::from_value::<ApiError>(json) {
			Ok(api_error) => Err(api_error),
			Err(_) => Err(ApiError {
				code: format!("http_{}", status),
				message: format!("HTTP error {}", status),
				details: None,
			}),
		}
	}

	/// Convert JsValue error to ApiError
	fn js_to_api_error(js_error: JsValue) -> ApiError {
		ApiError {
			code: "js_error".into(),
			message: format!("{:?}", js_error),
			details: None,
		}
	}
}
