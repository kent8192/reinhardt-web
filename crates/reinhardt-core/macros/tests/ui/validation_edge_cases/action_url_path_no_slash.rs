use reinhardt_macros::action;

struct ViewSet;

impl ViewSet {
	#[action(methods = "GET", detail = true, url_path = "no-slash")]
	async fn no_slash(&self) -> Result<(), ()> {
		Ok(())
	}
}

fn main() {}
