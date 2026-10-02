//! page! macro with mixed standalone and key-value boolean attributes.
//!
//! Spec §3.7 (no implicit captures): outer bindings must be declared as
//! explicit closure parameters.

use reinhardt_pages::page;

fn main() {
	let is_readonly = true;

	let _valid = page!(|is_readonly: bool| {
		div {
			input {
				r#type: "text",
				aria_label: "Required field",
				required: true,
				class: "form-input",
				readonly: is_readonly,
			}
		}
	})(is_readonly);
}
