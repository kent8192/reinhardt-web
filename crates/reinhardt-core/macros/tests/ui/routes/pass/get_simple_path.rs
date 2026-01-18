use reinhardt::prelude::*;

#[get("/users")]
async fn list_users() -> Result<Response> {
	Ok(Response::ok())
}

fn main() {}
