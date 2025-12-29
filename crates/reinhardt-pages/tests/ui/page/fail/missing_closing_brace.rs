//! page! macro with unclosed brace

// reinhardt-fmt: ignore-all

use reinhardt_pages::page;

fn main() {
	// Unclosed element - should fail
	let _invalid = page!(|| {
		div {
			"hello"
	});
}
