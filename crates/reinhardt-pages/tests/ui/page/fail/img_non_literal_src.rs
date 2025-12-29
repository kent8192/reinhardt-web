//! page! macro with img element having non-literal src attribute
//!
//! This test verifies that img src attribute must be a string literal.

// reinhardt-fmt: ignore-all

use reinhardt_pages::page;

fn main() {
	let image_url = "/image.jpg".to_string();

	// Error: Element <img> 'src' attribute must be a string literal
	let _invalid = page!(|| {
		img {
			src: image_url.clone(),
			alt: "A photo",
		}
	});

	// Another example with a function call
	let _also_invalid = page!(|| {
		img {
			src: get_image_url(),
			alt: "Another photo",
		}
	});
}

fn get_image_url() -> String {
	"/image.jpg".to_string()
}
