//! page! macro with void elements (self-closing HTML elements)

use reinhardt_pages::page;

fn main() {
	// Input element (void)
	let _input = page!(|| {
		input {
			class: "text-input",
		}
	});

	// Break element (void)
	let _br = page!(|| {
		div {
			span {
				"Line 1"
			}
			br {}
			span {
				"Line 2"
			}
		}
	});

	// Image element (void)
	let _img = page!(|| {
		img {
			src: "/logo.png",
			class: "logo-image",
			alt: "Logo",
		}
	});

	// Horizontal rule (void)
	let _hr = page!(|| {
		div {
			p {
				"Section 1"
			}
			hr {}
			p {
				"Section 2"
			}
		}
	});
}
