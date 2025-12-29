//! page! macro with void element containing children
//!
//! This test verifies that void elements (like input, br, hr) cannot have children.
//! Attempting to add child nodes to a void element should fail.

// reinhardt-fmt: ignore-all

use reinhardt_pages::page;

fn main() {
	// Error: Void element <input> cannot have children
	let _invalid = page!(|| {
		input {
			"This is not allowed"
		}
	});

	// Another example with br element
	let _also_invalid = page!(|| {
		br {
			span {
				"Cannot nest in br"
			}
		}
	});
}
