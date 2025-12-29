//! Unknown special macro - should fail

// reinhardt-fmt: ignore-all

use reinhardt_pages::page;

fn main() {
	// unknown_macro! is not a valid special macro
	let _invalid = page!(|| {
		img {
			src: unknown_macro!("test.png"),
			alt: "Test"
		}
	});
}
