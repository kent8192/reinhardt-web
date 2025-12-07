//! Common views for the project
//!
//! Health check and root endpoints

use reinhardt::get;
use reinhardt::http::ViewResult;
use reinhardt::{Response, StatusCode};

#[allow(unused_imports)]
use ViewResult as _;

/// Health check endpoint
#[get("/health", name = "health")]
pub async fn health_check() -> ViewResult<Response> {
	Ok(Response::new(StatusCode::OK).with_body("OK".as_bytes().to_vec()))
}
