//! page! macro with empty element bodies

use reinhardt_pages::page;

fn main() {
	// Empty div
	let _empty_div = __reinhardt_placeholder__!(/*0*/);

	// Empty span with attribute only
	let _empty_span = __reinhardt_placeholder__!(/*1*/);

	// Nested empty elements
	let _nested_empty = __reinhardt_placeholder__!(/*2*/);

	// Empty element in condition
	let _conditional_empty = __reinhardt_placeholder__!(/*3*/);

	// Empty element in loop
	let _loop_empty = __reinhardt_placeholder__!(/*4*/);
}
