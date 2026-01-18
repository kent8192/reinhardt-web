use reinhardt_macros::api_view;

#[api_view(methods = "INVALID")]
async fn invalid_method() -> Result<(), ()> {
	Ok(())
}

fn main() {}
