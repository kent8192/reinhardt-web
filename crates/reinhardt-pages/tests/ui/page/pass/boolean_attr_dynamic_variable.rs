//! page! macro with boolean attributes using variables.
//!
//! Spec §3.7 (no implicit captures): outer bindings must be declared as
//! explicit closure parameters.

use reinhardt_pages::page;

fn main() {
	let is_disabled = true;
	let is_checked = false;
	let is_readonly = true;

	let _valid = page!(|is_disabled: bool, is_checked: bool, is_readonly: bool| {
		div {
			button {
				disabled: is_disabled,
				"Submit"
			}
			input {
				r#type: "checkbox",
				checked: is_checked,
			}
			input {
				r#type: "text",
				readonly: is_readonly,
			}
		}
	})(is_disabled, is_checked, is_readonly);
}
