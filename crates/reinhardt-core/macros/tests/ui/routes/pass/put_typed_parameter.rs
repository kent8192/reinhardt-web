use reinhardt::prelude::*;

#[put("/api/users/{<int:user_id>}/profile")]
async fn update_profile() -> Result<Response> {
	Ok(Response::ok())
}

fn main() {}
