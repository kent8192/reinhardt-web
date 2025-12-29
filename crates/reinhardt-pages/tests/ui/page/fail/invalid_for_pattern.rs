//! page! macro with invalid for loop pattern

// reinhardt-fmt: ignore-all

use reinhardt_pages::page;

fn main() {
	// Missing 'in' keyword in for loop
	let _invalid = page!(|items: Vec<String>| {
	ul {
		for item
		items {
			li {
				item
			}
		}
	}
});
}
