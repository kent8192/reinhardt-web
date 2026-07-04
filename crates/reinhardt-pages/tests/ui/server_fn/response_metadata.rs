//! Test: Server function marker exposes declared response metadata.
//!
//! This test verifies that `#[server_fn]` implements
//! `ServerFnResponseMetadata` for the generated marker and that the associated
//! `Response` type is the declared `Ok` type from the server function result.

use reinhardt_pages::server_fn::ServerFnResponseMetadata;
use reinhardt_pages_macros::server_fn;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerFnError(String);

impl std::fmt::Display for ServerFnError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl std::error::Error for ServerFnError {}

impl From<serde_json::Error> for ServerFnError {
	fn from(err: serde_json::Error) -> Self {
		ServerFnError(format!("Serialization error: {}", err))
	}
}

#[derive(Debug, Serialize)]
pub struct DeclaredResponse {
	value: String,
}

#[server_fn]
async fn response_metadata_sample(value: String) -> Result<DeclaredResponse, ServerFnError> {
	Ok(DeclaredResponse { value })
}

type GeneratedResponse =
	<response_metadata_sample::marker as ServerFnResponseMetadata>::Response;
type GeneratedError = <response_metadata_sample::marker as ServerFnResponseMetadata>::Error;

fn assert_response_metadata<T>()
where
	T: ServerFnResponseMetadata<Response = DeclaredResponse, Error = ServerFnError>,
{
}

fn assert_declared_response(value: GeneratedResponse) -> DeclaredResponse {
	value
}

fn assert_declared_error(value: GeneratedError) -> ServerFnError {
	value
}

mod scoped {
	use super::*;

	pub(super) mod endpoint {
		use super::*;

		#[server_fn]
		pub(super) async fn scoped_response_metadata_sample(
			value: String,
		) -> Result<DeclaredResponse, ServerFnError> {
			Ok(DeclaredResponse { value })
		}
	}

	type ScopedResponse =
		<endpoint::scoped_response_metadata_sample::marker as ServerFnResponseMetadata>::Response;

	fn assert_scoped_response(value: ScopedResponse) -> DeclaredResponse {
		value
	}

	pub fn assert_scoped_marker_is_nameable() {
		assert_response_metadata::<endpoint::scoped_response_metadata_sample::marker>();
		let _assert_scoped_response: fn(ScopedResponse) -> DeclaredResponse = assert_scoped_response;
	}
}

fn main() {
	assert_response_metadata::<response_metadata_sample::marker>();
	let _assert_declared_response: fn(GeneratedResponse) -> DeclaredResponse =
		assert_declared_response;
	let _assert_declared_error: fn(GeneratedError) -> ServerFnError = assert_declared_error;
	scoped::assert_scoped_marker_is_nameable();
}
