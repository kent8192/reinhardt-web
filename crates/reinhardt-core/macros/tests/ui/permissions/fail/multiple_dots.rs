use reinhardt_macros::permission_required;

#[permission_required("auth.contrib.view_user")]
async fn view_user() -> Result<(), ()> {
	Ok(())
}

fn main() {}
