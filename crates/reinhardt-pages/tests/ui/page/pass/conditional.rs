//! page! macro with conditional rendering (if/else)

use reinhardt_pages::page;

fn main() {
	// Simple if
	let _with_if = page!(|show: bool| {
		div {
			if show {
				span {
					"Visible"
				}
			}
		}
	});

	// If/else
	let _if_else = page!(|is_admin: bool| {
		div {
			if is_admin {
				span {
					class: "badge",
					"Admin"
				}
			} else {
				span {
					"User"
				}
			}
		}
	});

	// Nested if
	let _nested_if = page!(|a: bool, b: bool| {
		div {
			if a {
				if b {
					span {
						"Both true"
					}
				}
			}
		}
	});
}
