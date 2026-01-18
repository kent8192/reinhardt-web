use reinhardt_macros::post;

#[post("/users/{}/posts")]
async fn create_post() -> Result<(), ()> {
	Ok(())
}

fn main() {}
