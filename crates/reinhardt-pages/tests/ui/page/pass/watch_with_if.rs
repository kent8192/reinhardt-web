//! Test: Watch block with if condition
//!
//! Validates that watch blocks can contain if statements for conditional rendering.
//! This is the primary use case for watch blocks.

use reinhardt_pages::Signal;
use reinhardt_pages::page;

fn main() {
	// Watch with if condition using Signal parameter
	let _with_if = page!(|show: Signal<bool>| {
		div {
			watch {
				if show.get() {
					span {
						"Visible when true"
					}
				}
			}
		}
	});

	// Watch with if condition using primitive parameter
	let _with_bool = page!(|visible: bool| {
		div {
			watch {
				if visible {
					p {
						"Content is visible"
					}
				}
			}
		}
	});
}
