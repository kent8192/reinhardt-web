//! Views for hello app
//!
//! Simple hello world view

use reinhardt::{Request, Response, StatusCode};
use reinhardt::core::macros::endpoint;

pub type ViewResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Hello World view
///
/// Returns a simple "Hello, World!" response
#[endpoint]
pub async fn hello_world(_req: Request) -> ViewResult<Response> {
	Ok(Response::new(StatusCode::OK)
		.with_body("Hello, World!"))
}
