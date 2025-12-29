//! page! macro with invalid data-* attribute names
//!
//! This test verifies that data-* attributes must follow the naming convention:
//! data-[a-z][a-z0-9-]*

// reinhardt-fmt: ignore-all

use reinhardt_pages::page;

fn main() {
	// Error: Invalid data attribute name 'data-'. Must match pattern: data-[a-z][a-z0-9-]*
	let _invalid = page!(|| {
		div {
			data_: "empty suffix",
			"Content"
		}
	});

	// Error: data attribute must start with lowercase letter
	let _also_invalid = page!(|| {
		div {
			data_123: "starts with number",
			"Content"
		}
	});

	// Error: data attribute cannot have uppercase letters
	let _another_invalid = page!(|| {
		div {
			data_TestId: "has uppercase",
			"Content"
		}
	});
}
