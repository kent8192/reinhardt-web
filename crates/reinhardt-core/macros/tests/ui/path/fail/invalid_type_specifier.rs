// Test: URL pattern with invalid type specifier should fail

use reinhardt_macros::path;

fn main() {
	let pattern = path!("polls/{<invalid:id>}/");
}
