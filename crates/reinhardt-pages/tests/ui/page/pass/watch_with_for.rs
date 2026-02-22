//! Test: Watch block with for loop
//!
//! Validates that watch blocks can contain for loops for list rendering.

use reinhardt_pages::Signal;
use reinhardt_pages::page;

fn main() {
	// Watch with for loop
	let _with_for = page!(|items: Signal<Vec<String>>| {
		ul {
			watch {
				for item in items.get().iter() {
					li {
						{ item.clone() }
					}
				}
			}
		}
	});

	// Watch with for loop and conditional
	let _for_with_condition = page!(|data: Signal<Vec<i32>>| {
		div {
			watch {
				if data.get().is_empty() {
					p {
						"No items"
					}
				} else {
					ul {
						for num in data.get().iter() {
							li {
								{ num.to_string() }
							}
						}
					}
				}
			}
		}
	});
}
