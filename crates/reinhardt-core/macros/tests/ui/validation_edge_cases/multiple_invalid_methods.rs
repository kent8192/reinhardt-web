use reinhardt_macros::api_view;

#[api_view(methods = "GET,INVALID1,POST,INVALID2")]
async fn multiple_invalid_methods() -> Result<(), ()> {
	Ok(())
}

fn main() {}
