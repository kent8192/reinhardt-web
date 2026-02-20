//! page! macro with if/else if/else chains

use reinhardt_pages::page;

fn main() {
	// Simple else if chain
	let _else_if = page!(|status: i32| {
		div {
			if status == 0 {
				span {
					"Pending"
				}
			} else if status == 1 {
				span {
					"Processing"
				}
			} else if status == 2 {
				span {
					"Complete"
				}
			} else {
				span {
					"Unknown"
				}
			}
		}
	});

	// Multiple else if branches
	let _grade = page!(|score: i32| {
		div {
			class: "grade",
			if score >= 90 {
				span {
					class: "a",
					"A"
				}
			} else if score >= 80 {
				span {
					class: "b",
					"B"
				}
			} else if score >= 70 {
				span {
					class: "c",
					"C"
				}
			} else if score >= 60 {
				span {
					class: "d",
					"D"
				}
			} else {
				span {
					class: "f",
					"F"
				}
			}
		}
	});

	// Else if with complex conditions
	let _complex = page!(|a: bool, b: bool| {
		div {
			if a &&b {
				span {
					"Both true"
				}
			} else if a {
				span {
					"Only A"
				}
			} else if b {
				span {
					"Only B"
				}
			} else {
				span {
					"Neither"
				}
			}
		}
	});
}
