//! Test: Basic watch block syntax
//!
//! Validates that the simplest form of watch block compiles successfully.
//! A watch block wraps content in a reactive context.

use reinhardt_pages::page;

fn main() {
	// Basic watch block with simple content
	let _basic = page!(|| {
		div {
			watch {
				span {
					"Reactive content"
				}
			}
		}
	});

	// Watch block with text content
	let _text = page!(|| {
		div {
			watch {
				"Dynamic text"
			}
		}
	});
}
