use reinhardt::prelude::*;

#[post("/users/{id}/activate")]
async fn activate_user() -> Result<Response> {
	Ok(Response::ok())
}

fn main() {}
