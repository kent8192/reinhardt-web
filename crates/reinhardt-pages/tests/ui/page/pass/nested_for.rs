//! page! macro with nested for loops

use reinhardt_pages::page;

fn main() {
	// Simple for loop
	// The for body is auto-wrapped in `Page::reactive(move || ...)` (an `Fn`
	// closure), so the captured `items` must be cloned before iteration to
	// avoid moving out of the closure (spec §4.1 auto-wrap contract).
	let _simple = page!(|items: Vec<String>| {
		ul {
			for item in items.clone() {
				li { { item } }
			}
		}
	});

	// For loop with nested if
	let _for_if = page!(|items: Vec<(i32, bool) >| {
		ul {
			for(num, active)in items.clone() {
				li {
					if active {
						strong { { format!("{}", num) } }
					} else {
						span { { format!("{}", num) } }
					}
				}
			}
		}
	});
}
