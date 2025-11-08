use reinhardt_macros::action;

struct ViewSet;

impl ViewSet {
	#[action(detail = true)]
	async fn missing_methods(&self) -> Result<(), ()> {
		Ok(())
	}
}

fn main() {}
