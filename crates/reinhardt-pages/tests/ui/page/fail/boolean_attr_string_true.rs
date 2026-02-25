//! page! macro with boolean attribute having string literal "true"
//!
//! This test verifies that boolean attributes cannot have string literal values.

use reinhardt_pages::page;

fn main() {
	// Error: Boolean attribute 'disabled' cannot have a string literal value
	let _invalid = __reinhardt_placeholder__!(/*0*/);
}
