//! page! macro with nested elements

use reinhardt_pages::page;

fn main() {
	// Nested elements
	let _nested = page!(|| {
		div {
			class: "container",
			header {
				h1 {
					"Title"
				}
			}
			main {
				p {
					"Content paragraph 1"
				}
				p {
					"Content paragraph 2"
				}
			}
			footer {
				span {
					"Footer text"
				}
			}
		}
	});
}
