// Test: URL pattern with a single parameter

use reinhardt_macros::path;

fn main() {
	let pattern = path!("polls/{id}/");
	assert_eq!(pattern, "polls/{id}/");
}
