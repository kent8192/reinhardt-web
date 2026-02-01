#[path = "../../../test_support.rs"]
mod test_support;

use reinhardt_macros::post;
use test_support::{Response, Result};

#[post("/users/{id}/activate")]
async fn activate_user() -> Result<Response> {
	Ok(Response::ok())
}

fn main() {}
