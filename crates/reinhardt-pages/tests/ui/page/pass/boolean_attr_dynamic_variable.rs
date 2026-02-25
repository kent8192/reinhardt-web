//! page! macro with boolean attributes using variables

use reinhardt_pages::page;

fn main() {
	let is_disabled = true;
	let is_checked = false;
	let is_readonly = true;

	let _valid = page!(|| {
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
	});
}
