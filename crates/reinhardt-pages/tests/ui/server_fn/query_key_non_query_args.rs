use reinhardt_pages_macros::server_fn;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Request {
	id: u32,
}

#[derive(Serialize, Deserialize)]
pub struct Response {
	id: u32,
}

#[server_fn]
async fn accepts_server_only_request(
	request: crate::Request,
) -> Result<Response, reinhardt_pages::server_fn::ServerFnError> {
	Ok(Response { id: request.id })
}

fn main() {
	let _ = accepts_server_only_request;
}
