//! Test: Watch block with if-else branching
//!
//! Validates that watch blocks can contain if-else statements.
//! Both branches should be wrapped in the reactive context.

use reinhardt_pages::Signal;
use reinhardt_pages::page;

fn main() {
	// Watch with if-else using Signal
	let _if_else = page!(|loading: Signal<bool>| {
		div {
			watch {
				if loading.get() {
					span {
						"Loading..."
					}
				} else {
					span {
						"Content loaded"
					}
				}
			}
		}
	});

	// Watch with if-else-if chain
	let _if_else_if = page!(|status: Signal<i32>| {
		div {
			watch {
				if status.get() == 0 {
					span {
						"Idle"
					}
				} else if status.get() == 1 {
					span {
						"Processing"
					}
				} else {
					span {
						"Complete"
					}
				}
			}
		}
	});
}
