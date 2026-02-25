//! page! macro with mixed child types (text, elements, expressions)

use reinhardt_pages::page;

fn main() {
	// Text and elements mixed
	let _mixed = __reinhardt_placeholder__!(/*0*/);

	// Nested mixed content with conditional
	let _nested_mixed = __reinhardt_placeholder__!(/*1*/);

	// Elements with expressions
	let _with_expr = __reinhardt_placeholder__!(/*2*/);
}
