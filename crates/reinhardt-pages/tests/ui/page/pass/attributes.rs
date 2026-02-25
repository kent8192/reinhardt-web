//! page! macro with various attributes

use reinhardt_pages::page;

fn main() {
	// Basic attributes
	let _with_attrs = page!(|| {
		div {
			class: "container",
			id: "main-content",
			"Hello"
		}
	});

	// Data attributes (underscore to hyphen conversion)
	let _data_attrs = page!(|| {
		div {
			data_testid: "test-element",
			data_value: "42",
			"Data attributes"
		}
	});

	// ARIA attributes
	let _aria_attrs = page!(|| {
		button {
			aria_label: "Close",
			aria_expanded: "false",
			"×"
		}
	});
}
