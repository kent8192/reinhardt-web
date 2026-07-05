//! UI compile-pass test for `#head` with direct `page!({ ... })` form.

use reinhardt_pages::component::Page;
use reinhardt_pages::{head, page};

fn main() {
	let title = "Dashboard".to_string();
	let page_head = head!(|| {
		title { "Dashboard" }
	});

	let view: Page = page!(#head: page_head, {
		main {
			h1 { { title.clone() } }
		}
	});

	let _ = view;
}
