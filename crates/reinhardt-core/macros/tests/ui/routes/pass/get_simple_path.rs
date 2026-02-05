#[path = "../../../test_support.rs"]
mod test_support;

use reinhardt_macros::get;
use test_support::{Response, Result};

#[get("/users")]
async fn list_users() -> Result<Response> {
	Ok(Response::ok())
}

fn main() {}
