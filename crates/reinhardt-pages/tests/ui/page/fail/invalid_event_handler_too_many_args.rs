//! page! macro with event handler having too many arguments
//!
//! This test verifies that event handlers can have at most 1 argument.
//! Closures with 2 or more arguments should fail.

// reinhardt-fmt: ignore-all

use reinhardt_pages::page;

fn main() {
	// Error: Event handler closure must have 0 or 1 arguments, found: 3 arguments
	let _invalid = page!(|| {
		button {
			@click: | a, b, c | { println!("Too many args: {}, {}, {}", a, b, c); },
			"Click me"
		}
	});
}
