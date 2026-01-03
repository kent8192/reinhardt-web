//! page! macro with boolean attributes using function calls

use reinhardt_pages::page;

fn is_button_disabled() -> &'static str {
	"disabled"
}

fn calculate_checked(value: i32) -> &'static str {
	if value > 0 { "checked" } else { "" }
}

fn main() {
	let _valid = page!(|| {
		div {
			button {
				disabled: is_button_disabled(),
				"Submit"
			}
			input {
				r#type: "checkbox",
				checked: calculate_checked(5),
			}
		}
	});
}
