use reinhardt_pages::server_fn::{ServerFnError, server_fn};
use serde::Serialize;

type ApiResult<T> = Result<T, ServerFnError>;

#[derive(Serialize)]
pub struct SaveResponse {
	value: String,
}

#[server_fn]
async fn aliased_result() -> ApiResult<SaveResponse> {
	Ok(SaveResponse {
		value: "saved".to_string(),
	})
}

fn main() {}
