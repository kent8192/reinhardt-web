//! EP-004: Missing fields section error.
//!
//! Tests that a form without a fields section produces a compile error.

use reinhardt_forms_macros::form;

fn main() {
	let _form = form! {
		name: "test_form",
	};
}
