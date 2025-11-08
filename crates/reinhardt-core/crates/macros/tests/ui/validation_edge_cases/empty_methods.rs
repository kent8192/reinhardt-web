use reinhardt_macros::api_view;

#[api_view(methods = "")]
async fn empty_methods() -> Result<(), ()> {
	Ok(())
}

fn main() {}
