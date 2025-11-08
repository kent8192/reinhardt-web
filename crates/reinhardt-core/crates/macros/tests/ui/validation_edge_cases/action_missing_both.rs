use reinhardt_macros::action;

struct ViewSet;

impl ViewSet {
	#[action]
	async fn missing_both(&self) -> Result<(), ()> {
		Ok(())
	}
}

fn main() {}
