//! page! macro with img element missing src attribute
//!
//! This test verifies that img elements must have a src attribute.

// reinhardt-fmt: ignore-all

use reinhardt_pages::page;

fn main() {
	// Error: Element <img> requires 'src' attribute
	let _invalid = page!(|| {
		img {
			alt: "A photo",
		}
	});

	// Another example with class but no src
	let _also_invalid = page!(|| {
		div {
			img {
				alt: "Another photo",
				class: "photo",
			}
		}
	});
}
