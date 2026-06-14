//! page! macro with boolean attributes using conditional expressions.
//!
//! Spec §3.7 (no implicit captures): outer bindings must be declared as
//! explicit closure parameters.

use reinhardt_pages::page;

fn main() {
	let condition = true;
	let count = 5;

	let _valid = page!(|condition: bool, count: i32| {
		div {
			button {
				disabled: condition,
				"Submit"
			}
			input {
				r#type: "checkbox",
				checked: count> 3,
			}
			input {
				r#type: "text",
				required: count<10 &&condition,
			}
		}
	})(condition, count);
}
