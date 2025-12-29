//! page! macro with nested interactive elements
//!
//! This test verifies that interactive elements (button, a, label, select, textarea)
//! cannot be nested inside other interactive elements.

// reinhardt-fmt: ignore-all

use reinhardt_pages::page;

fn main() {
	// Error: Interactive element <button> cannot be nested inside another interactive element <a>
	let _invalid = page!(|| {
		a {
			href: "/link",
			button {
				@click: | _ | { },
				"Click me"
			}
		}
	});

	// Another example: button inside button
	let _also_invalid = page!(|| {
		button {
			@click: | _ | { },
			button {
				@click: | _ | { },
				"Inner button"
			}
		}
	});

	// Yet another: label inside select
	let _another_invalid = page!(|| {
		select {
			label {
				"Option label"
			}
		}
	});
}
