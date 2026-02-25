//! page! macro with elements having many attributes
//!
//! This test verifies that the page! macro can handle elements with
//! a large number of attributes (boundary value testing for attribute count).

use reinhardt_pages::page;

fn main() {
	// Element with 20+ standard attributes
	let _many_attrs = __reinhardt_placeholder__!(/*0*/);

	// Input with 25+ attributes
	let _complex_input = __reinhardt_placeholder__!(/*1*/);

	// Button with 30+ attributes and events
	let _complex_button = __reinhardt_placeholder__!(/*2*/);

	// Form with many attributes
	let _complex_form = __reinhardt_placeholder__!(/*3*/);

	// Link with many attributes
	let _complex_link = __reinhardt_placeholder__!(/*4*/);

	// Image with all recommended attributes
	let _complex_img = __reinhardt_placeholder__!(/*5*/);
}
