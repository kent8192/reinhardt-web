//! page! macro with boolean attributes using conditional expressions

use reinhardt_pages::page;

fn main() {
	let condition = true;
	let count = 5;

	let _valid = page!(|| {
		div {
			button {
				disabled: if condition { "disabled" } else { "" },
				"Submit"
			}
			input {
				r#type: "checkbox",
				checked: if count>3 { "checked" } else { "" },
			}
			input {
				r#type: "text",
				required: if count<10 &&condition { "required" } else { "" },
			}
		}
	});
}
