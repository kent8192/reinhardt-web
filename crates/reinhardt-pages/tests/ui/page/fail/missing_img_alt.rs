//! page! macro with img element missing alt attribute
//!
//! This test verifies that img elements must have an alt attribute
//! for accessibility purposes (WCAG compliance).

// reinhardt-fmt: ignore-all

use reinhardt_pages::page;

fn main() {
	// Error: Element <img> requires 'alt' attribute for accessibility
	let _invalid = page!(|| {
		img {
			src: "/image.jpg",
		}
	});

	// Another example with multiple img elements
	let _also_invalid = page!(|| {
		div {
			img {
				src: "/image1.jpg",
				class: "photo",
			}
			img {
				src: "/image2.jpg",
			}
		}
	});
}
