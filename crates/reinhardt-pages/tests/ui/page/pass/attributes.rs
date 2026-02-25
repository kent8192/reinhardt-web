//! page! macro with various attributes

use reinhardt_pages::page;

fn main() {
	// Basic attributes
	let _with_attrs = __reinhardt_placeholder__!(/*0*/);

	// Data attributes (underscore to hyphen conversion)
	let _data_attrs = __reinhardt_placeholder__!(/*1*/);

	// ARIA attributes
	let _aria_attrs = __reinhardt_placeholder__!(/*2*/);
}
