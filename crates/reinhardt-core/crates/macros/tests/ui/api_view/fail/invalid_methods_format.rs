use reinhardt_macros::api_view;

#[api_view(methods = 123)]
async fn invalid_format() -> Result<(), ()> {
	Ok(())
}

fn main() {}
