//! page! macro with for loop rendering

use reinhardt_pages::page;

fn main() {
	// Simple for loop
	// The for body is auto-wrapped in `Page::reactive(move || ...)` (an `Fn`
	// closure), so the captured `items` must be cloned before iteration to
	// avoid moving out of the closure (spec §4.1 auto-wrap contract).
	let _with_for = page!(|items: Vec<String>| {
		ul {
			for item in items.clone() {
				li { { item } }
			}
		}
	});

	// For with tuple destructuring
	let _for_enumerate = page!(|items: Vec<(usize, String) >| {
		ul {
			for(index, item)in items.clone() {
				li {
					span { { index.to_string() } }
					span { { item } }
				}
			}
		}
	});
}
