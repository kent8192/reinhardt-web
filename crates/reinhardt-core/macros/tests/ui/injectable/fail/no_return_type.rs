use reinhardt_macros::injectable;

#[injectable]
fn bad_factory() {
	// No return type
}

fn main() {}
