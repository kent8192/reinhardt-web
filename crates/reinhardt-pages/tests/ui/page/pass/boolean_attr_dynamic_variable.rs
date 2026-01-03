//! page! macro with boolean attributes using variables

use reinhardt_pages::page;

fn main() {
	let is_disabled = "disabled";
	let is_checked = "";
	let is_readonly = "readonly";

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
