//! page! macro with img element having empty src attribute
//!
//! This test verifies that img src attribute must not be empty.

// reinhardt-fmt: ignore-all

use reinhardt_pages::page;

fn main() {
	// Error: Element <img> 'src' attribute must not be empty
	let _invalid = page!(|| {
		img {
			src: "",
			alt: "A photo",
		}
	});

	// Another example with whitespace-only src
	let _also_invalid = page!(|| {
		img {
			src: "   ",
			alt: "Another photo",
		}
	});
}
