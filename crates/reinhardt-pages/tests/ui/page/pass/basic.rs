//! Basic page! macro usage with a simple element

use reinhardt_pages::page;

fn main() {
	// Basic element with text child
	let _hello = page!(|| {
		div {
			"Hello, World!"
		}
	});

	// The page! macro returns a closure
	// We can call it to get a View
}
