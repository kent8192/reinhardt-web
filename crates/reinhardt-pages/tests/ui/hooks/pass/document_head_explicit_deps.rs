//! Compile-pass: retained document-head hooks require explicit dependencies.

use reinhardt_pages::reactive::Signal;
use reinhardt_pages::reactive::hooks;
use reinhardt_pages::{Head, deps, page};

fn main() {
	let _ = page!(|title: Signal<String>| {
		div { {
			hooks::use_head(|| Head::new().meta_description("description"), deps![]);
			hooks::use_page_title({
				let title = title.clone();
				move || title.get()
			}, deps![title.clone()]);
			"document head"
		} }
	});
}
