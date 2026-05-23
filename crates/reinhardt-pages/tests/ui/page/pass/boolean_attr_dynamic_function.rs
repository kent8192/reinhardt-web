//! page! macro with boolean attributes using function calls.
//!
//! Spec §3.7 (no implicit captures): single-segment lowercase idents are
//! treated as value bindings, so free functions must either be passed in as
//! parameters or referenced via a multi-segment path. Here we use the
//! `self::` prefix to make the path multi-segment.

use reinhardt_pages::page;

fn is_button_disabled() -> bool {
	true
}

fn calculate_checked(value: i32) -> bool {
	value > 0
}

fn main() {
	let _valid = page!(|| {
		div {
			button {
				disabled: self::is_button_disabled(),
				"Submit"
			}
			input {
				r#type: "checkbox",
				checked: self::calculate_checked(5),
			}
		}
	});
}
