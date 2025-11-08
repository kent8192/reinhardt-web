use reinhardt_macros::api_view;

#[api_view(methods)]
async fn invalid_syntax() -> Result<(), ()> {
	Ok(())
}

fn main() {}
