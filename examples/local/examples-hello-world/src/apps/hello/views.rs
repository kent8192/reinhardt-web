//! View handlers for hello app

use reinhardt_http::{Request, Response};
use serde_json::json;

/// Root endpoint - returns "Hello, World!"
pub async fn hello_world(_req: Request) -> reinhardt::Result<Response> {
	Ok(Response::ok().with_body("Hello, World!"))
}

/// Health check endpoint - returns JSON status
pub async fn health_check(_req: Request) -> reinhardt::Result<Response> {
	let body = json!({
		"status": "ok"
	});

	Response::ok().with_json(&body).map_err(Into::into)
}
