use reinhardt_macros::api_view;

// This should work - methods are case-insensitive
#[api_view(methods = "get")]
async fn lowercase_method() -> Result<(), ()> {
	Ok(())
}

#[api_view(methods = "PoSt")]
async fn mixed_case_method() -> Result<(), ()> {
	Ok(())
}

fn main() {}
