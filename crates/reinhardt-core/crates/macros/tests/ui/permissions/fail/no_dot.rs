use reinhardt_macros::permission_required;

#[permission_required("authviewuser")]
async fn view_user() -> Result<(), ()> {
	Ok(())
}

fn main() {}
