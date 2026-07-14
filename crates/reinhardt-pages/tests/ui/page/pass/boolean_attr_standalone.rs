//! page! macro with standalone boolean attributes

use reinhardt_pages::page;

fn main() {
	let _valid = page!(|| {
		div {
			input {
				r#type: "text",
				aria_label: "Required input",
				required: true,
			}
			button {
				disabled: true,
				"Submit"
			}
			select {
				aria_label: "Options",
				multiple: true,
				option { "A" }
				option { "B" }
			}
		}
	});
}
