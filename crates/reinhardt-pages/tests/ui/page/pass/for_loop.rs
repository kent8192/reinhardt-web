//! page! macro with for loop rendering

use reinhardt_pages::page;

fn main() {
	// Simple for loop
	let _with_for = page!(|items: Vec<String>| {
		ul {
			for item in items {
				li {
					item
				}
			}
		}
	});

	// For with tuple destructuring
	let _for_enumerate = page!(|items: Vec<(usize, String)>| {
		ul {
			for (index, item) in items {
				li {
					span {
						index.to_string()
					}
					span {
						item
					}
				}
			}
		}
	});
}
