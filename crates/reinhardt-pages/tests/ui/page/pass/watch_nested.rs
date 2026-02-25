//! Test: Nested watch blocks
//!
//! Validates that watch blocks can be nested within other elements
//! and multiple watch blocks can exist within the same parent.

use reinhardt_pages::Signal;
use reinhardt_pages::page;

fn main() {
	// Watch nested inside multiple elements
	let _nested_in_element = __reinhardt_placeholder__!(/*0*/);

	// Multiple watch blocks in same parent
	let _multiple_watches = __reinhardt_placeholder__!(/*1*/);
}
