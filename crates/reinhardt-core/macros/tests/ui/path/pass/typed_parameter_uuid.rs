// Test: URL pattern with typed UUID parameter

use reinhardt_macros::path;

fn main() {
	let pattern = path!("objects/{<uuid:id>}/");
	assert_eq!(pattern, "objects/{<uuid:id>}/");
}
