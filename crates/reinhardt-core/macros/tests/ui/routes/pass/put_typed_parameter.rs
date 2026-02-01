#[path = "../../../test_support.rs"]
mod test_support;

use reinhardt_macros::put;
use test_support::{Response, Result};

#[put("/api/users/{<int:user_id>}/profile")]
async fn update_profile() -> Result<Response> {
	Ok(Response::ok())
}

fn main() {}
