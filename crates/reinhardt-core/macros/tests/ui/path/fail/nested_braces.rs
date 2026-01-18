// Test: URL pattern with nested braces should fail

use reinhardt_macros::path;

fn main() {
	let pattern = path!("polls/{{id}}/");
}
