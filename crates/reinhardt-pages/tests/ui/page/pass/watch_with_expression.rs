//! Test: Watch block with expression node
//!
//! Validates that watch blocks can contain expression nodes
//! that evaluate to dynamic content.

use reinhardt_pages::Signal;
use reinhardt_pages::page;

fn main() {
	// Watch with expression node
	let _with_expr = __reinhardt_placeholder__!(/*0*/);

	// Watch with expression in element
	let _expr_in_element = __reinhardt_placeholder__!(/*1*/);

	// Watch with conditional expression
	let _conditional_expr = __reinhardt_placeholder__!(/*2*/);
}
