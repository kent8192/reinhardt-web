//! page! macro with props (closure parameters)

use reinhardt_pages::page;

fn main() {
	// Single prop
	let _greeting = __reinhardt_placeholder__!(/*0*/);

	// Multiple props
	let _user_card = __reinhardt_placeholder__!(/*1*/);

	// Props with trailing comma
	let _trailing = __reinhardt_placeholder__!(/*2*/);
}
