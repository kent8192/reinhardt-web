//! page! macro with deep element nesting
//!
//! This test verifies that the page! macro can handle deeply nested elements
//! without issues (boundary value testing for nesting depth).

use reinhardt_pages::page;

fn main() {
	// 10 levels of nesting
	let _deep_nesting_10 = __reinhardt_placeholder__!(/*0*/);

	// 15 levels with mixed elements
	let _deep_nesting_15 = __reinhardt_placeholder__!(/*1*/);

	// 20 levels (extreme case)
	let _extreme_nesting = __reinhardt_placeholder__!(/*2*/);
}
