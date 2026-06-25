//! Internal Fetch API wrapper used by generated WASM client code.

use crate::server_fn::ServerFnError;

/// HTTP response body and status returned by the internal Fetch API wrapper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchResponse {
	status: u16,
	body: String,
}

impl FetchResponse {
	/// Creates a response from a status code and text body.
	pub fn new(status: u16, body: String) -> Self {
		Self { status, body }
	}

	/// Returns the numeric HTTP status code.
	pub fn status(&self) -> u16 {
		self.status
	}

	/// Returns true for HTTP 2xx responses.
	pub fn is_success(&self) -> bool {
		(200..300).contains(&self.status)
	}

	/// Consumes the response and returns its text body.
	pub fn into_text(self) -> String {
		self.body
	}

	/// Deserializes the response text as JSON.
	pub fn json<T>(&self) -> Result<T, ServerFnError>
	where
		T: serde::de::DeserializeOwned,
	{
		serde_json::from_str(&self.body).map_err(|e| ServerFnError::deserialization(e.to_string()))
	}
}

/// Sends an HTTP request through the browser Fetch API.
#[cfg(wasm)]
pub async fn request(
	method: &str,
	url: &str,
	body: Option<&str>,
	headers: Vec<(String, String)>,
) -> Result<FetchResponse, ServerFnError> {
	use wasm_bindgen::JsCast;
	use wasm_bindgen::JsValue;
	use wasm_bindgen_futures::JsFuture;
	use web_sys::{Request, RequestCredentials, RequestInit, RequestMode, Response, window};

	let init = RequestInit::new();
	init.set_method(method);
	init.set_mode(RequestMode::Cors);
	init.set_credentials(RequestCredentials::Include);
	if let Some(body) = body {
		init.set_body(&JsValue::from_str(body));
	}

	let request = Request::new_with_str_and_init(url, &init)
		.map_err(|e| ServerFnError::network(js_error_message(e)))?;

	let request_headers = request.headers();
	for (name, value) in headers {
		request_headers
			.set(&name, &value)
			.map_err(|e| ServerFnError::network(js_error_message(e)))?;
	}

	let window = window().ok_or_else(|| ServerFnError::network("window is unavailable"))?;
	let response_value = JsFuture::from(window.fetch_with_request(&request))
		.await
		.map_err(|e| ServerFnError::network(js_error_message(e)))?;
	let response: Response = response_value
		.dyn_into()
		.map_err(|e| ServerFnError::network(js_error_message(e)))?;

	let status = response.status();
	let text_promise = response
		.text()
		.map_err(|e| ServerFnError::deserialization(js_error_message(e)))?;
	let text_value = JsFuture::from(text_promise)
		.await
		.map_err(|e| ServerFnError::deserialization(js_error_message(e)))?;
	let body = text_value.as_string().unwrap_or_default();

	Ok(FetchResponse::new(status, body))
}

/// Native placeholder for accidental non-WASM use.
#[cfg(native)]
pub async fn request(
	_method: &str,
	_url: &str,
	_body: Option<&str>,
	_headers: Vec<(String, String)>,
) -> Result<FetchResponse, ServerFnError> {
	Err(ServerFnError::network(
		"Fetch API is not available outside browser WASM".to_string(),
	))
}

#[cfg(wasm)]
fn js_error_message(value: wasm_bindgen::JsValue) -> String {
	value.as_string().unwrap_or_else(|| format!("{value:?}"))
}
